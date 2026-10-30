/// Order in which exhaustive search expands the frontier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExplorationType {
    /// Deepest node first.
    #[default]
    DepthFirst,
    /// Shallowest node first.
    BreadthFirst,
    /// Best-scoring node first.
    ScoreFirst,
    /// Node with the best optimistic bound first; unbounded nodes last.
    OptimisticBoundFirst,
}

impl std::fmt::Display for ExplorationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepthFirst => write!(f, "DepthFirst"),
            Self::BreadthFirst => write!(f, "BreadthFirst"),
            Self::ScoreFirst => write!(f, "ScoreFirst"),
            Self::OptimisticBoundFirst => write!(f, "OptimisticBoundFirst"),
        }
    }
}
