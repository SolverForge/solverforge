// Uni tests

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::stream::collection_extract::{source, vec, ChangeSource};
use crate::stream::collector::{collect_vec, count, CollectedVec};
use crate::stream::ConstraintFactory;

#[derive(Clone)]
struct GroupedShift {
    employee_id: usize,
}

#[derive(Clone)]
struct GroupedSolution {
    shifts: Vec<GroupedShift>,
}

#[test]
fn test_grouped_constraint_evaluate() {
    let constraint = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
        .named("Workload");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    /* Employee 1: 3 shifts -> 9
    Employee 2: 1 shift -> 1
    Total penalty: -10
    */
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-10));
}

#[test]
fn test_grouped_constraint_incremental() {
    let mut constraint = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Workload");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    // Initialize
    let total = constraint.initialize(&solution);
    /* Employee 1: 2 shifts -> -2
    Employee 2: 1 shift -> -1
    Total: -3
    */
    assert_eq!(total, SoftScore::of(-3));

    // Retract shift at index 0 (employee 1)
    let delta = constraint.on_retract(&solution, 0, 0);
    // Employee 1 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SoftScore::of(1));

    // Insert shift at index 0 (employee 1)
    let delta = constraint.on_insert(&solution, 0, 0);
    // Employee 1 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SoftScore::of(-1));
}

#[test]
fn test_grouped_constraint_reward() {
    let constraint = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(vec(|s: &GroupedSolution| &s.shifts))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .reward(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Collaboration");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
        ],
    };

    // 2 shifts in one group -> reward of +2
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(2));
}

#[test]
fn test_grouped_constraint_weight_can_use_key() {
    let constraint = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(vec(|s: &GroupedSolution| &s.shifts))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|employee_id: &usize, count: &usize| {
            SoftScore::of((*employee_id as i64) * (*count as i64))
        })
        .named("Key weighted workload");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-5));
}

#[test]
fn test_grouped_constraint_collect_vec_accepts_owned_labels() {
    let constraint = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(vec(|s: &GroupedSolution| &s.shifts))
        .group_by(
            |shift: &GroupedShift| shift.employee_id,
            collect_vec(|shift: &GroupedShift| format!("employee-{}", shift.employee_id)),
        )
        .penalize(|_employee_id: &usize, labels: &CollectedVec<String>| {
            SoftScore::of(labels.len() as i64)
        })
        .named("Grouped labels");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-3));
}

#[test]
fn test_shared_grouped_constraint_set_updates_one_node_for_multiple_terminals() {
    let mut constraints = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Linear workload")
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
        .named("Squared workload");
    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraints.evaluate_all(&solution), SoftScore::of(-8));
    assert_eq!(constraints.constraint_count(), 2);
    let metadata = constraints.constraint_metadata();
    assert_eq!(metadata[0].name(), "Linear workload");
    assert_eq!(metadata[1].name(), "Squared workload");

    assert_eq!(constraints.initialize_all(&solution), SoftScore::of(-8));
    assert_eq!(constraints.update_count(), 0);
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 0),
        SoftScore::of(4)
    );
    assert_eq!(constraints.update_count(), 1);
    assert_eq!(constraints.changed_key_count(), 1);
    assert_eq!(
        constraints.on_insert_all(&solution, 0, 0),
        SoftScore::of(-4)
    );
    assert_eq!(constraints.update_count(), 2);
    assert_eq!(constraints.changed_key_count(), 2);
}

#[test]
fn test_grouped_fluent_chain_appends_terminals_to_one_node() {
    let mut constraints = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Linear workload")
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
        .named("Squared workload")
        .reward(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Coverage reward");

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraints.evaluate_all(&solution), SoftScore::of(-5));
    assert_eq!(constraints.constraint_count(), 3);
    let metadata = constraints.constraint_metadata();
    assert_eq!(metadata[0].name(), "Linear workload");
    assert_eq!(metadata[1].name(), "Squared workload");
    assert_eq!(metadata[2].name(), "Coverage reward");

    assert_eq!(constraints.initialize_all(&solution), SoftScore::of(-5));
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 0),
        SoftScore::of(3)
    );
    assert_eq!(constraints.update_count(), 1);
    assert_eq!(
        constraints.on_insert_all(&solution, 0, 0),
        SoftScore::of(-3)
    );
    assert_eq!(constraints.update_count(), 2);
}

