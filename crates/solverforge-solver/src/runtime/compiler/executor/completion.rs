//! Structural completion checks for one compiled runtime execution.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::SolverTerminalReason;
use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
use crate::scope::{ProgressCallback, SolverScope};

use super::{PreparedRuntimeExecution, RuntimeInstantiationError};
use crate::runtime::compiler::DefaultRuntimeBindings;

pub(super) fn publish_if_mandatory_complete<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    completion_published: &mut bool,
    phase_index: usize,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> RuntimeBuildResult<bool>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if matches!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled | SolverTerminalReason::Failed
    ) {
        return Ok(false);
    }
    let unresolved = unresolved_mandatory_work(
        execution,
        bindings,
        phase_index,
        solver_scope.working_solution(),
    )
    .map_err(|error| RuntimeBuildError::Execution {
        phase_index: error.phase_index,
        message: error.to_string(),
    })?;
    if unresolved.is_some() {
        solver_scope.defer_best_solution_publication();
        *completion_published = false;
        return Ok(false);
    }
    if !*completion_published {
        solver_scope.publish_current_solution_as_best();
        *completion_published = true;
    }
    Ok(true)
}

pub(super) fn require_mandatory_completion<S, V, DM, IDM, Extension, D, ProgressCb>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    completion_published: &mut bool,
    phase_index: usize,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> RuntimeBuildResult<bool>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if matches!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled | SolverTerminalReason::Failed
    ) {
        return Ok(false);
    }
    let unresolved = unresolved_mandatory_work(
        execution,
        bindings,
        phase_index,
        solver_scope.working_solution(),
    )
    .map_err(|error| RuntimeBuildError::Execution {
        phase_index: error.phase_index,
        message: error.to_string(),
    })?;
    let Some(unresolved) = unresolved else {
        if !*completion_published {
            solver_scope.publish_current_solution_as_best();
            *completion_published = true;
        }
        return Ok(true);
    };
    solver_scope.defer_best_solution_publication();
    *completion_published = false;
    Err(RuntimeBuildError::Execution {
        phase_index,
        message: format!(
            "configured solve stopped with mandatory planning work incomplete: {unresolved}"
        ),
    })
}

