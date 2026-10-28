// Tests for ScoreDirector.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::score_director::{ReplicaDescriptor, ScoreDirector};
use crate::stream::collection_extract::{source, ChangeSource};
use crate::Director;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use std::any::TypeId;

#[derive(Clone, Debug)]
struct TestSolution {
    values: Vec<Option<i32>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn make_unassigned_constraint() -> impl IncrementalConstraint<TestSolution, SoftScore> {
    IncrementalUniConstraint::new(
        ConstraintRef::new("", "Unassigned"),
        ImpactType::Penalty,
        source(
            (|s: &TestSolution| s.values.as_slice()) as fn(&TestSolution) -> &[Option<i32>],
            ChangeSource::Descriptor(0),
        ),
        |_s: &TestSolution, v: &Option<i32>| v.is_none(),
        |_v: &Option<i32>| SoftScore::of(1),
        false,
    )
}

fn make_named_constraint(
    constraint_ref: ConstraintRef,
    is_hard: bool,
) -> impl IncrementalConstraint<TestSolution, SoftScore> {
    IncrementalUniConstraint::new(
        constraint_ref,
        ImpactType::Penalty,
        source(
            (|s: &TestSolution| s.values.as_slice()) as fn(&TestSolution) -> &[Option<i32>],
            ChangeSource::Descriptor(0),
        ),
        |_s: &TestSolution, _v: &Option<i32>| false,
        |_v: &Option<i32>| SoftScore::of(1),
        is_hard,
    )
}

#[test]
fn test_initial_score_calculation() {
    let solution = TestSolution {
        values: vec![Some(1), None, None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    assert!(!director.is_initialized());
    let score = director.calculate_score();
    assert!(director.is_initialized());
    assert_eq!(score, SoftScore::of(-2)); // 2 None values
}

#[test]
fn test_cached_score_on_subsequent_calls() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    let score1 = director.calculate_score();
    let score2 = director.calculate_score();
    assert_eq!(score1, score2);
    assert_eq!(score1, SoftScore::of(-1));
}

#[test]
fn test_incremental_update() {
    let solution = TestSolution {
        values: vec![Some(1), None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    // Initialize
    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(-1)); // One None at index 1

    // Change: None -> Some(3) at index 1
    // descriptor_index=0 since TestSolution has a single entity class
    director.before_variable_changed(0, 1);
    director.working_solution_mut().values[1] = Some(3);
    director.after_variable_changed(0, 1);

    // Score should improve (no more unassigned)
    let new_score = director.get_score();
    assert_eq!(new_score, SoftScore::of(0));
}

#[test]
fn test_do_change_convenience() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    director.calculate_score();

    // descriptor_index=0 since TestSolution has a single entity class
    let new_score = director.do_change(0, 1, |s| {
        s.values[1] = Some(5);
    });

    assert_eq!(new_score, SoftScore::of(0));
}

#[test]
fn test_reset() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    director.calculate_score();
    assert!(director.is_initialized());

    director.reset();
    assert!(!director.is_initialized());
    assert_eq!(director.get_score(), SoftScore::of(0)); // Zero after reset
}

#[test]
fn test_clone_working_solution() {
    let solution = TestSolution {
        values: vec![Some(1), None], // One unassigned = penalty of -1
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    // Calculate score first
    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(-1));

    // Clone and verify score is propagated
    let cloned = director.clone_working_solution();
    assert_eq!(cloned.values.len(), 2);
    assert_eq!(cloned.values[0], Some(1));
    assert_eq!(cloned.score, Some(SoftScore::of(-1)));
}

#[test]
fn test_constraint_count() {
    let solution = TestSolution {
        values: vec![],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let director = ScoreDirector::new(solution, (c1,));

    assert_eq!(director.constraint_count(), 1);
}

#[test]
fn score_director_preserves_package_qualified_constraint_metadata() {
    let solution = TestSolution {
        values: vec![Some(1)],
        score: None,
    };
    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
    let c1 = make_named_constraint(ConstraintRef::new("pkg_a", "same"), true);
    let c2 = make_named_constraint(ConstraintRef::new("pkg_b", "same"), false);
    let director =
        ScoreDirector::with_descriptor(solution, (c1, c2), descriptor, |s, _| s.values.len());
    let metadata = director.constraint_metadata();

    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0].full_name(), "pkg_a/same");
    assert_eq!(metadata[1].full_name(), "pkg_b/same");
    assert_eq!(
        director.constraint_is_hard(&ConstraintRef::new("pkg_a", "same")),
        Some(true)
    );
    assert_eq!(
        director.constraint_is_hard(&ConstraintRef::new("pkg_b", "same")),
        Some(false)
    );
    assert_eq!(
        director.constraint_is_hard(&ConstraintRef::new("", "same")),
        None
    );
}

#[test]
fn test_multiple_constraints() {
    let solution = TestSolution {
        values: vec![Some(1), None, Some(2)],
        score: None,
    };

    let c1 = make_unassigned_constraint();

    // Second constraint: reward assigned values
    let c2 = IncrementalUniConstraint::new(
        ConstraintRef::new("", "Assigned"),
        ImpactType::Reward,
        source(
            (|s: &TestSolution| s.values.as_slice()) as fn(&TestSolution) -> &[Option<i32>],
            ChangeSource::Descriptor(0),
        ),
        |_s: &TestSolution, v: &Option<i32>| v.is_some(),
        |_v: &Option<i32>| SoftScore::of(1),
        false,
    );

    let mut director = ScoreDirector::new(solution, (c1, c2));

    assert_eq!(director.constraint_count(), 2);

    // Score: -1 (one None) + 2 (two Some) = 1
    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(1));
}

