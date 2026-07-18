use std::fmt::{self, Debug};

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::{ScalarGroupBinding, ScalarGroupLimits};
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use crate::phase::construction::{
    BestFitForager, ConstructionForager, ConstructionHeuristicPhase, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

use super::placer::ScalarGroupPlacer;
use super::scalar_group_move_strength;

pub(crate) enum ScalarGroupConstruction<S>
where
    S: PlanningSolution,
{
    First(
        ConstructionHeuristicPhase<
            S,
            CompoundScalarMove<S>,
            ScalarGroupPlacer<S>,
            FirstFitForager<S, CompoundScalarMove<S>>,
        >,
    ),
    Best(
        ConstructionHeuristicPhase<
            S,
            CompoundScalarMove<S>,
            ScalarGroupPlacer<S>,
            BestFitForager<S, CompoundScalarMove<S>>,
        >,
    ),
    Weakest(
        ConstructionHeuristicPhase<
            S,
            CompoundScalarMove<S>,
            ScalarGroupPlacer<S>,
            WeakestFitForager<S, CompoundScalarMove<S>>,
        >,
    ),
    Strongest(
        ConstructionHeuristicPhase<
            S,
            CompoundScalarMove<S>,
            ScalarGroupPlacer<S>,
            StrongestFitForager<S, CompoundScalarMove<S>>,
        >,
    ),
}

impl<S> Debug for ScalarGroupConstruction<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::First(phase) => write!(f, "ScalarGroupConstruction::FirstFit({phase:?})"),
            Self::Best(phase) => write!(f, "ScalarGroupConstruction::BestFit({phase:?})"),
            Self::Weakest(phase) => {
                write!(f, "ScalarGroupConstruction::WeakestFit({phase:?})")
            }
            Self::Strongest(phase) => {
                write!(f, "ScalarGroupConstruction::StrongestFit({phase:?})")
            }
        }
    }
}

impl<S, D, ProgressCb> Phase<S, D, ProgressCb> for ScalarGroupConstruction<S>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::First(phase) => phase.solve(solver_scope),
            Self::Best(phase) => phase.solve(solver_scope),
            Self::Weakest(phase) => phase.solve(solver_scope),
            Self::Strongest(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "ScalarGroupConstruction"
    }
}

pub(crate) fn build_scalar_group_construction<S>(
    config: Option<&ConstructionHeuristicConfig>,
    group_index: usize,
    group: ScalarGroupBinding<S>,
    scalar_bindings: Vec<ResolvedVariableBinding<S>>,
    required_only: bool,
) -> ScalarGroupConstruction<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let construction_obligation = config
        .map(|cfg| cfg.construction_obligation)
        .unwrap_or_default();
    let limits = effective_group_limits(config, group.limits);
    let placer = ScalarGroupPlacer::new(
        group_index,
        group,
        scalar_bindings,
        limits,
        construction_type,
        construction_obligation,
        required_only,
    );

    match construction_type {
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::FirstFitDecreasing => {
            ScalarGroupConstruction::First(build_phase(
                placer,
                construction_obligation,
                FirstFitForager::new(),
            ))
        }
        ConstructionHeuristicType::CheapestInsertion => ScalarGroupConstruction::Best(build_phase(
            placer,
            construction_obligation,
            BestFitForager::new(),
        )),
        ConstructionHeuristicType::WeakestFit | ConstructionHeuristicType::WeakestFitDecreasing => {
            ScalarGroupConstruction::Weakest(build_phase(
                placer,
                construction_obligation,
                WeakestFitForager::new(scalar_group_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing => {
            ScalarGroupConstruction::Strongest(build_phase(
                placer,
                construction_obligation,
                StrongestFitForager::new(scalar_group_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue
        | ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => unreachable!(
            "grouped scalar construction only handles scalar grouped heuristics, got {:?}",
            construction_type
        ),
    }
}

/// Returns whether a grouped scalar construction phase has any unfinished
/// work in the current solution.
///
/// Construction and compiled execution use this one predicate so
/// an empty group keeps its established no-op boundary instead of creating a
/// second construction lifecycle.
pub(crate) fn scalar_group_work_remaining<S>(group: &ScalarGroupBinding<S>, solution: &S) -> bool
where
    S: PlanningSolution,
{
    if let Some(assignment) = group.assignment() {
        return assignment.unassigned_count(solution) > 0;
    }
    group.members.iter().any(|member| {
        (0..member.entity_count(solution))
            .any(|entity_index| member.current_value(solution, entity_index).is_none())
    })
}

/// Records the required-assignment frontier at the same boundaries as grouped
/// construction.  This is observed telemetry, not a second source of
/// construction state.
pub(crate) fn record_scalar_assignment_remaining<S, D, ProgressCb>(
    group: &ScalarGroupBinding<S>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if let Some(assignment) = group.assignment() {
        let remaining = assignment.remaining_required_count(solver_scope.working_solution());
        solver_scope
            .stats_mut()
            .record_scalar_assignment_required_remaining(group.group_name, remaining);
    }
}

fn build_phase<S, Fo>(
    placer: ScalarGroupPlacer<S>,
    construction_obligation: ConstructionObligation,
    forager: Fo,
) -> ConstructionHeuristicPhase<S, CompoundScalarMove<S>, ScalarGroupPlacer<S>, Fo>
where
    S: PlanningSolution + 'static,
    Fo: ConstructionForager<S, CompoundScalarMove<S>>,
{
    ConstructionHeuristicPhase::new(placer, forager)
        .with_construction_obligation(construction_obligation)
}

fn effective_group_limits(
    config: Option<&ConstructionHeuristicConfig>,
    group_limits: ScalarGroupLimits,
) -> ScalarGroupLimits {
    ScalarGroupLimits {
        value_candidate_limit: config
            .and_then(|cfg| cfg.value_candidate_limit)
            .or(group_limits.value_candidate_limit),
        group_candidate_limit: config
            .and_then(|cfg| cfg.group_candidate_limit)
            .or(group_limits.group_candidate_limit),
        max_moves_per_step: group_limits.max_moves_per_step,
        max_augmenting_depth: group_limits.max_augmenting_depth,
        max_rematch_size: group_limits.max_rematch_size,
    }
}
