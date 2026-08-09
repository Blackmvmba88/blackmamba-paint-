use bmp_brush::BrushLibrary;
use bmp_core::{Camera2D, Document, Stroke};
use bmp_reference::{world_bounds, world_corners};
use bmp_space::Bounds;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ScreenViewport {
    pub width: f64,
    pub height: f64,
}

impl ScreenViewport {
    pub fn new(width: f64, height: f64) -> Result<Self, RenderError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(RenderError::InvalidViewport { width, height });
        }
        Ok(Self { width, height })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderReferenceQuad {
    pub reference_id: Uuid,
    pub asset_id: Uuid,
    pub screen_corners: [[f64; 2]; 4],
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderDab {
    pub stroke_id: Uuid,
    pub world_x: f64,
    pub world_y: f64,
    pub screen_x: f64,
    pub screen_y: f64,
    pub radius_px: f32,
    pub opacity: f32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderWarning {
    MissingBrush { stroke_id: Uuid, brush_id: String },
    InvalidReferenceGeometry { reference_id: Uuid },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RenderScene {
    pub references: Vec<RenderReferenceQuad>,
    pub dabs: Vec<RenderDab>,
    pub warnings: Vec<RenderWarning>,
}

pub fn build_render_scene(
    document: &Document,
    camera: Camera2D,
    viewport: ScreenViewport,
    brushes: &BrushLibrary,
) -> Result<RenderScene, RenderError> {
    if !camera.zoom.is_finite() || camera.zoom <= 0.0 {
        return Err(RenderError::InvalidZoom(camera.zoom));
    }
    if !camera.x.is_finite() || !camera.y.is_finite() || !camera.rotation_rad.is_finite() {
        return Err(RenderError::InvalidCamera);
    }

    let visible_world = conservative_world_viewport(camera, viewport);
    let mut scene = RenderScene::default();

    for reference in document
        .raster_references
        .values()
        .filter(|reference| reference.visible && reference.opacity > 0.0)
    {
        let Ok(reference_bounds) = world_bounds(reference) else {
            scene
                .warnings
                .push(RenderWarning::InvalidReferenceGeometry {
                    reference_id: reference.id,
                });
            continue;
        };
        let bounds = Bounds::new(
            reference_bounds.min_x,
            reference_bounds.min_y,
            reference_bounds.max_x,
            reference_bounds.max_y,
        );
        if !bounds.intersects(&visible_world) {
            continue;
        }
        let Ok(corners) = world_corners(reference) else {
            scene
                .warnings
                .push(RenderWarning::InvalidReferenceGeometry {
                    reference_id: reference.id,
                });
            continue;
        };
        let mut screen_corners = [[0.0; 2]; 4];
        for (index, corner) in corners.into_iter().enumerate() {
            let (screen_x, screen_y) = world_to_screen(corner.x, corner.y, camera, viewport);
            screen_corners[index] = [screen_x, screen_y];
        }
        scene.references.push(RenderReferenceQuad {
            reference_id: reference.id,
            asset_id: reference.asset_id,
            screen_corners,
            opacity: reference.opacity.clamp(0.0, 1.0),
        });
    }

    for layer in document.layers.values().filter(|layer| layer.visible) {
        for stroke_id in &layer.stroke_ids {
            let Some(stroke) = document.strokes.get(stroke_id) else {
                continue;
            };
            let Some(bounds) = stroke_bounds(stroke) else {
                continue;
            };
            if !bounds.intersects(&visible_world) {
                continue;
            }

            let Some(brush) = brushes.get(&stroke.brush_id) else {
                scene.warnings.push(RenderWarning::MissingBrush {
                    stroke_id: stroke.id,
                    brush_id: stroke.brush_id.clone(),
                });
                continue;
            };

            for point in &stroke.points {
                let appearance = brush.appearance(point.pressure);
                let (screen_x, screen_y) = world_to_screen(point.x, point.y, camera, viewport);
                let radius_px = appearance.diameter * 0.5 * camera.zoom as f32;
                if dab_intersects_screen(screen_x, screen_y, f64::from(radius_px), viewport) {
                    scene.dabs.push(RenderDab {
                        stroke_id: stroke.id,
                        world_x: point.x,
                        world_y: point.y,
                        screen_x,
                        screen_y,
                        radius_px,
                        opacity: appearance.opacity,
                        timestamp_ms: point.timestamp_ms,
                    });
                }
            }
        }
    }

    Ok(scene)
}

pub fn world_to_screen(
    world_x: f64,
    world_y: f64,
    camera: Camera2D,
    viewport: ScreenViewport,
) -> (f64, f64) {
    let dx = world_x - camera.x;
    let dy = world_y - camera.y;
    let cosine = camera.rotation_rad.cos();
    let sine = camera.rotation_rad.sin();
    let rotated_x = dx * cosine + dy * sine;
    let rotated_y = -dx * sine + dy * cosine;

    (
        viewport.width * 0.5 + rotated_x * camera.zoom,
        viewport.height * 0.5 + rotated_y * camera.zoom,
    )
}

fn conservative_world_viewport(camera: Camera2D, viewport: ScreenViewport) -> Bounds {
    let half_width = viewport.width * 0.5 / camera.zoom;
    let half_height = viewport.height * 0.5 / camera.zoom;
    let radius = (half_width * half_width + half_height * half_height).sqrt();
    Bounds::new(
        camera.x - radius,
        camera.y - radius,
        camera.x + radius,
        camera.y + radius,
    )
}

fn stroke_bounds(stroke: &Stroke) -> Option<Bounds> {
    let first = stroke.points.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for point in stroke.points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Some(Bounds::new(min_x, min_y, max_x, max_y))
}

fn dab_intersects_screen(
    screen_x: f64,
    screen_y: f64,
    radius: f64,
    viewport: ScreenViewport,
) -> bool {
    screen_x + radius >= 0.0
        && screen_y + radius >= 0.0
        && screen_x - radius <= viewport.width
        && screen_y - radius <= viewport.height
}

#[derive(Debug, Error, PartialEq)]
pub enum RenderError {
    #[error("viewport dimensions must be finite and positive, got {width}x{height}")]
    InvalidViewport { width: f64, height: f64 },
    #[error("camera zoom must be finite and positive, got {0}")]
    InvalidZoom(f64),
    #[error("camera position/rotation must be finite")]
    InvalidCamera,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_brush::BrushDefinition;
    use bmp_core::{Layer, PointSample, RasterReference, Stroke};

    fn point(x: f64, y: f64, pressure: f32) -> PointSample {
        PointSample {
            x,
            y,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            timestamp_ms: 100,
        }
    }

    #[test]
    fn offscreen_strokes_do_not_enter_render_scene() {
        let mut document = Document::new("viewport");
        let layer_id = document.add_layer(Layer::new("ink"));
        document
            .add_stroke(Stroke::new(
                layer_id,
                "graphite",
                vec![point(0.0, 0.0, 0.5)],
            ))
            .unwrap();
        document
            .add_stroke(Stroke::new(
                layer_id,
                "graphite",
                vec![point(1_000_000.0, 1_000_000.0, 0.5)],
            ))
            .unwrap();

        let mut brushes = BrushLibrary::default();
        brushes
            .insert(BrushDefinition::pencil("graphite", "Graphite", 10.0))
            .unwrap();
        let scene = build_render_scene(
            &document,
            Camera2D::default(),
            ScreenViewport::new(800.0, 600.0).unwrap(),
            &brushes,
        )
        .unwrap();

        assert_eq!(scene.dabs.len(), 1);
        assert_eq!(scene.warnings.len(), 0);
    }

    #[test]
    fn visible_reference_emits_screen_quad_and_offscreen_reference_is_culled() {
        let mut document = Document::new("references");
        let near_asset = Uuid::new_v4();
        let mut near = RasterReference::new(near_asset, "near.png", "image/png", 100, 50);
        near.world_width = 200.0;
        near.world_height = 100.0;
        near.rotation_rad = std::f64::consts::FRAC_PI_2;
        let near_id = document.add_raster_reference(near);

        let mut far = RasterReference::new(Uuid::new_v4(), "far.png", "image/png", 100, 50);
        far.center_x = 1_000_000.0;
        far.center_y = 1_000_000.0;
        document.add_raster_reference(far);

        let scene = build_render_scene(
            &document,
            Camera2D::default(),
            ScreenViewport::new(800.0, 600.0).unwrap(),
            &BrushLibrary::default(),
        )
        .unwrap();

        assert_eq!(scene.references.len(), 1);
        let quad = &scene.references[0];
        assert_eq!(quad.reference_id, near_id);
        assert_eq!(quad.asset_id, near_asset);
        assert!((quad.screen_corners[0][0] - 450.0).abs() < 1e-9);
        assert!((quad.screen_corners[0][1] - 200.0).abs() < 1e-9);
        assert!((quad.screen_corners[2][0] - 350.0).abs() < 1e-9);
        assert!((quad.screen_corners[2][1] - 400.0).abs() < 1e-9);
    }

    #[test]
    fn changing_brush_changes_appearance_not_source_stroke() {
        let mut document = Document::new("brush swap");
        let layer_id = document.add_layer(Layer::new("ink"));
        document
            .add_stroke(Stroke::new(layer_id, "ink", vec![point(0.0, 0.0, 1.0)]))
            .unwrap();
        let before = document.clone();
        let viewport = ScreenViewport::new(800.0, 600.0).unwrap();

        let mut thin = BrushLibrary::default();
        thin.insert(BrushDefinition::pencil("ink", "Thin", 4.0))
            .unwrap();
        let thin_scene =
            build_render_scene(&document, Camera2D::default(), viewport, &thin).unwrap();

        let mut thick = BrushLibrary::default();
        thick
            .insert(BrushDefinition::pencil("ink", "Thick", 40.0))
            .unwrap();
        let thick_scene =
            build_render_scene(&document, Camera2D::default(), viewport, &thick).unwrap();

        assert_eq!(document, before);
        assert!(thick_scene.dabs[0].radius_px > thin_scene.dabs[0].radius_px);
    }

    #[test]
    fn invisible_layers_emit_no_dabs() {
        let mut document = Document::new("hidden");
        let mut layer = Layer::new("hidden layer");
        layer.visible = false;
        let layer_id = document.add_layer(layer);
        document
            .add_stroke(Stroke::new(
                layer_id,
                "graphite",
                vec![point(0.0, 0.0, 1.0)],
            ))
            .unwrap();

        let mut brushes = BrushLibrary::default();
        brushes
            .insert(BrushDefinition::pencil("graphite", "Graphite", 10.0))
            .unwrap();
        let scene = build_render_scene(
            &document,
            Camera2D::default(),
            ScreenViewport::new(640.0, 480.0).unwrap(),
            &brushes,
        )
        .unwrap();

        assert!(scene.dabs.is_empty());
    }
}
