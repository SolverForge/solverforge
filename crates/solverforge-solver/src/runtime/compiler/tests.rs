//! Compilation legality matrix for frozen list metadata policies.

use std::any::TypeId;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ListChangeMoveConfig,
    ListPrecedenceMoveConfig, ListRuinMoveSelectorConfig, LocalSearchConfig, MoveSelectorConfig,
    PhaseConfig, SolverConfig,
};
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, VariableDescriptor,
};
use solverforge_core::score::SoftScore;

use super::{
    compile_runtime_graph, CompiledConstruction, CompiledRuntimePhase, ListConstructionKind,
    RuntimeCapability, RuntimeCompileErrorKind, RuntimeGraphInput,
};
use crate::builder::{
    ListVariableSlot, NoDynamicExtensions, RuntimeModel, SearchContext, VariableSlot,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

type Meter = DefaultCrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
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

fn entity_count(_: &Plan) -> usize {
    0
}

fn element_count(_: &Plan) -> usize {
    0
}

fn assigned_elements(_: &Plan) -> Vec<usize> {
    Vec::new()
}

fn list_len(_: &Plan, _: usize) -> usize {
    0
}

fn list_remove(_: &mut Plan, _: usize, _: usize) -> Option<usize> {
    None
}

fn construction_list_remove(_: &mut Plan, _: usize, _: usize) -> usize {
    unreachable!("compiler legality tests never execute list construction")
}

fn list_insert(_: &mut Plan, _: usize, _: usize, _: usize) {}

fn list_get(_: &Plan, _: usize, _: usize) -> Option<usize> {
    None
}

fn list_set(_: &mut Plan, _: usize, _: usize, _: usize) {}

fn list_reverse(_: &mut Plan, _: usize, _: usize, _: usize) {}

fn sublist_remove(_: &mut Plan, _: usize, _: usize, _: usize) -> Vec<usize> {
    Vec::new()
}

fn sublist_insert(_: &mut Plan, _: usize, _: usize, _: Vec<usize>) {}

fn ruin_remove(_: &mut Plan, _: usize, _: usize) -> usize {
    unreachable!("compiler legality tests never execute list ruin")
}

fn ruin_insert(_: &mut Plan, _: usize, _: usize, _: usize) {}

fn index_to_element(_: &Plan, element: usize) -> usize {
    element
}

fn precedence_duration(_: &Plan, _: usize) -> usize {
    1
}

fn precedence_successors(_: &Plan, _: usize, _: &mut Vec<usize>) {}

fn route_values(_: &Plan, _: usize) -> Vec<usize> {
    Vec::new()
}

fn replace_route(_: &mut Plan, _: usize, _: Vec<usize>) {}

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn route_distance(_: &Plan, _: usize, _: usize, _: usize) -> i64 {
    0
}

fn savings_distance(_: &Plan, _: usize, _: usize, _: usize) -> i64 {
    0
}

fn route_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
            .with_variable(VariableDescriptor::list("visits")),
    )
}

#[derive(Clone, Copy)]
enum PrecedenceShape {
    Absent,
    SuccessorsOnly,
    Explicit,
}

fn model(shape: PrecedenceShape) -> RuntimeModel<Plan, usize, Meter, Meter> {
    let slot = ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        crate::builder::usize_element_source_key,
        entity_count,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
        "visits",
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    let slot = match shape {
        PrecedenceShape::Absent => slot,
        PrecedenceShape::SuccessorsOnly => {
            slot.with_precedence_hooks(None, Some(precedence_successors))
        }
        PrecedenceShape::Explicit => {
            slot.with_precedence_hooks(Some(precedence_duration), Some(precedence_successors))
        }
    };
    RuntimeModel::new(vec![VariableSlot::List(slot)])
}

fn savings_model() -> RuntimeModel<Plan, usize, Meter, Meter> {
    RuntimeModel::new(vec![VariableSlot::List(ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        crate::builder::usize_element_source_key,
        entity_count,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
        "visits",
        0,
        Some(route_values),
        Some(replace_route),
        Some(depot),
        Some(route_distance),
        Some(route_feasible),
        Some(depot),
        None,
        Some(savings_distance),
        Some(route_feasible),
    ))])
}

fn compile(
    shape: PrecedenceShape,
    selector: MoveSelectorConfig,
) -> Result<
    super::CompiledRuntimeGraph<Plan, usize, Meter, Meter, NoDynamicExtensions>,
    super::RuntimeCompileError,
> {
    let config = SolverConfig {
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            move_selector: Some(selector),
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), model(shape), config.random_seed);
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
}

fn compile_construction(
    model: RuntimeModel<Plan, usize, Meter, Meter>,
    construction_heuristic_type: ConstructionHeuristicType,
) -> Result<
    super::CompiledRuntimeGraph<Plan, usize, Meter, Meter, NoDynamicExtensions>,
    super::RuntimeCompileError,
> {
    let config = SolverConfig {
        phases: vec![PhaseConfig::ConstructionHeuristic(
            ConstructionHeuristicConfig {
                construction_heuristic_type,
                ..ConstructionHeuristicConfig::default()
            },
        )],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
}

#[test]
fn non_precedence_list_families_remain_legal_without_precedence_metadata() {
    compile(
        PrecedenceShape::Absent,
        MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig::default()),
    )
    .expect("list change has no precedence requirement");
    compile(
        PrecedenceShape::Absent,
        MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default()),
    )
    .expect("list ruin remains legal without precedence metadata");
    compile(
        PrecedenceShape::SuccessorsOnly,
        MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default()),
    )
    .expect("successor-only metadata remains legal for ListRuin");
}

#[test]
fn precedence_selector_rejects_absent_or_successors_only_metadata() {
    for shape in [PrecedenceShape::Absent, PrecedenceShape::SuccessorsOnly] {
        let error = compile(
            shape,
            MoveSelectorConfig::ListPrecedenceMoveSelector(ListPrecedenceMoveConfig::default()),
        )
        .expect_err("precedence selector requires duration and successors");
        assert!(matches!(
            error.kind,
            RuntimeCompileErrorKind::MissingCapability {
                capability: RuntimeCapability::ListPrecedence,
                ..
            }
        ));
    }
}

#[test]
fn precedence_selector_accepts_explicit_duration_and_successors() {
    compile(
        PrecedenceShape::Explicit,
        MoveSelectorConfig::ListPrecedenceMoveSelector(ListPrecedenceMoveConfig::default()),
    )
    .expect("explicit precedence metadata satisfies the selector contract");
}

#[test]
fn clarke_wright_compiles_as_its_own_savings_construction_kind() {
    let graph = compile_construction(savings_model(), ConstructionHeuristicType::ListClarkeWright)
        .expect("declared savings hooks compile Clarke-Wright");
    let [CompiledRuntimePhase::Construction(CompiledConstruction::List { kind, .. })] =
        graph.phases()
    else {
        panic!("Clarke-Wright must lower to one explicit list construction node");
    };
    assert_eq!(*kind, ListConstructionKind::ClarkeWright);
}

#[test]
fn clarke_wright_requires_savings_capability_before_execution() {
    let error = compile_construction(
        model(PrecedenceShape::Absent),
        ConstructionHeuristicType::ListClarkeWright,
    )
    .expect_err("Clarke-Wright must not degrade to cheapest insertion without savings hooks");
    assert!(matches!(
        error.kind,
        RuntimeCompileErrorKind::MissingCapability {
            capability: RuntimeCapability::ListSavings,
            ..
        }
    ));
}
