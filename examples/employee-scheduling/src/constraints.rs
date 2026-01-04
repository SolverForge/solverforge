//! Zero-erasure constraints for Employee Scheduling using fluent API.
//!
//! All constraints use the fluent constraint stream API with concrete generic
//! types - no Arc, no dyn, fully monomorphized.

use chrono::NaiveDate;
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;

use crate::domain::{Employee, EmployeeSchedule, Shift};

/// Creates all constraints using the fluent API (fully monomorphized).
pub fn create_fluent_constraints() -> impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore> {
    let factory = ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new();

    // =========================================================================
    // HARD: Required Skill
    // =========================================================================
    let required_skill = factory
        .clone()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .filter(|shift: &Shift, emp: &Employee| {
            shift.employee_idx.is_some() && !emp.skills.contains(&shift.required_skill)
        })
        .penalize(HardSoftDecimalScore::ONE_HARD)
        .as_constraint("Required skill");

    // =========================================================================
    // HARD: No Overlapping Shifts
    // =========================================================================
    // Note: overlapping joiner can't be composed with equality joiner for self-joins
    // because for_each_unique_pair requires EqualJoiner for hash indexing.
    // The filter approach is correct for self-join overlap detection.
    let no_overlap = factory
        .clone()
        .for_each_unique_pair(
            |s: &EmployeeSchedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| shift.employee_idx),
        )
        .filter(|a: &Shift, b: &Shift| {
            a.employee_idx.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize_hard_with(|a: &Shift, b: &Shift| {
            HardSoftDecimalScore::of_hard_scaled(overlap_minutes(a, b) * 100000)
        })
        .as_constraint("Overlapping shift");

    // =========================================================================
    // HARD: At Least 10 Hours Between Shifts
    // =========================================================================
    let at_least_10_hours = factory
        .clone()
        .for_each_unique_pair(
            |s: &EmployeeSchedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| shift.employee_idx),
        )
        .filter(|a: &Shift, b: &Shift| a.employee_idx.is_some() && gap_penalty_minutes(a, b) > 0)
        .penalize_hard_with(|a: &Shift, b: &Shift| {
            HardSoftDecimalScore::of_hard_scaled(gap_penalty_minutes(a, b) * 100000)
        })
        .as_constraint("At least 10 hours between 2 shifts");

    // =========================================================================
    // HARD: One Shift Per Day
    // =========================================================================
    let one_per_day = factory
        .clone()
        .for_each_unique_pair(
            |s: &EmployeeSchedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| (shift.employee_idx, shift.date())),
        )
        .filter(|a: &Shift, b: &Shift| a.employee_idx.is_some() && b.employee_idx.is_some())
        .penalize(HardSoftDecimalScore::ONE_HARD)
        .as_constraint("One shift per day");

    // =========================================================================
    // HARD: Unavailable Employee
    // =========================================================================
    // Uses flatten_last for O(1) lookup by date.
    // Pre-indexes unavailable dates, looks up by shift.date() in O(1).
    let unavailable = factory
        .clone()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .flatten_last(
            |emp: &Employee| emp.unavailable_days.as_slice(),
            |date: &NaiveDate| *date,      // C → index key
            |shift: &Shift| shift.date(),  // A → lookup key
        )
        .filter(|shift: &Shift, date: &NaiveDate| {
            shift.employee_idx.is_some() && shift_date_overlap_minutes(shift, *date) > 0
        })
        .penalize_hard_with(|shift: &Shift, date: &NaiveDate| {
            HardSoftDecimalScore::of_hard_scaled(shift_date_overlap_minutes(shift, *date) * 100000)
        })
        .as_constraint("Unavailable employee");

    // =========================================================================
    // SOFT: Undesired Day
    // =========================================================================
    // Uses flatten_last for O(1) lookup. Penalizes 1 per match (Timefold pattern).
    let undesired = factory
        .clone()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .flatten_last(
            |emp: &Employee| emp.undesired_days.as_slice(),
            |date: &NaiveDate| *date,
            |shift: &Shift| shift.date(),
        )
        .filter(|shift: &Shift, _date: &NaiveDate| shift.employee_idx.is_some())
        .penalize(HardSoftDecimalScore::ONE_SOFT)
        .as_constraint("Undesired day for employee");

    // =========================================================================
    // SOFT: Desired Day
    // =========================================================================
    // Uses flatten_last for O(1) lookup. Rewards 1 per match (Timefold pattern).
    let desired = factory
        .clone()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .flatten_last(
            |emp: &Employee| emp.desired_days.as_slice(),
            |date: &NaiveDate| *date,
            |shift: &Shift| shift.date(),
        )
        .filter(|shift: &Shift, _date: &NaiveDate| shift.employee_idx.is_some())
        .reward(HardSoftDecimalScore::ONE_SOFT)
        .as_constraint("Desired day for employee");

    // =========================================================================
    // SOFT: Balance Assignments
    // =========================================================================
    // Uses simple balance() for O(1) incremental std-dev calculation.
    let balanced = factory
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .balance(|shift: &Shift| shift.employee_idx)
        .penalize(HardSoftDecimalScore::of_soft(1))
        .as_constraint("Balance employee assignments");

    (
        required_skill,
        no_overlap,
        at_least_10_hours,
        one_per_day,
        unavailable,
        undesired,
        desired,
        balanced,
    )
}

// ============================================================================
// Helper functions
// ============================================================================

#[inline]
fn overlap_minutes(a: &Shift, b: &Shift) -> i64 {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    if start < end {
        (end - start).num_minutes()
    } else {
        0
    }
}

#[inline]
fn gap_penalty_minutes(a: &Shift, b: &Shift) -> i64 {
    const MIN_GAP_MINUTES: i64 = 600;

    let (earlier, later) = if a.end <= b.start {
        (a, b)
    } else if b.end <= a.start {
        (b, a)
    } else {
        return 0;
    };

    let gap = (later.start - earlier.end).num_minutes();
    if (0..MIN_GAP_MINUTES).contains(&gap) {
        MIN_GAP_MINUTES - gap
    } else {
        0
    }
}

#[inline]
fn shift_date_overlap_minutes(shift: &Shift, date: NaiveDate) -> i64 {
    let day_start = date.and_hms_opt(0, 0, 0).unwrap();
    let day_end = date.succ_opt().unwrap_or(date).and_hms_opt(0, 0, 0).unwrap();

    let start = shift.start.max(day_start);
    let end = shift.end.min(day_end);

    if start < end {
        (end - start).num_minutes()
    } else {
        0
    }
}

