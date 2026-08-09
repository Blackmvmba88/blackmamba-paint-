# ADR 0001 — AI-Native, Document-Authoritative Architecture

**Status:** Accepted

## Context

BlackMamba Paint is intended to use visual AI from the beginning for perception, critique, perspective, learning, references, style, and future 3D workflows. If model output directly owns document state, the editor becomes difficult to undo, test, explain, reproduce, or migrate between providers.

## Decision

The editable document is the single authority.

AI systems receive immutable snapshots and return structured proposals. Proposals may include observations, evidence, confidence, ghost geometry, critique, explanations, and suggested commands. Applying a proposal is an explicit command and therefore participates in history/undo/redo exactly like a human edit.

Model-provider identity is not stored as a required dependency of the file format.

## Consequences

### Positive

- AI can be aggressive without becoming destructive.
- local and cloud models can coexist.
- recommendations can be inspected and explained.
- deterministic analyzers can challenge or corroborate model output.
- intent locks can be enforced before application.
- undo/redo and branching history remain coherent.
- saved documents survive provider changes.

### Cost

- adapters and normalization layers must be implemented.
- AI output cannot simply paint arbitrary pixels into the canonical document.
- command conversion requires additional engineering.

The cost is accepted because BlackMamba Paint is an editor and learning system, not a single-shot image generator.
