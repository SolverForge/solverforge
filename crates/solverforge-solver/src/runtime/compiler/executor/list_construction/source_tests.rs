use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListVariableSlot, EntityClassId, PlanningSolution, VariableId,
};
use solverforge_core::score::SoftScore;

use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListConstructionKernelError,
    RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex,
};
use crate::builder::ListVariableSlot;
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

type KeyedSlot = RuntimeListSlot<
    KeyedPlan,
    KeyedPayload,
    DefaultCrossEntityDistanceMeter,
    DefaultCrossEntityDistanceMeter,
>;
type MissingDynamicSlot = RuntimeListSlot<
    MissingDynamicPlan,
    usize,
    DefaultCrossEntityDistanceMeter,
    DefaultCrossEntityDistanceMeter,
>;

static SOURCE_KEY_CALLS: AtomicUsize = AtomicUsize::new(0);
static DECLARATION_CALLS: AtomicUsize = AtomicUsize::new(0);
static SOURCE_KEY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
struct KeyedPayload {
    key: usize,
}

impl PartialEq for KeyedPayload {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Clone)]
struct KeyedPlan {
    score: Option<SoftScore>,
    elements: Vec<KeyedPayload>,
    routes: Vec<Vec<KeyedPayload>>,
}

impl PlanningSolution for KeyedPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn keyed_element_count(plan: &KeyedPlan) -> usize {
    plan.elements.len()
}

fn keyed_assigned_elements(plan: &KeyedPlan) -> Vec<KeyedPayload> {
    plan.routes.iter().flatten().cloned().collect()
}

fn keyed_list_len(plan: &KeyedPlan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn keyed_list_remove(plan: &mut KeyedPlan, entity: usize, position: usize) -> Option<KeyedPayload> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
}

fn keyed_construction_remove(plan: &mut KeyedPlan, entity: usize, position: usize) -> KeyedPayload {
    plan.routes[entity].remove(position)
}

fn keyed_list_insert(plan: &mut KeyedPlan, entity: usize, position: usize, value: KeyedPayload) {
    plan.routes[entity].insert(position, value);
}

fn keyed_list_get(plan: &KeyedPlan, entity: usize, position: usize) -> Option<KeyedPayload> {
    plan.routes.get(entity)?.get(position).cloned()
}

fn keyed_list_set(plan: &mut KeyedPlan, entity: usize, position: usize, value: KeyedPayload) {
    plan.routes[entity][position] = value;
}

fn keyed_list_reverse(plan: &mut KeyedPlan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn keyed_sublist_remove(
    plan: &mut KeyedPlan,
    entity: usize,
    start: usize,
    end: usize,
) -> Vec<KeyedPayload> {
    plan.routes[entity].drain(start..end).collect()
}

fn keyed_sublist_insert(
    plan: &mut KeyedPlan,
    entity: usize,
    position: usize,
    values: Vec<KeyedPayload>,
) {
    plan.routes[entity].splice(position..position, values);
}

fn keyed_index_to_element(plan: &KeyedPlan, source_index: usize) -> KeyedPayload {
    DECLARATION_CALLS.fetch_add(1, Ordering::SeqCst);
    plan.elements[source_index].clone()
}

fn keyed_source_key(_plan: &KeyedPlan, element: &KeyedPayload) -> usize {
    SOURCE_KEY_CALLS.fetch_add(1, Ordering::SeqCst);
    element.key
}

fn keyed_entity_count(plan: &KeyedPlan) -> usize {
    plan.routes.len()
}

fn keyed_slot() -> KeyedSlot {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Vehicle",
            keyed_element_count,
            keyed_assigned_elements,
            keyed_list_len,
            keyed_list_remove,
            keyed_construction_remove,
            keyed_list_insert,
            keyed_list_get,
            keyed_list_set,
            keyed_list_reverse,
            keyed_sublist_remove,
            keyed_sublist_insert,
            keyed_construction_remove,
            keyed_list_insert,
            keyed_index_to_element,
            keyed_source_key,
            keyed_entity_count,
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
        ),
        0,
    )
}

#[test]
fn source_keys_distinguish_equal_static_payloads_and_freeze_declared_values() {
    let _guard = SOURCE_KEY_TEST_LOCK.lock().expect("source-key test lock");
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let slot = keyed_slot();
    let mut plan = KeyedPlan {
        score: None,
        elements: vec![KeyedPayload { key: 101 }, KeyedPayload { key: 202 }],
        routes: vec![vec![KeyedPayload { key: 101 }]],
    };
    assert_eq!(plan.elements[0], plan.elements[1]);

    let source_index = RuntimeListSourceIndex::bind(&slot, &plan).expect("distinct keys bind");
    plan.elements[1].key = 999;
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("assigned key remains declared");

    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].source_index, 1);
    assert!(matches!(
        &unassigned[0].element,
        RuntimeListElement::Static(KeyedPayload { key: 202 })
    ));
}

