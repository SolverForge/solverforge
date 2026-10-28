// Weight-mode switching requested by applied problem changes.

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ConstraintWeights, ImpactType};
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
use solverforge_scoring::stream::collection_extract::{source, ChangeSource};
use solverforge_scoring::{ConstraintSet, ScoreDirector, UnweightedMode, WeightMode, WeightedMode};

use crate::realtime::{ClosureProblemChange, ProblemChange};
use crate::runtime_build_error::RuntimeBuildError;
use crate::scope::{SolverScope, WeightModeControl};

#[derive(Clone, Debug)]
struct Plan {
    values: Vec<Option<i64>>,
    weights: ConstraintWeights,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    const HAS_CONSTRAINT_WEIGHTS: bool = true;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }

    fn constraint_weights(&self) -> Option<&ConstraintWeights> {
        Some(&self.weights)
    }
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

fn plan(multiplier: Option<u32>) -> Plan {
    let mut weights = ConstraintWeights::new();
    if let Some(multiplier) = multiplier {
        weights.set("Unassigned", multiplier);
    }
    Plan {
        values: vec![Some(1), None, None],
        weights,
        score: None,
    }
}

fn set_unassigned(multiplier: Option<u32>) -> Vec<Box<dyn ProblemChange<Plan>>> {
    vec![Box::new(ClosureProblemChange::<Plan, _>::new(
        "reweight",
        move |director| director.working_solution_mut().weights = plan(multiplier).weights,
    ))]
}

type ModeDirector<C, M> = ScoreDirector<Plan, C, SolutionDescriptor, M>;

fn scope<C, M>(
    constraints: C,
    multiplier: Option<u32>,
    control: WeightModeControl,
) -> SolverScope<'static, Plan, ModeDirector<C, M>>
where
    C: ConstraintSet<Plan, SoftScore> + Send,
    M: WeightMode,
{
    let director = ModeDirector::<C, M>::with_descriptor_in_mode(
        plan(multiplier),
        constraints,
        SolutionDescriptor::new("", std::any::TypeId::of::<()>()),
        |_, _| 0,
    );
    let mut scope = SolverScope::new(director);
    scope.set_weight_mode_control(control);
    scope.start_solving();
    scope.calculate_score();
    scope
}

#[test]
fn change_to_unit_weights_defers_the_rescore_to_an_unweighted_director() {
    let mut scope = scope::<_, WeightedMode>(
        unassigned(),
        Some(3),
        WeightModeControl::Switchable { weighted: true },
    );
    assert_eq!(scope.current_score(), Some(&SoftScore::of(-6)));

    let rescored = scope
        .apply_problem_changes(set_unassigned(Some(1)))
        .expect("unit weights are valid");
    assert_eq!(rescored, None, "the rescore waits for the other director");
    assert!(scope.weight_mode_switch_pending());

    let mut scope =
        scope.switch_score_director(|director| director.into_weight_mode::<UnweightedMode>());
    assert!(!scope.weight_mode_switch_pending());
    assert_eq!(scope.current_score(), Some(&SoftScore::of(-2)));

    // Back up: the control now records an unweighted director.
    let rescored = scope
        .apply_problem_changes(set_unassigned(Some(4)))
        .expect("weights within the cap are valid");
    assert_eq!(rescored, None);
    assert!(scope.weight_mode_switch_pending());
    let scope = scope.switch_score_director(|director| director.into_weight_mode::<WeightedMode>());
    assert_eq!(scope.current_score(), Some(&SoftScore::of(-8)));
}

#[test]
fn change_within_the_running_mode_rescores_in_place() {
    let mut scope = scope::<_, UnweightedMode>(
        unassigned(),
        None,
        WeightModeControl::Switchable { weighted: false },
    );
    let rescored = scope
        .apply_problem_changes(set_unassigned(Some(1)))
        .expect("unit weights are valid");
    assert_eq!(rescored, Some(SoftScore::of(-2)));
    assert!(!scope.weight_mode_switch_pending());
}

#[test]
fn unweighted_only_solve_fails_a_change_that_needs_weighting() {
    let mut scope =
        scope::<_, UnweightedMode>(unassigned(), None, WeightModeControl::UnweightedOnly);
    let error = scope
        .apply_problem_changes(set_unassigned(Some(2)))
        .expect_err("an unweighted-only solve cannot apply multipliers");
    assert!(matches!(error, RuntimeBuildError::ConstraintWeights { .. }));
    assert!(!scope.weight_mode_switch_pending());
}

#[test]
fn fixed_mode_never_switches() {
    let mut scope = scope::<_, UnweightedMode>(unassigned(), None, WeightModeControl::Fixed);
    let rescored = scope
        .apply_problem_changes(set_unassigned(Some(2)))
        .expect("a fixed mode applies the change");
    assert_eq!(
        rescored,
        Some(SoftScore::of(-2)),
        "unweighted scoring ignores weights"
    );
    assert!(!scope.weight_mode_switch_pending());
}
