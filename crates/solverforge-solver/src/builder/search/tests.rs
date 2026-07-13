use std::any::TypeId;

use solverforge_core::domain::{
    DynamicModelBackend, DynamicScalarVariableSlot, EntityClassId, EntityDescriptor,
    PlanningSolution, SolutionDescriptor, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;

use crate::builder::{RuntimeModel, VariableSlot};
use crate::scope::{ProgressCallback, SolverScope};
use crate::RuntimeBuildError;

use super::{CustomSearchPhase, RuntimeExtensionRegistry, SearchContext};

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl DynamicModelBackend for TestSolution {
    type Score = SoftScore;

    fn entity_count(&self, _entity: EntityClassId) -> usize {
        0
    }

    fn get_scalar(
        &self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
    ) -> Option<usize> {
        None
    }

    fn set_scalar(
        &mut self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
        _value: Option<usize>,
    ) {
    }

    fn list_len(&self, _entity: EntityClassId, _row: usize, _variable: VariableId) -> usize {
        0
    }

    fn list_get(
        &self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
        _pos: usize,
    ) -> Option<usize> {
        None
    }

    fn list_insert(
        &mut self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
        _pos: usize,
        _value: usize,
    ) {
    }

    fn list_remove(
        &mut self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
        _pos: usize,
    ) -> Option<usize> {
        None
    }

    fn candidate_values(
        &self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
    ) -> &[usize] {
        &[]
    }

    fn scalar_value_is_legal(
        &self,
        _entity: EntityClassId,
        _row: usize,
        _variable: VariableId,
        _value: usize,
    ) -> bool {
        false
    }
}

#[derive(Debug)]
struct MarkerPhase(&'static str);

impl CustomSearchPhase<TestSolution> for MarkerPhase {
    fn solve<D, ProgressCb>(
        &mut self,
        _solver_scope: &mut SolverScope<'_, TestSolution, D, ProgressCb>,
    ) where
        D: solverforge_scoring::Director<TestSolution>,
        ProgressCb: ProgressCallback<TestSolution>,
    {
    }

    fn phase_type_name(&self) -> &'static str {
        self.0
    }
}

fn search_context() -> SearchContext<TestSolution> {
    SearchContext::new(
        SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>()),
        RuntimeModel::new(Vec::new()),
        Some(7),
    )
}

#[test]
fn search_context_resolves_dynamic_slots_for_custom_search() {
    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(
            EntityDescriptor::new("Vehicle", TypeId::of::<TestSolution>(), "vehicles")
                .with_logical_id(EntityClassId(1)),
        )
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<TestSolution>(), "tasks")
                .with_logical_id(EntityClassId(0))
                .with_variable(
                    VariableDescriptor::genuine("worker").with_logical_id(VariableId(0)),
                ),
        );
    let scalar =
        DynamicScalarVariableSlot::new(EntityClassId(0), VariableId(0), "Task", "worker", false);
    let model: RuntimeModel<TestSolution, usize, (), ()> =
        RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);

    let context = SearchContext::try_new(descriptor, model, Some(7))
        .expect("descriptor-backed dynamic slot should resolve");
    let scalar = context
        .model()
        .dynamic_scalar_variables()
        .next()
        .expect("dynamic scalar slot");

    assert!(scalar.is_descriptor_resolved());
    assert_eq!(scalar.descriptor_index(), 1);
}

#[test]
fn search_context_reports_descriptor_resolution_as_a_runtime_build_error() {
    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(99),
        VariableId(0),
        "Missing",
        "worker",
        false,
    );
    let model: RuntimeModel<TestSolution, usize, (), ()> =
        RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);

    let error = match SearchContext::try_new(descriptor, model, Some(7)) {
        Ok(_) => panic!("unknown dynamic entity must fail declaration construction"),
        Err(error) => error,
    };

    match error {
        RuntimeBuildError::Declaration { message } => assert!(!message.is_empty()),
        other => panic!("expected declaration error, got {other:?}"),
    }
}

#[test]
fn search_builder_transfers_only_declaration_parts() {
    let search = search_context()
        .defaults()
        .phase("weekend_repair", |_| MarkerPhase("weekend_repair"))
        .partitioned_phase("by_task", |_context, _config| MarkerPhase("by_task"));

    let (context, extensions) = search.into_runtime_parts();

    assert_eq!(context.seed(), Some(7));
    assert!(extensions.contains_custom("weekend_repair"));
    assert!(extensions.contains_partitioned("by_task"));
}

#[test]
#[should_panic(expected = "custom phase `repair` was registered more than once")]
fn custom_search_rejects_duplicate_registered_names() {
    let _ = search_context()
        .defaults()
        .phase("repair", |_| MarkerPhase("first"))
        .phase("repair", |_| MarkerPhase("second"));
}
