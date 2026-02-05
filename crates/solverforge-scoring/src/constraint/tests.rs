//! Consolidated tests for constraint module.
//!
//! Tests extracted from:
//! - complemented.rs (6 tests)
//! - if_exists.rs (2 tests)
//! - grouped.rs (3 tests)
//! - flattened_bi.rs (4 tests)
//! - balance.rs (7 tests)

use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::count;
use crate::stream::filter::TrueFilter;

use super::balance::BalanceConstraint;
use super::complemented::ComplementedGroupConstraint;
use super::flattened_bi::FlattenedBiConstraint;
use super::grouped::GroupedUniConstraint;
use super::if_exists::{ExistenceMode, IfExistsUniConstraint};

// ============================================================================
// Test fixtures
// ============================================================================

#[derive(Clone, Hash, PartialEq, Eq)]
struct Employee {
    id: usize,
}

#[derive(Clone)]
struct EmployeeWithDays {
    id: usize,
    unavailable_days: Vec<u32>,
}

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
}

#[derive(Clone)]
struct ShiftWithDay {
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone)]
struct Schedule {
    employees: Vec<Employee>,
    shifts: Vec<Shift>,
}

#[derive(Clone)]
struct ScheduleWithDays {
    employees: Vec<EmployeeWithDays>,
    shifts: Vec<ShiftWithDay>,
}

#[derive(Clone)]
struct ShiftSolution {
    shifts: Vec<Shift>,
}

#[derive(Clone)]
struct GroupedShift {
    employee_id: usize,
}

#[derive(Clone)]
struct GroupedSolution {
    shifts: Vec<GroupedShift>,
}

// For if_exists tests
#[derive(Clone)]
struct Task {
    _id: usize,
    assignee: Option<usize>,
}

#[derive(Clone)]
struct Worker {
    id: usize,
    available: bool,
}

#[derive(Clone)]
struct TaskSchedule {
    tasks: Vec<Task>,
    workers: Vec<Worker>,
}

// ============================================================================
// ComplementedGroupConstraint tests
// ============================================================================

#[test]
fn test_complemented_evaluate() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    // Total: -2
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
}

#[test]
fn test_complemented_skips_none_keys() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift { employee_id: None }, // Unassigned - should be skipped
            Shift { employee_id: None }, // Unassigned - should be skipped
        ],
    };

    // Only 2 assigned shifts count, both to employee 0
    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    // Total: -2 (unassigned shifts don't count)
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
}

#[test]
fn test_complemented_incremental() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Initialize
    let total = constraint.initialize(&schedule);
    // Employee 0: 2 shifts -> -2
    // Employee 1: 1 shift -> -1
    // Employee 2: 0 shifts -> 0
    // Total: -3
    assert_eq!(total, SimpleScore::of(-3));

    // Retract shift at index 0 (employee 0)
    let delta = constraint.on_retract(&schedule, 0, 0);
    // Employee 0 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SimpleScore::of(1));

    // Insert shift at index 0 (employee 0)
    let delta = constraint.on_insert(&schedule, 0, 0);
    // Employee 0 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SimpleScore::of(-1));
}

#[test]
fn test_complemented_incremental_with_none_keys() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift { employee_id: None }, // Unassigned
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // Initialize - only assigned shifts count
    let total = constraint.initialize(&schedule);
    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    // Total: -2
    assert_eq!(total, SimpleScore::of(-2));

    // Retract unassigned shift at index 1 - should be no-op
    let delta = constraint.on_retract(&schedule, 1, 0);
    assert_eq!(delta, SimpleScore::of(0));

    // Insert unassigned shift at index 1 - should be no-op
    let delta = constraint.on_insert(&schedule, 1, 0);
    assert_eq!(delta, SimpleScore::of(0));
}

#[test]
fn test_complemented_with_default() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Workload balance"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of((*count as i64).pow(2)),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // Employee 0: 3 shifts -> 9
    // Employee 1: 0 shifts -> 0
    // Employee 2: 0 shifts -> 0
    // Total penalty: -9
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-9));
}

#[test]
fn test_complemented_incremental_matches_evaluate() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count::<Shift>(),
        |_emp: &Employee| 0usize,
        |count: &usize| SimpleScore::of((*count as i64).pow(2)),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Verify initialize matches evaluate
    let init_total = constraint.initialize(&schedule);
    let eval_total = constraint.evaluate(&schedule);
    assert_eq!(init_total, eval_total);

    // Employee 0: 2 shifts -> 4, Employee 1: 1 shift -> 1
    // Total: -5
    assert_eq!(init_total, SimpleScore::of(-5));

    // Simulate retract + insert cycle and verify total remains consistent
    let mut running_total = init_total;

    // Retract shift 2 (employee 1)
    running_total = running_total + constraint.on_retract(&schedule, 2, 0);
    // Now: Employee 0: 2->4, Employee 1: 0->0, Total: -4
    assert_eq!(running_total, SimpleScore::of(-4));

    // Insert shift 2 back (employee 1)
    running_total = running_total + constraint.on_insert(&schedule, 2, 0);
    // Back to: Employee 0: 2->4, Employee 1: 1->1, Total: -5
    assert_eq!(running_total, SimpleScore::of(-5));
}

