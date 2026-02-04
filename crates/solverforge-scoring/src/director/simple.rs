//! Simple score director with full recalculation.

use std::any::Any;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::traits::ScoreDirector;

/// A simple score director that recalculates the full score each time (zero-erasure).
///
/// The calculator is stored as a concrete generic type parameter, not as `Arc<dyn Fn>`.
/// This is inefficient but correct - used for testing and simple problems.
pub struct SimpleScoreDirector<S: PlanningSolution, C> {
    working_solution: S,
    solution_descriptor: SolutionDescriptor,
    score_calculator: C,
    score_dirty: bool,
    cached_score: Option<S::Score>,
}

impl<S, C> SimpleScoreDirector<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a new SimpleScoreDirector.
    pub fn new(solution: S, solution_descriptor: SolutionDescriptor, score_calculator: C) -> Self {
        SimpleScoreDirector {
            working_solution: solution,
            solution_descriptor,
            score_calculator,
            score_dirty: true,
            cached_score: None,
        }
    }

    /// Creates a SimpleScoreDirector with a simple closure.
    ///
    /// This is an alias for `new()` for backward compatibility.
    pub fn with_calculator(
        solution: S,
        solution_descriptor: SolutionDescriptor,
        calculator: C,
    ) -> Self {
        Self::new(solution, solution_descriptor, calculator)
    }

    fn mark_dirty(&mut self) {
        self.score_dirty = true;
    }
}

impl<S, C> ScoreDirector<S> for SimpleScoreDirector<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.mark_dirty();
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        if !self.score_dirty {
            if let Some(ref score) = self.cached_score {
                return *score;
            }
        }

        let score = (self.score_calculator)(&self.working_solution);
        self.working_solution.set_score(Some(score));
        self.cached_score = Some(score);
        self.score_dirty = false;
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(
        &mut self,
        _descriptor_index: usize,
        _entity_index: usize,
        _variable_name: &str,
    ) {
        self.mark_dirty();
    }

    fn after_variable_changed(
        &mut self,
        _descriptor_index: usize,
        _entity_index: usize,
        _variable_name: &str,
    ) {
        // Already marked dirty in before_variable_changed
    }

    fn trigger_variable_listeners(&mut self) {
        // No shadow variables in simple score director
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.solution_descriptor
            .entity_descriptors
            .get(descriptor_index)?
            .entity_count(&self.working_solution as &dyn Any)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.solution_descriptor
            .total_entity_count(&self.working_solution as &dyn Any)
    }

    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any> {
        self.solution_descriptor.get_entity(
            &self.working_solution as &dyn Any,
            descriptor_index,
            entity_index,
        )
    }

    fn is_incremental(&self) -> bool {
        false
    }

    fn reset(&mut self) {
        self.mark_dirty();
        self.cached_score = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use std::any::TypeId;

    #[derive(Clone, Debug, PartialEq)]
    struct Queen {
        id: i64,
        row: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct NQueensSolution {
        queens: Vec<Queen>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for NQueensSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
        &s.queens
    }

    fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
        &mut s.queens
    }

    fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
        let mut conflicts = 0i64;
        let queens = &solution.queens;

        for i in 0..queens.len() {
            for j in (i + 1)..queens.len() {
                if let (Some(row_i), Some(row_j)) = (queens[i].row, queens[j].row) {
                    if row_i == row_j {
                        conflicts += 1;
                    }
                    let col_diff = (j - i) as i32;
                    if (row_i - row_j).abs() == col_diff {
                        conflicts += 1;
                    }
                }
            }
        }

        SimpleScore::of(-conflicts)
    }

    fn create_test_descriptor() -> SolutionDescriptor {
        let extractor = Box::new(TypedEntityExtractor::new(
            "Queen",
            "queens",
            get_queens,
            get_queens_mut,
        ));
        let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
            .with_extractor(extractor);

        SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
            .with_entity(entity_desc)
    }

    #[test]
    fn test_simple_score_director_calculate_score() {
        let solution = NQueensSolution {
            queens: vec![
                Queen {
                    id: 0,
                    row: Some(0),
                },
                Queen {
                    id: 1,
                    row: Some(1),
                },
                Queen {
                    id: 2,
                    row: Some(2),
                },
                Queen {
                    id: 3,
                    row: Some(3),
                },
            ],
            score: None,
        };

        let descriptor = create_test_descriptor();
        let mut director =
            SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

        // All on diagonal = 6 diagonal conflicts
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(-6));
    }

    #[test]
    fn test_score_director_factory() {
        use super::super::factory::ScoreDirectorFactory;

        let solution = NQueensSolution {
            queens: vec![Queen {
                id: 0,
                row: Some(0),
            }],
            score: None,
        };

        let descriptor = create_test_descriptor();
        let factory = ScoreDirectorFactory::new(descriptor, calculate_conflicts);

        let mut director = factory.build_score_director(solution);
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(0));
    }
}
