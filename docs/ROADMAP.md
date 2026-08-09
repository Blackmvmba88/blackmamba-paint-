# BlackMamba Paint Roadmap

This roadmap is ordered by dependency, not by ambition. AI is present from M0 and gains capability as more structured visual data becomes available.

## M0 — AI-Native Foundation

- Rust workspace
- authoritative document model
- time-aware stroke samples
- layers, camera, projection modes
- AI analysis contract with evidence/confidence/preserve locks
- perspective inference contract
- runnable integration shell
- CI

**Exit:** repository compiles and the AI/perspective paths can inspect a saved document without mutating it.

## M1 — First Stroke

- input event abstraction: mouse, stylus, touch
- stroke capture
- infinite-world coordinates
- pan / zoom / rotate camera
- sparse spatial index
- layer operations
- save/load `.bmpaint`
- command transactions
- undo/redo
- first AI live-inspection hook over selected strokes

**Exit:** draw in widely separated world regions, save, reopen, and recover exactly the same world.

## M2 — Geometry Editing

- path representation
- nodes and Bézier handles
- adaptive point density
- add/remove point
- simplify
- subdivide
- divide
- join
- non-destructive path-to-brush rendering
- AI geometry hints as Ghost overlays

## M3 — TracePulse

- image/reference layer
- wheel-button pixel magnifier interaction
- seed pixel selection
- local color/contrast/connectivity analysis
- directional contour following
- extracted path on a new layer
- start point and direction control
- edit extracted path using M2 tools
- animate line reveal from selected seed

## M4 — Time, Camera, Video

- timeline
- native stroke replay
- reveal/reverse/from-click/from-center
- keyframes
- camera pan/zoom/rotation
- recursive zoom storytelling on infinite canvas
- animation layers
- render/export pipeline

## M5 — Perspective + Spatial Canvas

- manual 1/2/3 point guides
- automatic line-family detection
- vanishing point inference
- horizon/FOV/camera estimates
- assisted perspective editing
- Perspective X-Ray
- Perspective Tutor
- cylindrical 360 document mode
- spherical document mode including zenith/nadir
- curvilinear/fisheye projections

## M6 — Visual Intelligence

- shape/form analysis
- values
- edges
- color hierarchy
- composition/focus map
- material behavior
- proportion/anatomy hooks
- Critique Lens with impact ranking
- Ghost Mentor
- Why Mode
- Intent Lock
- confidence overlay
- Blind Check / diagnostics

## M7 — Knowledge + References + Learning

- art knowledge graph
- reference roles
- concept bank
- style genome
- color theory + marketing/context layer
- fashion/context knowledge
- project/world rules
- visual continuity
- skill graph
- challenge generator
- Artist Replay process analysis

## M8 — 3D Bridge

- Blender interchange
- camera sync
- 2D paths to curves/planes
- draw/paint over 3D
- geometry hints from silhouettes
- material/reference transfer
- spatial scene context

## M9 — Spatial Art

- immersive spherical workspaces
- room-scale composition
- VR interaction experiments
- interactive paintings
- spatial animation and presentation

## Cross-cutting requirements

Every milestone must preserve:

- deterministic save/load
- non-destructive AI assistance
- explicit confidence/evidence
- intent locks
- testable commands
- no secret credentials in files or repository
- performance proportional to authored content, not virtual canvas size
