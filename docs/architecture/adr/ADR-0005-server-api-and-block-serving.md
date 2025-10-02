# ADR-0005: Server API and block serving

## Metadata
- ID: ADR-0005
- Title: Server API and block serving
- Status: Proposed
- Date: 2025-10-02
- Deciders: RFS core maintainers
- Tags: server, api, blocks, content-addressed, auth, observability
- Supersedes: None
- Superseded by: None

## Context
The server provides a stable HTTP surface within the rfs ecosystem to:
- Serve content-addressed blocks and file/list metadata to support pack exist/upload paths and mount lazy reads.
- Optionally serve flist downloads, a simple website/static content feature, and a web UI that consumes the same API.

Constraints and goals:
- Authentication/authorization: token-based (JWT) as described in [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md); endpoints other than login require auth unless explicitly public.
- Public endpoint stability: once published, URIs and semantics should remain backward compatible; deprecations follow ADR lifecycle.
- Content-addressing stability: block identity and addressing must remain backward compatible with pack outputs in [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md).
- Rate limiting/throttling: protect the service and backends from abuse while allowing high-throughput batch operations.

## Decision
Establish and document a content-addressed HTTP API for blocks and flists with token-based authentication, consistent error mapping, and delegation to configured storage backends.

- API surface — Current: Provide endpoints for blocks (GET to download, HEAD to exist-check, POST/PUT to upload), flist retrieval/management, authentication, and auxiliary features (e.g., website publishing). Handlers are implemented under [src/server/handlers.rs](src/server/handlers.rs), [src/server/block_handlers.rs](src/server/block_handlers.rs), [src/server/file_handlers.rs](src/server/file_handlers.rs), [src/server/serve_flists.rs](src/server/serve_flists.rs), and [src/server/website_handlers.rs](src/server/website_handlers.rs). See [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md) for examples.
- Content-addressing — Current: Blocks are addressed by cryptographic hash consistent with the pack pipeline described in [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md).
- Error mapping — Current: Backend and storage errors are translated to HTTP responses via [src/server/response.rs](src/server/response.rs) for consistent client behavior.
- Auth — Current/Proposed: JWT-based login and token validation as referenced in [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md) and implemented in [src/server/auth.rs](src/server/auth.rs). Token scopes/permissions beyond user identity are Proposed and may be expanded.
- Storage coupling — Current: Requests delegate to configured storage via [src/store/server.rs](src/store/server.rs) and backend adapters, following routing policies in [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md).
- Observability — Proposed: Emit structured logs and metrics per endpoint (latency, throughput, error rates) and per-backend, with labels for method, status, and handler.

## Rationale
- Provide a stable, content-addressed block service decoupled from clients so pack and mount can evolve independently.
- Enable efficient existence checks (HEAD) to minimize uploads and bandwidth.
- Secure uploads and controlled distribution of flists and blocks using a unified auth model.
- Align with lazy-loading requirements by supporting random-access block retrieval with predictable semantics.

## Alternatives Considered
- Direct client-to-backend access without a server — reduces moving parts but loses unified auth, routing, error mapping, and observability.
- Peer-to-peer block exchange — attractive for distribution but adds complexity, NAT traversal, and consistency concerns.
- Embedding blocks inside flists — simplifies transport but eliminates lazy fetch and dedup benefits; flists become large.
- gRPC instead of HTTP/REST — richer contracts but less friction for browsers/caches; HTTP remains broadly compatible.
- Filesystem gateway only without API — insufficient for non-FUSE clients and automation pipelines.

## Consequences
Positive:
- Consistent API across clients and environments.
- Unified authentication and authorization.
- Storage routing abstraction and portability across backends.
- Centralized observability and policy enforcement.

Negative:
- Operational overhead to run and scale the server.
- Throughput and latency SLOs must be maintained; adds another hop between clients and storage.
- Versioning and compatibility stewardship for public endpoints.

