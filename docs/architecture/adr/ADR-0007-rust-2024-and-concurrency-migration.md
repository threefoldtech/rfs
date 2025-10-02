# ADR-0007: Rust 2024 edition, concurrency refactor (semaphores), and polyfuse upstream evaluation

ID: ADR-0007  
Title: Rust 2024 edition, concurrency refactor (semaphores), and polyfuse upstream evaluation  
Status: Proposed  
Date: 2025-10-02  
Deciders: Core maintainers  
Tags: edition-2024, concurrency, tokio, polyfuse, maintenance  
Supersedes:  
Superseded by:

## Context

We want to:
- Migrate the project to Rust Edition 2024 to align with modern language/lint defaults, reduce deprecated patterns, and prepare for newer ecosystem crates.
- Remove reliance on external worker-pool crate and replace it with idiomatic Tokio primitives (Semaphore + JoinSet/JoinAll), simplifying concurrency and improving clarity.
- Evaluate and, if feasible, move off the forked polyfuse dependency (muhamadazmy fork) back to upstream (ubnt-intrepid/polyfuse) or adopt a different approach, while keeping our FUSE path correct and stable.
- Run `cargo update --verbose` to refresh dependencies, then triage and fix any breakages under the above modernization.

Current code touchpoints:
- Worker pool use: [src/pack.rs](src/pack.rs), [src/unpack.rs](src/unpack.rs), [src/clone.rs](src/clone.rs) rely on [workers::WorkerPool](src/pack.rs:13) to parallelize uploads/downloads/clones.
- FUSE stack use: [src/fs/mod.rs](src/fs/mod.rs) depends on polyfuse types (op/reply/attrs) and Async Session helpers.
- Cargo config:
  - Polyfuse fork: [Cargo.toml](Cargo.toml)
  - Worker pool: [workers dependency](Cargo.toml)

## Decision

1) Adopt Rust Edition 2024
- Set `edition = "2024"` in [Cargo.toml](Cargo.toml). Leverage edition lints to catch legacy patterns and simplify code (e.g., trait object syntax, precise capturing, macro hygiene improvements). No functional changes expected; compiler errors/warnings will guide small fixes.

2) Replace external worker pool with Tokio semaphores
- Remove the workers crate. Replace `WorkerPool`-based fans with:
  - `tokio::sync::Semaphore` to bound concurrency.
  - `tokio::task::JoinSet` (or `futures::future::join_all`) to track/await tasks.
- Benefits: fewer dependencies, explicit error handling and backpressure, idiomatic concurrency aligned with Tokio.

3) Differentiate and evaluate polyfuse fork vs upstream
- Keep muhamadazmy’s fork initially for stability, but document and test the delta with upstream polyfuse.
- Build a short-lived integration branch swapping [dependencies.polyfuse](Cargo.toml) source to the upstream and run `cargo update --verbose` and `cargo build` to capture deltas.
- If deltas are limited to minor API changes in mount/session wiring, consider migration; otherwise justify keeping the fork or explore alternatives (e.g., fuse3 crate).

4) Dependency refresh and triage path
- Run `cargo update --verbose` to pick up latest patch/minor updates respecting Cargo.lock constraints.
- Fix breakages in a controlled order: edition/lints → worker removal refactor → polyfuse API differences.
- Gate changes behind CI and keep the FUSE/mount e2e tests (or manual tests) green before merging.

## Rationale

- Edition upgrades reduce tech debt and unlock modern lints and patterns.
- Semaphore + JoinSet pattern is widely understood, supported, and removes an extra dependency, simplifying debugging and maintenance.
- Upstream alignment reduces maintenance risks of relying on a fork; if upstream doesn’t fit our needs, documenting the delta sharpens our long-term plan (contributing patches upstream or selecting another crate).
- Refreshing dependencies with a strategy ensures we catch and address issues while context is fresh.

## Alternatives considered

- Keep `tokio-worker-pool`: avoids churn but retains niche dependency and implicit scheduling. Rejected.
- Migrate FUSE crate immediately to a different ecosystem (e.g., fuse3): too risky without first quantifying our current fork deltas. Considered as a follow-up if upstream polyfuse is not a fit.
- Delay edition bump until after concurrency refactor: either order is viable; we choose to adjust Cargo edition early to surface lints sooner.

## Implementation plan (stepwise)

A. Edition 2024
- Change Cargo metadata:
  - Set edition in [Cargo.toml](Cargo.toml).
- Run `cargo check` and fix edition-related warnings/errors.

B. Concurrency refactor (remove worker pool)
- Remove workers dependency in [Cargo.toml](Cargo.toml).
- Replace the following callsites:
  - Pack:
    - WorkerPool usage at [workers::WorkerPool](src/pack.rs:13) and pool construction at [workers::WorkerPool::new](src/pack.rs:68).
    - Worker usage (send scheduling) within [pack_one](src/pack.rs:181) and worker runner [impl workers::Work for Uploader](src/pack.rs:253).
  - Unpack:
    - Pool types and scheduling references: [workers::WorkerPool](src/unpack.rs:15), function params using pool ([src/unpack.rs](src/unpack.rs:131)).
  - Clone:
    - Pool creation [workers::WorkerPool::new](src/clone.rs:17).
