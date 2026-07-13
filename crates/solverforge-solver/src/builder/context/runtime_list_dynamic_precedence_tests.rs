//! Dynamic successor-only precedence regression coverage.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_config::{
    ListRuinMoveSelectorConfig, LocalSearchConfig, MoveSelectorConfig, PhaseConfig, SolverConfig,
};
use solverforge_core::domain::{
    DynamicListAccess, DynamicListMetadata, DynamicListMetadataCapabilities,
    DynamicListVariableSlot, EntityClassId, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;

use super::list_access::ListAccess;
use super::runtime_list::RuntimeListElement;
use super::{RuntimeListSlot, RuntimeModel, VariableSlot};
use crate::builder::{NoDynamicExtensions, SearchContext};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::runtime::compiler::{compile_runtime_graph, RuntimeGraphInput};

type RuntimeSlot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
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

    fn element_count(&self, _: &Plan) -> usize {
        3
    }

    fn element(&self, _: &Plan, index: usize) -> Option<usize> {
        (index < 3).then_some(index)
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
        (position < solution.routes[entity].len()).then(|| solution.routes[entity].remove(position))
    }
}

#[derive(Debug)]
struct SuccessorsOnly;

impl DynamicListMetadata<Plan> for SuccessorsOnly {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            precedence_successors: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn construction_order_key(&self, _: &Plan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn extend_precedence_successors(
        &self,
        _: &Plan,
        element: usize,
        successors: &mut Vec<usize>,
    ) -> bool {
        if element == 0 {
            successors.push(1);
        }
        true
    }

    fn cross_position_distance(
        &self,
        _: &Plan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        None
    }

    fn intra_position_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<f64> {
        None
    }

    fn route_depot(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &Plan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn savings_metric_class(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn savings_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn savings_feasible(&self, _: &Plan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
            .with_logical_id(EntityClassId(0))
            .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
    )
}

fn slot() -> RuntimeSlot {
    let dynamic = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(Access),
        Arc::new(SuccessorsOnly),
    )
    .expect("test dynamic metadata identity matches");
    RuntimeListSlot::from_dynamic(dynamic)
}

#[test]
fn dynamic_successors_only_metadata_executes_for_list_ruin_without_duration() {
    let plan = Plan {
        score: None,
        routes: vec![vec![0, 1, 2]],
    };
    let slot = slot();
    let mut successors = Vec::new();

    slot.extend_precedence_successors(&plan, RuntimeListElement::Dynamic(0), &mut successors)
        .expect("successor-only metadata is executable by ListRuin");
    assert_eq!(successors, vec![RuntimeListElement::Dynamic(1)]);
    assert!(
        slot.precedence_duration(&plan, RuntimeListElement::Dynamic(0))
            .is_err(),
        "successor-only must not be misrepresented as full precedence",
    );

    let dynamic = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(Access),
        Arc::new(SuccessorsOnly),
    )
    .expect("test dynamic metadata identity matches");
    let model: RuntimeModel<
        Plan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicList(dynamic)]);
    let config = SolverConfig {
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            move_selector: Some(MoveSelectorConfig::ListRuinMoveSelector(
                ListRuinMoveSelectorConfig::default(),
            )),
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    };

    let context = SearchContext::new(descriptor(), model, config.random_seed);
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("successor-only dynamic metadata remains legal for ListRuin");
}
