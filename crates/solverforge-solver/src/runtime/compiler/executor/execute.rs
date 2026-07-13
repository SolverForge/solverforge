//! Canonical execution of prepared construction nodes.
//!
//! This module consumes only frozen graph data and solve-owned prepared
//! sources.  It does not rediscover a model, open a declaration callback, or
//! assemble a second phase tree. Every list branch delegates to the one
//! shared construction kernel, including the distinct Clarke-Wright
//! savings/merge/completion path.

use std::fmt;

use solverforge_config::ConstructionHeuristicConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::context::{ListConstructionKernelError, RuntimeListElement, SourceElement};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::construction::{
    build_scalar_group_construction, record_scalar_assignment_remaining,
    scalar_group_work_remaining, FrozenRuntimeListConstructionSlot,
    FrozenScalarOrMixedConstruction, ScalarOrMixedSlotOrder,
};
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

use super::list_construction::{
    execute_runtime_list_cheapest_insertion, execute_runtime_list_clarke_wright,
    execute_runtime_list_k_opt, execute_runtime_list_regret_insertion,
    execute_runtime_list_round_robin,
};
use super::{
    ConstructionExecution, DefaultConstructionStageExecutionRecord,
    DefaultRuntimeConstructionExecution, PreparedConstruction, PreparedDefaultRuntime,
    PreparedListSlot, PreparedRuntimeExecution, ResolvedConstructionExecutionOutcome,
    ResolvedConstructionExecutionStep, RuntimeInstantiationError,
};

/// Executes one explicitly prepared construction node.
///
/// The caller supplies the mutable per-solve prepared execution. A reached
/// list stage binds its source index by compact catalog entry exactly once,
/// validates current assignments, then uses the exact unassigned set as its
/// no-work authority. It never clones payloads, rebuilds the source-key map,
/// or invokes an unreached declaration callback.
/// `Default` is deliberately not accepted here: default construction is a
/// staged graph transition and is resolved by the runtime runner only after
/// each preceding boundary has completed.
pub(crate) fn execute_prepared_construction<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    construction: &PreparedConstruction<S, V, DM, IDM>,
    required_only: bool,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<ConstructionExecution, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let control_policy = StepControlPolicy::for_required_construction(required_only);
    match construction {
        PreparedConstruction::ScalarOrMixed {
            config,
            schedule,
            scalar_slots,
            list_slots,
            slot_order,
        } => execute_with_construction_termination(
            config,
            control_policy,
            solver_scope,
            |solver_scope| {
                if control_policy.should_terminate_construction(solver_scope) {
                    return Ok(false);
                }
                let mut active_list_slots = Vec::with_capacity(list_slots.len());
                for (old_index, prepared_slot) in list_slots.iter().copied().enumerate() {
                    let unassigned = execution
                        .current_list_source_work(prepared_slot, solver_scope.working_solution())?;
                    if !unassigned.is_empty() {
                        active_list_slots.push((old_index, prepared_slot));
                    }
                }
                let mut compact_list_indexes = vec![None; list_slots.len()];
                let frozen_list_slots = active_list_slots
                    .iter()
                    .enumerate()
                    .map(|(new_index, (old_index, prepared_slot))| {
                        compact_list_indexes[*old_index] = Some(new_index);
                        let (slot, source_index) = execution.bound_list_source(*prepared_slot);
                        FrozenRuntimeListConstructionSlot {
                            slot: slot.clone(),
                            source_index,
                        }
                    })
                    .collect::<Vec<_>>();
                let compact_slot_order = slot_order
                    .iter()
                    .filter_map(|entry| match entry {
                        ScalarOrMixedSlotOrder::Scalar { .. } => Some(*entry),
                        ScalarOrMixedSlotOrder::List {
                            list_index,
                            construction_slot_index,
                        } => compact_list_indexes[*list_index].map(|list_index| {
                            ScalarOrMixedSlotOrder::List {
                                list_index,
                                construction_slot_index: *construction_slot_index,
                            }
                        }),
                    })
                    .collect::<Vec<_>>();
                if scalar_slots.is_empty() && frozen_list_slots.is_empty() {
                    return Ok(false);
                }
                Ok(FrozenScalarOrMixedConstruction::new(
                    *schedule,
                    config.clone(),
                    scalar_slots.clone(),
                    frozen_list_slots,
                    compact_slot_order,
                )
                .solve(solver_scope))
            },
        ),
        PreparedConstruction::RoundRobin { config, slots } => {
            execute_with_construction_termination(
                config,
                control_policy,
                solver_scope,
                |solver_scope| {
                    execute_list_slots(
                        execution,
                        slots,
                        control_policy,
                        solver_scope,
                        execute_runtime_list_round_robin,
                    )
                },
            )
        }
        PreparedConstruction::CheapestInsertion { config, slots } => {
            execute_with_construction_termination(
                config,
                control_policy,
                solver_scope,
                |solver_scope| {
                    execute_list_slots(
                        execution,
                        slots,
                        control_policy,
                        solver_scope,
                        execute_runtime_list_cheapest_insertion,
                    )
                },
            )
        }
        PreparedConstruction::RegretInsertion { config, slots } => {
            execute_with_construction_termination(
                config,
                control_policy,
                solver_scope,
                |solver_scope| {
                    execute_list_slots(
                        execution,
                        slots,
                        control_policy,
                        solver_scope,
                        execute_runtime_list_regret_insertion,
                    )
                },
            )
        }
        PreparedConstruction::ClarkeWright { config, slots } => {
            execute_with_construction_termination(
                config,
                control_policy,
                solver_scope,
                |solver_scope| {
                    execute_list_slots(
                        execution,
                        slots,
                        control_policy,
                        solver_scope,
                        execute_runtime_list_clarke_wright,
                    )
                },
            )
        }
        PreparedConstruction::KOpt { config, slots } => execute_with_construction_termination(
            config,
            control_policy,
            solver_scope,
            |solver_scope| {
                for slot in slots {
                    if control_policy.should_terminate_construction(solver_scope) {
                        return Ok(false);
                    }
                    execute_runtime_list_k_opt(slot, config.k, control_policy, solver_scope);
                }
                Ok(!slots.is_empty())
            },
        ),
        PreparedConstruction::GroupedScalar {
            config,
            group_index,
            group,
            scalar_bindings,
        } => execute_with_construction_termination(
            config,
            control_policy,
            solver_scope,
            |solver_scope| {
                if control_policy.should_terminate_construction(solver_scope) {
                    return Ok(false);
                }
                if !scalar_group_work_remaining(group, solver_scope.working_solution()) {
                    return Ok(false);
                }
                record_scalar_assignment_remaining(group, solver_scope);
                let mut phase = build_scalar_group_construction(
                    Some(config),
                    *group_index,
                    group.clone(),
                    scalar_bindings.clone(),
                    required_only,
                );
                phase.solve(solver_scope);
                record_scalar_assignment_remaining(group, solver_scope);
                Ok(true)
            },
        ),
    }
}

