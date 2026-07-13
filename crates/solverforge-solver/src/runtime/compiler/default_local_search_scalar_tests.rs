//! Scalar and assignment parity for the compiler-owned default policy.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_config::{
    AcceptorConfig, ForagerConfig, LocalSearchConfig, LocalSearchType, PhaseConfig, SolverConfig,
    TerminationConfig,
};
use solverforge_core::domain::{
    DynamicScalarAccess, DynamicScalarAssignmentMetadata,
    DynamicScalarAssignmentMetadataCapabilities, DynamicScalarVariableSlot, EntityClassId,
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    ValueRangeType, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;

use super::{
    compile_runtime_graph, CompiledAcceptorForagerSelector, CompiledLocalSearch,
    CompiledRuntimePhase, DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchForagerPolicy,
    DefaultLocalSearchPlan, DefaultLocalSearchSelectorFamily, DefaultSelectorCapabilityPolicy,
    RuntimeGraphInput,
};
use crate::builder::{
    bind_scalar_groups, NoDynamicExtensions, RuntimeModel, ScalarGroupBinding, ScalarGroupLimits,
    ScalarVariableSlot, SearchContext, ValueSource, VariableSlot,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::planning::{ScalarGroup, ScalarTarget};

type Meter = DefaultCrossEntityDistanceMeter;
type Model = RuntimeModel<ScalarPlan, usize, Meter, Meter>;

#[derive(Clone, Debug)]
struct ScalarPlan {
    score: Option<SoftScore>,
    workers: Vec<Option<usize>>,
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

fn descriptor() -> SolutionDescriptor {
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
                    .with_allows_unassigned(true)
                    .with_value_range_type(ValueRangeType::EntityDependent)
                    .with_usize_accessors(worker_getter, worker_setter),
            ),
    )
}

fn worker_getter(entity: &dyn std::any::Any) -> Option<usize> {
    *entity
        .downcast_ref::<Option<usize>>()
        .expect("Task entity must be an optional worker")
}

fn worker_setter(entity: &mut dyn std::any::Any, value: Option<usize>) {
    *entity
        .downcast_mut::<Option<usize>>()
        .expect("Task entity must be an optional worker") = value;
}

fn entity_count(plan: &ScalarPlan) -> usize {
    plan.workers.len()
}

fn current_value(plan: &ScalarPlan, entity: usize, _: usize) -> Option<usize> {
    plan.workers[entity]
}

fn set_value(plan: &mut ScalarPlan, entity: usize, _: usize, value: Option<usize>) {
    plan.workers[entity] = value;
}

fn candidates(plan: &ScalarPlan, _: usize, _: usize) -> &[usize] {
    &plan.candidates
}

fn nearby_value_distance(_: &ScalarPlan, _: usize, _: usize, _: usize) -> Option<f64> {
    Some(0.0)
}

fn nearby_entity_distance(_: &ScalarPlan, _: usize, _: usize, _: usize) -> Option<f64> {
    Some(0.0)
}

fn scalar_slot() -> ScalarVariableSlot<ScalarPlan> {
    ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        current_value,
        set_value,
        ValueSource::EntitySlice {
            values_for_entity: candidates,
        },
        true,
    )
    .with_candidate_values(candidates)
    .with_nearby_value_candidates(candidates)
    .with_nearby_value_distance_meter(nearby_value_distance)
    .with_nearby_entity_candidates(candidates)
    .with_nearby_entity_distance_meter(nearby_entity_distance)
}

#[derive(Debug)]
struct DynamicAccess;

impl DynamicScalarAccess<ScalarPlan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, solution: &ScalarPlan) -> usize {
        entity_count(solution)
    }

    fn get(&self, solution: &ScalarPlan, row: usize) -> Option<usize> {
        solution.workers[row]
    }

    fn set(&self, solution: &mut ScalarPlan, row: usize, value: Option<usize>) {
        solution.workers[row] = value;
    }

    fn candidate_values<'a>(&self, solution: &'a ScalarPlan, _: usize) -> &'a [usize] {
        &solution.candidates
    }

    fn value_is_legal(&self, solution: &ScalarPlan, _: usize, value: usize) -> bool {
        solution.candidates.contains(&value)
    }

    fn has_nearby_value_candidates(&self) -> bool {
        true
    }

    fn visit_nearby_value_candidates(
        &self,
        solution: &ScalarPlan,
        _: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        for &value in solution.candidates.iter().take(limit) {
            visit(value);
        }
        true
    }

    fn nearby_value_distance(&self, _: &ScalarPlan, _: usize, _: usize) -> Option<f64> {
        Some(0.0)
    }

    fn has_nearby_entity_candidates(&self) -> bool {
        true
    }

    fn visit_nearby_entity_candidates(
        &self,
        solution: &ScalarPlan,
        _: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        for entity in 0..solution.workers.len().min(limit) {
            visit(entity);
        }
        true
    }

    fn nearby_entity_distance(&self, _: &ScalarPlan, _: usize, _: usize) -> Option<f64> {
        Some(0.0)
    }
}

