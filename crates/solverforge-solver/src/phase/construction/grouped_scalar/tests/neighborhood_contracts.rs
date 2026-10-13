use super::super::assignment_cycle::CycleWindowCursor;
use super::super::assignment_entity::{AssignmentMoveKind, EntityValueCursor};
use super::super::assignment_pair::PairWindowCursor;
use super::*;
use solverforge_core::fast_hash::FastMap;

#[test]
fn final_overlay_rejects_duplicate_entity_targets_without_changing_state() {
    let binding = assignment_binding(&descriptor(), ScalarGroupLimits::new());
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1)],
        candidates: vec![vec![0, 1, 2, 3], vec![0, 1, 2, 3]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &solution);
    assert!(!state.assignment_feasible_after_edits(
        &binding,
        &solution,
        &[(0, Some(2)), (0, Some(3))],
    ));
    assert_eq!(state.current_value(0), Some(0));
    assert_eq!(state.assigned_count(0), 1);
    assert_eq!(state.assigned_count(2), 0);
    assert_eq!(state.assigned_count(3), 0);
}

#[test]
fn final_overlay_does_not_unassign_a_required_entity() {
    let binding = assignment_binding(&descriptor(), ScalarGroupLimits::new());
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0)],
        candidates: vec![vec![0, 1]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &solution);
    assert!(!state.assignment_feasible_after_edits(&binding, &solution, &[(0, None)]));
    assert_eq!(state.current_value(0), Some(0));
}

#[test]
fn ejection_discovery_does_not_rebuild_the_whole_assignment_state() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static READS: AtomicUsize = AtomicUsize::new(0);
    fn counted_value(plan: &AssignmentPlan, entity: usize, _: usize) -> Option<usize> {
        READS.fetch_add(1, Ordering::Relaxed);
        plan.assignments[entity]
    }
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        counted_value,
        set_value,
        ValueSource::EntitySlice {
            values_for_entity: candidates,
        },
        true,
    )
    .with_candidate_values(candidates);
    let mut groups = bind_scalar_groups(
        vec![
            ScalarGroup::assignment("ejection", ScalarTarget::from_descriptor_index(0, "worker"))
                .with_required_entity(required)
                .with_capacity_key(capacity_key),
        ],
        &[slot],
    );
    let ScalarGroupBindingKind::Assignment(binding) = groups.remove(0).kind else {
        panic!("expected assignment group");
    };
    let mut plan = AssignmentPlan {
        score: None,
        assignments: (0..100).map(Some).collect(),
        candidates: (0..100).map(|i| vec![i]).collect(),
    };
    plan.candidates[0] = vec![1];
    plan.candidates[1] = vec![0];
    let mut state = ScalarAssignmentState::new(&binding, &plan);
    let mut cursor = CycleWindowCursor::ejection(&binding, &plan, &state, options());
    READS.store(0, Ordering::Relaxed);
    let candidate = cursor
        .next(&binding, &plan, &mut state)
        .expect("the first window swaps");
    assert_eq!(candidate.edits().len(), 2);
    assert!(
        READS.load(Ordering::Relaxed) < 10,
        "discovery read untouched assignment rows"
    );
}

fn options() -> ScalarAssignmentMoveOptions {
    ScalarAssignmentMoveOptions::for_selector(ScalarGroupLimits::new(), None, 256, 0)
}

#[test]
fn simultaneous_swap_vacates_both_capacity_slots() {
    let binding = assignment_binding(&descriptor(), ScalarGroupLimits::new());
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1)],
        candidates: vec![vec![0, 1], vec![0, 1]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &solution);
    let mut cursor = PairWindowCursor::swap(&binding, &solution, &state, options());
    let candidate = cursor
        .next(&binding, &solution, &mut state)
        .expect("a simultaneous swap vacates both occupied slots");
    assert_eq!(candidate.edits().len(), 2);
    assert_eq!(candidate.edits()[0].to_value, Some(1));
    assert_eq!(candidate.edits()[1].to_value, Some(0));
}

#[test]
fn required_ejection_reinsert_produces_one_final_edit_per_entity() {
    let binding = assignment_binding(&descriptor(), ScalarGroupLimits::new());
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1)],
        candidates: vec![vec![1], vec![0]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &solution);
    let mut cursor = CycleWindowCursor::ejection(&binding, &solution, &state, options());
    let candidate = cursor
        .next(&binding, &solution, &mut state)
        .expect("required entities must be reinserted in one valid compound move");
    assert_eq!(candidate.edits().len(), 2);
    assert_ne!(
        candidate.edits()[0].entity_index,
        candidate.edits()[1].entity_index
    );
    assert!(candidate.edits().iter().all(|edit| edit.to_value.is_some()));
}

#[test]
fn reassignment_preserves_required_blocker_rematching() {
    let binding = assignment_binding(&descriptor(), ScalarGroupLimits::new());
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1)],
        candidates: vec![vec![0, 1], vec![1, 2]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &solution);
    let mut cursor = EntityValueCursor {
        entities: vec![0],
        entity_pos: 0,
        values: Vec::new(),
        value_pos: 0,
        value_degrees: FastMap::default(),
        options: options(),
        kind: AssignmentMoveKind::Reassignment,
    };
    let candidate = cursor
        .next(&binding, &solution, &mut state, &mut || false)
        .expect("the blocked target has a legal required-entity rematch");
    assert_eq!(candidate.edits().len(), 2);
    assert!(candidate
        .edits()
        .iter()
        .any(|edit| edit.entity_index == 0 && edit.to_value == Some(1)));
    assert!(candidate
        .edits()
        .iter()
        .any(|edit| edit.entity_index == 1 && edit.to_value == Some(2)));
}