pub(super) fn unresolved_mandatory_work<S, V, DM, IDM, Extension>(
    execution: &mut PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    phase_index: usize,
    solution: &S,
) -> Result<Option<String>, RuntimeInstantiationError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    for slot in &bindings.list_slots {
        let unassigned = execution.current_list_unassigned_count(phase_index, slot, solution)?;
        if unassigned > 0 {
            return Ok(Some(format!(
                "list variable {} has {unassigned} unassigned element(s)",
                slot.identity()
            )));
        }
    }

    for binding in &bindings.assignment_groups {
        let Some(assignment) = binding.group.assignment() else {
            continue;
        };
        let remaining = assignment.remaining_required_count(solution);
        if remaining > 0 {
            return Ok(Some(format!(
                "assignment group {} has {remaining} unassigned required entity row(s)",
                binding.group.group_name
            )));
        }
    }

    for binding in &bindings.scalar_slots {
        if binding.assignment_owned || binding.slot.allows_unassigned() {
            continue;
        }
        let unassigned = (0..binding.slot.entity_count(solution))
            .filter(|entity_index| {
                binding
                    .slot
                    .current_value(solution, *entity_index)
                    .is_none()
            })
            .count();
        if unassigned > 0 {
            return Ok(Some(format!(
                "scalar variable {} has {unassigned} unassigned entity row(s)",
                binding.slot.id()
            )));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use solverforge_config::SolverConfig;
    use solverforge_core::domain::{
        EntityClassId, EntityCollectionExtractor, EntityDescriptor, PlanningSolution,
        SolutionDescriptor, ValueRangeType, VariableDescriptor, VariableId,
    };
    use solverforge_core::score::SoftScore;

    use super::unresolved_mandatory_work;
    use crate::builder::{
        bind_scalar_groups, NoDynamicExtensions, RuntimeModel, ScalarVariableSlot, SearchContext,
        ValueSource, VariableSlot,
    };
    use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
    use crate::planning::{ScalarGroup, ScalarTarget};
    use crate::runtime::compiler::{
        compile_runtime_graph, CompiledRuntimeExecutor, RuntimeGraphInput,
    };

    type Meter = DefaultCrossEntityDistanceMeter;
    type Model = RuntimeModel<ScalarPlan, usize, Meter, Meter>;

    #[derive(Clone, Debug)]
    struct ScalarPlan {
        score: Option<SoftScore>,
        workers: Vec<Option<usize>>,
        required: Vec<bool>,
        candidates: Vec<usize>,
    }

    impl PlanningSolution for ScalarPlan {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn descriptor(allows_unassigned: bool) -> SolutionDescriptor {
        SolutionDescriptor::new("ScalarPlan", TypeId::of::<ScalarPlan>()).with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Option<usize>>(), "tasks")
                .with_logical_id(EntityClassId(0))
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |plan: &ScalarPlan| &plan.workers,
                    |plan: &mut ScalarPlan| &mut plan.workers,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker")
                        .with_logical_id(VariableId(0))
                        .with_allows_unassigned(allows_unassigned)
                        .with_value_range_type(ValueRangeType::EntityDependent)
                        .with_usize_accessors(
                            |entity| {
                                *entity
                                    .downcast_ref::<Option<usize>>()
                                    .expect("Task must be an optional worker")
                            },
                            |entity, value| {
                                *entity
                                    .downcast_mut::<Option<usize>>()
                                    .expect("Task must be an optional worker") = value;
                            },
                        ),
                ),
        )
    }

    fn scalar_slot(allows_unassigned: bool) -> ScalarVariableSlot<ScalarPlan> {
        ScalarVariableSlot::new(
            0,
            0,
            "Task",
            |plan| plan.workers.len(),
            "worker",
            |plan, entity, _| plan.workers[entity],
            |plan, entity, _, value| plan.workers[entity] = value,
            ValueSource::EntitySlice {
                values_for_entity: |plan, _, _| &plan.candidates,
            },
            allows_unassigned,
        )
    }

    fn unresolved(
        model: Model,
        descriptor: SolutionDescriptor,
        plan: &ScalarPlan,
    ) -> Option<String> {
        let context = SearchContext::new(descriptor, model, None);
        let graph = compile_runtime_graph(
            &SolverConfig::default(),
            RuntimeGraphInput::new(context, NoDynamicExtensions),
        )
        .expect("scalar completion graph compiles");
        let executor = CompiledRuntimeExecutor::new(graph);
        let bindings = executor.graph().default_bindings().clone();
        let mut execution = executor.instantiate().expect("scalar graph instantiates");
        unresolved_mandatory_work(&mut execution, &bindings, 0, plan)
            .expect("scalar completion check succeeds")
    }

    #[test]
    fn required_scalar_slots_must_be_assigned() {
        let plan = ScalarPlan {
            score: None,
            workers: vec![None, Some(0)],
            required: vec![false, false],
            candidates: vec![0],
        };
        let unresolved = unresolved(
            RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot(false))]),
            descriptor(false),
            &plan,
        )
        .expect("one required scalar row is unassigned");

        assert!(unresolved.contains("1 unassigned entity row(s)"));
    }

    #[test]
    fn optional_scalar_slots_may_remain_unassigned() {
        let plan = ScalarPlan {
            score: None,
            workers: vec![None, Some(0)],
            required: vec![false, false],
            candidates: vec![0],
        };

        assert!(unresolved(
            RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot(true))]),
            descriptor(true),
            &plan,
        )
        .is_none());
    }

    #[test]
    fn assignment_groups_require_only_rows_declared_required() {
        let slot = scalar_slot(true);
        let groups = bind_scalar_groups(
            vec![ScalarGroup::assignment(
                "worker_assignment",
                ScalarTarget::from_descriptor_index(0, "worker"),
            )
            .with_required_entity(|plan: &ScalarPlan, entity| plan.required[entity])],
            &[slot],
        );
        let model = RuntimeModel::new(vec![VariableSlot::Scalar(slot)]).with_scalar_groups(groups);
        let incomplete = ScalarPlan {
            score: None,
            workers: vec![None, None],
            required: vec![true, false],
            candidates: vec![0],
        };
        let complete = ScalarPlan {
            score: None,
            workers: vec![Some(0), None],
            required: vec![true, false],
            candidates: vec![0],
        };

        let message = unresolved(model.clone(), descriptor(true), &incomplete)
            .expect("required assignment row is incomplete");
        assert!(message.contains("1 unassigned required entity row(s)"));
        assert!(unresolved(model, descriptor(true), &complete).is_none());
    }
}