## Implementation Notes
- Handlers: See [src/server/block_handlers.rs](src/server/block_handlers.rs), [src/server/file_handlers.rs](src/server/file_handlers.rs), and the entry wiring in [src/server/handlers.rs](src/server/handlers.rs) and [src/server/mod.rs](src/server/mod.rs).
- Configuration: Server options and storage/router configuration are loaded via [src/server/config.rs](src/server/config.rs); see user guidance in [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md).
- Storage delegation: Requests are dispatched to storage via [src/store/server.rs](src/store/server.rs) and backend implementations; policy aligns with [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md).
- Data model: Types are defined under [src/server/models/](src/server/models/) including [src/server/models/block.rs](src/server/models/block.rs), [src/server/models/file.rs](src/server/models/file.rs), and [src/server/models/user.rs](src/server/models/user.rs).
- Persistence: Database and mapping layers live under [src/server/db/](src/server/db/) including [src/server/db/sqlite.rs](src/server/db/sqlite.rs), [src/server/db/map.rs](src/server/db/map.rs), and [src/server/db/storage.rs](src/server/db/storage.rs).
- Keep specifics high-level: Do not rely on unverified URL shapes or payload schemas beyond what is documented in [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md).

## Security and Privacy
- Token handling: Issue and validate JWTs; store secrets securely; enforce expiration and optional rotation. Consider scopes/permissions for write vs read — Proposed.
- Transport security: Prefer TLS termination; support secure cookies/headers for the web UI.
- Enumeration resistance: Use HEAD for existence but apply rate limiting and uniform error mapping to avoid user/content enumeration.
- Abuse prevention: Rate limit, throttle, and apply backpressure per client/IP/token; consider request signing for uploads — Proposed.

## Operational Considerations
- Deployment: Run as a stateless service where possible; externalize storage and database dependencies.
- Configuration management: Manage via TOML as shown in [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md); validate at startup and on reload — Proposed.
- Health checks: Provide readiness/liveness endpoints; include dependency checks to storage/backends — Proposed.
- Scaling: Horizontal scaling behind a load balancer; ensure idempotent handlers and shared backing stores.
- Runbooks and SLOs: Define alerting on error rates, latency, and saturation; document remediation steps — Proposed.

## Performance and Scaling
- Request mix: HEAD exist checks are latency-sensitive; GET transfers are throughput-heavy. Tune thread pools and IO accordingly.
- Concurrency: Support parallel block operations; bound concurrency to protect backends and avoid tail latency.
- Caching: Allow HTTP caches/CDNs for GET of immutable blocks; validate caching semantics for HEAD and auth — Proposed.
- Backpressure: Apply fair queuing and server-side throttling when queues grow; propagate retry-after where appropriate — Proposed.

## Open Questions
- API versioning strategy and deprecation policy.
- Compatibility contracts with diverse clients (CLI versions, mount, third-party tools).
- Multi-region deployments and read-locality/failover strategies.
- Audit logging requirements and retention.

## References
- [docs/user-guides/fl-server.md](docs/user-guides/fl-server.md)
- [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md)
- [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md)
- [docs/call-stacks/pack.md](docs/call-stacks/pack.md)
- [src/server/mod.rs](src/server/mod.rs)
- [src/server/handlers.rs](src/server/handlers.rs)
- [src/server/block_handlers.rs](src/server/block_handlers.rs)
- [src/server/file_handlers.rs](src/server/file_handlers.rs)
- [src/server/serve_flists.rs](src/server/serve_flists.rs)
- [src/server/website_handlers.rs](src/server/website_handlers.rs)
- [src/server/config.rs](src/server/config.rs)
- [src/server/response.rs](src/server/response.rs)
- [src/server/db/mod.rs](src/server/db/mod.rs)
- [src/server/db/sqlite.rs](src/server/db/sqlite.rs)
- [src/server/db/storage.rs](src/server/db/storage.rs)
- [src/server/db/map.rs](src/server/db/map.rs)
- [src/server/models/mod.rs](src/server/models/mod.rs)
- [src/server/models/block.rs](src/server/models/block.rs)
- [src/server/models/file.rs](src/server/models/file.rs)
- [src/server/models/user.rs](src/server/models/user.rs)
- [src/store/server.rs](src/store/server.rs)

## Changelog
- 2025-10-02 — Created