// Score assertion per environment mode: `fast_assert` checks scope-level
// score calculations, `full_assert` also every director calculation.

use std::panic::{catch_unwind, AssertUnwindSafe};

use solverforge_config::EnvironmentMode;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
use solverforge_scoring::stream::collection_extract::{source, ChangeSource};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector};

use crate::scope::SolverScope;

#[derive(Clone, Debug)]
struct Plan {
    values: Vec<Option<i64>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }
}

fn unassigned_for(_: &Plan) -> impl ConstraintSet<Plan, SoftScore> {
    unassigned()
}

fn unassigned() -> impl ConstraintSet<Plan, SoftScore> {
    (IncrementalUniConstraint::new(
        ConstraintRef::new("", "Unassigned"),
        ImpactType::Penalty,
        source(
            (|plan: &Plan| plan.values.as_slice()) as fn(&Plan) -> &[Option<i64>],
            ChangeSource::Descriptor(0),
        ),
        |_: &Plan, value: &Option<i64>| value.is_none(),
        |_: &Option<i64>| SoftScore::of(1),
        false,
    ),)
}

/// A scored scope whose working solution was then changed without the
/// variable-change notifications, so the incremental score is stale: it
/// still says -1 while a fresh calculation says -2.
fn drifted_scope(
    mode: EnvironmentMode,
) -> SolverScope<'static, Plan, impl Director<Plan> + 'static> {
    let descriptor: &'static SolutionDescriptor = Box::leak(Box::new(SolutionDescriptor::new(
        "",
        std::any::TypeId::of::<()>(),
    )));
    let solution = Plan {
        values: vec![Some(1), None, Some(2)],
        score: None,
    };
    let constraints = unassigned_for(&solution);
    let director =
        ScoreDirector::with_replicas(solution, constraints, unassigned_for, descriptor, |_, _| 0);
    let mut scope = SolverScope::new(director).with_environment_mode(mode);
    scope.start_solving();
    assert_eq!(scope.calculate_score(), SoftScore::of(-1));
    scope.score_director_mut().working_solution_mut().values[0] = None;
    scope
}

fn panics(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn modes_without_assertion_return_the_stale_score() {
    for mode in [
        EnvironmentMode::NonReproducible,
        EnvironmentMode::Reproducible,
    ] {
        let mut scope = drifted_scope(mode);
        assert_eq!(scope.calculate_score(), SoftScore::of(-1), "{mode:?}");
        assert_eq!(
            scope.score_director_mut().calculate_score(),
            SoftScore::of(-1),
            "{mode:?}"
        );
    }
}

#[test]
fn fast_assert_checks_scope_calculations_only() {
    let mut scope = drifted_scope(EnvironmentMode::FastAssert);
    // A move evaluation scores through the director directly.
    assert_eq!(
        scope.score_director_mut().calculate_score(),
        SoftScore::of(-1)
    );
    assert!(panics(|| {
        scope.calculate_score();
    }));
}

#[test]
fn full_assert_checks_every_director_calculation() {
    let mut scope = drifted_scope(EnvironmentMode::FullAssert);
    assert!(panics(|| {
        scope.score_director_mut().calculate_score();
    }));

    let mut scope = drifted_scope(EnvironmentMode::FullAssert);
    assert!(panics(|| {
        scope.calculate_score();
    }));
}

#[test]
fn full_assert_reaches_evaluator_replicas() {
    let scope = drifted_scope(EnvironmentMode::FullAssert);
    let mut replica = scope
        .score_director()
        .fork_replica()
        .expect("a descriptor director forks replicas");
    // The replica starts from the drifted solution with fresh state, so it
    // agrees with itself until it, too, is changed without notification.
    assert_eq!(replica.calculate_score(), SoftScore::of(-2));
    replica.working_solution_mut().values[2] = None;
    assert!(panics(|| {
        replica.calculate_score();
    }));
}
