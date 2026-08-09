use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CanvasPoint {
    /// Horizontal coordinate. Values wrap every 1.0 for 360-degree projections.
    pub u: f64,
    /// Vertical coordinate. 0.0 is top and 1.0 is bottom.
    pub v: f64,
}

impl CanvasPoint {
    pub fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub fn distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanvasTopology {
    Flat,
    Cylindrical360,
    Spherical360x180,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CylindricalProjection {
    pub radius: f64,
    /// Physical/conceptual height represented by the canvas.
    pub vertical_span: f64,
}

impl CylindricalProjection {
    pub fn new(radius: f64, vertical_span: f64) -> Result<Self, ProjectionError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ProjectionError::InvalidRadius(radius));
        }
        if !vertical_span.is_finite() || vertical_span <= 0.0 {
            return Err(ProjectionError::InvalidVerticalSpan(vertical_span));
        }
        Ok(Self {
            radius,
            vertical_span,
        })
    }

    /// Maps a normalized 360 canvas coordinate onto a finite vertical cylinder.
    /// `u=0` and `u=1` are the same seam in space.
    pub fn canvas_to_world(&self, point: CanvasPoint) -> Point3 {
        let u = wrap_unit(point.u);
        let v = point.v.clamp(0.0, 1.0);
        let longitude = (u - 0.5) * TAU;
        Point3 {
            x: self.radius * longitude.sin(),
            y: (0.5 - v) * self.vertical_span,
            z: self.radius * longitude.cos(),
        }
    }

    pub fn world_to_canvas(&self, point: Point3) -> CanvasPoint {
        let longitude = point.x.atan2(point.z);
        CanvasPoint {
            u: wrap_unit(longitude / TAU + 0.5),
            v: (0.5 - point.y / self.vertical_span).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SphericalProjection;

impl SphericalProjection {
    /// Equirectangular 360x180 mapping. Unlike the cylinder, v=0 reaches the
    /// zenith and v=1 reaches the nadir, so ceiling and floor are first-class.
    pub fn canvas_to_direction(&self, point: CanvasPoint) -> Point3 {
        let u = wrap_unit(point.u);
        let v = point.v.clamp(0.0, 1.0);
        let longitude = (u - 0.5) * TAU;
        let latitude = (0.5 - v) * PI;
        let cos_latitude = latitude.cos();

        Point3 {
            x: cos_latitude * longitude.sin(),
            y: latitude.sin(),
            z: cos_latitude * longitude.cos(),
        }
    }

    pub fn direction_to_canvas(&self, direction: Point3) -> Result<CanvasPoint, ProjectionError> {
        let length =
            (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z)
                .sqrt();
        if !length.is_finite() || length <= f64::EPSILON {
            return Err(ProjectionError::ZeroDirection);
        }

        let x = direction.x / length;
        let y = direction.y / length;
        let z = direction.z / length;
        let longitude = x.atan2(z);
        let latitude = y.clamp(-1.0, 1.0).asin();

        Ok(CanvasPoint {
            u: wrap_unit(longitude / TAU + 0.5),
            v: (0.5 - latitude / PI).clamp(0.0, 1.0),
        })
    }
}

pub fn wrapped_u_distance(left: f64, right: f64) -> f64 {
    let difference = (wrap_unit(left) - wrap_unit(right)).abs();
    difference.min(1.0 - difference)
}

fn wrap_unit(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.rem_euclid(1.0)
}

#[derive(Debug, Error, PartialEq)]
pub enum ProjectionError {
    #[error("cylindrical radius must be finite and positive, got {0}")]
    InvalidRadius(f64),
    #[error("cylindrical vertical span must be finite and positive, got {0}")]
    InvalidVerticalSpan(f64),
    #[error("direction cannot be zero or non-finite")]
    ZeroDirection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cylindrical_left_and_right_edges_are_the_same_spatial_seam() {
        let projection = CylindricalProjection::new(1.5, 3.0).unwrap();
        let left = projection.canvas_to_world(CanvasPoint::new(0.0, 0.4));
        let right = projection.canvas_to_world(CanvasPoint::new(1.0, 0.4));
        assert!(left.distance(right) < 1.0e-12);
    }

    #[test]
    fn cylindrical_canvas_has_finite_vertical_extent() {
        let projection = CylindricalProjection::new(1.0, 1.5).unwrap();
        let top = projection.canvas_to_world(CanvasPoint::new(0.5, 0.0));
        let bottom = projection.canvas_to_world(CanvasPoint::new(0.5, 1.0));
        assert_eq!(top.y, 0.75);
        assert_eq!(bottom.y, -0.75);
    }

    #[test]
    fn spherical_canvas_reaches_zenith_and_nadir() {
        let projection = SphericalProjection;
        let zenith = projection.canvas_to_direction(CanvasPoint::new(0.37, 0.0));
        let nadir = projection.canvas_to_direction(CanvasPoint::new(0.83, 1.0));
        assert!((zenith.y - 1.0).abs() < 1.0e-12);
        assert!((nadir.y + 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn spherical_round_trip_preserves_view_direction() {
        let projection = SphericalProjection;
        let source = CanvasPoint::new(0.91, 0.23);
        let direction = projection.canvas_to_direction(source);
        let restored = projection.direction_to_canvas(direction).unwrap();
        assert!(wrapped_u_distance(source.u, restored.u) < 1.0e-12);
        assert!((source.v - restored.v).abs() < 1.0e-12);
    }

    #[test]
    fn spherical_seam_is_continuous() {
        let projection = SphericalProjection;
        let left = projection.canvas_to_direction(CanvasPoint::new(0.0, 0.5));
        let right = projection.canvas_to_direction(CanvasPoint::new(1.0, 0.5));
        assert!(left.distance(right) < 1.0e-12);
    }
}
