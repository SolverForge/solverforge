//! Consolidated tests for director module.
//!
//! Tests extracted from:
//! - typed.rs (11 tests)
//! - simple.rs (2 tests)
//! - shadow_aware.rs (3 tests)

use std::any::TypeId;

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::shadow_aware::{ShadowAwareScoreDirector, ShadowVariableSupport};
use crate::director::simple::SimpleScoreDirector;
use crate::director::typed::TypedScoreDirector;
use crate::director::ScoreDirector;

// ============================================================================
// TypedScoreDirector test fixtures
// ============================================================================

#[derive(Clone, Debug)]
struct TestSolution {
    values: Vec<Option<i32>>,
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

fn make_unassigned_constraint() -> impl IncrementalConstraint<TestSolution, SimpleScore> {
    IncrementalUniConstraint::new(
        ConstraintRef::new("", "Unassigned"),
        ImpactType::Penalty,
        |s: &TestSolution| s.values.as_slice(),
        |_s: &TestSolution, v: &Option<i32>| v.is_none(),
        |_v: &Option<i32>| SimpleScore::of(1),
        false,
    )
}

// ============================================================================
// TypedScoreDirector tests
// ============================================================================

#[test]
fn test_typed_initial_score_calculation() {
    let solution = TestSolution {
        values: vec![Some(1), None, None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    assert!(!director.is_initialized());
    let score = director.calculate_score();
    assert!(director.is_initialized());
    assert_eq!(score, SimpleScore::of(-2)); // 2 None values
}

#[test]
fn test_typed_cached_score_on_subsequent_calls() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    let score1 = director.calculate_score();
    let score2 = director.calculate_score();
    assert_eq!(score1, score2);
    assert_eq!(score1, SimpleScore::of(-1));
}

#[test]
fn test_typed_incremental_update() {
    let solution = TestSolution {
        values: vec![Some(1), None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    // Initialize
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(-1)); // One None at index 1

    // Change: None -> Some(3) at index 1
    // descriptor_index=0 since TestSolution has a single entity class
    director.before_variable_changed(0, 1);
    director.working_solution_mut().values[1] = Some(3);
    director.after_variable_changed(0, 1);

    // Score should improve (no more unassigned)
    let new_score = director.get_score();
    assert_eq!(new_score, SimpleScore::of(0));
}

#[test]
fn test_typed_do_change_convenience() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    director.calculate_score();

    // descriptor_index=0 since TestSolution has a single entity class
    let new_score = director.do_change(0, 1, |s| {
        s.values[1] = Some(5);
    });

    assert_eq!(new_score, SimpleScore::of(0));
}

#[test]
fn test_typed_reset() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    director.calculate_score();
    assert!(director.is_initialized());

    director.reset();
    assert!(!director.is_initialized());
    assert_eq!(director.get_score(), SimpleScore::of(0)); // Zero after reset
}

#[test]
fn test_typed_clone_working_solution() {
    let solution = TestSolution {
        values: vec![Some(1), None], // One unassigned = penalty of -1
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    // Calculate score first
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(-1));

    // Clone and verify score is propagated
    let cloned = director.clone_working_solution();
    assert_eq!(cloned.values.len(), 2);
    assert_eq!(cloned.values[0], Some(1));
    assert_eq!(cloned.score, Some(SimpleScore::of(-1)));
}

#[test]
fn test_typed_constraint_count() {
    let solution = TestSolution {
        values: vec![],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let director = TypedScoreDirector::new(solution, (c1,));

    assert_eq!(director.constraint_count(), 1);
}

#[test]
fn test_typed_multiple_constraints() {
    let solution = TestSolution {
        values: vec![Some(1), None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();

    // Second constraint: reward assigned values
    let c2 = IncrementalUniConstraint::new(
        ConstraintRef::new("", "Assigned"),
        ImpactType::Reward,
        |s: &TestSolution| s.values.as_slice(),
        |_s: &TestSolution, v: &Option<i32>| v.is_some(),
        |_v: &Option<i32>| SimpleScore::of(1),
        false,
    );

    let mut director = TypedScoreDirector::new(solution, (c1, c2));

    assert_eq!(director.constraint_count(), 2);

    // Score: -1 (one None) + 2 (two Some) = 1
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(1));
}

#[test]
fn test_typed_debug_impl() {
    let solution = TestSolution {
        values: vec![Some(1)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let director = TypedScoreDirector::new(solution, (c1,));

    let debug = format!("{:?}", director);
    assert!(debug.contains("TypedScoreDirector"));
    assert!(debug.contains("initialized"));
}

#[test]
fn test_typed_before_change_without_initialization() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    // Call before/after without initialization - should not panic
    // descriptor_index=0 since TestSolution has a single entity class
    director.before_variable_changed(0, 0);
    director.after_variable_changed(0, 0);

    // Score should be calculated correctly on first call
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(-1));
}

#[test]
fn test_typed_add_then_remove_value() {
    let solution = TestSolution {
        values: vec![None, None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = TypedScoreDirector::new(solution, (c1,));

    // Initialize: 2 Nones = -2
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(-2));

    // Assign first value: 1 None = -1
    // descriptor_index=0 since TestSolution has a single entity class
    director.do_change(0, 0, |s| s.values[0] = Some(1));
    assert_eq!(director.get_score(), SimpleScore::of(-1));

    // Unassign first value: back to 2 Nones = -2
    director.do_change(0, 0, |s| s.values[0] = None);
    assert_eq!(director.get_score(), SimpleScore::of(-2));
}

// ============================================================================
// SimpleScoreDirector test fixtures
// ============================================================================

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

fn create_nqueens_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens").with_extractor(extractor);

    SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc)
}

// ============================================================================
// SimpleScoreDirector tests
// ============================================================================

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

    let descriptor = create_nqueens_descriptor();
    let mut director =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    // All on diagonal = 6 diagonal conflicts
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(-6));
}

