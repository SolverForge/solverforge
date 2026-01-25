//! K-opt configuration and distance meter types.
//!
//! This module provides configuration and distance calculation utilities
//! for k-opt move generation.

use std::fmt::Debug;

/// Configuration for k-opt move generation.
#[derive(Debug, Clone)]
pub struct KOptConfig {
    /// The k value (2-5).
    pub k: usize,
    /// Minimum segment length between cuts (default: 1).
    pub min_segment_len: usize,
    /// Whether to use only a subset of reconnection patterns.
    pub limited_patterns: bool,
}

impl KOptConfig {
    /// Creates a new k-opt configuration.
    ///
    /// # Panics
    ///
    /// Panics if k < 2 or k > 5.
    pub fn new(k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        Self {
            k,
            min_segment_len: 1,
            limited_patterns: false,
        }
    }

    /// Sets minimum segment length between cuts.
    pub fn with_min_segment_len(mut self, len: usize) -> Self {
        self.min_segment_len = len;
        self
    }

    /// Enables limited pattern mode (faster but less thorough).
    pub fn with_limited_patterns(mut self, limited: bool) -> Self {
        self.limited_patterns = limited;
        self
    }
}

/// A distance meter for list element positions.
///
/// Measures distance between elements at two positions in a list.
/// Used by nearby k-opt move generation to limit search space.
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
