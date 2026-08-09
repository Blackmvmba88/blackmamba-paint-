use bmp_core::Camera2D;
use bmp_path::EditablePath;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevealDirection {
    TowardStart,
    TowardEnd,
    Both,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RevealTiming {
    FixedDuration { duration_ms: u64 },
    UnitsPerSecond { units_per_second: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevealBranch {
    Seed,
    Start,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevealEvent {
    pub node_index: usize,
    pub at_ms: u64,
    pub branch: RevealBranch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevealPlan {
    pub seed_node_index: usize,
    pub duration_ms: u64,
    pub events: Vec<RevealEvent>,
}

impl RevealPlan {
    pub fn revealed_nodes_at(&self, at_ms: u64) -> Vec<usize> {
        self.events
            .iter()
            .filter(|event| event.at_ms <= at_ms)
            .map(|event| event.node_index)
            .collect()
    }
}

pub fn build_reveal_plan(
    path: &EditablePath,
    seed_node_index: usize,
    direction: RevealDirection,
    timing: RevealTiming,
) -> Result<RevealPlan, AnimationError> {
    if path.nodes.is_empty() {
        return Err(AnimationError::EmptyPath);
    }
    if seed_node_index >= path.nodes.len() {
        return Err(AnimationError::InvalidSeedIndex(seed_node_index));
    }
    if let RevealTiming::UnitsPerSecond { units_per_second } = timing {
        if !units_per_second.is_finite() || units_per_second <= 0.0 {
            return Err(AnimationError::NonPositiveSpeed(units_per_second));
        }
    }

    let mut raw = vec![RawRevealEvent {
        node_index: seed_node_index,
        distance: 0.0,
        branch: RevealBranch::Seed,
    }];

    if matches!(
        direction,
        RevealDirection::TowardStart | RevealDirection::Both
    ) {
        append_toward_start(path, seed_node_index, &mut raw);
    }
    if matches!(
        direction,
        RevealDirection::TowardEnd | RevealDirection::Both
    ) {
        append_toward_end(path, seed_node_index, &mut raw);
    }

    let max_distance = raw
        .iter()
        .map(|event| event.distance)
        .fold(0.0_f64, f64::max);

    let mut events = raw
        .into_iter()
        .map(|event| RevealEvent {
            node_index: event.node_index,
            at_ms: event_time_ms(event.distance, max_distance, timing),
            branch: event.branch,
        })
        .collect::<Vec<_>>();
    events.sort_by_key(|event| (event.at_ms, event.node_index));
    let duration_ms = events.iter().map(|event| event.at_ms).max().unwrap_or(0);

    Ok(RevealPlan {
        seed_node_index,
        duration_ms,
        events,
    })
}

#[derive(Debug, Clone, Copy)]
struct RawRevealEvent {
    node_index: usize,
    distance: f64,
    branch: RevealBranch,
}

fn append_toward_start(path: &EditablePath, seed: usize, output: &mut Vec<RawRevealEvent>) {
    let mut previous = path.nodes[seed].position;
    let mut distance = 0.0;
    for index in (0..seed).rev() {
        let current = path.nodes[index].position;
        distance += previous.distance(current);
        output.push(RawRevealEvent {
            node_index: index,
            distance,
            branch: RevealBranch::Start,
        });
        previous = current;
    }
}

fn append_toward_end(path: &EditablePath, seed: usize, output: &mut Vec<RawRevealEvent>) {
    let mut previous = path.nodes[seed].position;
    let mut distance = 0.0;
    for index in seed + 1..path.nodes.len() {
        let current = path.nodes[index].position;
        distance += previous.distance(current);
        output.push(RawRevealEvent {
            node_index: index,
            distance,
            branch: RevealBranch::End,
        });
        previous = current;
    }
}

fn event_time_ms(distance: f64, max_distance: f64, timing: RevealTiming) -> u64 {
    match timing {
        RevealTiming::FixedDuration { duration_ms } => {
            if max_distance <= f64::EPSILON {
                0
            } else {
                (distance / max_distance * duration_ms as f64).round() as u64
            }
        }
        RevealTiming::UnitsPerSecond { units_per_second } => {
            (distance / units_per_second * 1000.0).round() as u64
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CameraKeyframe {
    pub at_ms: u64,
    pub camera: Camera2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraTrack {
    pub keyframes: Vec<CameraKeyframe>,
}

impl CameraTrack {
    pub fn new(mut keyframes: Vec<CameraKeyframe>) -> Result<Self, AnimationError> {
        if keyframes.is_empty() {
            return Err(AnimationError::EmptyCameraTrack);
        }
        keyframes.sort_by_key(|keyframe| keyframe.at_ms);
        for pair in keyframes.windows(2) {
            if pair[0].at_ms == pair[1].at_ms {
                return Err(AnimationError::DuplicateCameraTime(pair[0].at_ms));
            }
        }
        Ok(Self { keyframes })
    }

    pub fn sample(&self, at_ms: u64) -> Camera2D {
        let first = self.keyframes.first().expect("camera track is non-empty");
        if at_ms <= first.at_ms {
            return first.camera;
        }
        let last = self.keyframes.last().expect("camera track is non-empty");
        if at_ms >= last.at_ms {
            return last.camera;
        }

        for pair in self.keyframes.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if at_ms >= start.at_ms && at_ms <= end.at_ms {
                let span = end.at_ms - start.at_ms;
                let t = (at_ms - start.at_ms) as f64 / span as f64;
                return Camera2D {
                    x: lerp(start.camera.x, end.camera.x, t),
                    y: lerp(start.camera.y, end.camera.y, t),
                    zoom: lerp(start.camera.zoom, end.camera.zoom, t),
                    rotation_rad: lerp(start.camera.rotation_rad, end.camera.rotation_rad, t),
                };
            }
        }

        last.camera
    }
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

#[derive(Debug, Error, PartialEq)]
pub enum AnimationError {
    #[error("cannot animate an empty path")]
    EmptyPath,
    #[error("invalid seed node index: {0}")]
    InvalidSeedIndex(usize),
    #[error("reveal speed must be finite and positive, got {0}")]
    NonPositiveSpeed(f64),
    #[error("camera track requires at least one keyframe")]
    EmptyCameraTrack,
    #[error("camera track contains duplicate keyframe time: {0} ms")]
    DuplicateCameraTime(u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_path::Vec2;

    #[test]
    fn reveal_from_click_reaches_both_ends_by_distance() {
        let path = EditablePath::from_polyline([
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(30.0, 0.0),
            Vec2::new(40.0, 0.0),
        ]);

        let plan = build_reveal_plan(
            &path,
            2,
            RevealDirection::Both,
            RevealTiming::FixedDuration { duration_ms: 1000 },
        )
        .unwrap();

        assert_eq!(plan.seed_node_index, 2);
        assert_eq!(plan.duration_ms, 1000);
        assert!(plan.events.contains(&RevealEvent {
            node_index: 1,
            at_ms: 500,
            branch: RevealBranch::Start,
        }));
        assert!(plan.events.contains(&RevealEvent {
            node_index: 3,
            at_ms: 500,
            branch: RevealBranch::End,
        }));
        assert!(plan
            .events
            .iter()
            .any(|event| event.node_index == 0 && event.at_ms == 1000));
        assert!(plan
            .events
            .iter()
            .any(|event| event.node_index == 4 && event.at_ms == 1000));
    }

    #[test]
    fn constant_speed_uses_real_path_distance() {
        let path = EditablePath::from_polyline([
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(30.0, 0.0),
        ]);
        let plan = build_reveal_plan(
            &path,
            0,
            RevealDirection::TowardEnd,
            RevealTiming::UnitsPerSecond {
                units_per_second: 10.0,
            },
        )
        .unwrap();

        assert_eq!(plan.events[1].at_ms, 1000);
        assert_eq!(plan.events[2].at_ms, 3000);
    }

    #[test]
    fn camera_track_interpolates_pan_zoom_and_rotation() {
        let track = CameraTrack::new(vec![
            CameraKeyframe {
                at_ms: 0,
                camera: Camera2D {
                    x: 0.0,
                    y: 0.0,
                    zoom: 1.0,
                    rotation_rad: 0.0,
                },
            },
            CameraKeyframe {
                at_ms: 1000,
                camera: Camera2D {
                    x: 100.0,
                    y: -50.0,
                    zoom: 3.0,
                    rotation_rad: 1.0,
                },
            },
        ])
        .unwrap();

        let sample = track.sample(500);
        assert_eq!(sample.x, 50.0);
        assert_eq!(sample.y, -25.0);
        assert_eq!(sample.zoom, 2.0);
        assert_eq!(sample.rotation_rad, 0.5);
    }
}
