# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for BlackMamba Paint. Each ADR documents a significant architectural decision, its context, and consequences.

## Index

| ADR | Title | Status | Impact |
|-----|-------|--------|--------|
| [0001](0001-data-model-and-document-structure.md) | Data Model and Document Structure | Accepted | Core |
| [0002](0002-ai-observation-contract.md) | AI Observation Contract | Accepted | Core |
| [0003](0003-perspective-system.md) | Perspective System Architecture | Accepted | Core |

## Format

Each ADR follows this structure:
- **Status:** Proposed, Accepted, Rejected, Superseded
- **Context:** Why did we make this decision?
- **Decision:** What did we decide?
- **Consequences:** Trade-offs and implications
- **Examples:** Code or diagrams showing the decision

## How to Propose a New ADR

1. Copy the template below
2. Fill in Context, Decision, and Consequences
3. Add concrete examples
4. Submit as a PR with `[ADR]` in the title
5. Link to related issues or milestones

### Template

```markdown
# ADR-NNNN: Title of Decision

## Status
Proposed

## Context
Why did we need to make this decision?

## Decision
What did we decide?

## Consequences
### Positive
- 

### Trade-offs
- 

## Examples
```

## Related Documentation

- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contributor guide
- [ARCHITECTURE.md](../ARCHITECTURE.md) - High-level architecture overview
- [ROADMAP.md](../ROADMAP.md) - Milestone roadmap
- [EPICS.md](../EPICS.md) - Feature epics
