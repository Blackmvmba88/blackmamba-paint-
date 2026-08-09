use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResponseCurve {
    Constant(f32),
    Linear,
    Power(f32),
}

impl ResponseCurve {
    pub fn evaluate(self, input: f32) -> f32 {
        let input = input.clamp(0.0, 1.0);
        match self {
            Self::Constant(value) => value.clamp(0.0, 1.0),
            Self::Linear => input,
            Self::Power(exponent) => {
                if !exponent.is_finite() || exponent <= 0.0 {
                    input
                } else {
                    input.powf(exponent)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrushDefinition {
    pub id: String,
    pub name: String,
    pub base_size: f32,
    pub min_size_ratio: f32,
    pub base_opacity: f32,
    pub min_opacity_ratio: f32,
    pub pressure_size: ResponseCurve,
    pub pressure_opacity: ResponseCurve,
}

impl BrushDefinition {
    pub fn pencil(id: impl Into<String>, name: impl Into<String>, base_size: f32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_size: base_size.max(0.01),
            min_size_ratio: 0.15,
            base_opacity: 1.0,
            min_opacity_ratio: 0.2,
            pressure_size: ResponseCurve::Linear,
            pressure_opacity: ResponseCurve::Linear,
        }
    }

    pub fn appearance(&self, pressure: f32) -> BrushAppearance {
        let size_response = self.pressure_size.evaluate(pressure);
        let opacity_response = self.pressure_opacity.evaluate(pressure);
        let size_ratio = mix(self.min_size_ratio.clamp(0.0, 1.0), 1.0, size_response);
        let opacity_ratio = mix(
            self.min_opacity_ratio.clamp(0.0, 1.0),
            1.0,
            opacity_response,
        );

        BrushAppearance {
            diameter: self.base_size.max(0.01) * size_ratio,
            opacity: (self.base_opacity.clamp(0.0, 1.0) * opacity_ratio).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BrushAppearance {
    pub diameter: f32,
    pub opacity: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BrushLibrary {
    brushes: BTreeMap<String, BrushDefinition>,
}

impl BrushLibrary {
    pub fn insert(&mut self, brush: BrushDefinition) -> Result<(), BrushError> {
        if brush.id.trim().is_empty() {
            return Err(BrushError::EmptyBrushId);
        }
        self.brushes.insert(brush.id.clone(), brush);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&BrushDefinition> {
        self.brushes.get(id)
    }

    pub fn len(&self) -> usize {
        self.brushes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.brushes.is_empty()
    }
}

fn mix(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}

#[derive(Debug, Error, PartialEq)]
pub enum BrushError {
    #[error("brush id cannot be empty")]
    EmptyBrushId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_changes_size_and_opacity_without_special_cases() {
        let brush = BrushDefinition::pencil("graphite", "Graphite", 20.0);
        let light = brush.appearance(0.2);
        let hard = brush.appearance(0.9);

        assert!(hard.diameter > light.diameter);
        assert!(hard.opacity > light.opacity);
        assert!(hard.opacity <= 1.0);
    }

    #[test]
    fn power_curve_can_make_pressure_response_more_deliberate() {
        let linear = ResponseCurve::Linear.evaluate(0.5);
        let squared = ResponseCurve::Power(2.0).evaluate(0.5);
        assert_eq!(linear, 0.5);
        assert_eq!(squared, 0.25);
    }
}
