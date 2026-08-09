use bmp_core::RasterReference;
use bmp_trace::PixelCoord;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

impl WorldPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldAabb {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl WorldAabb {
    pub fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ReferenceError {
    #[error("reference has invalid pixel dimensions {width}x{height}")]
    InvalidPixelDimensions { width: u32, height: u32 },
    #[error("reference has invalid world dimensions {width}x{height}")]
    InvalidWorldDimensions { width: f64, height: f64 },
    #[error("pixel ({x}, {y}) is outside reference bounds")]
    PixelOutOfBounds { x: u32, y: u32 },
}

pub fn pixel_center_to_world(
    reference: &RasterReference,
    pixel: PixelCoord,
) -> Result<WorldPoint, ReferenceError> {
    validate(reference)?;
    if pixel.x >= reference.pixel_width || pixel.y >= reference.pixel_height {
        return Err(ReferenceError::PixelOutOfBounds {
            x: pixel.x,
            y: pixel.y,
        });
    }

    let u = (f64::from(pixel.x) + 0.5) / f64::from(reference.pixel_width);
    let v = (f64::from(pixel.y) + 0.5) / f64::from(reference.pixel_height);
    let local_x = (u - 0.5) * reference.world_width;
    let local_y = (v - 0.5) * reference.world_height;
    Ok(local_to_world(reference, local_x, local_y))
}

pub fn world_to_pixel(
    reference: &RasterReference,
    world: WorldPoint,
) -> Result<Option<PixelCoord>, ReferenceError> {
    validate(reference)?;
    let (local_x, local_y) = world_to_local(reference, world);
    let u = local_x / reference.world_width + 0.5;
    let v = local_y / reference.world_height + 0.5;

    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return Ok(None);
    }

    let x = (u * f64::from(reference.pixel_width)).floor() as u32;
    let y = (v * f64::from(reference.pixel_height)).floor() as u32;
    Ok(Some(PixelCoord::new(
        x.min(reference.pixel_width - 1),
        y.min(reference.pixel_height - 1),
    )))
}

pub fn world_corners(reference: &RasterReference) -> Result<[WorldPoint; 4], ReferenceError> {
    validate(reference)?;
    let half_width = reference.world_width * 0.5;
    let half_height = reference.world_height * 0.5;
    Ok([
        local_to_world(reference, -half_width, -half_height),
        local_to_world(reference, half_width, -half_height),
        local_to_world(reference, half_width, half_height),
        local_to_world(reference, -half_width, half_height),
    ])
}

pub fn world_bounds(reference: &RasterReference) -> Result<WorldAabb, ReferenceError> {
    let corners = world_corners(reference)?;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in corners {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Ok(WorldAabb {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn validate(reference: &RasterReference) -> Result<(), ReferenceError> {
    if reference.pixel_width == 0 || reference.pixel_height == 0 {
        return Err(ReferenceError::InvalidPixelDimensions {
            width: reference.pixel_width,
            height: reference.pixel_height,
        });
    }
    if !reference.world_width.is_finite()
        || !reference.world_height.is_finite()
        || reference.world_width <= 0.0
        || reference.world_height <= 0.0
    {
        return Err(ReferenceError::InvalidWorldDimensions {
            width: reference.world_width,
            height: reference.world_height,
        });
    }
    Ok(())
}

fn local_to_world(reference: &RasterReference, local_x: f64, local_y: f64) -> WorldPoint {
    let cosine = reference.rotation_rad.cos();
    let sine = reference.rotation_rad.sin();
    WorldPoint {
        x: reference.center_x + local_x * cosine - local_y * sine,
        y: reference.center_y + local_x * sine + local_y * cosine,
    }
}

fn world_to_local(reference: &RasterReference, world: WorldPoint) -> (f64, f64) {
    let dx = world.x - reference.center_x;
    let dy = world.y - reference.center_y;
    let cosine = reference.rotation_rad.cos();
    let sine = reference.rotation_rad.sin();
    (dx * cosine + dy * sine, -dx * sine + dy * cosine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;
    use uuid::Uuid;

    fn reference() -> RasterReference {
        let mut reference =
            RasterReference::new(Uuid::new_v4(), "reference.png", "image/png", 100, 50);
        reference.center_x = 1_000.0;
        reference.center_y = -2_000.0;
        reference.world_width = 200.0;
        reference.world_height = 100.0;
        reference
    }

    #[test]
    fn exact_pixel_center_round_trips_through_world_space() {
        let reference = reference();
        for pixel in [
            PixelCoord::new(0, 0),
            PixelCoord::new(49, 24),
            PixelCoord::new(99, 49),
        ] {
            let world = pixel_center_to_world(&reference, pixel).unwrap();
            assert_eq!(world_to_pixel(&reference, world).unwrap(), Some(pixel));
        }
    }

    #[test]
    fn rotation_does_not_break_pixel_identity() {
        let mut reference = reference();
        reference.rotation_rad = FRAC_PI_2;
        let pixel = PixelCoord::new(73, 11);
        let world = pixel_center_to_world(&reference, pixel).unwrap();
        assert_eq!(world_to_pixel(&reference, world).unwrap(), Some(pixel));
    }

    #[test]
    fn points_outside_reference_return_no_pixel() {
        let reference = reference();
        let outside = WorldPoint::new(reference.center_x + 500.0, reference.center_y);
        assert_eq!(world_to_pixel(&reference, outside).unwrap(), None);
    }

    #[test]
    fn world_corners_preserve_reference_orientation() {
        let mut reference = reference();
        reference.rotation_rad = FRAC_PI_2;
        let corners = world_corners(&reference).unwrap();
        assert!((corners[0].x - 1_050.0).abs() < 1e-9);
        assert!((corners[0].y + 2_100.0).abs() < 1e-9);
        assert!((corners[2].x - 950.0).abs() < 1e-9);
        assert!((corners[2].y + 1_900.0).abs() < 1e-9);
    }

    #[test]
    fn rotated_bounds_are_conservative_for_viewport_culling() {
        let mut reference = reference();
        reference.rotation_rad = FRAC_PI_2;
        let bounds = world_bounds(&reference).unwrap();
        assert!((bounds.max_x - bounds.min_x - 100.0).abs() < 1e-9);
        assert!((bounds.max_y - bounds.min_y - 200.0).abs() < 1e-9);
    }
}