// ============================================================================
// IfExistsUniConstraint tests
// ============================================================================

#[test]
fn test_if_exists_penalizes_assigned_to_unavailable() {
    // Penalize tasks assigned to unavailable workers
    let constraint = IfExistsUniConstraint::new(
        ConstraintRef::new("", "Unavailable worker"),
        ImpactType::Penalty,
        ExistenceMode::Exists,
        |s: &TaskSchedule| s.tasks.as_slice(),
        |s: &TaskSchedule| s.workers.iter().filter(|w| !w.available).cloned().collect(),
        |t: &Task| t.assignee,
        |w: &Worker| Some(w.id),
        |_s: &TaskSchedule, t: &Task| t.assignee.is_some(),
        |_t: &Task| SimpleScore::of(1),
        false,
    );

    let schedule = TaskSchedule {
        tasks: vec![
            Task {
                _id: 0,
                assignee: Some(0),
            }, // assigned to unavailable
            Task {
                _id: 1,
                assignee: Some(1),
            }, // assigned to available
            Task {
                _id: 2,
                assignee: None,
            }, // unassigned
        ],
        workers: vec![
            Worker {
                id: 0,
                available: false,
            },
            Worker {
                id: 1,
                available: true,
            },
        ],
    };

    // Only task 0 matches
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}

#[test]
fn test_if_not_exists_penalizes_unassigned() {
    // Penalize tasks not assigned to any available worker
    let constraint = IfExistsUniConstraint::new(
        ConstraintRef::new("", "No available worker"),
        ImpactType::Penalty,
        ExistenceMode::NotExists,
        |s: &TaskSchedule| s.tasks.as_slice(),
        |s: &TaskSchedule| s.workers.iter().filter(|w| w.available).cloned().collect(),
        |t: &Task| t.assignee,
        |w: &Worker| Some(w.id),
        |_s: &TaskSchedule, t: &Task| t.assignee.is_some(),
        |_t: &Task| SimpleScore::of(1),
        false,
    );

    let schedule = TaskSchedule {
        tasks: vec![
            Task {
                _id: 0,
                assignee: Some(0),
            }, // assigned to unavailable - no match in available
            Task {
                _id: 1,
                assignee: Some(1),
            }, // assigned to available
            Task {
                _id: 2,
                assignee: None,
            }, // unassigned - filtered out by filter_a
        ],
        workers: vec![
            Worker {
                id: 0,
                available: false,
            },
            Worker {
                id: 1,
                available: true,
            },
        ],
    };

    // Task 0 is assigned but worker 0 is not available
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}

// ============================================================================
// GroupedUniConstraint tests
// ============================================================================

#[test]
fn test_grouped_constraint_evaluate() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of((*count * *count) as i64),
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

    // Employee 1: 3 shifts -> 9
    // Employee 2: 1 shift -> 1
    // Total penalty: -10
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
}

#[test]
fn test_grouped_constraint_incremental() {
    let mut constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of(*count as i64),
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
    // Employee 1: 2 shifts -> -2
    // Employee 2: 1 shift -> -1
    // Total: -3
    assert_eq!(total, SimpleScore::of(-3));

    // Retract shift at index 0 (employee 1)
    let delta = constraint.on_retract(&solution, 0, 0);
    // Employee 1 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SimpleScore::of(1));

    // Insert shift at index 0 (employee 1)
    let delta = constraint.on_insert(&solution, 0, 0);
    // Employee 1 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SimpleScore::of(-1));
}

#[test]
fn test_grouped_constraint_reward() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Collaboration"),
        ImpactType::Reward,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
        ],
    };

    // 2 shifts in one group -> reward of +2
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(2));
}

// ============================================================================
// FlattenedBiConstraint tests
// ============================================================================

fn create_flattened_test_constraint() -> FlattenedBiConstraint<
    ScheduleWithDays,
    ShiftWithDay,
    EmployeeWithDays,
    u32,
    Option<usize>,
    u32,
    impl Fn(&ScheduleWithDays) -> &[ShiftWithDay],
    impl Fn(&ScheduleWithDays) -> &[EmployeeWithDays],
    impl Fn(&ShiftWithDay) -> Option<usize>,
    impl Fn(&EmployeeWithDays) -> Option<usize>,
    impl Fn(&EmployeeWithDays) -> &[u32],
    impl Fn(&u32) -> u32,
    impl Fn(&ShiftWithDay) -> u32,
    impl Fn(&ScheduleWithDays, &ShiftWithDay, &u32) -> bool,
    impl Fn(&ShiftWithDay, &u32) -> SimpleScore,
    SimpleScore,
