use super::super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::super::assignment_value_index::assigned_value_sequence_index;
use super::*;
use crate::builder::{
    bind_scalar_groups, ScalarGroupBindingKind, ScalarGroupLimits, ScalarVariableSlot, ValueSource,
};
use crate::planning::{ScalarGroup, ScalarTarget};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct Plan {
    values: Vec<Option<usize>>,
    sequence_reads: &'static AtomicUsize,
    capacity_reads: &'static AtomicUsize,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;
    fn score(&self) -> Option<SoftScore> {
        None
    }
    fn set_score(&mut self, _: Option<SoftScore>) {}
}

fn binding() -> ScalarAssignmentBinding<Plan> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Row",
        |plan: &Plan| plan.values.len(),
        "value",
        |plan, entity, _| plan.values[entity],
        |plan, entity, _, value| plan.values[entity] = value,
        ValueSource::EntitySlice {
            values_for_entity: |_, _, _| &[0, 1, 2],
        },
        true,
    );
    let mut groups = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "membership",
            ScalarTarget::from_descriptor_index(0, "value"),
        )
        .with_capacity_key(|plan: &Plan, entity, value| {
            plan.capacity_reads.fetch_add(1, Ordering::Relaxed);
            Some(entity * 3 + value)
        })
        .with_sequence_key(|plan: &Plan, entity, _| {
            plan.sequence_reads.fetch_add(1, Ordering::Relaxed);
            Some(entity)
        })],
        &[slot],
    );
    let ScalarGroupBindingKind::Assignment(binding) = groups.remove(0).kind else {
        panic!("expected assignment binding");
    };
    binding
}

#[test]
fn value_index_uses_recorded_membership_keys_without_reinvoking_callbacks() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let plan = Plan {
        values: vec![Some(2), None, Some(0)],
        sequence_reads: &SEQUENCE_READS,
        capacity_reads: &CAPACITY_READS,
    };
    let state = ScalarAssignmentState::new(&group, &plan);
    SEQUENCE_READS.store(0, Ordering::Relaxed);
    CAPACITY_READS.store(0, Ordering::Relaxed);
    let index = assigned_value_sequence_index(
        &group,
        &plan,
        &state,
        ScalarAssignmentMoveOptions::for_construction(ScalarGroupLimits::new()),
    );
    assert_eq!(index.values, vec![0, 2]);
    assert_eq!(index.sequence_keys, vec![0, 2]);
    assert_eq!(index.by_value_sequence.get(&(2, 0)), Some(&vec![0]));
    assert_eq!(index.by_value_sequence.get(&(0, 2)), Some(&vec![2]));
    assert_eq!(
        SEQUENCE_READS.load(Ordering::Relaxed),
        0,
        "the base already owns the membership keys"
    );
    assert_eq!(CAPACITY_READS.load(Ordering::Relaxed), 0);
}

#[test]
fn overlay_retracts_recorded_capacity_membership_and_leaves_base_unchanged() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let plan = Plan {
        values: vec![Some(2), None, Some(0)],
        sequence_reads: &SEQUENCE_READS,
        capacity_reads: &CAPACITY_READS,
    };
    let mut state = ScalarAssignmentState::new(&group, &plan);
    let before = state.index_snapshot();
    CAPACITY_READS.store(0, Ordering::Relaxed);
    assert!(state.assignment_feasible_after_edits(&group, &plan, &[(0, None), (1, Some(1))]));
    assert_eq!(
        CAPACITY_READS.load(Ordering::Relaxed),
        1,
        "only arrivals evaluate a new capacity key"
    );
    assert_eq!(state.index_snapshot(), before);
    assert_eq!(state.capacity_keys, vec![Some(2), None, Some(6)]);
    assert_eq!(state.sequence_keys, vec![Some(0), None, Some(2)]);
}
