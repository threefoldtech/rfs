# ADR-0006: S3 preflight auth, client pooling, and retry/backoff

ID: ADR-0006
Title: S3 preflight auth, client pooling, and retry/backoff
Status: Proposed
Date: 2025-10-02
Deciders: Core maintainers
Tags: s3, pack, reliability, error-handling
Supersedes:
Superseded by:

## Context
Packaging to S3 currently initializes an S3 client and attempts uploads per file and per block. When credentials are invalid, each block/upload attempts and fails, prolonging runtime. When connectivity is flaky, uploads fail individually without a unified retry policy. This ADR proposes: (1) a single preflight authentication/authorization check before the pack loop, (2) client pooling so the same Bucket/client is reused, and (3) error classification with retry/backoff for transient failures, with fail-fast on authentication errors.

Relevant code:
- [src/store/s3store.rs](src/store/s3store.rs)
- [src/store/mod.rs](src/store/mod.rs)
- [src/store/router.rs](src/store/router.rs)
- [src/pack.rs](src/pack.rs)
- [src/upload.rs](src/upload.rs)

Constraints/goals:
- POSIX-like semantics for uploads not required, but correctness and deterministic flist output are.
- Abort quickly on authentication/authorization failures (no repeated attempts per file).
- Retry transient network/storage errors with bounded attempts and backoff.
- Keep changes localized and additive (avoid breaking the Store trait).

## Decision
We will introduce a preflight and retry layer around stores used by pack:

1. Preflight auth/connectivity check (once per store URL before pack):
   - For S3: perform a harmless operation (e.g., HEAD on a non-existent key) against the configured bucket via the existing Bucket client in [src/store/s3store.rs](src/store/s3store.rs). Treat 404 as success (connectivity and auth OK), 401/403 as authentication/authorization failure (fail-fast), and 5xx/IO/timeouts as transient connectivity errors.
   - For other backends: no-op or minimal checks; may be implemented incrementally.

2. Client pooling:
   - Ensure that for a given S3 URL, a single [src/store/s3store.rs](src/store/s3store.rs) instance (holding a Bucket) is created and reused across the entire pack run. The current code already constructs the store once per route. We will explicitly cache constructed Stores when parsing routes so repeated calls to [src/store/mod.rs](src/store/mod.rs) do not rebuild clients for the same URL.

3. Retry/backoff decorator for Store:
   - Introduce a RetryStore<T: Store> wrapper that implements Store and delegates get/set with error classification and exponential backoff. Pack will wrap its store in RetryStore so all set/get operations benefit transparently.

4. Error classification policy for S3:
   - Authentication error: HTTP 401/403 (S3Error variants mapping) → do not retry; mark fatal.
   - Transient connectivity: IO errors, HTTP 5xx, timeouts → retry with exponential backoff and jitter, capped attempts.
   - Not found (404) is only used during preflight on a random key and considered success; for get(), 404 maps to KeyNotFound as today.

5. Fail-fast behavior:
   - If preflight returns authentication failure for any configured S3 route, abort pack immediately with a clear error. Do not attempt per-file or per-block uploads.
   - If a retry-able transient error persists after max attempts, surface a single aggregated error and stop the pack (consistent with existing failure collection).

6. Configuration knobs (optional, defaults reasonable):
   - --retry-attempts (default 5)
   - --retry-backoff-ms (initial delay, default 250)
   - --retry-backoff-max-ms (cap, default 5000)
   - Can be surfaced later; code defaults must be safe even without flags.

## Rationale
- Preflight avoids repeating the same authentication failure N×files×blocks.
- Client pooling reuses established connections/TLS, lowers overhead.
- A lightweight RetryStore keeps the Store trait unchanged and applies policy consistently across upload paths.
- Explicit classification ensures we do not waste time retrying on permanent auth failures while still being resilient to transient network issues.

## Alternatives considered
- Implement retries directly in [src/pack.rs](src/pack.rs) uploader: couples policy to one caller, harder to reuse. Rejected in favor of a reusable wrapper.
- Extend the Store trait with preflight(): breaks existing implementations and requires broad changes. Rejected; instead define an optional StoreHealth trait and implement for types we control.
- Rely on backend-specific SDK automatic retries: opaque, inconsistent across backends, and often does not classify auth vs transient cleanly.

## Implementation notes (high-level)

1) Preflight health check
- Define a StoreHealth trait in [src/store/mod.rs](src/store/mod.rs):

  Trait sketch:
  - async fn preflight(&self) -> std::result::Result<(), PreflightError>;

  Supporting enum:
  - enum PreflightError { Auth(String), Connectivity(String), Other(String) }

- Implement StoreHealth for S3Store in [src/store/s3store.rs](src/store/s3store.rs):
  - Use bucket.head_object with a random/non-existent key, or get_object and treat 404 as success.
  - Map S3Error::HttpFailWithBody(401|403, _) → Auth, S3Error::Io(_) or 5xx → Connectivity, others → Other.

- Implement StoreHealth for Router<Stores> in [src/store/router.rs](src/store/router.rs):
  - Iterate underlying stores and call preflight() on each unique backend once (deduplicate by URL).
  - Run in parallel (join_all) and aggregate errors; return the first Auth error; otherwise if any Connectivity error exists, return Connectivity.