> {
    FlattenedBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        |s: &ScheduleWithDays| s.shifts.as_slice(),
        |s: &ScheduleWithDays| s.employees.as_slice(),
        |shift: &ShiftWithDay| shift.employee_id,
        |emp: &EmployeeWithDays| Some(emp.id),
        |emp: &EmployeeWithDays| emp.unavailable_days.as_slice(),
        |day: &u32| *day,
        |shift: &ShiftWithDay| shift.day,
        |_s: &ScheduleWithDays, shift: &ShiftWithDay, day: &u32| {
            shift.employee_id.is_some() && shift.day == *day
        },
        |_shift: &ShiftWithDay, _day: &u32| SimpleScore::of(1),
        false,
    )
}

#[test]
fn test_flattened_evaluate_single_match() {
    let constraint = create_flattened_test_constraint();
    let schedule = ScheduleWithDays {
        shifts: vec![
            ShiftWithDay {
                employee_id: Some(0),
                day: 5,
            },
            ShiftWithDay {
                employee_id: Some(0),
                day: 10,
            },
        ],
        employees: vec![EmployeeWithDays {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Day 5 shift conflicts with employee's unavailable day 5
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
}

#[test]
fn test_flattened_evaluate_no_match() {
    let constraint = create_flattened_test_constraint();
    let schedule = ScheduleWithDays {
        shifts: vec![ShiftWithDay {
            employee_id: Some(0),
            day: 10,
        }],
        employees: vec![EmployeeWithDays {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Day 10 doesn't conflict
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(0));
}

#[test]
fn test_flattened_incremental() {
    let mut constraint = create_flattened_test_constraint();
    let schedule = ScheduleWithDays {
        shifts: vec![
            ShiftWithDay {
                employee_id: Some(0),
                day: 5,
            }, // Conflicts
            ShiftWithDay {
                employee_id: Some(0),
                day: 10,
            }, // No conflict
        ],
        employees: vec![EmployeeWithDays {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Initialize
    let initial = constraint.initialize(&schedule);
    assert_eq!(initial, SimpleScore::of(-1));

    // Retract conflicting shift
    let delta = constraint.on_retract(&schedule, 0, 0);
    assert_eq!(delta, SimpleScore::of(1)); // Removing penalty

    // Re-insert it
    let delta = constraint.on_insert(&schedule, 0, 0);
    assert_eq!(delta, SimpleScore::of(-1)); // Adding penalty back
}

#[test]
fn test_flattened_unassigned_shift() {
    let constraint = create_flattened_test_constraint();
    let schedule = ScheduleWithDays {
        shifts: vec![ShiftWithDay {
            employee_id: None, // Unassigned
            day: 5,
        }],
        employees: vec![EmployeeWithDays {
            id: 0,
            unavailable_days: vec![5],
        }],
    };

    // Unassigned shift doesn't match
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(0));
}

// ============================================================================
// BalanceConstraint tests
// ============================================================================

#[test]
fn test_balance_evaluate_equal_distribution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000), // 1000 per unit std_dev
        false,
    );

    // Equal distribution: 2 shifts each
    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Mean = 2, all counts = 2, variance = 0, std_dev = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_evaluate_unequal_distribution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000), // 1000 per unit std_dev
        false,
    );

    // Unequal: employee 0 has 3, employee 1 has 1
    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Mean = 2, variance = ((3-2)^2 + (1-2)^2) / 2 = 1, std_dev = 1.0
    // base_score * std_dev = 1000 * 1.0 = 1000, negated = -1000
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1000));
}

#[test]
fn test_balance_filters_unassigned() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    // Employee 0: 2, Employee 1: 2, plus unassigned (ignored)
    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift { employee_id: None },
        ],
    };

    // Balanced, std_dev = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_incremental() {
    let mut constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Initialize with balanced state (std_dev = 0)
    let initial = constraint.initialize(&solution);
    assert_eq!(initial, SimpleScore::of(0));

    // Retract one shift from employee 0
    let delta = constraint.on_retract(&solution, 0, 0);
    // Now: employee 0 has 1, employee 1 has 2
    // Mean = 1.5, variance = (0.25 + 0.25) / 2 = 0.25, std_dev = 0.5
    // Score = -1000 * 0.5 = -500
    assert_eq!(delta, SimpleScore::of(-500));

    // Insert it back
    let delta = constraint.on_insert(&solution, 0, 0);
    // Back to balanced: delta = +500
    assert_eq!(delta, SimpleScore::of(500));
}

#[test]
fn test_balance_empty_solution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = ShiftSolution { shifts: vec![] };
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_single_employee() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    // Single employee with 5 shifts - no variance possible
    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // With only one group, variance = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_reward() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance reward"),
        ImpactType::Reward,
        |s: &ShiftSolution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = ShiftSolution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // std_dev = 1.0, reward = +1000
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(1000));
}