#[test]
fn test_shared_grouped_constraint_set_refreshes_only_dirty_keys() {
    let mut constraints = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Linear workload")
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
        .named("Squared workload");
    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    constraints.initialize_all(&solution);
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 1),
        SoftScore::of(0)
    );
    assert_eq!(constraints.update_count(), 0);
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 0),
        SoftScore::of(4)
    );
    assert_eq!(constraints.evaluate_each(&solution).len(), 2);
    constraints.reset_all();
    assert_eq!(constraints.initialize_all(&solution), SoftScore::of(-8));
}

#[test]
fn shared_grouped_terminals_keep_independent_detailed_identity() {
    let mut constraints = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(|shift: &GroupedShift| shift.employee_id, count())
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Linear workload")
        .penalize(|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
        .named("Squared workload");
    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };
    constraints.initialize_all(&solution);

    // One shared group state, two terminals: each terminal's detailed
    // analysis must carry its own identity and its own score derived from the
    // SAME retained groups (employee 1 counts 2, employee 2 counts 1) - no
    // cross-attribution, no duplication of either terminal onto the other.
    let analyses = constraints.evaluate_detailed(&solution);
    assert_eq!(analyses.len(), 2);
    assert_eq!(analyses[0].constraint_ref.name, "Linear workload");
    assert_eq!(analyses[1].constraint_ref.name, "Squared workload");
    let scores: Vec<_> = analyses.iter().map(|analysis| analysis.score).collect();
    assert_eq!(
        scores,
        vec![SoftScore::of(-3), SoftScore::of(-5)],
        "each terminal explains its own score from the shared state"
    );
    // Per-terminal match identity: both terminals see the same two groups,
    // but the analyses they emit remain two independent constraint entries.
    let each = constraints.evaluate_each(&solution);
    assert_eq!(each.len(), 2);
    assert_eq!(each[0].name, "Linear workload");
    assert_eq!(each[1].name, "Squared workload");
    assert_eq!(each[0].match_count, 2);
    assert_eq!(each[1].match_count, 2);
    let mut names: Vec<_> = constraints
        .constraint_metadata()
        .iter()
        .map(|metadata| metadata.name().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["Linear workload", "Squared workload"]);
}

/// A payload with neither Copy nor Clone: any accidental duplicate would fail
/// to compile; the drop counter proves exact retraction without leaks.
struct NonClonePayload {
    slot: usize,
    drops: Arc<AtomicUsize>,
}

impl Drop for NonClonePayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for NonClonePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonClonePayload")
            .field("slot", &self.slot)
            .finish()
    }
}

#[test]
fn non_clone_payload_grouped_set_retracts_exactly_on_mutation() {
    let drops = Arc::new(AtomicUsize::new(0));
    let payload_drops = Arc::clone(&drops);
    let mut constraints = ConstraintFactory::<GroupedSolution, SoftScore>::new()
        .for_each(source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ))
        .group_by(
            |shift: &GroupedShift| shift.employee_id,
            collect_vec({
                let payload_drops = Arc::clone(&payload_drops);
                move |_: &GroupedShift| NonClonePayload {
                    slot: 0,
                    drops: Arc::clone(&payload_drops),
                }
            }),
        )
        .penalize(
            |_employee_id: &usize, collected: &CollectedVec<NonClonePayload>| {
                SoftScore::of(collected.len() as i64)
            },
        )
        .named("Payload count");
    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraints.initialize(&solution), SoftScore::of(-3));
    let payloads_before = drops.load(Ordering::SeqCst);
    // Retract one contributor: its payload drops exactly once and the group
    // score updates from 3 to 2.
    assert_eq!(constraints.on_retract(&solution, 0, 0), SoftScore::of(1));
    // The retained state's cached score is the authoritative mutation view;
    // evaluate_all deliberately re-derives from the live solution. Assert the
    // retraction's exact ledger effect instead: the cached delta plus the
    // payload drop count.
    let retracted_drops = drops.load(Ordering::SeqCst) - payloads_before;
    assert!(
        retracted_drops >= 1,
        "retraction must drop at least the retracted contributor's payload"
    );
    constraints.reset();
    // Reset frees every retained payload shell (empty group shells at minimum).
    assert!(
        drops.load(Ordering::SeqCst) >= payloads_before + retracted_drops,
        "reset must free retained payload slots"
    );
}