fn dynamic_scalar_slot() -> DynamicScalarVariableSlot<ScalarPlan> {
    DynamicScalarVariableSlot::with_access(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        true,
        Arc::new(DynamicAccess),
    )
    .resolved_against(&descriptor())
    .expect("test dynamic scalar slot resolves against descriptor")
}

fn static_scalar_model() -> Model {
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot())])
}

fn dynamic_scalar_model() -> Model {
    RuntimeModel::new(vec![VariableSlot::DynamicScalar(dynamic_scalar_slot())])
}

fn required(_: &ScalarPlan, _: usize) -> bool {
    false
}

fn static_assignment_model() -> Model {
    let slot = scalar_slot();
    let groups = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "worker_assignment",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_entity(required)],
        &[slot],
    );
    RuntimeModel::new(vec![VariableSlot::Scalar(slot)]).with_scalar_groups(groups)
}

#[derive(Debug)]
struct DynamicAssignmentMetadata;

impl DynamicScalarAssignmentMetadata<ScalarPlan> for DynamicAssignmentMetadata {
    fn capabilities(&self) -> DynamicScalarAssignmentMetadataCapabilities {
        DynamicScalarAssignmentMetadataCapabilities::default()
    }

    fn required_entity(&self, _: &ScalarPlan, _: usize) -> bool {
        false
    }

