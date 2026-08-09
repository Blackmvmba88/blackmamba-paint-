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
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            visible: true,
            stroke_ids: Vec::new(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiSessionContext {
    pub artist_intent: Option<String>,
    pub intent_locks: IntentLocks,
    pub active_reference_ids: Vec<Uuid>,
}

impl Default for AiSessionContext {
    fn default() -> Self {
        Self {
            artist_intent: None,
            intent_locks: IntentLocks::default(),
            active_reference_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub projection: ProjectionMode,
    pub camera: Camera2D,
    pub layers: BTreeMap<Uuid, Layer>,
    pub strokes: BTreeMap<Uuid, Stroke>,
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

        let json = doc.to_json_pretty().unwrap();
        let restored = Document::from_json(&json).unwrap();
        assert_eq!(doc, restored);
    }
}
