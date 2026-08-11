# ADR-0001: Data Model and Document Structure

## Status
Accepted

## Context

BlackMamba Paint is an AI-native visual creation system where:
- The artist is authoritative; AI proposes, the document changes only through explicit commands
- Everything important is editable: strokes, paths, perspective, references, guides, cameras
- Time is native: every stroke preserves timing, direction, pressure, and replay information
- Space is virtual: empty canvas costs nothing; only authored content is stored

We needed a foundational data model that:
1. Represents infinite canvas space efficiently
2. Preserves temporal information at capture resolution
3. Remains independent of any single AI provider
4. Supports backward compatibility as we evolve

## Decision

We structured the core document model around these principles:

### Document as Root Authority

```rust
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub projection: ProjectionMode,
    pub camera: Camera2D,
    pub layers: BTreeMap<Uuid, Layer>,
    pub strokes: BTreeMap<Uuid, Stroke>,
    pub paths: BTreeMap<Uuid, PathObject>,
    pub raster_references: BTreeMap<Uuid, RasterReference>,
    pub ai: AiSessionContext,
}
```

**Why BTreeMap?**
- Deterministic iteration order (important for serialization and diffing)
- Efficient spatial queries via viewport bounds
- No hash randomization issues across machines

### Stroke as Time-Native Primitive

```rust
pub struct Stroke {
    pub id: Uuid,
    pub layer_id: Uuid,
    pub brush_id: String,
    pub points: Vec<PointSample>,
}

pub struct PointSample {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub timestamp_ms: u64,
}
```

**Why this structure?**
- Each sample is immutable and timestamped at capture time
- Pressure and tilt are first-class (not post-effects)
- `duration_ms()` is computed, not stored (single source of truth)
- Replay and animation use the same data as editing

### Layer Organization

```rust
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub stroke_ids: Vec<Uuid>,
    pub path_ids: Vec<Uuid>,
}
```

**Design:** Layers hold identity references, not copies. This allows:
- Efficient rendering by visibility state
- Independent editing of stroke and path properties
- Future layer effects/blend modes as metadata, not data duplication

### Path Objects (Editable Geometry)

```rust
pub struct PathObject {
    pub layer_id: Uuid,
    pub geometry: EditablePath,
    pub origin: PathOrigin,
    pub brush_id: Option<String>,
}

pub enum PathOrigin {
    Manual,
    TracePulse { reference_id, seed_pixel_x, seed_pixel_y, seed_node_index },
}
```

**Why separate from Stroke?** Paths are editable geometry (nodes, Bézier handles), while strokes are captured motion. Both can coexist on the same layer.

### AI Session Context (Non-Mutating)

```rust
pub struct AiSessionContext {
    pub artist_intent: Option<String>,
    pub intent_locks: IntentLock,
}

pub struct IntentLock {
    pub preserve: Vec<String>,  // What the artist wants kept unchanged
}
```

**Critical principle:** AI observations, critiques, and suggestions live in separate analysis results, NOT in the document. This ensures:
- Document state is artist-authored only
- AI output is always explicitly reviewed before becoming document state
- Full audit trail of what changed and why

## Consequences

### Positive
- **Immutability by default:** Strokes and point samples cannot be altered retroactively
- **Time fidelity:** All temporal information is preserved from capture
- **Backward compatibility:** Adding fields with `#[serde(default)]` doesn't break old documents
- **AI clarity:** Separation of document and analysis keeps intent explicit

### Trade-offs
- **Memory overhead:** Storing every point sample is more expensive than lossy compression, but supports replay and learning
- **Serialization size:** JSON representation can be large, but structured well for delta compression
- **No mutable references:** Require cloning or splitting borrows, but ensures correctness

## Examples

### Creating and Saving a Document

```rust
let mut document = Document::new("First Sketch");
let layer = document.add_layer(Layer::new("ink"));

let points = vec![
    PointSample { x: 0.0, y: 0.0, pressure: 0.4, tilt_x: 0.0, tilt_y: 0.0, timestamp_ms: 100 },
    PointSample { x: 50.0, y: 10.0, pressure: 0.9, tilt_x: 0.0, tilt_y: 0.0, timestamp_ms: 350 },
];
let stroke = Stroke::new(layer.id, "pencil", points);
document.add_stroke(stroke)?;

// Round-trip through JSON
let json = serde_json::to_string(&document)?;
let restored = Document::from_json(&json)?;
assert_eq!(document, restored);
```

### Setting Intent Locks

```rust
document.ai.intent_locks.preserve = vec!["silhouette".into(), "expression".into()];
let result = ai_engine.analyze(&document, &request);
// AI respects these locks in suggestions
assert_eq!(result.critique.do_not_touch, vec!["silhouette", "expression"]);
```

## Rationale

This model prioritizes **correctness and clarity** over convenience:
- By making time explicit and space virtual, we enable replay and infinite canvas
- By separating document from analysis, we make AI behavior transparent and auditable
- By choosing immutable strokes, we preserve the full history naturally
- By using BTreeMap for collections, we ensure deterministic behavior across machines

This is the foundation for M1 (First Stroke), M2 (Editable Paths), and beyond.
