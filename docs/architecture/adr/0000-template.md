# ADR-0000: Template

## How to use this template
- Copy this file to [docs/architecture/adr/ADR-XXXX-kebab-title.md](docs/architecture/adr/ADR-XXXX-kebab-title.md). Use a four-digit, zero-padded number.
- Set Status to Proposed initially; update as it evolves (Accepted, then possibly Superseded or Deprecated).
- Link the new ADR from [docs/architecture/adr/index.md](docs/architecture/adr/index.md) under Active ADRs.
- Follow documentation conventions in [docs/docs-conventions.md](docs/docs-conventions.md), including filename links and lifecycle states.

## Metadata
- ID: ADR-0000 (Template)
- Title: Replace with a concise, descriptive title
- Status: Proposed | Accepted | Superseded | Deprecated
- Date: YYYY-MM-DD
- Deciders: [list of names/roles]
- Tags: [optional list]
- Supersedes: [link to prior ADR file, if applicable]
- Superseded by: [link to successor ADR file, if applicable]

Note: Use filename links for Supersedes/Superseded by fields, for example:
- [docs/architecture/adr/0001-metadata-layout-and-pack-pipeline.md](docs/architecture/adr/0001-metadata-layout-and-pack-pipeline.md)

## Context
Briefly state the problem, drivers, constraints, and explicit goals/non-goals. Include any relevant background and assumptions.

## Decision
State the decision in one clear sentence.

## Rationale
Explain why this decision was chosen, the trade-offs considered, and why alternatives were not selected.

## Alternatives Considered
- Alternative A — summary, pros, cons
- Alternative B — summary, pros, cons
- Alternative C — summary, pros, cons

## Consequences
Describe positive and negative consequences, risks, and mitigations. Note impacts on developers, users, operations, and timelines.

## Implementation Notes
Outline the migration/rollout plan, milestones, and compatibility considerations. Include deprecation or backfill steps where needed.

## Security and Privacy
Identify threats, attack surfaces, data sensitivity, and mitigations. Note any access control, encryption, logging, or data retention requirements.

## Operational Considerations
Observability (metrics, logs, traces), failure modes and recovery, runbooks, alerts, capacity planning, and SLOs.

## Performance and Scaling
Expected performance characteristics, known bottlenecks, scaling strategies, and any benchmark results or test plans.

## Open Questions
List items that must be resolved before acceptance, and any follow-ups for later iterations.

## References
Link to related documents, issues, or PRs using filename links. Examples:
- [README.md](README.md)
- [docs/architecture/overview.md](docs/architecture/overview.md)
- [docs/docs-conventions.md](docs/docs-conventions.md)

## Changelog
- YYYY-MM-DD — Created