    fn capacity_key(&self, _: &ScalarPlan, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn position_key(&self, _: &ScalarPlan, _: usize) -> Option<i64> {
        None
    }

    fn sequence_key(&self, _: &ScalarPlan, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn entity_order_key(&self, _: &ScalarPlan, _: usize) -> Option<i64> {
        None
    }

    fn value_order_key(&self, _: &ScalarPlan, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn assignment_edge_allowed(
        &self,
        _: &ScalarPlan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> bool {
        true
    }
}

fn dynamic_assignment_model() -> Model {
    let slot = dynamic_scalar_slot();
    let group = ScalarGroupBinding::dynamic_assignment(
        "worker_assignment",
        slot.clone(),
        Arc::new(DynamicAssignmentMetadata),
        ScalarGroupLimits::new(),
    );
    RuntimeModel::new(vec![VariableSlot::DynamicScalar(slot)]).with_scalar_groups(vec![group])
}

fn default_plan(model: Model) -> DefaultLocalSearchPlan {
    default_plan_with_config(model, SolverConfig::default())
}

fn default_plan_with_config(model: Model, config: SolverConfig) -> DefaultLocalSearchPlan {
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("default scalar graph compiles")
    .default_bindings()
    .local_search_plan
    .clone()
    .expect("default scalar graph has an omitted-local-search plan")
}

fn trace_value(plan: &DefaultLocalSearchPlan, key: &str) -> Option<String> {
    plan.candidate_trace_plan()
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .map(|attribute| attribute.value.clone())
}

fn phase_trace_value(plan: &crate::stats::CandidateTracePhasePlan, key: &str) -> Option<String> {
    plan.attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .map(|attribute| attribute.value.clone())
}

fn signature(
    plan: &DefaultLocalSearchPlan,
) -> Vec<(
    DefaultLocalSearchSelectorFamily,
    DefaultSelectorCapabilityPolicy,
)> {
    plan.selectors
        .iter()
        .map(|selector| (selector.family, selector.capability_policy))
        .collect()
}

#[test]
fn typed_and_dynamic_scalar_capabilities_have_identical_default_policy() {
    let typed = default_plan(static_scalar_model());
    let dynamic = default_plan(dynamic_scalar_model());
    let expected = vec![
        (
            DefaultLocalSearchSelectorFamily::NearbyScalarChange,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::NearbyScalarSwap,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::ScalarChange,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::ScalarSwap,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
    ];
    assert_eq!(signature(&typed), expected);
    assert_eq!(signature(&dynamic), expected);
    assert_eq!(typed.components, dynamic.components);
    assert_eq!(
        typed.components.acceptor,
        DefaultLocalSearchAcceptorPolicy::SimulatedAnnealing {
            decay_rate_bits: 0.999_985_f64.to_bits(),
            random_seed: None,
        }
    );
    assert_eq!(
        typed.components.forager,
        DefaultLocalSearchForagerPolicy::AcceptedCount { limit: 256 }
    );
    assert_eq!(
        trace_value(&typed, "simulated_annealing_decay_rate_bits"),
        Some(0.999_985_f64.to_bits().to_string()),
    );
    assert_eq!(
        trace_value(&typed, "simulated_annealing_random_seed"),
        Some("none".to_string()),
    );
    assert_eq!(
        trace_value(&typed, "forager_accepted_count_limit"),
        Some("256".to_string())
    );
}

#[test]
fn seeded_default_scalar_acceptor_is_frozen_in_the_declaration_and_trace() {
    let seed = 41;
    let plan = default_plan_with_config(
        dynamic_scalar_model(),
        SolverConfig {
            random_seed: Some(seed),
            ..SolverConfig::default()
        },
    );

    assert_eq!(
        plan.components.acceptor,
        DefaultLocalSearchAcceptorPolicy::SimulatedAnnealing {
            decay_rate_bits: 0.999_985_f64.to_bits(),
            random_seed: Some(seed),
        }
    );
    assert_eq!(
        trace_value(&plan, "simulated_annealing_random_seed"),
        Some("41".to_string()),
    );
}

#[test]
fn omitted_explicit_acceptor_forager_phase_refs_the_compiled_default_declaration() {
    let config = SolverConfig {
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            local_search_type: LocalSearchType::AcceptorForager,
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), dynamic_scalar_model(), config.random_seed);
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("omitted selector compiles as an exact default-declaration reference");

    assert!(graph.default_bindings().local_search_plan.is_some());
    let [CompiledRuntimePhase::LocalSearch(CompiledLocalSearch::AcceptorForager {
        selector, ..
    })] = graph.phases()
    else {
        panic!("one acceptor-forager phase must compile");
    };
    assert!(matches!(
        selector,
        CompiledAcceptorForagerSelector::OmittedDefault
    ));
}

#[test]
fn omitted_selector_provenance_keeps_the_explicit_local_search_policy() {
    let local_search = LocalSearchConfig {
        local_search_type: LocalSearchType::AcceptorForager,
        acceptor: Some(AcceptorConfig::HillClimbing),
        forager: Some(ForagerConfig::FirstAccepted),
        termination: Some(TerminationConfig {
            step_count_limit: Some(17),
            ..TerminationConfig::default()
        }),
        ..LocalSearchConfig::default()
    };
    let config = SolverConfig {
        phases: vec![PhaseConfig::LocalSearch(local_search)],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), dynamic_scalar_model(), config.random_seed);
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("explicit policy with an omitted selector compiles");
    let expected_selector_count = graph
        .default_bindings()
        .local_search_plan
        .as_ref()
        .expect("the omitted selector has a default declaration")
        .selectors
        .len();
    let executor = super::executor::CompiledRuntimeExecutor::new(graph);
    let runner = super::executor::CompiledRuntimePhaseRunner::try_new(&executor)
        .expect("the explicit phase lowers against the default selector tree");
    let phase = runner.phase_plan_for_test(0);
    let declaration = &phase.children[0];

    assert_eq!(
        declaration.kind,
        "solverforge.runtime.local_search.acceptor_forager"
    );
    assert_eq!(
        phase_trace_value(declaration, "acceptor").as_deref(),
        Some("configured:HillClimbing")
    );
    assert_eq!(
        phase_trace_value(declaration, "forager").as_deref(),
        Some("configured:FirstAccepted")
    );
    assert!(phase_trace_value(declaration, "config")
        .expect("explicit configuration is retained")
        .contains("step_count_limit: Some(17)"));
    assert_eq!(declaration.children.len(), expected_selector_count);
    assert!(declaration
        .children
        .iter()
        .all(|child| { child.kind == "solverforge.runtime.default_local_search.selector" }));
}

#[test]
fn typed_and_dynamic_assignment_groups_hide_scalar_leaves_and_keep_group_policy() {
    for plan in [
        default_plan(static_assignment_model()),
        default_plan(dynamic_assignment_model()),
    ] {
        assert_eq!(
            signature(&plan),
            vec![(
                DefaultLocalSearchSelectorFamily::GroupedScalar,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            )],
        );
        assert_eq!(
            plan.components.acceptor,
            DefaultLocalSearchAcceptorPolicy::DiversifiedLateAcceptance { history_size: 400 }
        );
        assert_eq!(
            plan.components.forager,
            DefaultLocalSearchForagerPolicy::FirstLastStepScoreImproving {
                accepted_count_limit: None,
            }
        );
        assert!(plan.candidate_trace_plan().is_complete());
    }
}
