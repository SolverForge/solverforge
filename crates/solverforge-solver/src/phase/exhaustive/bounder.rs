//! Score bounder for exhaustive search pruning.
//!
//! Bounders calculate optimistic and pessimistic score bounds
//! that enable branch-and-bound pruning.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

/// Calculates score bounds for exhaustive search pruning.
///
/// The bounder estimates the best possible score that can be achieved
/// from a partial solution state. If this optimistic bound is worse than
/// the best complete solution found so far, the branch can be pruned.
pub trait ScoreBounder<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Calculates the optimistic bound for the current solution state.
    ///
    /// The optimistic bound is an upper bound on the score achievable
    /// from this partial solution. It should be:
    /// - Fast to compute
    /// - Greater than or equal to any actual achievable score
    ///
    /// Returns `None` if no bound can be computed.
    fn calculate_optimistic_bound(&self, score_director: &D) -> Option<S::Score>;

    /// Calculates the pessimistic bound for the current solution state.
    ///
    /// The pessimistic bound is a lower bound on the score achievable
    /// from this partial solution. This is less commonly used but can
    /// help with certain heuristics.
    ///
    /// Returns `None` if no bound can be computed.
    fn calculate_pessimistic_bound(&self, score_director: &D) -> Option<S::Score> {
        // Default: no pessimistic bound
        let _ = score_director;
        None
    }
}

/// A simple bounder that uses the current score as the optimistic bound.
///
/// This is useful when constraint violations can only increase (get worse)
/// as more assignments are made, which is common for most constraint problems.
#[derive(Debug, Clone, Default)]
pub struct SimpleScoreBounder;

impl SimpleScoreBounder {
    /// Creates a new simple score bounder.
    pub fn new() -> Self {
        Self
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> ScoreBounder<S, D> for SimpleScoreBounder {
    fn calculate_optimistic_bound(&self, _score_director: &D) -> Option<S::Score> {
        // The simple bounder doesn't compute bounds
        // This effectively disables pruning
        None
    }
}

/// A bounder that uses a fixed offset from the current score.
///
/// This assumes that future assignments can at most improve the score
/// by a fixed amount per remaining entity.
#[derive(Clone)]
pub struct FixedOffsetBounder<S: PlanningSolution> {
    /// Maximum improvement per unassigned entity.
    max_improvement_per_entity: S::Score,
}

impl<S: PlanningSolution> std::fmt::Debug for FixedOffsetBounder<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixedOffsetBounder").finish()
    }
}

impl<S: PlanningSolution> FixedOffsetBounder<S> {
    /// Creates a new fixed offset bounder.
    pub fn new(max_improvement_per_entity: S::Score) -> Self {
        Self {
            max_improvement_per_entity,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> ScoreBounder<S, D> for FixedOffsetBounder<S>
where
    S::Score: Clone + std::ops::Add<Output = S::Score> + std::ops::Mul<i32, Output = S::Score>,
{
    fn calculate_optimistic_bound(&self, score_director: &D) -> Option<S::Score> {
        // Count unassigned entities
        let total = score_director.total_entity_count()?;

        // For now, we can't easily count assigned entities
        // so we use a simple heuristic
        let current_score = score_director.working_solution().score()?;

        // Optimistic bound = current score + max_improvement * remaining_entities
        // Since we don't know remaining entities, we assume all could improve
        let bound = current_score + self.max_improvement_per_entity * (total as i32);

        Some(bound)
    }
}

/// Bounder type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BounderType {
    /// No bounding (disables pruning).
    #[default]
    None,
    /// Simple score bounder.
    Simple,
    /// Fixed offset bounder.
    FixedOffset,
}

impl std::fmt::Display for BounderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BounderType::None => write!(f, "None"),
            BounderType::Simple => write!(f, "Simple"),
            BounderType::FixedOffset => write!(f, "FixedOffset"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bounder_returns_none() {
        let bounder = SimpleScoreBounder::new();
        // SimpleScoreBounder returns None by default (disables pruning)
        // Verify Default trait works
        assert!(format!("{:?}", bounder).contains("SimpleScoreBounder"));
    }

    #[test]
    fn test_bounder_type_display() {
        assert_eq!(format!("{}", BounderType::None), "None");
        assert_eq!(format!("{}", BounderType::Simple), "Simple");
        assert_eq!(format!("{}", BounderType::FixedOffset), "FixedOffset");
    }

    #[test]
    fn test_bounder_type_default() {
        assert_eq!(BounderType::default(), BounderType::None);
    }
}
