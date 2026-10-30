//! Exhaustive search phase using branch-and-bound.
//!
//! Exhaustive search explores the entire solution space systematically,
//! using pruning to avoid exploring branches that cannot improve on the
//! best solution found so far.
//!
//! # Exploration Types
//!
//! - **Depth First**: Explores deepest nodes first (memory efficient)
//! - **Breadth First**: Explores level by level (finds shortest paths)
//! - **Score First**: Explores best-scoring nodes first (greedy)
//! - **Optimistic Bound First**: Explores most promising bounds first (A*)

pub(crate) mod bounder;
pub(crate) mod config;
pub(crate) mod decider;
pub(crate) mod exploration_type;
pub(crate) mod node;
pub(crate) mod phase;
mod priority_node;

pub(crate) use exploration_type::ExplorationType;
