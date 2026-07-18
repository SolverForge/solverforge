//! Canonical cheapest-insertion execution.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::{
    ListConstructionKernelError, RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex,
    SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{run_cheapest, PhaseCheapestInsertionObserver};
use crate::phase::construction::run_construction_phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

/// Executes canonical cheapest insertion against a caller-owned prepared
/// source binding.
///
/// This is the execution seam used by the compiled runtime runner. Keeping
/// the source borrowed is important:
/// a prepared graph owns exactly one frozen declaration/source-key index per
/// list slot and must not clone it merely to manufacture a short-lived phase
/// object.
pub(crate) fn execute_runtime_list_cheapest_insertion<S, V, DM, IDM, D, ProgressCb>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    source_index: &RuntimeListSourceIndex<RuntimeListElement<V>>,
    unassigned: &[SourceElement<RuntimeListElement<V>>],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<(), ListConstructionKernelError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    run_construction_phase(
        solver_scope,
        0,
        "Cheapest-Insertion Construction",
        |phase_scope| {
            let mut observer = PhaseCheapestInsertionObserver::new(phase_scope, control_policy);
            run_cheapest(slot, source_index, unassigned, &mut observer);
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::sync::Arc;

    use solverforge_core::domain::{
        DynamicListAccess, DynamicListVariableSlot, EntityClassId, EntityDescriptor,
        SolutionDescriptor, VariableDescriptor, VariableId,
    };
    use solverforge_core::score::SoftScore;
    use solverforge_scoring::ScoreDirector;

    use super::*;
    use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

    #[derive(Clone)]
    struct Plan {
        score: Option<SoftScore>,
        elements: Vec<usize>,
        routes: Vec<Vec<usize>>,
    }

    impl PlanningSolution for Plan {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[derive(Debug)]
    struct Access;

    impl DynamicListAccess<Plan> for Access {
        fn entity_class(&self) -> EntityClassId {
            EntityClassId(0)
        }

        fn variable(&self) -> VariableId {
            VariableId(0)
        }

        fn entity_count(&self, solution: &Plan) -> usize {
            solution.routes.len()
        }

        fn element_count(&self, solution: &Plan) -> usize {
            solution.elements.len()
        }

        fn element(&self, solution: &Plan, index: usize) -> Option<usize> {
            solution.elements.get(index).copied()
        }

        fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
            solution.routes.iter().flatten().copied().collect()
        }

        fn len(&self, solution: &Plan, entity: usize) -> usize {
            solution.routes[entity].len()
        }

        fn get(&self, solution: &Plan, entity: usize, position: usize) -> Option<usize> {
            solution.routes.get(entity)?.get(position).copied()
        }

        fn insert(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) {
            solution.routes[entity].insert(position, value);
        }

        fn remove(&self, solution: &mut Plan, entity: usize, position: usize) -> Option<usize> {
            (position < solution.routes[entity].len())
                .then(|| solution.routes[entity].remove(position))
        }
    }

    fn descriptor() -> SolutionDescriptor {
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
            EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
                .with_logical_id(EntityClassId(0))
                .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
        )
    }

    #[test]
    fn dynamic_slot_refreshes_current_assignment_for_each_cheapest_phase_execution() {
        let descriptor = descriptor();
        let dynamic = DynamicListVariableSlot::try_with_access(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(Access),
        )
        .expect("dynamic test access identity matches its slot")
        .resolved_against(&descriptor)
        .expect("dynamic test slot resolves against its descriptor");
        let slot: RuntimeListSlot<
            Plan,
            usize,
            DefaultCrossEntityDistanceMeter,
            DefaultCrossEntityDistanceMeter,
        > = RuntimeListSlot::from_dynamic(dynamic);
        let plan = Plan {
            score: None,
            elements: vec![0, 1, 2],
            routes: vec![Vec::new()],
        };
        let binding = crate::builder::context::bind_runtime_list_source(&slot, &plan)
            .expect("well-formed dynamic source binds before phase construction");
        let source_index = binding.into_source_index();
        let unassigned = crate::builder::context::unassigned_from_current_assignment(
            &slot,
            &source_index,
            &plan,
        )
        .expect("initial assignment resolves against the bound source");
        let director = ScoreDirector::simple(plan, descriptor, |solution, _| solution.routes.len());
        let mut scope = SolverScope::new(director);

        execute_runtime_list_cheapest_insertion(
            &slot,
            &source_index,
            &unassigned,
            StepControlPolicy::ObserveConfigLimits,
            &mut scope,
        )
        .expect("prepared cheapest insertion executes");
        let first_routes = scope.working_solution().routes.clone();
        let unassigned = crate::builder::context::unassigned_from_current_assignment(
            &slot,
            &source_index,
            scope.working_solution(),
        )
        .expect("updated assignment resolves against the bound source");
        execute_runtime_list_cheapest_insertion(
            &slot,
            &source_index,
            &unassigned,
            StepControlPolicy::ObserveConfigLimits,
            &mut scope,
        )
        .expect("prepared cheapest insertion refreshes current assignment");

        assert_eq!(first_routes, vec![vec![2, 1, 0]]);
        assert_eq!(scope.working_solution().routes, first_routes);
        assert!(scope.stats().score_calculations >= 6);
    }
}
