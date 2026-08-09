use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct SparseSpatialIndex {
    entries: BTreeMap<Uuid, Bounds>,
}

impl SparseSpatialIndex {
    pub fn insert(&mut self, id: Uuid, bounds: Bounds) {
        self.entries.insert(id, bounds);
    }

    pub fn remove(&mut self, id: &Uuid) -> Option<Bounds> {
        self.entries.remove(id)
    }

    pub fn bounds_of(&self, id: &Uuid) -> Option<Bounds> {
        self.entries.get(id).copied()
    }

    pub fn query(&self, viewport: Bounds) -> Vec<Uuid> {
        self.entries
            .iter()
            .filter_map(|(id, bounds)| bounds.intersects(&viewport).then_some(*id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distant_content_does_not_appear_in_viewport_query() {
        let near = Uuid::new_v4();
        let far = Uuid::new_v4();
        let mut index = SparseSpatialIndex::default();
        index.insert(near, Bounds::new(-10.0, -10.0, 10.0, 10.0));
        index.insert(
            far,
            Bounds::new(98_000_000.0, -12_000_000.0, 98_000_100.0, -11_999_900.0),
        );

        let visible = index.query(Bounds::new(-50.0, -50.0, 50.0, 50.0));
        assert_eq!(visible, vec![near]);
    }
}