/// Applies the configured construction termination overlay and captures its
/// outcome before the overlay is restored. This is the one place that turns a
/// child kernel's `ran` bit into an exact execution result, so callers never
/// mistake phase-local termination for an ordinary no-work result.
fn execute_with_construction_termination<S, D, ProgressCb>(
    config: &ConstructionHeuristicConfig,
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    work: impl FnOnce(&mut SolverScope<'_, S, D, ProgressCb>) -> Result<bool, RuntimeInstantiationError>,
) -> Result<ConstructionExecution, RuntimeInstantiationError>
where
    S: PlanningSolution,
    S::Score: ParseableScore,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    solver_scope.with_phase_termination(config.termination.as_ref(), |solver_scope| {
        if control_policy.should_terminate_construction(solver_scope) {
            return Ok(ConstructionExecution::skipped_terminated());
        }
        let ran = work(solver_scope)?;
        if ran {
            Ok(ConstructionExecution::executed())
        } else if control_policy.should_terminate_construction(solver_scope) {
            Ok(ConstructionExecution::skipped_terminated())
        } else {
            Ok(ConstructionExecution::skipped_no_work())
        }
    })
}

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
        let resolved = super::super::defaults::resolve_default_preconstruction_stage(
            &default.bindings,
            stage,
            solver_scope.working_solution(),
        );
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
        let construction =
            execution.prepare_resolved_construction(default.phase_index, &step.construction)?;
        let execution = execute_prepared_construction(
            execution,
            &construction,
            step.required_only,
            solver_scope,
        )?;
        steps.push(ResolvedConstructionExecutionStep {
            kind: step.kind,
            required_only: step.required_only,
            target: step.target.clone(),
            list_policies: step.list_policies.clone(),
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

/// Executes one configured list construction family in declaration order.
///
/// The callback type is monomorphized at the call site.  It is not a phase
/// factory or trait-object adapter: every invocation passes the exact
/// `RuntimeListSlot` and its borrowed prepared source to one concrete kernel.
fn execute_list_slots<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    slots: &[PreparedListSlot],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    execute: fn(
        &crate::builder::context::RuntimeListSlot<S, V, DM, IDM>,
        &crate::builder::context::RuntimeListSourceIndex<
            crate::builder::context::RuntimeListElement<V>,
        >,
        &[SourceElement<RuntimeListElement<V>>],
        StepControlPolicy,
        &mut SolverScope<'_, S, D, ProgressCb>,
    ) -> Result<(), ListConstructionKernelError>,
) -> Result<bool, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut ran = false;
    for prepared_slot in slots {
        if control_policy.should_terminate_construction(solver_scope) {
            break;
        }
        let unassigned =
            execution.current_list_source_work(*prepared_slot, solver_scope.working_solution())?;
        if unassigned.is_empty() {
            continue;
        }
        let (slot, source_index) = execution.bound_list_source(*prepared_slot);
        execute(
            slot,
            source_index,
            &unassigned,
            control_policy,
            solver_scope,
        )
        .map_err(|error| RuntimeInstantiationError {
            phase_index: prepared_slot.phase_index,
            kind: super::RuntimeInstantiationErrorKind::SourceRefresh {
                target: slot.identity(),
                error,
            },
        })?;
        ran = true;
    }
    Ok(ran)
}
