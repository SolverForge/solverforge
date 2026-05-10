#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExplorationType {
    #[default]
    DepthFirst,
    BreadthFirst,
    ScoreFirst,
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
