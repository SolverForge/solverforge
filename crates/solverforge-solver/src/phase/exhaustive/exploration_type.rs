// Exploration type for exhaustive search.

// Type of exploration strategy for exhaustive search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExplorationType {
    // Depth-first search: explores deepest nodes first.
    // Most memory efficient, but may take longer to find optimal.
    #[default]
    DepthFirst,

    // Breadth-first search: explores level by level.
    // Memory intensive, but finds shortest solution paths.
    BreadthFirst,

    // Score-first search: explores best-scoring nodes first.
    // Greedy approach that may find good solutions quickly.
    ScoreFirst,

    // Optimistic bound first: explores most promising nodes first.
    // A*-like behavior, requires a good bounder.
    OptimisticBoundFirst,
}

impl std::fmt::Display for ExplorationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExplorationType::DepthFirst => write!(f, "DepthFirst"),
            ExplorationType::BreadthFirst => write!(f, "BreadthFirst"),
            ExplorationType::ScoreFirst => write!(f, "ScoreFirst"),
            ExplorationType::OptimisticBoundFirst => write!(f, "OptimisticBoundFirst"),
        }
    }
}