#[test]
fn source_key_resolution_is_linear_in_declared_and_assigned_elements() {
    const ELEMENT_COUNT: usize = 4096;

    let _guard = SOURCE_KEY_TEST_LOCK.lock().expect("source-key test lock");
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let slot = keyed_slot();
    let elements = (0..ELEMENT_COUNT)
        .map(|key| KeyedPayload { key })
        .collect::<Vec<_>>();
    let plan = KeyedPlan {
        score: None,
        routes: vec![elements[..ELEMENT_COUNT / 2].to_vec()],
        elements,
    };

    let source_index = RuntimeListSourceIndex::bind(&slot, &plan).expect("source index binds");
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("assigned elements resolve");

    assert_eq!(unassigned.len(), ELEMENT_COUNT / 2);
    assert_eq!(unassigned[0].source_index, ELEMENT_COUNT / 2);
    assert_eq!(
        SOURCE_KEY_CALLS.load(Ordering::SeqCst),
        ELEMENT_COUNT + ELEMENT_COUNT / 2,
        "bind keys each declared element once and resolves each assigned element once"
    );
}

#[test]
fn assignment_refresh_uses_the_frozen_source_without_rereading_declarations() {
    let _guard = SOURCE_KEY_TEST_LOCK.lock().expect("source-key test lock");
    DECLARATION_CALLS.store(0, Ordering::SeqCst);
    let slot = keyed_slot();
    let mut plan = KeyedPlan {
        score: None,
        elements: vec![KeyedPayload { key: 101 }, KeyedPayload { key: 202 }],
        routes: vec![Vec::new()],
    };
    let source_index = bind_runtime_list_source(&slot, &plan)
        .expect("initial declaration stream binds exactly once")
        .into_source_index();
    assert_eq!(DECLARATION_CALLS.load(Ordering::SeqCst), 2);

    plan.routes[0].push(KeyedPayload { key: 101 });
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("current assignment resolves through the frozen source key index");

    assert_eq!(DECLARATION_CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].source_index, 1);
    assert!(matches!(
        &unassigned[0].element,
        RuntimeListElement::Static(KeyedPayload { key: 202 })
    ));
}

#[derive(Clone)]
struct MissingDynamicPlan {
    score: Option<SoftScore>,
    elements: Vec<Option<usize>>,
    routes: Vec<Vec<usize>>,
}

impl PlanningSolution for MissingDynamicPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Debug)]
struct MissingDynamicAccess;

impl DynamicListAccess<MissingDynamicPlan> for MissingDynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &MissingDynamicPlan) -> usize {
        plan.routes.len()
    }

    fn element_count(&self, plan: &MissingDynamicPlan) -> usize {
        plan.elements.len()
    }

    fn element(&self, plan: &MissingDynamicPlan, source_index: usize) -> Option<usize> {
        plan.elements.get(source_index).copied().flatten()
    }

    fn assigned_elements(&self, plan: &MissingDynamicPlan) -> Vec<usize> {
        plan.routes.iter().flatten().copied().collect()
    }

    fn len(&self, plan: &MissingDynamicPlan, entity: usize) -> usize {
        plan.routes[entity].len()
    }

    fn get(&self, plan: &MissingDynamicPlan, entity: usize, position: usize) -> Option<usize> {
        plan.routes.get(entity)?.get(position).copied()
    }

    fn insert(&self, plan: &mut MissingDynamicPlan, entity: usize, position: usize, value: usize) {
        plan.routes[entity].insert(position, value);
    }

    fn remove(
        &self,
        plan: &mut MissingDynamicPlan,
        entity: usize,
        position: usize,
    ) -> Option<usize> {
        (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
    }
}

#[test]
fn dynamic_missing_declared_source_element_is_a_typed_bind_error() {
    let slot: MissingDynamicSlot = RuntimeListSlot::from_dynamic(
        DynamicListVariableSlot::try_with_access(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(MissingDynamicAccess),
        )
        .expect("dynamic test access identity matches its slot"),
    );
    let plan = MissingDynamicPlan {
        score: None,
        elements: vec![Some(7), None],
        routes: vec![Vec::new()],
    };

    assert!(matches!(
        RuntimeListSourceIndex::bind(&slot, &plan),
        Err(ListConstructionKernelError::MissingDeclaredElement { source_index: 1 })
    ));
}
