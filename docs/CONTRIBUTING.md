# Contributing to BlackMamba Paint

Thank you for contributing to BlackMamba Paint. This document outlines the standards we maintain across the codebase to ensure quality, consistency, and clarity.

## Core Principles

1. **The artist is authoritative** - Our code reflects the product law: everything important is editable, AI proposes through evidence, and the document changes only through explicit commands.
2. **Rigor in foundation** - Testing, documentation, and architecture are not afterthoughts. They are the foundation upon which everything else stands.
3. **Time is native** - Every stroke preserves timing. Every architectural decision preserves intent.
4. **Only those familiar with the standard will recognize it** - We maintain high standards not because everyone will notice, but because those who do will understand this is serious work.

## Development Setup

### Local Build Environment

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and navigate to workspace
cd blackmamba-paint-

# Quick checks
cargo fmt --all              # Format all code
cargo clippy --all --all-targets -- -D warnings  # Lint all code
cargo test --workspace       # Run all tests
cargo doc --no-deps --all --open  # View documentation
```

### Development Commands (via `.cargo/config.toml`)

```bash
cargo dev           # Quick check (cargo check --all)
cargo t             # Run all tests
cargo tc            # Run all tests without fail-fast (see all failures)
cargo fmt-check     # Verify formatting
cargo lint          # Run clippy with warnings-as-errors
cargo doc-open      # Generate and open documentation
```

## Code Style & Formatting

### Rust Code

We enforce **strict formatting** via `rustfmt.toml` and `clippy.toml`:

- **Max line width:** 100 characters
- **Indentation:** 4 spaces (Rust standard)
- **Edition:** 2021
- **Comments:** Sparse and meaningful. Use comment blocks (`// ========== Section Name ==========`) to organize test modules and large impl blocks.

**Example format:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ========== Constructor Tests ==========

    #[test]
    fn document_initializes_with_correct_defaults() {
        // test body
    }

    // ========== Serialization Tests ==========

    #[test]
    fn document_round_trips_without_losing_state() {
        // test body
    }
}
```

### Verification Before Commit

```bash
cargo fmt --all -- --check      # Must pass
cargo clippy --all --all-targets -- -D warnings   # Must pass (no warnings)
cargo test --workspace          # Must pass
```

## Testing Standards

### Test Organization

Tests are organized into **thematic sections** within `#[cfg(test)]` modules:

- **Helper Functions** - Utility functions for test setup
- **Type-Specific Tests** - Tests grouped by the types/structures being tested
- **Integration Tests** - Tests that verify multiple components work together
- **Regression Tests** - Tests for previously discovered bugs
- **Backward Compatibility Tests** - Tests ensuring schema evolution doesn't break old data

### Test Naming

Tests should describe **what is being verified**, not just "test_foo":

```rust
✅ #[test]
fn stroke_preserves_timing_information_across_serialization() { }

❌ #[test]
fn test_stroke() { }
```

### Test Coverage

Minimum coverage expectations:

- **Core data structures** (`Document`, `Stroke`, `Layer`, `Camera`): 80%+
- **AI contracts** (`Observation`, `Critique`, `Evidence`): 70%+
- **Perspective system**: 75%+
- **Immutability guarantees**: 100% (non-negotiable)

## Architecture & Dependencies

### Workspace Structure

```
apps/blackmamba-paint/              Runnable CLI shell
apps/blackmamba-paint-desktop/      Desktop UI (future)

crates/bmp-core/                    Document, Strokes, Layers, Camera
crates/bmp-ai/                      AI Contracts (Observation, Critique)
crates/bmp-perspective/             Perspective models & inference
crates/bmp-space/                   Infinite space abstractions
crates/bmp-path/                    Editable paths & Bézier
crates/bmp-history/                 Undo/redo system
crates/bmp-storage/                 Document persistence
crates/bmp-input/                   Input handling
crates/bmp-*/ (other)               Feature crates
```

### Dependency Rules

1. **No circular dependencies** - Verify with `cargo tree --duplicates`
2. **Core crates are foundation** - `bmp-core`, `bmp-path`, and `bmp-space` should have minimal external dependencies
3. **AI crates are contracts** - `bmp-ai` defines interfaces, not implementations. It depends on `bmp-core` and `bmp-perspective` but not on concrete implementations.
4. **Feature crates are users** - Crates like `bmp-render`, `bmp-reference`, and `bmp-tracepulse` may depend on `bmp-core` and other feature crates, but not the reverse.

### Adding Dependencies

Before adding an external crate:

1. **Check workspace.dependencies** - Prefer shared versions
2. **Justify in PR** - Explain why this dependency is necessary
3. **Verify MSRV** - Ensure it supports Rust 1.70+
4. **Test locally** - Run the full test suite with the new dependency

## Documentation

### ADRs (Architecture Decision Records)

Significant architectural decisions are documented in `docs/adr/`:

```
docs/
  adr/
    0001-data-model-and-document-structure.md
    0002-ai-observation-contract.md
    0003-perspective-system.md
    README.md (links all ADRs)
```

**ADR Template:**

```markdown
# ADR-NNN: Title of Decision

## Status
Accepted | Proposed | Rejected | Superseded

## Context
Why did we need to make this decision?

## Decision
What did we decide?

## Consequences
What are the trade-offs?

## Examples
Code or diagrams showing the decision in action.
```

### Code Documentation

- **Public items** require doc comments (`///`)
- **Non-obvious implementations** warrant inline comments (sparse, targeted)
- **Examples in doc comments** help users understand intent

## Submitting Changes

### Before Submitting a PR

```bash
# Format your code
cargo fmt --all

# Check for warnings
cargo clippy --all --all-targets -- -D warnings

# Run tests locally
cargo test --workspace

# Ensure no unstaged changes break the build
cargo check --workspace --all-targets
```

### PR Title & Description

- **Title:** Descriptive, imperative voice (e.g., "Add time-native stroke sampling")
- **Description:** Explain the why, not just the what
- **Link issues:** Use "Fixes #123" to auto-close related issues
- **Test coverage:** Describe new tests added

### Code Review Focus

We look for:

1. **Correctness** - Does this do what it claims to do?
2. **Clarity** - Can another developer understand this in 6 months?
3. **Tests** - Are there tests? Do they verify the right thing?
4. **Backward compatibility** - Does this break existing APIs or data formats?
5. **Performance** - Are there any obvious inefficiencies?

## CI/CD Pipeline

Every push triggers:

1. **Format check** - `cargo fmt --all -- --check`
2. **Compilation** - `cargo check --workspace --all-targets`
3. **Linting** - `cargo clippy --workspace --all-targets -- -D warnings`
4. **Tests** - `cargo test --workspace`

All must pass before merging to `main`.

## Questions?

- **Architecture questions:** Open an issue with the `question` label
- **Setup help:** Check `.cargo/config.toml` and `.editorconfig`
- **Test guidance:** Look at existing tests in the same crate (they're the spec)

---

**Remember:** Rigor is invisible until someone appreciates it. We do this for that moment.
