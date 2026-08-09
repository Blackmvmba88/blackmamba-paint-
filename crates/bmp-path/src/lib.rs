use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn midpoint(self, other: Self) -> Self {
        Self {
            x: (self.x + other.x) * 0.5,
            y: (self.y + other.y) * 0.5,
        }
    }

    pub fn distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathNode {
    pub position: Vec2,
    pub in_handle: Option<Vec2>,
    pub out_handle: Option<Vec2>,
}

impl PathNode {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            in_handle: None,
            out_handle: None,
        }
    }

    fn reversed(mut self) -> Self {
        std::mem::swap(&mut self.in_handle, &mut self.out_handle);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditablePath {
    pub id: Uuid,
    pub nodes: Vec<PathNode>,
    pub closed: bool,
}

impl EditablePath {
    pub fn from_polyline(points: impl IntoIterator<Item = Vec2>) -> Self {
        Self {
            id: Uuid::new_v4(),
            nodes: points.into_iter().map(PathNode::new).collect(),
            closed: false,
        }
    }

    pub fn add_point(&mut self, index: usize, point: Vec2) -> Result<(), PathError> {
        if index > self.nodes.len() {
            return Err(PathError::InvalidIndex(index));
        }
        self.nodes.insert(index, PathNode::new(point));
        Ok(())
    }

    pub fn remove_point(&mut self, index: usize) -> Result<PathNode, PathError> {
        if index >= self.nodes.len() {
            return Err(PathError::InvalidIndex(index));
        }
        Ok(self.nodes.remove(index))
    }

    pub fn subdivide(&mut self, levels: u32) {
        for _ in 0..levels {
            self.subdivide_once();
        }
    }

    pub fn simplify(&mut self, tolerance: f64) {
        if self.nodes.len() <= 2 {
            return;
        }

        let tolerance = tolerance.max(0.0);
        self.nodes = if self.closed {
            simplify_closed(&self.nodes, tolerance)
        } else {
            ramer_douglas_peucker(&self.nodes, tolerance)
        };
    }

    pub fn divide_at(&self, index: usize) -> Result<(Self, Self), PathError> {
        if self.closed {
            return Err(PathError::CannotDivideClosedPath);
        }
        if index == 0 || index + 1 >= self.nodes.len() {
            return Err(PathError::InvalidDivideIndex(index));
        }

        Ok((
            Self {
                id: Uuid::new_v4(),
                nodes: self.nodes[..=index].to_vec(),
                closed: false,
            },
            Self {
                id: Uuid::new_v4(),
                nodes: self.nodes[index..].to_vec(),
                closed: false,
            },
        ))
    }

    pub fn joined(&self, other: &Self, tolerance: f64) -> Result<Self, PathError> {
        if self.closed || other.closed {
            return Err(PathError::CannotJoinClosedPath);
        }
        if self.nodes.is_empty() || other.nodes.is_empty() {
            return Err(PathError::EmptyPath);
        }

        let tolerance = tolerance.max(0.0);
        let a_first = self.nodes.first().expect("checked non-empty").position;
        let a_last = self.nodes.last().expect("checked non-empty").position;
        let b_first = other.nodes.first().expect("checked non-empty").position;
        let b_last = other.nodes.last().expect("checked non-empty").position;

        let mut candidates = [
            (a_last.distance(b_first), JoinMode::TailHead),
            (a_last.distance(b_last), JoinMode::TailTail),
            (a_first.distance(b_last), JoinMode::HeadTail),
            (a_first.distance(b_first), JoinMode::HeadHead),
        ];
        candidates.sort_by(|left, right| left.0.total_cmp(&right.0));

        let (distance, mode) = candidates[0];
        if distance > tolerance {
            return Err(PathError::EndpointsTooFar {
                distance,
                tolerance,
            });
        }

        let nodes = match mode {
            JoinMode::TailHead => append_without_duplicate(self.nodes.clone(), other.nodes.clone()),
            JoinMode::TailTail => {
                append_without_duplicate(self.nodes.clone(), reversed_nodes(&other.nodes))
            }
            JoinMode::HeadTail => append_without_duplicate(other.nodes.clone(), self.nodes.clone()),
            JoinMode::HeadHead => {
                append_without_duplicate(reversed_nodes(&self.nodes), other.nodes.clone())
            }
        };

        Ok(Self {
            id: Uuid::new_v4(),
            nodes,
            closed: false,
        })
    }

    fn subdivide_once(&mut self) {
        if self.nodes.len() < 2 {
            return;
        }

        if self.closed {
            let mut refined = Vec::with_capacity(self.nodes.len() * 2);
            for index in 0..self.nodes.len() {
                let current = self.nodes[index].clone();
                let next = self.nodes[(index + 1) % self.nodes.len()].clone();
                refined.push(current.clone());
                refined.push(PathNode::new(current.position.midpoint(next.position)));
            }
            self.nodes = refined;
            return;
        }

        let mut refined = Vec::with_capacity(self.nodes.len() * 2 - 1);
        for pair in self.nodes.windows(2) {
            let current = pair[0].clone();
            let next = pair[1].clone();
            refined.push(current.clone());
            refined.push(PathNode::new(current.position.midpoint(next.position)));
        }
        refined.push(self.nodes.last().expect("checked non-empty").clone());
        self.nodes = refined;
    }
}

#[derive(Debug, Clone, Copy)]
enum JoinMode {
    TailHead,
    TailTail,
    HeadTail,
    HeadHead,
}

#[derive(Debug, Error, PartialEq)]
pub enum PathError {
    #[error("invalid node index: {0}")]
    InvalidIndex(usize),
    #[error("cannot divide at node index: {0}")]
    InvalidDivideIndex(usize),
    #[error("a closed path requires two cuts before it can be divided")]
    CannotDivideClosedPath,
    #[error("join expects open paths")]
    CannotJoinClosedPath,
    #[error("cannot join an empty path")]
    EmptyPath,
    #[error("path endpoints are {distance:.3} units apart; tolerance is {tolerance:.3}")]
    EndpointsTooFar { distance: f64, tolerance: f64 },
}

fn append_without_duplicate(mut left: Vec<PathNode>, right: Vec<PathNode>) -> Vec<PathNode> {
    left.extend(right.into_iter().skip(1));
    left
}

fn reversed_nodes(nodes: &[PathNode]) -> Vec<PathNode> {
    nodes
        .iter()
        .cloned()
        .rev()
        .map(PathNode::reversed)
        .collect()
}

fn ramer_douglas_peucker(nodes: &[PathNode], tolerance: f64) -> Vec<PathNode> {
    if nodes.len() <= 2 {
        return nodes.to_vec();
    }

    let start = nodes.first().expect("checked length").position;
    let end = nodes.last().expect("checked length").position;
    let mut max_distance = 0.0;
    let mut split_index = 0;

    for (index, node) in nodes.iter().enumerate().take(nodes.len() - 1).skip(1) {
        let distance = point_segment_distance(node.position, start, end);
        if distance > max_distance {
            max_distance = distance;
            split_index = index;
        }
    }

    if max_distance <= tolerance {
        return vec![
            nodes.first().expect("checked length").clone(),
            nodes.last().expect("checked length").clone(),
        ];
    }

    let mut left = ramer_douglas_peucker(&nodes[..=split_index], tolerance);
    let right = ramer_douglas_peucker(&nodes[split_index..], tolerance);
    left.pop();
    left.extend(right);
    left
}

fn simplify_closed(nodes: &[PathNode], tolerance: f64) -> Vec<PathNode> {
    if nodes.len() <= 3 {
        return nodes.to_vec();
    }

    let mut kept = Vec::with_capacity(nodes.len());
    for index in 0..nodes.len() {
        let previous = nodes[(index + nodes.len() - 1) % nodes.len()].position;
        let current = nodes[index].position;
        let next = nodes[(index + 1) % nodes.len()].position;
        if point_segment_distance(current, previous, next) > tolerance {
            kept.push(nodes[index].clone());
        }
    }

    if kept.len() < 3 {
        nodes.to_vec()
    } else {
        kept
    }
}

fn point_segment_distance(point: Vec2, start: Vec2, end: Vec2) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return point.distance(start);
    }

    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    let projection = Vec2::new(start.x + t * dx, start.y + t * dy);
    point.distance(projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdivide_adds_control_without_changing_endpoints() {
        let mut path = EditablePath::from_polyline([
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(20.0, 0.0),
        ]);
        path.subdivide(1);

        assert_eq!(path.nodes.len(), 5);
        assert_eq!(path.nodes.first().unwrap().position, Vec2::new(0.0, 0.0));
        assert_eq!(path.nodes.last().unwrap().position, Vec2::new(20.0, 0.0));
    }

    #[test]
    fn simplify_removes_redundant_collinear_density() {
        let mut path = EditablePath::from_polyline([
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.01),
            Vec2::new(10.0, -0.01),
            Vec2::new(15.0, 0.0),
        ]);
        path.simplify(0.05);
        assert_eq!(path.nodes.len(), 2);
    }

    #[test]
    fn divide_and_join_restore_the_same_polyline_positions() {
        let path = EditablePath::from_polyline([
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(30.0, 5.0),
        ]);
        let original_positions: Vec<_> = path.nodes.iter().map(|node| node.position).collect();

        let (left, right) = path.divide_at(2).unwrap();
        let joined = left.joined(&right, 0.0).unwrap();
        let joined_positions: Vec<_> = joined.nodes.iter().map(|node| node.position).collect();

        assert_eq!(joined_positions, original_positions);
    }
}
