use super::ExplorationType;

#[derive(Debug, Clone)]
pub struct ExhaustiveSearchConfig {
    pub exploration_type: ExplorationType,
    pub node_limit: Option<u64>,
    pub depth_limit: Option<usize>,
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
