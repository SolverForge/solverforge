// GroupedUniConstraint tests

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::grouped::{
    GroupedNodeState, GroupedTerminalScorer, GroupedUniConstraint, SharedGroupedConstraintSet,
};
use crate::stream::collection_extract::{source, vec, ChangeSource};
use crate::stream::collector::{collect_vec, count, CollectedVec};
use crate::stream::filter::TrueFilter;
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
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64),
        false,
    );

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
    let mut constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

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
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Collaboration"),
        ImpactType::Reward,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

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
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Key weighted workload"),
        ImpactType::Penalty,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |employee_id: &usize, count: &usize| SoftScore::of((*employee_id as i64) * (*count as i64)),
        false,
    );

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
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Grouped labels"),
        ImpactType::Penalty,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        collect_vec(|shift: &GroupedShift| format!("employee-{}", shift.employee_id)),
        |_employee_id: &usize, labels: &CollectedVec<String>| SoftScore::of(labels.len() as i64),
        false,
    );

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
    let state = GroupedNodeState::new(
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
    );
    let scorers = (
        GroupedTerminalScorer::new(
            ConstraintRef::new("", "Linear workload"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
            false,
        ),
        GroupedTerminalScorer::new(
            ConstraintRef::new("", "Squared workload"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64),
            false,
        ),
    );
    let mut constraints = SharedGroupedConstraintSet::new(state, scorers);
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
    assert_eq!(constraints.state().update_count(), 0);
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 0),
        SoftScore::of(4)
    );
    assert_eq!(constraints.state().update_count(), 1);
    assert_eq!(constraints.state().changed_key_count(), 1);
    assert_eq!(
        constraints.on_insert_all(&solution, 0, 0),
        SoftScore::of(-4)
    );
    assert_eq!(constraints.state().update_count(), 2);
    assert_eq!(constraints.state().changed_key_count(), 2);
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
    assert_eq!(constraints.state().update_count(), 1);
    assert_eq!(
        constraints.on_insert_all(&solution, 0, 0),
        SoftScore::of(-3)
    );
    assert_eq!(constraints.state().update_count(), 2);
}

#[test]
fn test_shared_grouped_constraint_set_refreshes_only_dirty_keys() {
    let state = GroupedNodeState::new(
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
    );
    let scorers = (
        GroupedTerminalScorer::new(
            ConstraintRef::new("", "Linear workload"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
            false,
        ),
        GroupedTerminalScorer::new(
            ConstraintRef::new("", "Squared workload"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64),
            false,
        ),
    );
    let mut constraints = SharedGroupedConstraintSet::new(state, scorers);
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
    assert_eq!(constraints.state().update_count(), 0);
    assert_eq!(
        constraints.on_retract_all(&solution, 0, 0),
        SoftScore::of(4)
    );
    assert_eq!(constraints.evaluate_each(&solution).len(), 2);
    constraints.reset_all();
    assert_eq!(constraints.initialize_all(&solution), SoftScore::of(-8));
}
