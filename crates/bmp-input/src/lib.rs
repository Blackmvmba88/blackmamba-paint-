use bmp_core::{PointSample, Stroke};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PointerKind {
    Mouse,
    Pen,
    Touch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PointerSample {
    pub kind: PointerKind,
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub timestamp_ms: u64,
}

impl From<PointerSample> for PointSample {
    fn from(sample: PointerSample) -> Self {
        Self {
            x: sample.x,
            y: sample.y,
            pressure: sample.pressure.clamp(0.0, 1.0),
            tilt_x: sample.tilt_x,
            tilt_y: sample.tilt_y,
            timestamp_ms: sample.timestamp_ms,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrokeCapture {
    layer_id: Uuid,
    brush_id: String,
    samples: Vec<PointerSample>,
}

impl StrokeCapture {
    pub fn begin(layer_id: Uuid, brush_id: impl Into<String>) -> Self {
        Self {
            layer_id,
            brush_id: brush_id.into(),
            samples: Vec::new(),
        }
    }

    pub fn push(&mut self, sample: PointerSample) {
        self.samples.push(sample);
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn finish(self) -> Stroke {
        Stroke::new(
            self.layer_id,
            self.brush_id,
            self.samples.into_iter().map(PointSample::from).collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::Layer;

    #[test]
    fn capture_preserves_pressure_tilt_and_time() {
        let layer = Layer::new("ink");
        let mut capture = StrokeCapture::begin(layer.id, "pencil");
        capture.push(PointerSample {
            kind: PointerKind::Pen,
            x: 10.0,
            y: 20.0,
            pressure: 0.75,
            tilt_x: 0.2,
            tilt_y: -0.1,
            timestamp_ms: 100,
        });
        capture.push(PointerSample {
            kind: PointerKind::Pen,
            x: 30.0,
            y: 40.0,
            pressure: 0.9,
            tilt_x: 0.3,
            tilt_y: -0.2,
            timestamp_ms: 260,
        });

        let stroke = capture.finish();
        assert_eq!(stroke.duration_ms(), 160);
        assert_eq!(stroke.points[0].pressure, 0.75);
        assert_eq!(stroke.points[1].tilt_y, -0.2);
    }
}
