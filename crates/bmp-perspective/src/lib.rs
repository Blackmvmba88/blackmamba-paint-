use bmp_core::ProjectionMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VanishingPoint {
    pub x: f64,
    pub y: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Horizon {
    pub y: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerspectiveSolution {
    pub projection: ProjectionMode,
    pub horizon: Option<Horizon>,
    pub vanishing_points: Vec<VanishingPoint>,
    pub field_of_view_deg: Option<f64>,
    pub camera_pitch_deg: Option<f64>,
    pub camera_roll_deg: Option<f64>,
    pub explanation: String,
    pub confidence: f32,
}

impl PerspectiveSolution {
    pub fn empty(projection: ProjectionMode) -> Self {
        Self {
            projection,
            horizon: None,
            vanishing_points: Vec::new(),
            field_of_view_deg: None,
            camera_pitch_deg: None,
            camera_roll_deg: None,
            explanation: String::new(),
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LineObservation {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub weight: f32,
}

/// Contract for automatic or assisted perspective inference.
/// A deterministic geometric implementation can live beside an AI-assisted one.
pub trait PerspectiveInference {
    fn infer(&self, lines: &[LineObservation], projection_hint: ProjectionMode)
        -> PerspectiveSolution;
}

#[derive(Debug, Default)]
pub struct StubPerspectiveInference;

impl PerspectiveInference for StubPerspectiveInference {
    fn infer(
        &self,
        lines: &[LineObservation],
        projection_hint: ProjectionMode,
    ) -> PerspectiveSolution {
        let mut result = PerspectiveSolution::empty(projection_hint);
        result.explanation = format!(
            "Perspective inference pipeline initialized with {} line observations.",
            lines.len()
        );
        result.confidence = if lines.is_empty() { 0.0 } else { 0.25 };
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_is_explicit_in_perspective_solution() {
        let result = StubPerspectiveInference.infer(&[], ProjectionMode::Spherical);
        assert_eq!(result.projection, ProjectionMode::Spherical);
    }
}
