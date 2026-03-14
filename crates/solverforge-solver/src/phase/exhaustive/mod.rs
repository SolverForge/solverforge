/* Exhaustive search phase using branch-and-bound.

Exhaustive search explores the entire solution space systematically,
using pruning to avoid exploring branches that cannot improve on the
best solution found so far.

# Exploration Types

- **Depth First**: Explores deepest nodes first (memory efficient)
- **Breadth First**: Explores level by level (finds shortest paths)
- **Score First**: Explores best-scoring nodes first (greedy)
- **Optimistic Bound First**: Explores most promising bounds first (A*)
*/

mod bounder;
mod config;
mod decider;
mod exploration_type;
mod node;
mod phase;
mod priority_node;

pub use bounder::{BounderType, FixedOffsetBounder, ScoreBounder, SoftScoreBounder};
pub use config::ExhaustiveSearchConfig;
pub use decider::{ExhaustiveSearchDecider, SimpleDecider};
pub use exploration_type::ExplorationType;
pub use node::{ExhaustiveSearchNode, MoveSequence};
pub use phase::ExhaustiveSearchPhase;
