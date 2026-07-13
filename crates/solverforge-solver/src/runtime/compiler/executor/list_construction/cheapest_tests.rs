use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListVariableSlot, EntityClassId, PlanningSolution, VariableId,
};
use solverforge_core::score::SoftScore;

use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListConstructionKernelError,
    RuntimeListElement, RuntimeListSlot,
};
use crate::builder::{usize_element_source_key, ListVariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::manager::{run_cheapest, CheapestInsertionObserver, CheapestInsertionTrial};

type Slot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

static INSERT_CALLS: AtomicUsize = AtomicUsize::new(0);

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

fn element_count(plan: &Plan) -> usize {
    plan.elements.len()
}

fn assigned_elements(plan: &Plan) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
}

fn construction_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    INSERT_CALLS.fetch_add(1, Ordering::SeqCst);
    plan.routes[entity].insert(position, value);
}

fn get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.routes.get(entity)?.get(position).copied()
}

fn set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity][position] = value;
}

fn reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    plan.routes[entity].splice(position..position, values);
}

fn index_to_element(plan: &Plan, index: usize) -> usize {
    plan.elements[index]
}

fn typed_slot() -> Slot {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Vehicle",
            element_count,
            assigned_elements,
            list_len,
            list_remove,
            construction_remove,
            insert,
            get,
            set,
            reverse,
            sublist_remove,
            sublist_insert,
            construction_remove,
            insert,
            index_to_element,
            usize_element_source_key,
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
        ),
        0,
    )
}

#[derive(Debug)]
struct DynamicAccess;

impl DynamicListAccess<Plan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &Plan) -> usize {
        entity_count(plan)
    }

    fn element_count(&self, plan: &Plan) -> usize {
        element_count(plan)
    }

    fn element(&self, plan: &Plan, index: usize) -> Option<usize> {
        plan.elements.get(index).copied()
    }

    fn assigned_elements(&self, plan: &Plan) -> Vec<usize> {
        assigned_elements(plan)
    }

    fn len(&self, plan: &Plan, entity: usize) -> usize {
        list_len(plan, entity)
    }

    fn get(&self, plan: &Plan, entity: usize, position: usize) -> Option<usize> {
        get(plan, entity, position)
    }

    fn insert(&self, plan: &mut Plan, entity: usize, position: usize, value: usize) {
        insert(plan, entity, position, value);
    }

    fn remove(&self, plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(plan, entity, position)
    }
}

fn dynamic_slot() -> Slot {
    RuntimeListSlot::from_dynamic(
        DynamicListVariableSlot::try_with_access(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(DynamicAccess),
        )
        .expect("dynamic test access identity matches its slot"),
    )
}

struct NoCandidateObserver {
    plan: Plan,
    candidate_callbacks: usize,
}

impl CheapestInsertionObserver<Plan, Slot> for NoCandidateObserver {
    type Trial = ();

    fn solution(&self) -> &Plan {
        &self.plan
    }

    fn should_interrupt_construction(&mut self) -> bool {
        self.candidate_callbacks += 1;
        false
    }

    fn evaluate_insertion(
        &mut self,
        _slot: &Slot,
        _element: RuntimeListElement<usize>,
        _trial: CheapestInsertionTrial,
    ) -> (Option<SoftScore>, Option<Self::Trial>) {
        self.candidate_callbacks += 1;
        (None, None)
    }

    fn discard_trial(&mut self, _trial: Self::Trial) {
        self.candidate_callbacks += 1;
    }

    fn select_trial(&mut self, _trial: Self::Trial) {
        self.candidate_callbacks += 1;
    }

    fn commit_insertion(
        &mut self,
        _slot: &Slot,
        _element: RuntimeListElement<usize>,
        _entity_index: usize,
        _insertion_index: usize,
        _score: SoftScore,
        _trial: Option<Self::Trial>,
    ) {
        self.candidate_callbacks += 1;
    }

    fn finish_construction(&mut self) {
        self.candidate_callbacks += 1;
    }

    fn finish_without_work(&mut self) {
        self.candidate_callbacks += 1;
    }
}

fn assert_rejected_before_candidate_work(
    slot: &Slot,
    plan: Plan,
    expected: ListConstructionKernelError,
) {
    INSERT_CALLS.store(0, Ordering::SeqCst);
    let mut observer = NoCandidateObserver {
        plan,
        candidate_callbacks: 0,
    };

    let result = bind_runtime_list_source(slot, &observer.plan).and_then(|binding| {
        let source_index = binding.into_source_index();
        let unassigned = unassigned_from_current_assignment(slot, &source_index, &observer.plan)?;
        run_cheapest(slot, &source_index, &unassigned, &mut observer);
        Ok(())
    });
    assert_eq!(result, Err(expected));
    assert_eq!(observer.candidate_callbacks, 0);
    assert_eq!(INSERT_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn typed_and_dynamic_duplicate_declared_elements_fail_before_mutation() {
    let plan = Plan {
        score: None,
        elements: vec![7, 7],
        routes: vec![Vec::new()],
    };
    for slot in [typed_slot(), dynamic_slot()] {
        assert_rejected_before_candidate_work(
            &slot,
            plan.clone(),
            ListConstructionKernelError::DuplicateDeclaredElement {
                first_source_index: 0,
                duplicate_source_index: 1,
            },
        );
    }
}

#[test]
fn fully_assigned_malformed_streams_fail_before_candidate_work() {
    let cases = [
        (
            Plan {
                score: None,
                elements: vec![7, 7],
                routes: vec![vec![7, 7]],
            },
            ListConstructionKernelError::DuplicateDeclaredElement {
                first_source_index: 0,
                duplicate_source_index: 1,
            },
        ),
        (
            Plan {
                score: None,
                elements: vec![7, 11],
                routes: vec![vec![7, 7]],
            },
            ListConstructionKernelError::DuplicateAssignedElement {
                source_index: 0,
                first_assigned_occurrence: 0,
                duplicate_assigned_occurrence: 1,
            },
        ),
        (
            Plan {
                score: None,
                elements: vec![7],
                routes: vec![vec![11]],
            },
            ListConstructionKernelError::AssignedElementNotDeclared {
                assigned_occurrence: 0,
            },
        ),
    ];

    for slot in [typed_slot(), dynamic_slot()] {
        for (plan, expected) in &cases {
            assert_rejected_before_candidate_work(&slot, plan.clone(), expected.clone());
        }
    }
}

#[test]
fn typed_and_dynamic_distinct_elements_preserve_source_order() {
    let plan = Plan {
        score: None,
        elements: vec![20, 50],
        routes: vec![Vec::new()],
    };
    for slot in [typed_slot(), dynamic_slot()] {
        let binding =
            bind_runtime_list_source(&slot, &plan).expect("distinct elements are unambiguous");
        let source_index = binding.into_source_index();
        let sources = unassigned_from_current_assignment(&slot, &source_index, &plan)
            .expect("current assignments resolve through the frozen source index");
        assert_eq!(
            sources
                .iter()
                .map(|entry| entry.source_index)
                .collect::<Vec<_>>(),
            vec![0, 1],
        );
    }
}