#[test]
fn test_debug_impl() {
    let solution = TestSolution {
        values: vec![Some(1)],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let director = ScoreDirector::new(solution, (c1,));

    let debug = format!("{:?}", director);
    assert!(debug.contains("ScoreDirector"));
    assert!(debug.contains("initialized"));
}

#[test]
fn test_before_change_without_initialization() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    // Call before/after without initialization - should not panic
    // descriptor_index=0 since TestSolution has a single entity class
    director.before_variable_changed(0, 0);
    director.after_variable_changed(0, 0);

    // Score should be calculated correctly on first call
    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(-1));
}

#[test]
fn test_add_then_remove_value() {
    let solution = TestSolution {
        values: vec![None, None],
        score: None,
    };

    let c1 = make_unassigned_constraint();
    let mut director = ScoreDirector::new(solution, (c1,));

    // Initialize: 2 Nones = -2
    let score = director.calculate_score();
    assert_eq!(score, SoftScore::of(-2));

    // Assign first value: 1 None = -1
    // descriptor_index=0 since TestSolution has a single entity class
    director.do_change(0, 0, |s| s.values[0] = Some(1));
    assert_eq!(director.get_score(), SoftScore::of(-1));

    // Unassign first value: back to 2 Nones = -2
    director.do_change(0, 0, |s| s.values[0] = None);
    assert_eq!(director.get_score(), SoftScore::of(-2));
}

fn test_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
}

fn unassigned_constraints(
    _solution: &TestSolution,
) -> (impl IncrementalConstraint<TestSolution, SoftScore>,) {
    (make_unassigned_constraint(),)
}

/* Penalizes each unassigned value by the value count observed when the set
was built, so a stale set scores a resized solution with the old weight. */
fn solution_weighted_constraints(
    solution: &TestSolution,
) -> (impl IncrementalConstraint<TestSolution, SoftScore>,) {
    let weight = solution.values.len() as i64;
    (IncrementalUniConstraint::new(
        ConstraintRef::new("", "Weighted unassigned"),
        ImpactType::Penalty,
        source(
            (|s: &TestSolution| s.values.as_slice()) as fn(&TestSolution) -> &[Option<i32>],
            ChangeSource::Descriptor(0),
        ),
        |_s: &TestSolution, v: &Option<i32>| v.is_none(),
        move |_v: &Option<i32>| SoftScore::of(weight),
        false,
    ),)
}

#[test]
fn reset_rebuilds_replica_constraints_from_the_rewritten_solution() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };
    let descriptor = test_descriptor();
    let mut director = ScoreDirector::with_descriptor(
        solution.clone(),
        solution_weighted_constraints(&solution),
        ReplicaDescriptor::new(&descriptor, solution_weighted_constraints),
        |_, _| 0,
    );
    assert_eq!(director.calculate_score(), SoftScore::of(-2));

    director.working_solution_mut().values.push(None);
    director.reset();

    assert!(!director.is_initialized());
    assert_eq!(director.calculate_score(), SoftScore::of(-6));
    assert_eq!(Director::fresh_score(&director), Some(SoftScore::of(-6)));
}

#[test]
fn fork_replica_requires_a_replica_descriptor() {
    let solution = TestSolution {
        values: vec![Some(1), None],
        score: None,
    };
    let mut director = ScoreDirector::new(solution, (make_unassigned_constraint(),));
    director.calculate_score();

    assert!(director.fork_replica().is_none());
}

#[test]
fn fork_replica_matches_score_and_stays_independent() {
    let solution = TestSolution {
        values: vec![Some(1), None, None],
        score: None,
    };
    let descriptor = test_descriptor();
    let mut director = ScoreDirector::with_descriptor(
        solution.clone(),
        unassigned_constraints(&solution),
        ReplicaDescriptor::new(&descriptor, unassigned_constraints),
        |_, _| 0,
    );
    let main_score = director.calculate_score();

    let mut replica = director
        .fork_replica()
        .expect("director with a constraint constructor must fork");
    assert!(replica.is_initialized());
    assert_eq!(replica.get_score(), main_score);
    assert!(
        std::ptr::eq(
            Director::solution_descriptor(&replica),
            Director::solution_descriptor(&director)
        ),
        "a replica must share the descriptor, not copy it"
    );

    replica.before_variable_changed(0, 1);
    replica.working_solution_mut().values[1] = Some(7);
    replica.after_variable_changed(0, 1);

    assert_eq!(replica.get_score(), SoftScore::of(-1));
    assert_eq!(director.get_score(), SoftScore::of(-2));
    assert_eq!(director.working_solution().values[1], None);
}

#[test]
fn fork_of_uninitialized_director_initializes_the_replica() {
    let solution = TestSolution {
        values: vec![None, Some(3), None, None],
        score: None,
    };
    let descriptor = test_descriptor();
    let director = ScoreDirector::with_descriptor(
        solution.clone(),
        unassigned_constraints(&solution),
        ReplicaDescriptor::new(&descriptor, unassigned_constraints),
        |_, _| 0,
    );
    assert!(!director.is_initialized());

    let replica = director
        .fork_replica()
        .expect("director with a constraint constructor must fork");

    assert!(replica.is_initialized());
    assert_eq!(replica.get_score(), SoftScore::of(-3));
    assert!(!director.is_initialized());
}
