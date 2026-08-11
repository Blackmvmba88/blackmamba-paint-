# ADR-0003: Perspective System Architecture

## Status
Accepted

## Context

BlackMamba Paint's 6th product law: **"Projection is a document property."** Flat, infinite, cylindrical/360, spherical, fisheye, and future custom projections are not post-effects—they are foundational.

This means:
1. The projection mode affects how strokes are captured and rendered
2. Perspective inference should be automatic where possible, assisted where needed, and tutoring when learning
3. Coordinate systems must remain consistent across projections
4. Camera transformations must be independent of projection

We needed to separate concerns:
- **Projection:** How is the infinite 3D space flattened to 2D canvas?
- **Perspective inference:** How do we detect perspective from user strokes?
- **Camera:** How do we navigate the space (zoom, pan, rotate)?

## Decision

### Projection Modes (Document-Level)

```rust
pub enum ProjectionMode {
    InfiniteFlat,    // Orthographic, traditional 2D canvas
    Spherical,       // Equirectangular 360° panorama
    Cylindrical,     // 360° horizontal wrap, flat vertical
    Fisheye(f32),    // Wide-angle distortion (fov_degrees)
    Custom(String),  // Future: per-project custom projections
}

// In Document
pub projection: ProjectionMode,
```

**Design principle:** Projection is immutable once set (or requires explicit migration if changed). This ensures:
- All strokes are captured in a consistent coordinate space
- Historical strokes don't suddenly distort if projection changes
- Rendering knows exactly how to interpret coordinates

### Perspective Inference (Analysis Layer)

```rust
pub trait PerspectiveInference {
    fn infer(&self, observations: &[LineObservation], projection: ProjectionMode) 
        -> PerspectiveSolution;
}

pub struct LineObservation {
    pub point: Vec2,            // Observed point on canvas
    pub vanishing_point: Vec2,  // Inferred direction (possibly infinite)
}

pub struct PerspectiveSolution {
    pub projection: ProjectionMode,  // Echoes back the input
    pub vanishing_points: Vec<VanishingPoint>,
    pub horizon: Option<Horizon>,
    pub confidence: f32,
}
```

**Key decision:** Inference is a pure function:
- Takes observations (what the artist drew)
- Takes projection mode (how the space is defined)
- Returns a solution with confidence

### Geometric Solver (Deterministic)

```rust
pub struct GeometricPerspectiveInference {
    config: PerspectiveConfig,
}

impl PerspectiveInference for GeometricPerspectiveInference {
    fn infer(&self, lines: &[LineObservation], projection: ProjectionMode) 
        -> PerspectiveSolution {
        // 1. Cluster similar lines (they converge to same VP)
        let clusters = self.cluster_lines(lines);
        
        // 2. Solve for vanishing points via line-line intersection
        let vps = self.recover_vanishing_points(&clusters);
        
        // 3. If two VPs are roughly horizontal, infer horizon line
        let horizon = self.infer_horizon(&vps);
        
        // 4. Confidence = (cluster support, geometric quality, consistency)
        let confidence = self.confidence_score(&vps, &clusters);
        
        PerspectiveSolution { projection, vanishing_points: vps, horizon, confidence }
    }
}
```

**Why geometric, not ML?**
- Deterministic (same strokes → same perspective)
- Testable with synthetic data
- Fast (runs locally, no network)
- Transparent (we can explain why it inferred what it did)
- Foundation for AI tutor later

### Camera System (Independent)

```rust
pub struct Camera2D {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
    pub rotation: f64,  // For cylindrical/spherical: camera yaw
}

impl Camera2D {
    pub fn viewport_bounds(&self, width: f64, height: f64) -> Bounds {
        // Transform screen space to world space
        // Independent of projection
    }
}
```

**Why separate camera from projection?**
- You can pan/zoom on a spherical projection
- Projection change doesn't reset the camera
- Perspective solution doesn't affect camera position

### Coordinate Transforms

For **flat projection** (default):
```
world (x, y) → flat 2D → screen space
```

For **spherical projection** (360° panorama):
```
world (x, y, z) → spherical map (lat, lon) → flat 2D → screen
```

The key: **document stores strokes in world space**, projection interprets during render.

## Consequences

### Positive
- **Future-proof:** New projections (custom, perspective-corrected) don't break existing code
- **Perspective-aware:** AI and rendering both understand the projection context
- **Deterministic:** Geometric solver means consistent inference
- **Auditable:** We can show why perspective was detected

### Trade-offs
- **Complexity:** Supporting multiple projections requires per-projection rendering
- **Learning curve:** Artists must understand projection choice affects stroke interpretation
- **Performance:** Spherical/fisheye rendering is more expensive than flat

## Examples

### Detecting Perspective in a Flat Drawing

```rust
let mut document = Document::new("One-Point Perspective");
document.projection = ProjectionMode::InfiniteFlat;
let layer = document.add_layer(Layer::new("construction"));

// Artist draws converging lines
let vp = (-100.0, 50.0);
for point in [(0.0, 0.0), (30.0, 120.0), (80.0, -20.0)] {
    let stroke = Stroke::new(layer.id, "pencil", vec![
        PointSample { x: point.0, y: point.1, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, timestamp_ms: 0 },
        PointSample { x: vp.0, y: vp.1, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, timestamp_ms: 100 },
    ]);
    document.add_stroke(stroke)?;
}

// Infer perspective
let result = GeometricPerspectiveInference::default()
    .infer(&extract_lines(&document.strokes), document.projection);

assert_eq!(result.projection, ProjectionMode::InfiniteFlat);
assert_eq!(result.vanishing_points.len(), 1);
assert!(result.confidence > 0.5);
```

### Drawing in Spherical Space

```rust
let mut document = Document::new("360° Panorama");
document.projection = ProjectionMode::Spherical;  // Immutable after initial set

// Camera starts centered
let mut camera = document.camera;

// Pan left (rotate 45°)
camera.rotation = 45.0;

// Strokes are captured in spherical coordinates
// Rendering interprets them appropriately
```

## Rationale

This architecture makes projection a **first-class document property**, not a post-effect:

1. **By making projection immutable:** We ensure spatial consistency across the lifetime of a document
2. **By separating inference:** We allow different algorithms (geometric, ML-based, artist-guided) to coexist
3. **By decoupling camera:** We allow navigation independent of projection
4. **By using deterministic geometry:** We create a foundation for learning and teaching

This supports M3 (editable paths with perspective-aware simplification) and M5 (perspective inference, tutor, advanced projections).
