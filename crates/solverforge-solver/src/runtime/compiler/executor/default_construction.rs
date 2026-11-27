//! Staged execution of the omitted-construction profile.
//!
//! Each default boundary is resolved against the current working solution
//! and run through the same prepared dispatcher as explicit construction.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::context::list_access::ListAccess;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::runtime::compiler::DefaultConstructionStepKind;
use crate::scope::{ProgressCallback, SolverScope};

use super::execute::execute_prepared_construction;
use super::{
    DefaultConstructionStageExecutionRecord, DefaultRuntimeConstructionExecution,
    PreparedConstruction, PreparedDefaultRuntime, PreparedRuntimeExecution,
    ResolvedConstructionExecutionOutcome, ResolvedConstructionExecutionStep,
    RuntimeInstantiationError,
};

/// Executes the staged omitted-construction profile through the same prepared
/// dispatcher as an explicitly configured construction phase.
///
/// Each boundary is resolved against the current working solution. Structural
/// list declarations were registered before phase execution, but a reached
/// source-backed child binds and validates its source before deciding whether
/// work remains. Stage resolution can therefore choose Clarke-Wright,
/// assignment, or K-opt without rebuilding model metadata. In particular, the
/// post-construction K-opt boundary observes routes only after preceding
/// construction stages.
pub(crate) fn execute_prepared_default_construction<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    default: &PreparedDefaultRuntime<S, V, DM, IDM>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<DefaultRuntimeConstructionExecution, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut record = DefaultRuntimeConstructionExecution::default();
    for stage in crate::runtime::compiler::DefaultPreconstructionStage::ORDERED {
        let resolved =
            if stage == crate::runtime::compiler::DefaultPreconstructionStage::ListCompletion {
                let needs_completion = list_slots_needing_completion(
                    execution,
                    default,
                    record.stages.last(),
                    solver_scope.working_solution(),
                )?;
                super::super::defaults::resolve_default_list_completion(
                    &default.bindings,
                    &needs_completion,
                )
            } else {
                super::super::defaults::resolve_default_preconstruction_stage(
                    &default.bindings,
                    stage,
                    solver_scope.working_solution(),
                )
            };
        record.push(execute_resolved_default_construction_plan(
            execution,
            default,
            &resolved,
            solver_scope,
        )?);
    }

    let resolved = super::super::defaults::resolve_default_postconstruction_kopt(
        &default.bindings,
        solver_scope.working_solution(),
    );
    record.push(execute_resolved_default_construction_plan(
        execution,
        default,
        &resolved,
        solver_scope,
    )?);
    Ok(record)
}

/// Which list slots the list completion stage must place, from the list
/// construction stage that just ran: only Clarke-Wright can leave elements
/// unassigned without being terminated, so only a slot whose Clarke-Wright
/// child executed has its remaining source work read.
fn list_slots_needing_completion<S, V, DM, IDM, Extension>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    default: &PreparedDefaultRuntime<S, V, DM, IDM>,
    list_construction: Option<&DefaultConstructionStageExecutionRecord>,
    solution: &S,
) -> Result<Vec<bool>, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let steps = list_construction.map_or(&[][..], |stage| stage.steps.as_slice());
    let mut needs_completion = vec![false; default.bindings.list_slots.len()];
    for (needs, (slot, step)) in needs_completion
        .iter_mut()
        .zip(default.bindings.list_slots.iter().zip(steps))
    {
        let clarke_wright_ran = step.kind
            == DefaultConstructionStepKind::ListConstruction(
                solverforge_config::ConstructionHeuristicType::ListClarkeWright,
            )
            && step.outcome == ResolvedConstructionExecutionOutcome::Executed;
        if clarke_wright_ran {
            // The source is bound by the construction that just ran, so
            // comparing list lengths with it reads no element keys.
            *needs = match execution.bound_list_source_len(default.phase_index, slot)? {
                Some(source_len) => {
                    let assigned: usize = (0..ListAccess::entity_count(slot, solution))
                        .map(|entity| ListAccess::list_len(slot, solution, entity))
                        .sum();
                    assigned < source_len
                }
                None => {
                    execution.current_list_unassigned_count(default.phase_index, slot, solution)?
                        > 0
                }
            };
        }
    }
    Ok(needs_completion)
}

/// Share of a time-limited solve, in percent, after which the optional
/// assignment pass stops so local search keeps the rest of the budget. On
/// the rostering bench a fifth lets the pass finish in a 60 s solve, where a
/// tenth already costs quality, and still frees a 10 s solve for search.
/// Optional rows stay reachable to local search's optional-assignment moves.
const OPTIONAL_CONSTRUCTION_BUDGET_PERCENT: u32 = 20;

/// The optional pass's remaining share of the scope's time span, rounded up
/// to whole seconds, or `None` when no time limit applies. The span is the
/// solver-active time so far plus the remaining time, so a top-level solve
/// shares its time limit and a partition shares the time its inherited
/// deadline leaves it.
pub(super) fn optional_construction_termination<S, D, ProgressCb>(
    solver_scope: &SolverScope<'_, S, D, ProgressCb>,
) -> Option<solverforge_config::TerminationConfig>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let elapsed = solver_scope.elapsed().unwrap_or_default();
    let span = elapsed + solver_scope.remaining_time()?;
    let share = span * OPTIONAL_CONSTRUCTION_BUDGET_PERCENT / 100;
    let remaining = share.saturating_sub(elapsed);
    Some(solverforge_config::TerminationConfig {
        seconds_spent_limit: Some(remaining.as_millis().div_ceil(1000) as u64),
        ..solverforge_config::TerminationConfig::default()
    })
}

fn execute_resolved_default_construction_plan<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    default: &PreparedDefaultRuntime<S, V, DM, IDM>,
    resolved: &crate::runtime::compiler::ResolvedDefaultConstructionPlan<S, V, DM, IDM>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<DefaultConstructionStageExecutionRecord, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut steps = Vec::with_capacity(resolved.steps.len());
    for step in &resolved.steps {
        let mut construction =
            execution.prepare_resolved_construction(default.phase_index, &step.construction)?;
        let mut time_budget_seconds = None;
        if step.kind == DefaultConstructionStepKind::AssignmentOptional {
            if let PreparedConstruction::GroupedScalar { config, .. } = &mut construction {
                config.termination = optional_construction_termination(solver_scope);
                time_budget_seconds = config
                    .termination
                    .as_ref()
                    .and_then(|termination| termination.seconds_spent_limit);
            }
        }
        let execution = execute_prepared_construction(
            execution,
            &construction,
            step.required_only,
            solver_scope,
        )?;
        steps.push(ResolvedConstructionExecutionStep {
            kind: step.kind,
            reason: step.reason,
            required_only: step.required_only,
            target: step.target.clone(),
            list_policies: step.list_policies.clone(),
            time_budget_seconds,
            outcome: execution.outcome,
        });
    }
    let outcome = if steps
        .iter()
        .any(|step| step.outcome == ResolvedConstructionExecutionOutcome::Executed)
    {
        ResolvedConstructionExecutionOutcome::Executed
    } else if steps
        .iter()
        .any(|step| step.outcome == ResolvedConstructionExecutionOutcome::SkippedTerminated)
    {
        ResolvedConstructionExecutionOutcome::SkippedTerminated
    } else {
        ResolvedConstructionExecutionOutcome::SkippedNoWork
    };
    Ok(DefaultConstructionStageExecutionRecord {
        stage: resolved.stage,
        outcome,
        steps,
    })
}
