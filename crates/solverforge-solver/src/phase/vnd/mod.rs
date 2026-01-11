//! Variable Neighborhood Descent (VND) phase.
//!
//! VND systematically explores multiple neighborhood structures, restarting
//! from the first neighborhood whenever an improvement is found. This provides
//! a structured way to combine multiple move types for better optimization.
//!
//! # Algorithm
//!
//! 1. Start with neighborhood k = 0
//! 2. Find the best improving move in neighborhood k
//! 3. If improvement found: apply move, restart from k = 0
//! 4. If no improvement: move to k = k + 1
//! 5. Terminate when k exceeds the number of neighborhoods
//!
//! # Zero-Erasure Design
//!
//! Uses macro-generated tuple implementations for neighborhoods. Each neighborhood
//! is a concrete `MoveSelector` type, enabling full monomorphization.
//! Moves are never cloned - ownership transfers via `arena.take(index)`.

mod phase;

pub use phase::VndPhase;
pub use solverforge_scoring::ScoreDirector as ScoreDirectorTrait;
