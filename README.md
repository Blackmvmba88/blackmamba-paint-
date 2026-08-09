# BlackMamba Paint

BlackMamba Paint is an **AI-native visual creation system** for drawing, analysis, learning, animation, spatial canvases, perspective, references, and eventually 3D workflows.

The goal is not to replace the artist. The goal is to give the artist a visual intelligence layer that can see structure, explain decisions, preserve intent, and help create at any skill level.

## Product laws

1. **The artist is authoritative.** AI proposes; the document changes only through explicit commands.
2. **Everything important is editable.** Strokes, paths, perspective, references, guides, AI observations, cameras, and animation are first-class data.
3. **AI must show evidence and confidence.** No opaque oracle behavior.
4. **Time is native.** Every stroke may preserve timing, direction, pressure, and replay information.
5. **Space is virtual.** Empty canvas costs nothing; only authored content is stored and rendered.
6. **Projection is a document property.** Flat, infinite, cylindrical/360, spherical, fisheye, and future custom projections are not post-effects.
7. **Learning is built into creation.** A recommendation can explain why, show construction, and become an exercise.
8. **References have roles.** Pose, color, material, fashion, lighting, composition, context, and other intent are tracked independently.

## First architecture

```text
BlackMamba Paint
       |
       +-- Document / Space / Time
       |
       +-- Stroke + Path + Brush + Layers
       |
       +-- Perspective Core
       |
       +-- Visual Intelligence
       |      +-- perception
       |      +-- critique
       |      +-- ghost mentor
       |      +-- evidence + confidence
       |
       +-- Knowledge / References / Style
       |
       +-- Animation / Camera
       |
       +-- 3D Bridge [roadmap]
```

## Current branch goal

`agent/ai-native-foundation` establishes the first compilable Rust contracts for:

- infinite-space-ready documents
- time-aware strokes
- layers and cameras
- perspective models
- AI observations, critique, evidence, confidence, intent locks, and ghost suggestions
- a runnable example proving the modules can talk to each other

## Workspace

```text
apps/blackmamba-paint/       runnable shell
crates/bmp-core/             document, strokes, layers, camera, commands
crates/bmp-ai/               visual-intelligence contracts and mock engine
crates/bmp-perspective/      perspective models and inference contracts
docs/                        architecture, roadmap, epics, ADRs
```

## Near-term milestones

- **M0 Foundation:** compile, document model, AI contract, tests, CI.
- **M1 First Stroke:** infinite canvas, stroke capture, layers, save/load, undo/redo.
- **M2 Editable Paths:** nodes, Bézier handles, simplify, subdivide, divide, join.
- **M3 TracePulse:** pixel magnifier, seed-based line extraction, editable contour, animated reveal.
- **M4 Time + Camera:** stroke replay, timeline, zoom/pan camera animation, video export path.
- **M5 Perspective:** manual, assisted, automatic inference, tutor, 360 and spherical projection.
- **M6 Visual Intelligence:** critique, color, values, edges, materials, composition, style genome, learning engine.
- **M7 3D Bridge:** Blender interoperability and spatial workflows.

This repository intentionally starts with AI in the architecture from day one, while keeping the editable document model independent from any single model provider.
