use super::ExplorationType;

/// Settings of an exhaustive search phase.
#[derive(Debug, Clone)]
pub struct ExhaustiveSearchConfig {
    /// Order in which the search frontier is expanded. Default: depth first.
    pub exploration_type: ExplorationType,
    /// Stops after exploring this many nodes; `None` means no limit. Default:
    /// 10 000.
    pub node_limit: Option<u64>,
    /// Does not expand non-leaf nodes at or below this depth; `None` means no
    /// limit. Default: none.
    pub depth_limit: Option<usize>,
    /// Skips nodes whose optimistic bound cannot beat the best score. Default:
    /// `true`.
    pub enable_pruning: bool,
}

impl Default for ExhaustiveSearchConfig {
    fn default() -> Self {
        Self {
            exploration_type: ExplorationType::default(),
            node_limit: Some(10_000),
            depth_limit: None,
            enable_pruning: true,
        }
    }
}
