# ADR-0002: AI Observation Contract

## Status
Accepted

## Context

BlackMamba Paint's second product law is: **"AI must show evidence and confidence."** No opaque oracle behavior.

We needed to define how:
1. AI proposes insights without mutating the document
2. Each observation is traceable back to evidence
3. Confidence is quantified, not magical
4. The artist can understand and challenge AI reasoning

This required:
- A contract that any AI engine can implement
- Clear separation between analysis and document mutation
- Evidence trails showing what the AI saw and why it said something

## Decision

We created an immutable AI observation contract with two phases:

### Phase 1: Observation (What Did AI See?)

```rust
pub trait VisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) 
        -> AnalysisResult;
}

pub struct AnalysisRequest {
    pub goal: String,
    pub enabled_kinds: Vec<InsightKind>,
}

pub struct AnalysisResult {
    pub insights: Vec<Insight>,
    pub ghosts: Vec<GhostSuggestion>,
    pub critique: Critique,
}
```

**Key principle:** The `VisualIntelligence` trait takes `&Document`, not `&mut Document`. It cannot change the document.

### Phase 2: Insight (What Did AI Conclude?)

```rust
pub struct Insight {
    pub kind: InsightKind,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub suggested_stroke: Option<Stroke>,  // Never auto-applied
    pub preserve: Vec<String>,  // What this insight protects
}

pub enum InsightKind {
    Perspective,
    Composition,
    Color,
    Anatomy,
    Learning,
}

pub struct Evidence {
    pub label: String,
    pub description: String,
    pub region: Option<(f64, f64, f64, f64)>,  // x_min, y_min, x_max, y_max
}
```

**Critical elements:**
- **Confidence [0.0, 1.0]:** Quantified belief, not subjective
- **Evidence:** Always present. Every claim has at least one evidence item
- **Suggested stroke:** Proposed, never applied. Artist reviews first
- **Preserve list:** This insight depends on these artistic choices existing

### Phase 3: Critique (How Good Is This Drawing?)

```rust
pub struct Critique {
    pub high_impact: Vec<String>,    // Things that would improve the drawing significantly
    pub low_impact: Vec<String>,     // Nice-to-haves
    pub preserve_and_learn: Vec<String>,  // Artist strengths to preserve
    pub do_not_touch: Vec<String>,   // Aspects locked by artist intent
}
```

**Why separate from Insight?** Critique summarizes many observations into actionable guidance, but stays advisory.

### Phase 4: Ghost Suggestions (Learning & Teaching)

```rust
pub struct GhostSuggestion {
    pub stroke: Stroke,
    pub purpose: String,            // "show the horizon line"
    pub instruction: String,        // "this construction demonstrates..."
    pub can_reveal: bool,          // Artist can make this ghost visible
}
```

**Purpose:** Ghost suggestions are learning tools, not document mutations. They show alternative approaches without overwriting artist work.

### Deterministic Implementation

```rust
pub struct DeterministicVisualIntelligence {
    // No state, no async, no external calls
    // Pure function of document → analysis result
}

impl VisualIntelligence for DeterministicVisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) -> AnalysisResult {
        // Deterministic: same input → same output
        // Testable: unit tests fully exercise behavior
        // Auditable: every observation has evidence
    }
}
```

**Why deterministic?** 
- Tests can verify that strokes X, Y, Z always produce critique C
- Replay is possible: feed old document, get old analysis
- No network calls or model versioning surprises

## Consequences

### Positive
- **Transparency:** Every AI suggestion includes evidence
- **Control:** Artist can reject, modify, or ignore suggestions
- **Testability:** Deterministic behavior means comprehensive test coverage
- **Provider independence:** Any language model or algorithm can implement the trait
- **Audit trail:** Analysis results are separate from document, so we can see what AI said vs. what changed

### Trade-offs
- **No real-time streaming:** Deterministic analysis means batch mode
- **No persistent AI state:** Each analysis is fresh from the document (simpler, but re-computes)
- **Cannot guarantee optimal suggestions:** Trade off perfect recommendations for transparency

## Examples

### Running Deterministic Analysis

```rust
let mut document = Document::new("Perspective Study");
let layer = document.add_layer(Layer::new("construction"));

// Draw some lines...
for point in [(0.0, 0.0), (30.0, 120.0), (80.0, -20.0)] {
    document.add_stroke(Stroke::new(layer.id, "pencil", [point]));
}

// Ask AI to analyze
let result = DeterministicVisualIntelligence::default().analyze(
    &document,
    &AnalysisRequest {
        goal: "detect the perspective".into(),
        enabled_kinds: vec![InsightKind::Perspective],
    },
);

// Inspect results
for insight in &result.insights {
    println!("Found: {} (confidence: {})", insight.label, insight.confidence);
    for evidence in &insight.evidence {
        println!("  - {}: {}", evidence.label, evidence.description);
    }
}

// Artist can approve the suggested stroke
if let Some(suggested) = &result.insights[0].suggested_stroke {
    document.add_stroke(suggested.clone())?;  // Explicit!
}
```

### Setting Intent Locks

```rust
document.ai.intent_locks.preserve = vec!["silhouette".into()];

let result = ai.analyze(&document, &request);

// AI respects the lock
assert!(result.critique.do_not_touch.contains(&"silhouette".to_string()));
```

## Rationale

This contract implements the product law "**AI must show evidence and confidence**" by:
1. Making all observations immutable (no side effects)
2. Requiring evidence for every claim (transparency)
3. Quantifying confidence (measurable, not magical)
4. Separating analysis from document state (explicit approval required)
5. Providing learning through ghost suggestions, not replacements

The artist remains authoritative. AI is a consultant, not a collaborator making edits.
