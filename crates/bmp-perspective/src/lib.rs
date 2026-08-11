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

impl LineObservation {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            weight: 1.0,
        }
    }

    fn is_finite(self) -> bool {
        self.x1.is_finite()
            && self.y1.is_finite()
            && self.x2.is_finite()
            && self.y2.is_finite()
            && self.weight.is_finite()
    }
}

/// Contract for automatic or assisted perspective inference.
/// A deterministic geometric implementation can live beside an AI-assisted one.
pub trait PerspectiveInference {
    fn infer(
        &self,
        lines: &[LineObservation],
        projection_hint: ProjectionMode,
    ) -> PerspectiveSolution;
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PerspectiveConfig {
    /// Lines whose homogeneous determinant is smaller than this are treated as parallel.
    pub parallel_epsilon: f64,
    /// Pairwise intersections closer than this distance vote for the same vanishing point.
    pub cluster_radius: f64,
    /// Maximum number of VP candidates returned to the caller.
    pub max_vanishing_points: usize,
    /// Minimum number of pairwise-intersection votes required for a VP candidate.
    pub min_cluster_support: usize,
}

impl Default for PerspectiveConfig {
    fn default() -> Self {
        Self {
            parallel_epsilon: 1.0e-9,
            cluster_radius: 12.0,
            max_vanishing_points: 3,
            min_cluster_support: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GeometricPerspectiveInference {
    pub config: PerspectiveConfig,
}

impl PerspectiveInference for GeometricPerspectiveInference {
    fn infer(
        &self,
        lines: &[LineObservation],
        projection_hint: ProjectionMode,
    ) -> PerspectiveSolution {
        let valid_lines = lines
            .iter()
            .copied()
            .filter(|line| line.is_finite() && line_length_squared(*line) > f64::EPSILON)
            .collect::<Vec<_>>();

        if valid_lines.len() < 2 {
            let mut result = PerspectiveSolution::empty(projection_hint);
            result.explanation = format!(
                "Need at least two valid line observations; received {}.",
                valid_lines.len()
            );
            return result;
        }

        let intersections = pairwise_intersections(&valid_lines, self.config.parallel_epsilon);
        if intersections.is_empty() {
            let mut result = PerspectiveSolution::empty(projection_hint);
            result.explanation = "Observed lines are parallel or degenerate; no finite vanishing point could be inferred.".into();
            return result;
        }

        let mut clusters =
            cluster_intersections(&intersections, self.config.cluster_radius.max(0.0));
        clusters.retain(|cluster| cluster.support >= self.config.min_cluster_support.max(1));

        if clusters.is_empty() {
            let mut result = PerspectiveSolution::empty(projection_hint);
            result.explanation = format!(
                "Computed {} finite line intersections, but none received enough repeated votes to form a reliable vanishing point.",
                intersections.len()
            );
            result.confidence = 0.1;
            return result;
        }

        let max_support = clusters
            .iter()
            .map(|cluster| cluster.support)
            .max()
            .unwrap_or(1) as f32;

        clusters.sort_by(|left, right| {
            right
                .support
                .cmp(&left.support)
                .then_with(|| right.weight.total_cmp(&left.weight))
                .then_with(|| left.x.total_cmp(&right.x))
                .then_with(|| left.y.total_cmp(&right.y))
        });

        let vanishing_points = clusters
            .iter()
            .take(self.config.max_vanishing_points.max(1))
            .map(|cluster| VanishingPoint {
                x: cluster.x,
                y: cluster.y,
                confidence: (cluster.support as f32 / max_support).clamp(0.0, 1.0),
            })
            .collect::<Vec<_>>();

        let horizon = infer_horizon(&vanishing_points, self.config.cluster_radius);
        let vp_confidence = vanishing_points.iter().map(|vp| vp.confidence).sum::<f32>()
            / vanishing_points.len() as f32;
        let horizon_bonus = horizon.map_or(0.0, |value| value.confidence * 0.25);
        let support_coverage = clusters
            .iter()
            .take(self.config.max_vanishing_points.max(1))
            .map(|cluster| cluster.support)
            .sum::<usize>() as f32
            / intersections.len() as f32;
        let confidence = (vp_confidence * 0.6 + support_coverage.min(1.0) * 0.15 + horizon_bonus)
            .clamp(0.0, 1.0);

        let explanation = match (vanishing_points.len(), horizon) {
            (1, _) => format!(
                "Detected one dominant vanishing point from {} valid lines and {} finite intersections.",
                valid_lines.len(),
                intersections.len()
            ),
            (_, Some(horizon)) => format!(
                "Detected {} vanishing-point candidates and a probable horizon at y={:.3} from repeated geometric convergence.",
                vanishing_points.len(),
                horizon.y
            ),
            _ => format!(
                "Detected {} vanishing-point candidates; their vertical spread is too large for a confident shared horizon.",
                vanishing_points.len()
            ),
        };

        PerspectiveSolution {
            projection: projection_hint,
            horizon,
            vanishing_points,
            field_of_view_deg: None,
            camera_pitch_deg: None,
            camera_roll_deg: None,
            explanation,
            confidence,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct IntersectionVote {
    x: f64,
    y: f64,
    weight: f64,
}

#[derive(Debug, Clone, Copy)]
struct IntersectionCluster {
    x: f64,
    y: f64,
    weight: f64,
    support: usize,
}

fn line_length_squared(line: LineObservation) -> f64 {
    let dx = line.x2 - line.x1;
    let dy = line.y2 - line.y1;
    dx * dx + dy * dy
}

fn pairwise_intersections(lines: &[LineObservation], epsilon: f64) -> Vec<IntersectionVote> {
    let epsilon = epsilon.abs().max(f64::EPSILON);
    let mut output = Vec::new();
    for left_index in 0..lines.len() {
        for right_index in left_index + 1..lines.len() {
            if let Some((x, y)) = line_intersection(lines[left_index], lines[right_index], epsilon)
            {
                let weight = f64::from(
                    lines[left_index]
                        .weight
                        .max(0.0)
                        .min(lines[right_index].weight.max(0.0)),
                )
                .max(f64::EPSILON);
                output.push(IntersectionVote { x, y, weight });
            }
        }
    }
    output
}

fn line_intersection(
    left: LineObservation,
    right: LineObservation,
    epsilon: f64,
) -> Option<(f64, f64)> {
    let left_a = left.y1 - left.y2;
    let left_b = left.x2 - left.x1;
    let left_c = left.x1 * left.y2 - left.x2 * left.y1;
    let right_a = right.y1 - right.y2;
    let right_b = right.x2 - right.x1;
    let right_c = right.x1 * right.y2 - right.x2 * right.y1;

    let determinant = left_a * right_b - right_a * left_b;
    if determinant.abs() <= epsilon {
        return None;
    }

    let x = (left_b * right_c - right_b * left_c) / determinant;
    let y = (left_c * right_a - right_c * left_a) / determinant;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

fn cluster_intersections(votes: &[IntersectionVote], radius: f64) -> Vec<IntersectionCluster> {
    let radius_squared = radius * radius;
    let mut clusters: Vec<IntersectionCluster> = Vec::new();

    for vote in votes {
        let nearest = clusters
            .iter()
            .enumerate()
            .filter_map(|(index, cluster)| {
                let dx = cluster.x - vote.x;
                let dy = cluster.y - vote.y;
                let distance_squared = dx * dx + dy * dy;
                (distance_squared <= radius_squared).then_some((index, distance_squared))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index);

        if let Some(index) = nearest {
            let cluster = &mut clusters[index];
            let new_weight = cluster.weight + vote.weight;
            cluster.x = (cluster.x * cluster.weight + vote.x * vote.weight) / new_weight;
            cluster.y = (cluster.y * cluster.weight + vote.y * vote.weight) / new_weight;
            cluster.weight = new_weight;
            cluster.support += 1;
        } else {
            clusters.push(IntersectionCluster {
                x: vote.x,
                y: vote.y,
                weight: vote.weight,
                support: 1,
            });
        }
    }

    clusters
}

fn infer_horizon(vanishing_points: &[VanishingPoint], radius: f64) -> Option<Horizon> {
    if vanishing_points.len() < 2 {
        return None;
    }

    let first = vanishing_points[0];
    let second = vanishing_points[1];
    let vertical_spread = (first.y - second.y).abs();
    let scale = radius.max(1.0) * 4.0;
    let alignment = (1.0 - vertical_spread / scale).clamp(0.0, 1.0) as f32;
    if alignment <= 0.05 {
        return None;
    }

    let confidence = (alignment * first.confidence.min(second.confidence)).clamp(0.0, 1.0);
    Some(Horizon {
        y: (first.y + second.y) * 0.5,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Test Helpers ==========

    fn line_through(point: (f64, f64), vanishing_point: (f64, f64)) -> LineObservation {
        LineObservation::new(point.0, point.1, vanishing_point.0, vanishing_point.1)
    }

    // ========== Projection Mode Tests ==========

    #[test]
    fn projection_is_explicit_in_perspective_solution() {
        let result = StubPerspectiveInference.infer(&[], ProjectionMode::Spherical);
        assert_eq!(result.projection, ProjectionMode::Spherical);
    }

    // ========== Geometric Solver Tests ==========

    #[test]
    fn geometric_solver_recovers_two_vanishing_points_and_horizon() {
        let left_vp = (-100.0, 50.0);
        let right_vp = (300.0, 50.0);
        let lines = vec![
            line_through((0.0, 0.0), left_vp),
            line_through((30.0, 120.0), left_vp),
            line_through((80.0, -20.0), left_vp),
            line_through((20.0, 10.0), right_vp),
            line_through((90.0, 130.0), right_vp),
            line_through((120.0, -20.0), right_vp),
        ];

        let solver = GeometricPerspectiveInference {
            config: PerspectiveConfig {
                cluster_radius: 2.0,
                max_vanishing_points: 2,
                min_cluster_support: 2,
                ..PerspectiveConfig::default()
            },
        };
        let result = solver.infer(&lines, ProjectionMode::InfiniteFlat);

        assert_eq!(result.vanishing_points.len(), 2);
        let mut points = result.vanishing_points.clone();
        points.sort_by(|left, right| left.x.total_cmp(&right.x));
        assert!((points[0].x - left_vp.0).abs() < 1.0e-6);
        assert!((points[0].y - left_vp.1).abs() < 1.0e-6);
        assert!((points[1].x - right_vp.0).abs() < 1.0e-6);
        assert!((points[1].y - right_vp.1).abs() < 1.0e-6);

        let horizon = result.horizon.expect("two horizontal VPs imply a horizon");
        assert!((horizon.y - 50.0).abs() < 1.0e-6);
        assert!(horizon.confidence > 0.9);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn geometric_solver_rejects_parallel_lines_without_fake_finite_vp() {
        let lines = vec![
            LineObservation::new(0.0, 0.0, 100.0, 0.0),
            LineObservation::new(0.0, 10.0, 100.0, 10.0),
            LineObservation::new(0.0, 20.0, 100.0, 20.0),
        ];
        let result =
            GeometricPerspectiveInference::default().infer(&lines, ProjectionMode::InfiniteFlat);

        assert!(result.vanishing_points.is_empty());
        assert!(result.horizon.is_none());
        assert_eq!(result.confidence, 0.0);
    }
}
