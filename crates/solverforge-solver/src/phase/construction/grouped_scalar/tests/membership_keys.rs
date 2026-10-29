use super::super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::super::assignment_value_index::assigned_value_sequence_index;
use super::*;
use crate::builder::context::scalar::group::bind_scalar_groups;
use crate::builder::{ScalarGroupBindingKind, ScalarGroupLimits, ScalarVariableSlot, ValueSource};
use crate::planning::scalar::target::ScalarTarget;
use crate::planning::ScalarGroup;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct Plan {
    values: Vec<Option<usize>>,
    key_epoch: usize,
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
        .with_required_entity(|plan: &Plan, entity| plan.key_epoch % 2 == 1 && entity % 3 == 0)
        .with_capacity_key(|plan: &Plan, entity, value| {
            plan.capacity_reads.fetch_add(1, Ordering::Relaxed);
            Some(plan.key_epoch * 1000 + (entity % 3) * 3 + value)
        })
        .with_sequence_key(|plan: &Plan, entity, _| {
            plan.sequence_reads.fetch_add(1, Ordering::Relaxed);
            Some(entity % 5 + plan.key_epoch)
        })],
        &[slot],
    );
    let ScalarGroupBindingKind::Assignment(binding) = groups.remove(0).kind else {
        panic!("expected assignment binding");
    };
    binding
}

#[test]
fn sparse_membership_refresh_matches_rebuild_through_batches_and_global_key_changes() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let mut plan = Plan {
        values: vec![None; 31],
        key_epoch: 0,
        sequence_reads: &SEQUENCE_READS,
        capacity_reads: &CAPACITY_READS,
    };
    let mut state = ScalarAssignmentState::new(&group, &plan);
    let mut seed = 7_u64;
    for step in 0..400 {
        if step % 29 == 0 {
            plan.key_epoch += 1;
        }
        let mut rows = Vec::new();
        for entity in 0..plan.values.len() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            if seed.is_multiple_of(7) || step % 29 == 0 {
                let value = (seed >> 32) % 4;
                plan.values[entity] = (value < 3).then_some(value as usize);
                rows.push((entity, plan.values[entity]));
            }
        }
        let assigned_updates = rows.iter().filter(|(_, value)| value.is_some()).count();
        SEQUENCE_READS.store(0, Ordering::Relaxed);
        CAPACITY_READS.store(0, Ordering::Relaxed);
        state.refresh_rows(&group, &plan, rows);
        assert_eq!(SEQUENCE_READS.load(Ordering::Relaxed), assigned_updates);
        assert_eq!(CAPACITY_READS.load(Ordering::Relaxed), assigned_updates);
        let rebuilt = ScalarAssignmentState::new(&group, &plan);
        assert_eq!(
            state.index_snapshot(),
            rebuilt.index_snapshot(),
            "step {step}"
        );
        assert_eq!(state.values, rebuilt.values);
        assert_eq!(state.required, rebuilt.required);
        assert_eq!(state.capacity_keys, rebuilt.capacity_keys);
        assert_eq!(state.sequence_keys, rebuilt.sequence_keys);
    }
}

#[test]
fn value_index_uses_recorded_membership_keys_without_reinvoking_callbacks() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let plan = Plan {
        values: vec![Some(2), None, Some(0)],
        key_epoch: 0,
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

/// The LS step loop drops its cursor after each committed step. The grouped
/// pool must (a) hand the maintained base back to the solve-owned slot,
/// (b) reuse it without callback-driven rebuild when nothing moved, and
/// (c) refresh only the changed rows when a commit moved values.
#[test]
fn grouped_assignment_pool_refreshes_only_changed_rows_between_steps() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let mut plan = Plan {
        values: vec![None, None, Some(2), None],
        key_epoch: 0,
        sequence_reads: &SEQUENCE_READS,
        capacity_reads: &CAPACITY_READS,
    };
    let mut pool = Some(ScalarAssignmentState::new(&group, &plan));

    // Step 2: one committed change moved row 0 to Some(0). Maintenance must
    // see the diff, refresh exactly that row, and match a fresh rebuild.
    plan.values = vec![Some(0), None, Some(2), None];
    {
        let state = pool.as_mut().expect("pool live between steps");
        let changed = (0..state.row_count())
            .filter(|&entity| state.current_value(entity) != group.current_value(&plan, entity))
            .collect::<Vec<_>>();
        assert_eq!(changed, vec![0]);
        SEQUENCE_READS.store(0, Ordering::Relaxed);
        CAPACITY_READS.store(0, Ordering::Relaxed);
        state.refresh_rows(
            &group,
            &plan,
            changed
                .into_iter()
                .map(|entity| (entity, group.current_value(&plan, entity))),
        );
        assert_eq!(
            CAPACITY_READS.load(Ordering::Relaxed),
            1,
            "one moved row re-derives its keys once"
        );
        let rebuilt = ScalarAssignmentState::new(&group, &plan);
        assert_eq!(state.index_snapshot(), rebuilt.index_snapshot());
    }

    // Step 3 with no committed change: zero key callbacks.
    SEQUENCE_READS.store(0, Ordering::Relaxed);
    CAPACITY_READS.store(0, Ordering::Relaxed);
    {
        let state = pool.as_ref().expect("pool live between steps");
        assert!((0..state.row_count())
            .all(|entity| state.current_value(entity) == group.current_value(&plan, entity)));
    }
    assert_eq!(CAPACITY_READS.load(Ordering::Relaxed), 0);
    assert_eq!(SEQUENCE_READS.load(Ordering::Relaxed), 0);
}

#[test]
fn overlay_retracts_recorded_capacity_membership_and_leaves_base_unchanged() {
    static SEQUENCE_READS: AtomicUsize = AtomicUsize::new(0);
    static CAPACITY_READS: AtomicUsize = AtomicUsize::new(0);
    let group = binding();
    let plan = Plan {
        values: vec![Some(2), None, Some(0)],
        key_epoch: 0,
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
