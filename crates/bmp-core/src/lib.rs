use bmp_path::EditablePath;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PointSample {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stroke {
    pub id: Uuid,
    pub layer_id: Uuid,
    pub brush_id: String,
    pub points: Vec<PointSample>,
}

impl Stroke {
    pub fn new(layer_id: Uuid, brush_id: impl Into<String>, points: Vec<PointSample>) -> Self {
        Self {
            id: Uuid::new_v4(),
            layer_id,
            brush_id: brush_id.into(),
            points,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match (self.points.first(), self.points.last()) {
            (Some(first), Some(last)) => last.timestamp_ms.saturating_sub(first.timestamp_ms),
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub stroke_ids: Vec<Uuid>,
    #[serde(default)]
    pub path_ids: Vec<Uuid>,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            visible: true,
            stroke_ids: Vec::new(),
            path_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PathOrigin {
    Manual,
    TracePulse {
        reference_id: Uuid,
        seed_pixel_x: u32,
        seed_pixel_y: u32,
        seed_node_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathObject {
    pub layer_id: Uuid,
    pub geometry: EditablePath,
    pub brush_id: Option<String>,
    pub visible: bool,
    pub origin: PathOrigin,
}

impl PathObject {
    pub fn new(layer_id: Uuid, geometry: EditablePath, origin: PathOrigin) -> Self {
        Self {
            layer_id,
            geometry,
            brush_id: None,
            visible: true,
            origin,
        }
    }

    pub fn id(&self) -> Uuid {
        self.geometry.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RasterReference {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub name: String,
    pub media_type: String,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub center_x: f64,
    pub center_y: f64,
    pub world_width: f64,
    pub world_height: f64,
    pub rotation_rad: f64,
    pub opacity: f32,
    pub visible: bool,
    #[serde(default)]
    pub z_index: i64,
}

impl RasterReference {
    pub fn new(
        asset_id: Uuid,
        name: impl Into<String>,
        media_type: impl Into<String>,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Self {
        let safe_width = pixel_width.max(1);
        let safe_height = pixel_height.max(1);
        Self {
            id: Uuid::new_v4(),
            asset_id,
            name: name.into(),
            media_type: media_type.into(),
            pixel_width: safe_width,
            pixel_height: safe_height,
            center_x: 0.0,
            center_y: 0.0,
            world_width: f64::from(safe_width),
            world_height: f64::from(safe_height),
            rotation_rad: 0.0,
            opacity: 1.0,
            visible: true,
            z_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ProjectionMode {
    Flat,
    InfiniteFlat,
    Cylindrical360,
    Spherical,
    Fisheye,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Camera2D {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
    pub rotation_rad: f64,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            rotation_rad: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IntentLocks {
    pub preserve: Vec<String>,
    pub flexible: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AiSessionContext {
    pub artist_intent: Option<String>,
    pub intent_locks: IntentLocks,
    pub active_reference_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub projection: ProjectionMode,
    pub camera: Camera2D,
    pub layers: BTreeMap<Uuid, Layer>,
    pub strokes: BTreeMap<Uuid, Stroke>,
    #[serde(default)]
    pub paths: BTreeMap<Uuid, PathObject>,
    #[serde(default)]
    pub raster_references: BTreeMap<Uuid, RasterReference>,
    pub ai: AiSessionContext,
}

impl Document {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            projection: ProjectionMode::InfiniteFlat,
            camera: Camera2D::default(),
            layers: BTreeMap::new(),
            strokes: BTreeMap::new(),
            paths: BTreeMap::new(),
            raster_references: BTreeMap::new(),
            ai: AiSessionContext::default(),
        }
    }

    pub fn add_layer(&mut self, layer: Layer) -> Uuid {
        let id = layer.id;
        self.layers.insert(id, layer);
        id
    }

    pub fn add_stroke(&mut self, stroke: Stroke) -> Result<Uuid, DocumentError> {
        let layer = self
            .layers
            .get_mut(&stroke.layer_id)
            .ok_or(DocumentError::MissingLayer(stroke.layer_id))?;

        let id = stroke.id;
        layer.stroke_ids.push(id);
        self.strokes.insert(id, stroke);
        Ok(id)
    }

    pub fn add_path(&mut self, path: PathObject) -> Result<Uuid, DocumentError> {
        let layer = self
            .layers
            .get_mut(&path.layer_id)
            .ok_or(DocumentError::MissingLayer(path.layer_id))?;
        let id = path.id();
        layer.path_ids.push(id);
        self.paths.insert(id, path);
        Ok(id)
    }

    pub fn add_raster_reference(&mut self, reference: RasterReference) -> Uuid {
        let id = reference.id;
        self.raster_references.insert(id, reference);
        id
    }

    pub fn to_json_pretty(&self) -> Result<String, DocumentError> {
        serde_json::to_string_pretty(self).map_err(DocumentError::Serialize)
    }

    pub fn from_json(input: &str) -> Result<Self, DocumentError> {
        serde_json::from_str(input).map_err(DocumentError::Deserialize)
    }
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("layer {0} does not exist")]
    MissingLayer(Uuid),
    #[error("failed to serialize document: {0}")]
    Serialize(serde_json::Error),
    #[error("failed to deserialize document: {0}")]
    Deserialize(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_path::{EditablePath, Vec2};

    #[test]
    fn stroke_is_time_native() {
        let layer = Layer::new("ink");
        let stroke = Stroke::new(
            layer.id,
            "pencil",
            vec![
                PointSample {
                    x: 0.0,
                    y: 0.0,
                    pressure: 0.4,
                    tilt_x: 0.0,
                    tilt_y: 0.0,
                    timestamp_ms: 100,
                },
                PointSample {
                    x: 50.0,
                    y: 10.0,
                    pressure: 0.9,
                    tilt_x: 0.0,
                    tilt_y: 0.0,
                    timestamp_ms: 350,
                },
            ],
        );
        assert_eq!(stroke.duration_ms(), 250);
    }

    #[test]
    fn document_round_trips_without_losing_world_state() {
        let mut doc = Document::new("First Stroke");
        doc.camera.x = 98_432_882.0;
        doc.camera.y = -12_004_291.0;
        doc.ai.artist_intent = Some("industrial concept sketch".into());
        let layer = Layer::new("construction");
        let layer_id = doc.add_layer(layer);
        doc.add_stroke(Stroke::new(layer_id, "graphite", vec![]))
            .unwrap();
        let mut reference = RasterReference::new(
            Uuid::new_v4(),
            "ref.png",
            "image/png",
            2048,
            1024,
        );
        reference.z_index = 7;
        let reference_id = doc.add_raster_reference(reference);
        let path = EditablePath::from_polyline([
            Vec2::new(98_500_000.0, -12_100_000.0),
            Vec2::new(98_500_050.0, -12_100_020.0),
        ]);
        let mut object = PathObject::new(
            layer_id,
            path,
            PathOrigin::TracePulse {
                reference_id,
                seed_pixel_x: 31,
                seed_pixel_y: 44,
                seed_node_index: 1,
            },
        );
        object.brush_id = Some("ink".into());
        doc.add_path(object).unwrap();

        let json = doc.to_json_pretty().unwrap();
        let restored = Document::from_json(&json).unwrap();
        assert_eq!(doc, restored);
        assert_eq!(restored.raster_references[&reference_id].z_index, 7);
    }

    #[test]
    fn path_is_owned_by_layer_and_preserves_tracepulse_seed() {
        let mut document = Document::new("trace path");
        let layer_id = document.add_layer(Layer::new("trace"));
        let reference_id = Uuid::new_v4();
        let path = EditablePath::from_polyline([Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0)]);
        let object = PathObject::new(
            layer_id,
            path,
            PathOrigin::TracePulse {
                reference_id,
                seed_pixel_x: 7,
                seed_pixel_y: 9,
                seed_node_index: 1,
            },
        );
        let path_id = document.add_path(object).unwrap();

        assert_eq!(document.layers[&layer_id].path_ids, vec![path_id]);
        assert!(matches!(
            document.paths[&path_id].origin,
            PathOrigin::TracePulse {
                reference_id: id,
                seed_pixel_x: 7,
                seed_pixel_y: 9,
                seed_node_index: 1,
            } if id == reference_id
        ));
    }

    #[test]
    fn legacy_document_without_paths_still_loads() {
        let mut document = Document::new("legacy");
        document.add_layer(Layer::new("ink"));
        let mut value = serde_json::to_value(document).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("paths");
        for layer in object["layers"].as_object_mut().unwrap().values_mut() {
            layer.as_object_mut().unwrap().remove("path_ids");
        }

        let restored = Document::from_json(&value.to_string()).unwrap();
        assert!(restored.paths.is_empty());
        assert!(restored
            .layers
            .values()
            .all(|layer| layer.path_ids.is_empty()));
    }

    #[test]
    fn legacy_reference_without_z_index_defaults_to_zero() {
        let mut document = Document::new("legacy reference");
        let reference_id = document.add_raster_reference(RasterReference::new(
            Uuid::new_v4(),
            "legacy.png",
            "image/png",
            16,
            16,
        ));
        let mut value = serde_json::to_value(document).unwrap();
        value["raster_references"][reference_id.to_string()]
            .as_object_mut()
            .unwrap()
            .remove("z_index");
        let restored = Document::from_json(&value.to_string()).unwrap();
        assert_eq!(restored.raster_references[&reference_id].z_index, 0);
    }
}
