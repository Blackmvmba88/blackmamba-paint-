# BlackMamba Paint Architecture

## Architectural thesis

BlackMamba Paint treats drawing as structured, time-aware, spatial data and treats AI as an advisory intelligence layer over that data.

The editable document remains authoritative. AI modules observe immutable document state and return proposals with evidence and confidence. Applying a proposal must go through the same command/history system as any human edit.

## Top-level flow

```text
Input devices
   |
   v
Stroke / Path / Selection Commands
   |
   v
Authoritative Document Model <---- Save / Load / History
   |
   +---- Spatial Index ---- Renderer ---- Viewport
   |
   +---- Time / Camera ---- Animation
   |
   +---- Perspective Core
   |
   +---- Visual Intelligence
             |
             +---- deterministic analyzers
             +---- local models
             +---- cloud multimodal models
             +---- knowledge graph
             |
             v
       evidence-backed proposals
             |
             v
      Ghost / Critique / Tutor
```

## Core invariants

### 1. AI cannot mutate the document directly

`VisualIntelligence::analyze(&Document, ...)` receives immutable state and returns `AnalysisResult`.

This protects undo/redo, reproducibility, intent locks, and user trust.

### 2. Strokes are not baked pixels

A stroke records samples including world coordinates, pressure, tilt, and time. Rendering is a projection of that source data through a brush engine.

### 3. Infinite canvas is sparse

The world is coordinates plus authored entities. Empty regions allocate no raster surface. A spatial index will later answer which entities intersect the current viewport.

### 4. Projection is explicit

`ProjectionMode` is part of the document model. Flat, 360/cylindrical, spherical, fisheye, and future projections use explicit transforms instead of destructive image warps.

### 5. AI output is inspectable

Every meaningful recommendation should carry:

- kind
- claim
- reason
- suggested action
- confidence
- evidence
- regions when relevant
- preserve/do-not-touch intent

### 6. Time is source data

Stroke replay and animation are natural consequences of timestamped input rather than reconstructed effects.

## Crates

### `bmp-core`

Owns the authoritative document contracts: strokes, layers, camera, projection, serialization, artist intent, and future command/history primitives.

### `bmp-ai`

Owns provider-neutral intelligence contracts: observations, critique, ghost suggestions, evidence, confidence, and the immutable analysis boundary.

### `bmp-perspective`

Owns geometric perspective models and inference contracts. Deterministic solvers and AI-assisted inference can coexist behind the same structures.

### `blackmamba-paint`

Thin executable shell used to prove integration while the production UI is still being selected.

## Planned crates

```text
bmp-geometry       editable curves, nodes, Bézier, simplify/subdivide/divide/join
bmp-space          spatial index, viewport queries, LOD, projection transforms
bmp-render         GPU renderer and brush rasterization
bmp-brush          brush definitions and procedural behavior
bmp-history        commands, transactions, branching visual history
bmp-animation      timeline, keyframes, stroke reveal, camera motion
bmp-tracepulse     seed-based contour/path extraction
bmp-knowledge      references, art concepts, style genome, world bible
bmp-learning       skill graph, exercises, challenge generation, replay analysis
bmp-io             .bmpaint package, images, video, interchange
bmp-3d             future Blender and spatial bridge
```

## AI pipeline

The target pipeline is deliberately hybrid:

```text
Document snapshot
      |
      +--> deterministic geometry analysis
      +--> perspective solver
      +--> color/value/edge analysis
      +--> multimodal model
      +--> knowledge retrieval
      |
      v
Evidence normalization
      |
      v
Visual Reasoner
      |
      +--> Critique Lens
      +--> Ghost Mentor
      +--> Why Mode
      +--> Perspective Tutor
      +--> Challenge Generator
```

A model may be brilliant and still be uncertain. Confidence is therefore data, not UI decoration.

## Performance direction

- f64 world coordinates for large virtual spaces
- sparse spatial indexing
- viewport culling
- level of detail
- cached raster tiles for distant views
- authoritative vector/stroke source retained beneath caches
- GPU rendering through a portable abstraction, currently expected to be `wgpu`

## Security and privacy direction

AI providers are adapters, never baked into the file format. Projects may choose local-only, cloud-assisted, or hybrid analysis later. Credentials must never be committed to the repository or serialized into `.bmpaint` documents.
