# Polyfuse differentiation plan and dependency update procedure

Status: Proposed  
Date: 2025-10-02  
Scope: Compare the current forked polyfuse dependency to the upstream crate, outline a safe evaluation/migration process, and define a repeatable workflow for `cargo update --verbose` triage. This complements [docs/architecture/adr/ADR-0007-rust-2024-and-concurrency-migration.md](docs/architecture/adr/ADR-0007-rust-2024-and-concurrency-migration.md).

## Current state

- The project pins a fork of polyfuse:
  - Cargo manifest entry: see [Cargo.toml](Cargo.toml)
  - Lockfile shows fork commit: see [Cargo.lock](Cargo.lock)
- Call sites are primarily in the FUSE mount implementation:
  - Filesystem and handlers: [src/fs/mod.rs](src/fs/mod.rs)

## Goals

- Explain how to differentiate the fork (muhamadazmy/polyfuse) from upstream (ubnt-intrepid/polyfuse).
- Provide a safe, incremental process to try upstream and measure the delta without destabilizing main.
- Combine this with a dependency refresh (`cargo update --verbose`), capturing and triaging breakages.

## Quick inventory of our polyfuse usage

- Imports and surface APIs currently referenced:
  - reply types, ops, and session:
    - [src/fs/mod.rs](src/fs/mod.rs)
- Expect differences (to validate during trial):
  - Module paths (e.g., reply/op modules)
  - Session creation and async polling (AsyncFd glue)
  - Minor trait or type name changes

## Differentiation workflow (branch-based)

1) Create a dedicated evaluation branch
- `git checkout -b chore/polyfuse-upstream-eval`

2) Point dependency to upstream polyfuse
- Edit [Cargo.toml](Cargo.toml) to replace the fork with upstream. Example:
  ```
  [dependencies.polyfuse]
  git = "https://github.com/ubnt-intrepid/polyfuse"
  # optionally pin a tag or commit for reproducibility:
  # rev = "<commit>"
  # or version = "x.y.z" if crates.io release works for our needs
  ```
- Keep the forked line commented for quick rollback if needed.

3) Update the lockfile and fetch upstream changes
- `cargo update --verbose -p polyfuse`
- If you changed only the git source, a plain `cargo update --verbose` is sufficient.

4) Build and capture compiler errors
- `cargo build --verbose`
- Note mismatches in:
  - Imports (module paths moved)
  - Session initialization (mount helper differences)
  - Reply/op types
- Copy error notes into a scratchpad for systematic fixes (preferred order of attack below).

5) Adjust call sites in [src/fs/mod.rs](src/fs/mod.rs)
- Typical changes:
  - Rename/move of reply/op/module paths.
  - Session mounting signature changes.
  - Readiness/poll integration (ensure our AsyncFd loop matches upstream expectations).
- Keep diffs minimal; don’t refactor behavior during this evaluation.

6) Run local mount tests
- Verify that mount works (create a small flist, mount it, list directories, read a small file).
- Capture any regressions in behavior (error mapping, getattr/readdir/read semantics).

7) Decision checkpoint
- If changes are small and behavior stable, consider switching to upstream on main.
- If gaps are large:
  - Document the delta and rationale to keep the fork temporarily.
  - Consider proposing patches upstream or exploring alternative crates (e.g., fuse3) as a follow-up.

## Cargo update triage procedure

We use `cargo update --verbose` to refresh dependencies and then fix breakages in deterministic order.

Recommended order of operations:
1) Move to Rust 2024 edition
- Update manifest: set edition in [Cargo.toml](Cargo.toml)
- Run `cargo check` and apply `cargo fix --edition` (if applicable) locally.
- Address edition lints and minor API changes.

2) Concurrency refactor (if in the same sprint)
- Replace worker pool dependency with idiomatic Tokio (Semaphore + JoinSet).
- Targeted files:
  - [src/pack.rs](src/pack.rs)
  - [src/unpack.rs](src/unpack.rs)
  - [src/clone.rs](src/clone.rs)
- Keep functional logic (chunking, writer.block, error surfaces) the same; only change scheduling/backpressure.

3) Refresh dependencies
- Run: `cargo update --verbose`
- Build: `cargo build --verbose`
- Triage errors by module:
  - Stores and S3: adjust API if rust-s3 changed types or errors.
  - FUSE/polyfuse: see differentiation workflow above.
  - Server/reqwest/serde changes: adjust derives or builder calls as needed.
- Use `cargo tree -i <crate>` to inspect reverse dependencies for targeted issues.

4) Validate end-to-end
- Run unit and integration tests (if present).
- Manual sanity checks:
  - Pack a small directory and verify flist.
  - Mount flist and list/read file.
  - Upload/download basic flows to the server.

## Notes on replacing tokio-worker-pool with semaphores

Pattern to replace `workers::WorkerPool`:
- Concurrency control:
  - Use `tokio::sync::Semaphore` with capacity `PARALLEL_UPLOAD`.
- Task management:
  - Use `tokio::task::JoinSet` to spawn and drain tasks, propagating errors.

Example upload fan-out sketch (adapt for pack, unpack, clone):
```
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

let sem = Arc::new(Semaphore::new(PARALLEL_UPLOAD));
let mut joinset = JoinSet::new();

for work in work_items {
    let permit = Arc::clone(&sem).acquire_owned().await.expect("semaphore");
    joinset.spawn(async move {
        let _permit = permit; // keep until task end
        // do work, return anyhow::Result<()>
        Ok(())
    });
}

while let Some(res) = joinset.join_next().await {
    res??; // join error, then task error, both propagated
}
```
- Error aggregation can mimic the current behavior by collecting results and emitting a single summary at the end.

## Deliverables checklist

- [ ] Branch evaluating upstream polyfuse with recorded deltas.
- [ ] Edition set to 2024 in [Cargo.toml](Cargo.toml).
- [ ] Worker pool removed; replaced with Semaphore/JoinSet patterns in pack/unpack/clone.
- [ ] `cargo update --verbose` executed; triaged and fixed breakages.
- [ ] Minimal e2e validation (pack/mount/upload/download) performed.

## Acceptance criteria

- The codebase compiles with Rust 2024 edition.
- No reliance on the external worker-pool crate.
- A documented conclusion on polyfuse: either migrated to upstream or a rationale for keeping the fork (with tracked follow-up).
- Green basic flows and no regression in error handling behavior.

## References

- Edition and concurrency plan: [docs/architecture/adr/ADR-0007-rust-2024-and-concurrency-migration.md](docs/architecture/adr/ADR-0007-rust-2024-and-concurrency-migration.md)
- FUSE usage: [src/fs/mod.rs](src/fs/mod.rs)
- Current fork pin: [Cargo.toml](Cargo.toml), [Cargo.lock](Cargo.lock)