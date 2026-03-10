//! Exhaustive search phase configuration.

use super::exploration_type::ExplorationType;

/// Configuration for exhaustive search phase.
#[derive(Debug, Clone)]
pub struct ExhaustiveSearchConfig {
    /// The exploration type to use.
    pub exploration_type: ExplorationType,
    /// Maximum number of nodes to explore (None = unlimited).
    pub node_limit: Option<u64>,
    /// Maximum depth to explore (None = unlimited).
    pub depth_limit: Option<usize>,
    /// Whether to enable pruning based on bounds.
    pub enable_pruning: bool,
}

impl Default for ExhaustiveSearchConfig {
    fn default() -> Self {
        Self {
            exploration_type: ExplorationType::DepthFirst,
            node_limit: Some(10_000),
            depth_limit: None,
            enable_pruning: true,
        }
    }
}