#[test]
fn test_simple_score_director_factory() {
    use crate::director::ScoreDirectorFactory;

    let solution = NQueensSolution {
        queens: vec![Queen {
            id: 0,
            row: Some(0),
        }],
        score: None,
    };

    let descriptor = create_nqueens_descriptor();
    let factory = ScoreDirectorFactory::new(descriptor, calculate_conflicts);

    let mut director = factory.build_score_director(solution);
    let score = director.calculate_score();
    assert_eq!(score, SimpleScore::of(0));
}

// ============================================================================
// ShadowAwareScoreDirector test fixtures
// ============================================================================

#[derive(Clone, Debug)]
struct ShadowSolution {
    values: Vec<i32>,
    cached_sum: i32,
    score: Option<SimpleScore>,
}

impl PlanningSolution for ShadowSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl ShadowVariableSupport for ShadowSolution {
    fn update_entity_shadows(&mut self, _entity_index: usize) {
        self.cached_sum = self.values.iter().sum();
    }
}

fn make_sum_constraint() -> IncrementalUniConstraint<
    ShadowSolution,
    i32,
    fn(&ShadowSolution) -> &[i32],
    fn(&ShadowSolution, &i32) -> bool,
    fn(&i32) -> SimpleScore,
    SimpleScore,
> {
    fn extract(s: &ShadowSolution) -> &[i32] {
        std::slice::from_ref(&s.cached_sum)
    }
    fn filter(_s: &ShadowSolution, &sum: &i32) -> bool {
        sum > 100
    }
    fn score(&sum: &i32) -> SimpleScore {
        SimpleScore::of((sum - 100) as i64)
    }

    IncrementalUniConstraint::new(
        ConstraintRef::new("", "SumLimit"),
        ImpactType::Penalty,
        extract as fn(&ShadowSolution) -> &[i32],
        filter as fn(&ShadowSolution, &i32) -> bool,
        score as fn(&i32) -> SimpleScore,
        false,
    )
}

// ============================================================================
// ShadowAwareScoreDirector tests
// ============================================================================

#[test]
fn test_shadow_update_called_on_variable_change() {
    let solution = ShadowSolution {
        values: vec![10, 20, 30],
        cached_sum: 0, // Will be updated by shadow
        score: None,
    };

    let constraint = make_sum_constraint();
    let inner = TypedScoreDirector::new(solution, (constraint,));
    let mut director = ShadowAwareScoreDirector::new(inner);

    // Initialize
    director.calculate_score();

    // Shadow should have been updated during initialization
    // (via working_solution_mut access pattern)
    assert_eq!(director.working_solution().cached_sum, 0);

    // Change value and verify shadow update
    director.before_variable_changed(0, 0, "values");
    director.working_solution_mut().values[0] = 50;
    director.after_variable_changed(0, 0, "values");

    assert_eq!(director.working_solution().cached_sum, 100); // 50 + 20 + 30
}

#[test]
fn test_shadow_inner_access() {
    let solution = ShadowSolution {
        values: vec![1, 2, 3],
        cached_sum: 0,
        score: None,
    };

    let constraint = make_sum_constraint();
    let inner = TypedScoreDirector::new(solution, (constraint,));
    let director = ShadowAwareScoreDirector::new(inner);

    assert!(!director.inner().is_initialized());
}

#[test]
fn test_shadow_into_inner_consumes() {
    let solution = ShadowSolution {
        values: vec![1],
        cached_sum: 0,
        score: None,
    };

    let constraint = make_sum_constraint();
    let inner = TypedScoreDirector::new(solution, (constraint,));
    let director = ShadowAwareScoreDirector::new(inner);

    let recovered = director.into_inner();
    assert_eq!(recovered.working_solution().values.len(), 1);
}