- Integration point in [src/pack.rs](src/pack.rs):
  - Before writing routes and spinning workers, call preflight() on the provided store (Router<Stores>), and abort early on PreflightError::Auth. For Connectivity, print a clear message and still allow retry logic to operate during upload, or abort depending on configuration.

2) Retry/backoff wrapper
- Add a RetryStore<T: Store> type in [src/store/mod.rs](src/store/mod.rs) (or a new module) that implements Store and wraps an inner T.
- On set()/get(), perform up to N attempts with exponential backoff and jitter. Abort immediately on classified auth errors (if inner type exposes classification) or on Error::InvalidKey.
- For S3Store specifically, convert underlying S3Error variants to Store::Error to preserve classification (map 401/403 to a distinct Store::Error::Unavailable or a new variant).

Pseudo-code:

```
// Retry policy (defaults)
const MAX_ATTEMPTS: usize = 5;
const BASE_DELAY_MS: u64 = 250;
const MAX_DELAY_MS: u64 = 5000;

async fn retry<F, T, E>(op: F) -> Result<T, E>
where F: Fn() -> Future<Output=Result<T, E>> {
    let mut attempt = 0;
    let mut delay = BASE_DELAY_MS;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if is_auth_error(&e) => return Err(e), // fail-fast
            Err(e) if is_transient(&e) && attempt + 1 < MAX_ATTEMPTS => {
                sleep(jitter(delay)).await;
                delay = (delay * 2).min(MAX_DELAY_MS);
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

- Wrap the store passed to pack: let store = RetryStore::new(store); then let store: BlockStore<_> = store.into();
- Uploader remains unchanged; it uses store.set(), now retried transparently.

3) Store caching (client pooling)
- In [src/store/mod.rs](src/store/mod.rs) parse_router(), cache Stores by URL in a HashMap<String, Stores> so multiple ranges referencing the same URL reuse the same S3Store instance (and therefore the same Bucket).
- This avoids redundant client instantiation if the same endpoint appears in multiple ranges.

4) Error messages and exit behavior
- On PreflightError::Auth: return an error from pack with a clear message “S3 auth failed for <endpoint>. Check credentials. Aborting before uploading any file.”
- On persistent transient failures after retries: surface a single aggregated error listing failing endpoints.

## Integration points summary
- [src/store/s3store.rs](src/store/s3store.rs): implement StoreHealth::preflight() with HEAD/get on non-existent key and variant mapping; optionally extend get/set to surface classified errors.
- [src/store/mod.rs](src/store/mod.rs): add StoreHealth trait, PreflightError; add RetryStore wrapper; add optional URL→Store cache in parse_router().
- [src/store/router.rs](src/store/router.rs): implement StoreHealth for Router<Stores> to preflight all underlying stores; deduplicate per URL; run in parallel.
- [src/pack.rs](src/pack.rs): call preflight() before route writing/worker pool; wrap incoming store with RetryStore prior to BlockStore conversion.
- [src/upload.rs](src/upload.rs): no change for server-based uploads; policy remains at the server boundary. If client-side stores are used there in the future, the same RetryStore can be applied.

## Security and privacy
- Fail-fast on auth errors reduces credential spray against the backend.
- No secrets are written to flists; existing redaction remains (see [src/pack.rs](src/pack.rs)).
- Ensure logs do not include secrets; only endpoints and bucket names.

## Operational considerations
- Provide counters/metrics: preflight success/failure, retry counts, final failures per backend.
- Configurable retry/backoff can be exposed via CLI; defaults suffice if flags are omitted.
- Preflight can be optionally disabled for advanced scenarios (future flag).

## Performance and scaling
- Preflight is O(number of stores); negligible compared to full upload.
- Retry adds latency only on failures; success path unchanged. Jitter avoids thundering herd on shared backends.
- Client pooling reuses connections and reduces TLS handshakes.

## Open questions
- How to represent “auth error” uniformly in Store::Error without widening the enum? Option A: map to Unavailable; Option B: introduce a new variant Auth; Option C: carry anyhow::Error with a marker type. Proposed: introduce Auth variant.
- Exact S3 preflight operation (HEAD non-existent key vs List bucket) — choose the least-privileged call supported across deployments.
- Expose retry knobs via CLI now or later?

## References
- [src/store/s3store.rs](src/store/s3store.rs)
- [src/store/mod.rs](src/store/mod.rs)
- [src/store/router.rs](src/store/router.rs)
- [src/pack.rs](src/pack.rs)
- [src/upload.rs](src/upload.rs)
- [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md)
- [docs/architecture/adr/ADR-0002-fuse-lazy-loading.md](docs/architecture/adr/ADR-0002-fuse-lazy-loading.md)
- [docs/architecture/adr/ADR-0003-cache-strategy.md](docs/architecture/adr/ADR-0003-cache-strategy.md)
- [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md)
- [docs/architecture/adr/ADR-0005-server-api-and-block-serving.md](docs/architecture/adr/ADR-0005-server-api-and-block-serving.md)

## Changelog
- 2025-10-02: Initial proposal.