- Pseudo-code replacement for pack (skeleton):

```
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

let semaphore = Arc::new(Semaphore::new(PARALLEL_UPLOAD));
let mut join_set = JoinSet::new();

for file in files_to_upload {
    let permit = semaphore.clone().acquire_owned().await?;
    let store = store.clone();
    let writer = writer.clone();
    join_set.spawn(async move {
        let _p = permit;
        // perform chunked read + store.set() + writer.block() same as Uploader::upload
        // return Result<()>
    });
}

// Drain tasks
while let Some(res) = join_set.join_next().await {
    res??; // propagate errors
}
```

- Keep the existing per-block logic (chunk size, writer.block, etc.) unchanged for now. Only the scheduling/backpressure mechanism changes.

C. Polyfuse fork differentiation
- Current fork pin (from lockfile) indicates a custom commit; we use:
  - polyfuse fork: [Cargo.toml polyfuse section](Cargo.toml:90)
  - Used API in [src/fs/mod.rs](src/fs/mod.rs):
    - `use polyfuse::reply::FileAttr;`
    - `use polyfuse::op;`
    - Session/mount wiring in the Filesystem mount path (e.g., AsyncSession + next_request loop).
- Comparison method:
  1. Create a feature flag or temporary branch to point polyfuse to upstream:
     - Change [dependencies.polyfuse](Cargo.toml:90) to upstream `git = "https://github.com/ubnt-intrepid/polyfuse"` (and tag/branch if needed).
  2. `cargo update --verbose` to fetch upstream and rebuild.
  3. Capture compiler errors in [src/fs/mod.rs](src/fs/mod.rs) around op/reply/session types; adjust module paths/type names if they changed.
  4. If Async session glue differs, decide whether to:
     - Recreate our async wrapper (using `AsyncFd`), or
     - Use upstream recommended async approach if available.
  5. If the delta is large or breaks FUSE semantics we rely on, park the upstream migration and record a justification for staying on the fork.

D. Dependency refresh procedure
- Commands to run (locally/CI):
  - `rustup toolchain update`
  - `cargo update --verbose`
  - `cargo build -Zunstable-options` (optional flags if needed for diagnostics)
  - Address breakages in the following priority:
    1) Edition warnings/errors (fix imports, trait bounds, explicit lifetimes).
    2) Concurrency refactor compile errors (remove worker pool types).
    3) Polyfuse API adjustments (only on the test branch; don’t block Step B).
- Add or update CI to run the full matrix:
  - Linux with FUSE2/FUSE3 present.
  - x86_64-unknown-linux-gnu and musl targets if we ship static.

## Risk and mitigation

- Risk: Upstream polyfuse may have diverged significantly from the fork.
  - Mitigation: Contain investigation to a branch; keep current fork on main; document gaps.
- Risk: Concurrency refactor could alter throughput or error aggregation semantics.
  - Mitigation: Maintain same concurrency bounds, reuse block logic, add integration tests for pack/unpack flows.
- Risk: Edition bump surfaces a larger-than-expected refactor.
  - Mitigation: Stage the work; commit small fixes; rely on CI to guide.

## Open questions

- Do we adopt `JoinSet` or keep `join_all` + Vec<JoinHandle>? `JoinSet` simplifies draining and cancellation; propose `JoinSet`.
- Should we expose concurrency as CLI knobs (e.g., `--parallel-upload`)? Could keep defaults now and align with existing constants later.
- For polyfuse, do we plan to contribute patches upstream if we find gaps? Likely yes if the delta is reasonable.

## “How to run” (operator notes)

- After setting edition and refactors are staged, run:
  - `cargo update --verbose`
  - `cargo build --all-targets`
  - `cargo test` (if tests exist/enabled)
- For polyfuse upstream trial:
  - Update [Cargo.toml](Cargo.toml:90) to upstream; run `cargo update --verbose`; build; capture errors; summarize API drift and a recommended path.

## References

- Worker pool callsites:
  - [workers::WorkerPool](src/pack.rs:13)
  - [workers::WorkerPool::new(...)](src/pack.rs:68)
  - [WorkerPool param usage in pack_one](src/pack.rs:106)
  - [impl workers::Work for Uploader](src/pack.rs:253)
  - [workers::WorkerPool](src/unpack.rs:15)
  - [WorkerPool param usage in unpack structs](src/unpack.rs:131)
  - [workers::WorkerPool::new(...)](src/clone.rs:17)

- Polyfuse use:
  - [src/fs/mod.rs](src/fs/mod.rs)
  - Polyfuse fork pin: [Cargo.toml](Cargo.toml:90)

- Prior decisions:
  - [ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md)
  - [ADR-0002-fuse-lazy-loading.md](docs/architecture/adr/ADR-0002-fuse-lazy-loading.md)
  - [ADR-0003-cache-strategy.md](docs/architecture/adr/ADR-0003-cache-strategy.md)
  - [ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md)
  - [ADR-0006-s3-preflight-and-retry.md](docs/architecture/adr/ADR-0006-s3-preflight-and-retry.md)

## Changelog
- 2025-10-02: Initial proposal for Edition 2024, semaphore-based concurrency, and polyfuse upstream evaluation.