# Architecture Decision Records (ADR) Index

ADRs capture significant technical decisions, the context in which they were made, and their consequences. They create a durable history and shared rationale. Follow documentation conventions in [docs/docs-conventions.md](docs/docs-conventions.md).

## Numbering scheme
- File name: ADR-XXXX-kebab-title.md (four-digit, zero-padded).
- ADR-0000 is reserved as the reusable template.
- Numbers are assigned when an ADR is proposed and never reused, even if a proposal is withdrawn.

## Active ADRs

| ID | Title | Status | Date |
| --- | --- | --- | --- |
| ADR-0000 | Template — see [docs/architecture/adr/0000-template.md](docs/architecture/adr/0000-template.md) | Accepted | 2025-10-02 |
| ADR-0001 | Metadata layout and pack pipeline — see [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md) | Proposed | 2025-10-02 |
| ADR-0002 | FUSE lazy-loading and on-demand fetch — see [docs/architecture/adr/ADR-0002-fuse-lazy-loading.md](docs/architecture/adr/ADR-0002-fuse-lazy-loading.md) | Proposed | 2025-10-02 |
| ADR-0003 | Cache strategy and eviction policy — see [docs/architecture/adr/ADR-0003-cache-strategy.md](docs/architecture/adr/ADR-0003-cache-strategy.md) | Proposed | 2025-10-02 |
| ADR-0004 | Storage backends and routing — see [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md) | Proposed | 2025-10-02 |
| ADR-0005 | Server API and block serving — see [docs/architecture/adr/ADR-0005-server-api-and-block-serving.md](docs/architecture/adr/ADR-0005-server-api-and-block-serving.md) | Proposed | 2025-10-02 |

## Planned ADRs (placeholders)
- ADR-0001 — Metadata layout and pack pipeline
- ADR-0002 — FUSE lazy-loading and on-demand fetch
- ADR-0003 — Cache strategy and eviction policy
- ADR-0004 — Storage backends and routing
- ADR-0005 — Server API and block serving

## Workflow
- Draft: Copy [docs/architecture/adr/0000-template.md](docs/architecture/adr/0000-template.md) to a new file named ADR-XXXX-kebab-title.md in [docs/architecture/adr/](docs/architecture/adr/). Set Status to Proposed and fill in metadata.
- Review and accept: Iterate via PR. When approved, update Status to Accepted and add the ADR to the table above.
- Evolve or retire: If an ADR replaces a prior one, add filename links in both documents:
  - Supersedes: link to the prior ADR file in the new ADR.
  - Superseded by: link to the new ADR file in the prior ADR.
- Deprecation: Use Deprecated when guidance is no longer in use but remains documented for reference.
- Maintenance: Keep this index in sync with actual ADR files and statuses; do not renumber or reuse IDs. Update dates using YYYY-MM-DD.