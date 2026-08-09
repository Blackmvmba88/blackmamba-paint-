use bmp_path::{EditablePath, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PixelCoord {
    pub x: u32,
    pub y: u32,
}

impl PixelCoord {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x as f64, self.y as f64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pixels: Vec<Rgba>,
}

impl RasterImage {
    pub fn new(width: u32, height: u32, pixels: Vec<Rgba>) -> Result<Self, TraceError> {
        let expected = width
            .checked_mul(height)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(TraceError::ImageTooLarge)?;
        if pixels.len() != expected {
            return Err(TraceError::InvalidPixelCount {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn filled(width: u32, height: u32, color: Rgba) -> Result<Self, TraceError> {
        let count = width
            .checked_mul(height)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(TraceError::ImageTooLarge)?;
        Self::new(width, height, vec![color; count])
    }

    pub fn pixel(&self, coord: PixelCoord) -> Option<Rgba> {
        self.index_of(coord).map(|index| self.pixels[index])
    }

    pub fn set_pixel(&mut self, coord: PixelCoord, color: Rgba) -> Result<(), TraceError> {
        let index = self
            .index_of(coord)
            .ok_or(TraceError::PixelOutOfBounds(coord))?;
        self.pixels[index] = color;
        Ok(())
    }

    fn index_of(&self, coord: PixelCoord) -> Option<usize> {
        if coord.x >= self.width || coord.y >= self.height {
            return None;
        }
        let index = coord.y.checked_mul(self.width)?.checked_add(coord.x)?;
        usize::try_from(index).ok()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TraceConfig {
    pub channel_tolerance: u8,
    pub max_steps_per_direction: usize,
    pub ambiguity_margin: f64,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            channel_tolerance: 16,
            max_steps_per_direction: 100_000,
            ambiguity_margin: 0.12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceAmbiguity {
    pub at: PixelCoord,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceResult {
    pub path: EditablePath,
    pub seed: PixelCoord,
    pub seed_node_index: usize,
    pub confidence: f32,
    pub ambiguities: Vec<TraceAmbiguity>,
}

pub fn trace_line(
    image: &RasterImage,
    seed: PixelCoord,
    config: TraceConfig,
) -> Result<TraceResult, TraceError> {
    let seed_color = image.pixel(seed).ok_or(TraceError::SeedOutOfBounds(seed))?;
    let mut visited = BTreeSet::from([seed]);
    let mut ambiguities = Vec::new();
    let mut initial = candidates(image, seed, seed_color, &visited, config.channel_tolerance);

    if initial.len() > 2 {
        ambiguities.push(TraceAmbiguity {
            at: seed,
            candidate_count: initial.len(),
        });
    }

    initial.sort_by(|left, right| {
        color_cost(image.pixel(*left).unwrap_or(seed_color), seed_color)
            .total_cmp(&color_cost(
                image.pixel(*right).unwrap_or(seed_color),
                seed_color,
            ))
    });

    let first = initial.first().copied();
    let second = first.and_then(|first_coord| {
        let first_direction = direction(seed, first_coord);
        initial
            .iter()
            .copied()
            .skip(1)
            .min_by(|left, right| {
                cosine(first_direction, direction(seed, *left))
                    .total_cmp(&cosine(first_direction, direction(seed, *right)))
            })
    });

    let branch_a = if let Some(first_coord) = first {
        trace_branch(
            image,
            seed,
            first_coord,
            seed_color,
            config,
            &mut visited,
            &mut ambiguities,
        )
    } else {
        Vec::new()
    };

    let branch_b = if let Some(second_coord) = second {
        if visited.contains(&second_coord) {
            Vec::new()
        } else {
            trace_branch(
                image,
                seed,
                second_coord,
                seed_color,
                config,
                &mut visited,
                &mut ambiguities,
            )
        }
    } else {
        Vec::new()
    };

    let (coords, seed_node_index) = if branch_b.is_empty() {
        let mut coords = Vec::with_capacity(branch_a.len() + 1);
        coords.push(seed);
        coords.extend(branch_a);
        (coords, 0)
    } else {
        let mut coords = Vec::with_capacity(branch_a.len() + branch_b.len() + 1);
        coords.extend(branch_a.iter().rev().copied());
        let seed_node_index = coords.len();
        coords.push(seed);
        coords.extend(branch_b);
        (coords, seed_node_index)
    };

    let path = EditablePath::from_polyline(coords.into_iter().map(PixelCoord::as_vec2));
    let ambiguity_ratio = ambiguities.len() as f32 / path.nodes.len().max(1) as f32;
    let confidence = (1.0 - ambiguity_ratio * 0.5).clamp(0.0, 1.0);

    Ok(TraceResult {
        path,
        seed,
        seed_node_index,
        confidence,
        ambiguities,
    })
}

fn trace_branch(
    image: &RasterImage,
    previous: PixelCoord,
    first: PixelCoord,
    seed_color: Rgba,
    config: TraceConfig,
    visited: &mut BTreeSet<PixelCoord>,
    ambiguities: &mut Vec<TraceAmbiguity>,
) -> Vec<PixelCoord> {
    let mut branch = Vec::new();
    let mut previous = previous;
    let mut current = first;

    for _ in 0..config.max_steps_per_direction {
        if !visited.insert(current) {
            break;
        }
        branch.push(current);

        let previous_direction = direction(previous, current);
        let mut next = candidates(
            image,
            current,
            seed_color,
            visited,
            config.channel_tolerance,
        )
        .into_iter()
        .map(|coord| {
            let turn_penalty = 1.0 - cosine(previous_direction, direction(current, coord));
            let color_penalty = image
                .pixel(coord)
                .map(|color| color_cost(color, seed_color))
                .unwrap_or(1.0);
            (turn_penalty + color_penalty * 0.25, coord)
        })
        .collect::<Vec<_>>();

        if next.is_empty() {
            break;
        }

        next.sort_by(|left, right| left.0.total_cmp(&right.0));
        if next.len() > 1 && next[1].0 - next[0].0 <= config.ambiguity_margin {
            ambiguities.push(TraceAmbiguity {
                at: current,
                candidate_count: next.len(),
            });
        }

        previous = current;
        current = next[0].1;
    }

    branch
}

fn candidates(
    image: &RasterImage,
    center: PixelCoord,
    seed_color: Rgba,
    visited: &BTreeSet<PixelCoord>,
    tolerance: u8,
) -> Vec<PixelCoord> {
    let mut output = Vec::with_capacity(8);
    for dy in -1_i32..=1 {
        for dx in -1_i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let x = center.x as i64 + i64::from(dx);
            let y = center.y as i64 + i64::from(dy);
            if x < 0 || y < 0 {
                continue;
            }
            let Ok(x) = u32::try_from(x) else {
                continue;
            };
            let Ok(y) = u32::try_from(y) else {
                continue;
            };
            let coord = PixelCoord::new(x, y);
            if visited.contains(&coord) {
                continue;
            }
            if image
                .pixel(coord)
                .is_some_and(|color| similar_color(color, seed_color, tolerance))
            {
                output.push(coord);
            }
        }
    }
    output
}

fn similar_color(left: Rgba, right: Rgba, tolerance: u8) -> bool {
    left.r.abs_diff(right.r) <= tolerance
        && left.g.abs_diff(right.g) <= tolerance
        && left.b.abs_diff(right.b) <= tolerance
        && left.a.abs_diff(right.a) <= tolerance
}

fn color_cost(left: Rgba, right: Rgba) -> f64 {
    let total = u32::from(left.r.abs_diff(right.r))
        + u32::from(left.g.abs_diff(right.g))
        + u32::from(left.b.abs_diff(right.b))
        + u32::from(left.a.abs_diff(right.a));
    f64::from(total) / (4.0 * 255.0)
}

fn direction(from: PixelCoord, to: PixelCoord) -> (f64, f64) {
    (
        f64::from(to.x) - f64::from(from.x),
        f64::from(to.y) - f64::from(from.y),
    )
}

fn cosine(left: (f64, f64), right: (f64, f64)) -> f64 {
    let dot = left.0 * right.0 + left.1 * right.1;
    let left_length = (left.0 * left.0 + left.1 * left.1).sqrt();
    let right_length = (right.0 * right.0 + right.1 * right.1).sqrt();
    if left_length == 0.0 || right_length == 0.0 {
        return 0.0;
    }
    dot / (left_length * right_length)
}

#[derive(Debug, Error, PartialEq)]
pub enum TraceError {
    #[error("image dimensions are too large")]
    ImageTooLarge,
    #[error("expected {expected} pixels but received {actual}")]
    InvalidPixelCount { expected: usize, actual: usize },
    #[error("seed pixel is outside the image: {0:?}")]
    SeedOutOfBounds(PixelCoord),
    #[error("pixel is outside the image: {0:?}")]
    PixelOutOfBounds(PixelCoord),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_click_extracts_only_the_connected_local_line() {
        let mut image = RasterImage::filled(9, 5, Rgba::WHITE).unwrap();
        for x in 1..=5 {
            image
                .set_pixel(PixelCoord::new(x, 2), Rgba::BLACK)
                .unwrap();
        }
        image
            .set_pixel(PixelCoord::new(8, 0), Rgba::BLACK)
            .unwrap();

        let result = trace_line(
            &image,
            PixelCoord::new(3, 2),
            TraceConfig {
                channel_tolerance: 0,
                ..TraceConfig::default()
            },
        )
        .unwrap();

        let positions: Vec<_> = result
            .path
            .nodes
            .iter()
            .map(|node| node.position)
            .collect();
        assert_eq!(positions.len(), 5);
        assert_eq!(positions.first().unwrap(), &Vec2::new(1.0, 2.0));
        assert_eq!(positions.last().unwrap(), &Vec2::new(5.0, 2.0));
        assert_eq!(result.seed_node_index, 2);
    }

    #[test]
    fn junctions_are_reported_as_ambiguous_instead_of_hidden() {
        let mut image = RasterImage::filled(5, 5, Rgba::WHITE).unwrap();
        for coord in [
            PixelCoord::new(1, 2),
            PixelCoord::new(2, 2),
            PixelCoord::new(3, 2),
            PixelCoord::new(2, 1),
        ] {
            image.set_pixel(coord, Rgba::BLACK).unwrap();
        }

        let result = trace_line(
            &image,
            PixelCoord::new(2, 2),
            TraceConfig {
                channel_tolerance: 0,
                ..TraceConfig::default()
            },
        )
        .unwrap();

        assert!(!result.ambiguities.is_empty());
        assert!(result.confidence < 1.0);
    }
}
