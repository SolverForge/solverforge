//! Score calculation count termination.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when a maximum number of score calculations is reached.
///
/// This termination condition requires a `StatisticsCollector` to be attached
/// to the `SolverScope`. If no collector is attached, it will never terminate.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::ScoreCalculationCountTermination;
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
/// // Terminate after 10,000 score calculations
/// let termination = ScoreCalculationCountTermination::<MySolution>::new(10_000);
/// ```
#[derive(Clone)]
pub struct ScoreCalculationCountTermination<S: PlanningSolution> {
    /// Maximum number of score calculations before termination.
    score_calculation_count_limit: u64,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for ScoreCalculationCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreCalculationCountTermination")
            .field("score_calculation_count_limit", &self.score_calculation_count_limit)
            .finish()
    }
}

impl<S: PlanningSolution> ScoreCalculationCountTermination<S> {
    /// Creates a new score calculation count termination.
    ///
    /// # Arguments
    /// * `score_calculation_count_limit` - Maximum score calculations before terminating
    pub fn new(score_calculation_count_limit: u64) -> Self {
        Self {
            score_calculation_count_limit,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: PlanningSolution> Termination<S> for ScoreCalculationCountTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool {
        if let Some(stats) = solver_scope.statistics() {
            stats.current_score_calculations() >= self.score_calculation_count_limit
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
        let termination = ScoreCalculationCountTermination::<TestSolution>::new(100);
        assert!(!termination.is_terminated(&scope));
    }

    #[test]
    fn test_terminates_at_limit() {
        let scope = create_scope_with_stats();
        let stats = scope.statistics().unwrap().clone();

        // Record 99 calculations - not yet terminated
        for _ in 0..99 {
            stats.record_score_calculation();
        }
        let termination = ScoreCalculationCountTermination::<TestSolution>::new(100);
        assert!(!termination.is_terminated(&scope));

        // Record one more - now terminated
        stats.record_score_calculation();
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

        let termination = ScoreCalculationCountTermination::<TestSolution>::new(1);
        // Without statistics collector, never terminates
        assert!(!termination.is_terminated(&scope));
    }
}
