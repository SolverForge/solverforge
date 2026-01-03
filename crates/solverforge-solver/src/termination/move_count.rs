//! Move count termination.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when a maximum number of moves have been evaluated.
///
/// This termination condition requires a `StatisticsCollector` to be attached
/// to the `SolverScope`. If no collector is attached, it will never terminate.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::MoveCountTermination;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Terminate after evaluating 100,000 moves
/// let termination = MoveCountTermination::<MySolution>::new(100_000);
/// ```
#[derive(Clone)]
pub struct MoveCountTermination<S: PlanningSolution> {
    /// Maximum number of moves before termination.
    move_count_limit: u64,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for MoveCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveCountTermination")
            .field("move_count_limit", &self.move_count_limit)
            .finish()
    }
}

impl<S: PlanningSolution> MoveCountTermination<S> {
    /// Creates a new move count termination.
    ///
    /// # Arguments
    /// * `move_count_limit` - Maximum moves to evaluate before terminating
    pub fn new(move_count_limit: u64) -> Self {
        Self {
            move_count_limit,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: PlanningSolution> Termination<S> for MoveCountTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool {
        if let Some(stats) = solver_scope.statistics() {
            stats.current_moves_evaluated() >= self.move_count_limit
        } else {
            false // No statistics collector, never terminate based on this
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::StatisticsCollector;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    struct Entity {
        value: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TestSolution {
        entities: Vec<Entity>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_entities(s: &TestSolution) -> &Vec<Entity> {
        &s.entities
    }
    fn get_entities_mut(s: &mut TestSolution) -> &mut Vec<Entity> {
        &mut s.entities
    }

    fn create_scope_with_stats() -> SolverScope<TestSolution> {
        let solution = TestSolution {
            entities: vec![],
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Entity",
            "entities",
            get_entities,
            get_entities_mut,
        ));
        let entity_desc = EntityDescriptor::new("Entity", TypeId::of::<Entity>(), "entities")
            .with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
            .with_entity(entity_desc);
        let director =
            SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0));
        let collector = Arc::new(StatisticsCollector::<SimpleScore>::new());
        SolverScope::new(Box::new(director)).with_statistics(collector)
    }

    #[test]
    fn test_not_terminated_initially() {
        let scope = create_scope_with_stats();
        let termination = MoveCountTermination::<TestSolution>::new(100);
        assert!(!termination.is_terminated(&scope));
    }

    #[test]
    fn test_terminates_at_limit() {
        let scope = create_scope_with_stats();
        let stats = scope.statistics().unwrap().clone();

        // Record 99 moves - not yet terminated
        for _ in 0..99 {
            stats.record_move_evaluated();
        }
        let termination = MoveCountTermination::<TestSolution>::new(100);
        assert!(!termination.is_terminated(&scope));

        // Record one more - now terminated
        stats.record_move_evaluated();
        assert!(termination.is_terminated(&scope));
    }

    #[test]
    fn test_no_stats_never_terminates() {
        let solution = TestSolution {
            entities: vec![],
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Entity",
            "entities",
            get_entities,
            get_entities_mut,
        ));
        let entity_desc = EntityDescriptor::new("Entity", TypeId::of::<Entity>(), "entities")
            .with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
            .with_entity(entity_desc);
        let director =
            SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0));
        let scope = SolverScope::new(Box::new(director));

        let termination = MoveCountTermination::<TestSolution>::new(1);
        // Without statistics collector, never terminates
        assert!(!termination.is_terminated(&scope));
    }
}
