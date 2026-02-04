//! Distance meters for k-opt nearby selection.

use std::fmt::Debug;

/// A distance meter for list element positions.
///
/// Measures distance between elements at two positions in a list.
/// Used by `NearbyKOptMoveSelector` to limit k-opt search space.
pub trait ListPositionDistanceMeter<S>: Send + Sync + Debug {
    /// Measures distance between elements at two positions in the same entity.
    fn distance(&self, solution: &S, entity_idx: usize, pos_a: usize, pos_b: usize) -> f64;
}

/// Default distance meter using position difference.
#[derive(Debug, Clone, Copy)]
pub struct DefaultDistanceMeter;

impl<S> ListPositionDistanceMeter<S> for DefaultDistanceMeter {
    fn distance(&self, _solution: &S, _entity_idx: usize, pos_a: usize, pos_b: usize) -> f64 {
        (pos_a as f64 - pos_b as f64).abs()
    }
}
