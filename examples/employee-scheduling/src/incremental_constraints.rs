//! Incremental constraints for Employee Scheduling.
//!
//! These constraints implement `IncrementalConstraint` for use with `TypedScoreDirector`,
//! enabling O(1) incremental score updates instead of O(n²) full recalculation.

#![allow(clippy::new_without_default)]

use solverforge::prelude::*;
use solverforge::IncrementalConstraint;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

use crate::domain::{Employee, EmployeeSchedule, Shift};

// ============================================================================
// Type alias for the constraint tuple
// ============================================================================

/// All constraints as a typed tuple for zero-erasure scoring.
/// HARD constraints:
/// 1. Required skill - Employee must have the skill required by the shift
/// 2. No overlapping shifts - Same employee cannot work two shifts at once
/// 3. At least 10 hours between shifts - Rest time between consecutive shifts
/// 4. One shift per day - Maximum one shift per employee per day
/// 5. Unavailable employee - Employee cannot work on unavailable dates
///
/// SOFT constraints:
/// 6. Undesired day - Penalize scheduling on undesired dates
/// 7. Desired day - Reward scheduling on desired dates
/// 8. Balance assignments - Fair distribution of shifts across employees
pub type ScheduleConstraints = (
    RequiredSkillConstraint,
    NoOverlappingShiftsConstraint,
    AtLeast10HoursBetweenConstraint,
    OneShiftPerDayConstraint,
    UnavailableEmployeeConstraint,
    UndesiredDayConstraint,
    DesiredDayConstraint,
    BalanceAssignmentsConstraint,
);

/// Creates all constraints for the employee scheduling problem.
pub fn create_constraints() -> ScheduleConstraints {
    (
        RequiredSkillConstraint::new(),
        NoOverlappingShiftsConstraint::new(),
        AtLeast10HoursBetweenConstraint::new(),
        OneShiftPerDayConstraint::new(),
        UnavailableEmployeeConstraint::new(),
        UndesiredDayConstraint::new(),
        DesiredDayConstraint::new(),
        BalanceAssignmentsConstraint::new(),
    )
}

// ============================================================================
// Helper functions
// ============================================================================

/// Looks up an employee by index from the solution (O(1)).
#[inline]
fn get_employee(solution: &EmployeeSchedule, idx: usize) -> Option<&Employee> {
    solution.get_employee(idx)
}

/// Returns the overlap in minutes between a shift and a date (full 24h day).
/// Handles shifts that span midnight correctly.
#[inline]
fn shift_date_overlap_minutes(shift: &Shift, date: chrono::NaiveDate) -> i64 {
    let day_start = date.and_hms_opt(0, 0, 0).unwrap();
    let day_end = date
        .succ_opt()
        .unwrap_or(date)
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let start = shift.start.max(day_start);
    let end = shift.end.min(day_end);

    if start < end {
        (end - start).num_minutes()
    } else {
        0
    }
}

// ============================================================================
// HARD: Required Skill Constraint
// ============================================================================

/// Employee must have the required skill for the shift.
pub struct RequiredSkillConstraint {
    /// Tracks which shifts currently violate this constraint (by index).
    violations: HashSet<usize>,
}

impl RequiredSkillConstraint {
    pub fn new() -> Self {
        Self {
            violations: HashSet::new(),
        }
    }

    fn check_violation(&self, solution: &EmployeeSchedule, shift_idx: usize) -> bool {
        let shift = &solution.shifts[shift_idx];
        if let Some(emp_idx) = shift.employee_idx {
            if let Some(employee) = get_employee(solution, emp_idx) {
                return !employee.skills.contains(&shift.required_skill);
            }
        }
        false
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> for RequiredSkillConstraint {
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let mut count = 0i64;
        for (idx, _) in solution.shifts.iter().enumerate() {
            if self.check_violation(solution, idx) {
                count += 1;
            }
        }
        HardSoftDecimalScore::of_hard(-count)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        solution
            .shifts
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.check_violation(solution, *idx))
            .count()
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.violations.clear();
        let mut score = HardSoftDecimalScore::ZERO;
        for idx in 0..solution.shifts.len() {
            if self.check_violation(solution, idx) {
                self.violations.insert(idx);
                score = score + HardSoftDecimalScore::of_hard(-1);
            }
        }
        score
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        if self.check_violation(solution, entity_index) && self.violations.insert(entity_index) {
            return HardSoftDecimalScore::of_hard(-1);
        }
        HardSoftDecimalScore::ZERO
    }

    fn on_retract(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        if self.violations.remove(&entity_index) {
            return HardSoftDecimalScore::of_hard(1);
        }
        HardSoftDecimalScore::ZERO
    }

    fn reset(&mut self) {
        self.violations.clear();
    }

    fn name(&self) -> &str {
        "Required skill"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// HARD: No Overlapping Shifts Constraint
// ============================================================================

/// Same employee cannot have overlapping shifts.
/// Penalty = overlap duration in minutes (scaled by 1000 for HardSoftDecimalScore).
pub struct NoOverlappingShiftsConstraint {
    /// Tracks conflicting pairs (low_idx, high_idx) -> overlap_minutes.
    conflicts: HashMap<(usize, usize), i64>,
    /// Reverse index: entity → conflicts involving it (O(k) retraction).
    entity_to_conflicts: HashMap<usize, HashSet<(usize, usize)>>,
    /// Reverse index: employee → shift indices (O(k) insertion).
    employee_to_shifts: HashMap<usize, HashSet<usize>>,
}

impl NoOverlappingShiftsConstraint {
    pub fn new() -> Self {
        Self {
            conflicts: HashMap::new(),
            entity_to_conflicts: HashMap::new(),
            employee_to_shifts: HashMap::new(),
        }
    }

    /// Returns overlap in minutes between two shifts, or 0 if no overlap.
    #[inline]
    fn overlap_minutes(s1: &Shift, s2: &Shift) -> i64 {
        let start = s1.start.max(s2.start);
        let end = s1.end.min(s2.end);
        if start < end {
            (end - start).num_minutes()
        } else {
            0
        }
    }

    /// Checks if two shifts conflict (same employee + overlap).
    fn conflict_minutes(s1: &Shift, s2: &Shift) -> i64 {
        match (s1.employee_idx, s2.employee_idx) {
            (Some(e1), Some(e2)) if e1 == e2 => Self::overlap_minutes(s1, s2),
            _ => 0,
        }
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore>
    for NoOverlappingShiftsConstraint
{
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        let mut total_minutes = 0i64;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                total_minutes += Self::conflict_minutes(&shifts[i], &shifts[j]);
            }
        }
        // Penalty in minutes (scaled by 1000 internally)
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        let shifts = &solution.shifts;
        let mut count = 0;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                if Self::conflict_minutes(&shifts[i], &shifts[j]) > 0 {
                    count += 1;
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.reset();
        let shifts = &solution.shifts;

        // Build employee→shifts index
        for (idx, shift) in shifts.iter().enumerate() {
            if let Some(emp_idx) = shift.employee_idx {
                self.employee_to_shifts
                    .entry(emp_idx)
                    .or_default()
                    .insert(idx);
            }
        }

        // Find conflicts (only need to check within same employee's shifts)
        let mut total_minutes = 0i64;
        for emp_shifts in self.employee_to_shifts.values() {
            let indices: Vec<_> = emp_shifts.iter().copied().collect();
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let (a, b) = (indices[i], indices[j]);
                    let minutes = Self::overlap_minutes(&shifts[a], &shifts[b]);
                    if minutes > 0 {
                        let pair = if a < b { (a, b) } else { (b, a) };
                        if let Entry::Vacant(e) = self.conflicts.entry(pair) {
                            e.insert(minutes);
                            self.entity_to_conflicts
                                .entry(pair.0)
                                .or_default()
                                .insert(pair);
                            self.entity_to_conflicts
                                .entry(pair.1)
                                .or_default()
                                .insert(pair);
                            total_minutes += minutes;
                        }
                    }
                }
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        let shift = &shifts[entity_index];
        let Some(emp_idx) = shift.employee_idx else {
            return HardSoftDecimalScore::ZERO;
        };

        // Add to employee→shifts index
        self.employee_to_shifts
            .entry(emp_idx)
            .or_default()
            .insert(entity_index);

        // Only check other shifts for THIS employee (O(k) not O(n))
        let mut total_minutes = 0i64;
        if let Some(emp_shifts) = self.employee_to_shifts.get(&emp_idx) {
            for &other_idx in emp_shifts {
                if other_idx == entity_index {
                    continue;
                }
                let minutes = Self::overlap_minutes(shift, &shifts[other_idx]);
                if minutes > 0 {
                    let pair = if entity_index < other_idx {
                        (entity_index, other_idx)
                    } else {
                        (other_idx, entity_index)
                    };
                    if let Entry::Vacant(e) = self.conflicts.entry(pair) {
                        e.insert(minutes);
                        self.entity_to_conflicts
                            .entry(pair.0)
                            .or_default()
                            .insert(pair);
                        self.entity_to_conflicts
                            .entry(pair.1)
                            .or_default()
                            .insert(pair);
                        total_minutes += minutes;
                    }
                }
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn on_retract(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        // Remove from employee→shifts index
        if let Some(emp_idx) = shifts[entity_index].employee_idx {
            if let Some(emp_shifts) = self.employee_to_shifts.get_mut(&emp_idx) {
                emp_shifts.remove(&entity_index);
                if emp_shifts.is_empty() {
                    self.employee_to_shifts.remove(&emp_idx);
                }
            }
        }

        // Use reverse index for O(k) removal
        let Some(pairs) = self.entity_to_conflicts.remove(&entity_index) else {
            return HardSoftDecimalScore::ZERO;
        };

        let mut total_minutes = 0i64;
        for pair in pairs {
            if let Some(minutes) = self.conflicts.remove(&pair) {
                total_minutes += minutes;
            }
            // Remove from other entity's reverse index
            let other = if pair.0 == entity_index {
                pair.1
            } else {
                pair.0
            };
            if let Some(other_set) = self.entity_to_conflicts.get_mut(&other) {
                other_set.remove(&pair);
            }
        }
        HardSoftDecimalScore::of_hard_scaled(total_minutes * 1000)
    }

    fn reset(&mut self) {
        self.conflicts.clear();
        self.entity_to_conflicts.clear();
        self.employee_to_shifts.clear();
    }

    fn name(&self) -> &str {
        "Overlapping shift"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// HARD: At Least 10 Hours Between Shifts Constraint
// ============================================================================

/// At least 10 hours between consecutive shifts for the same employee.
/// Penalty = (600 - gap_minutes) for gaps under 10 hours.
pub struct AtLeast10HoursBetweenConstraint {
    /// Tracks conflicting pairs (low_idx, high_idx) -> missing_minutes.
    conflicts: HashMap<(usize, usize), i64>,
    entity_to_conflicts: HashMap<usize, HashSet<(usize, usize)>>,
    employee_to_shifts: HashMap<usize, HashSet<usize>>,
}

const MIN_GAP_MINUTES: i64 = 10 * 60; // 600 minutes

impl AtLeast10HoursBetweenConstraint {
    pub fn new() -> Self {
        Self {
            conflicts: HashMap::new(),
            entity_to_conflicts: HashMap::new(),
            employee_to_shifts: HashMap::new(),
        }
    }

    /// Returns the penalty in minutes for violating the 10-hour gap rule.
    /// Penalty = (600 - actual_gap_minutes), or 0 if no violation.
    #[inline]
    fn gap_penalty_minutes(s1: &Shift, s2: &Shift) -> i64 {
        let (earlier, later) = if s1.start < s2.start {
            (s1, s2)
        } else {
            (s2, s1)
        };
        let gap_minutes = (later.start - earlier.end).num_minutes();
        if (0..MIN_GAP_MINUTES).contains(&gap_minutes) {
            MIN_GAP_MINUTES - gap_minutes
        } else {
            0
        }
    }

    /// Returns penalty for same employee, or 0 if different/no employee.
    fn violation_minutes(s1: &Shift, s2: &Shift) -> i64 {
        match (s1.employee_idx, s2.employee_idx) {
            (Some(e1), Some(e2)) if e1 == e2 => Self::gap_penalty_minutes(s1, s2),
            _ => 0,
        }
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore>
    for AtLeast10HoursBetweenConstraint
{
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        let mut total_minutes = 0i64;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                total_minutes += Self::violation_minutes(&shifts[i], &shifts[j]);
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        let shifts = &solution.shifts;
        let mut count = 0;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                if Self::violation_minutes(&shifts[i], &shifts[j]) > 0 {
                    count += 1;
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.reset();
        let shifts = &solution.shifts;

        for (idx, shift) in shifts.iter().enumerate() {
            if let Some(emp_idx) = shift.employee_idx {
                self.employee_to_shifts
                    .entry(emp_idx)
                    .or_default()
                    .insert(idx);
            }
        }

        let mut total_minutes = 0i64;
        for emp_shifts in self.employee_to_shifts.values() {
            let indices: Vec<_> = emp_shifts.iter().copied().collect();
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let (a, b) = (indices[i], indices[j]);
                    let minutes = Self::gap_penalty_minutes(&shifts[a], &shifts[b]);
                    if minutes > 0 {
                        let pair = if a < b { (a, b) } else { (b, a) };
                        if let Entry::Vacant(e) = self.conflicts.entry(pair) {
                            e.insert(minutes);
                            self.entity_to_conflicts
                                .entry(pair.0)
                                .or_default()
                                .insert(pair);
                            self.entity_to_conflicts
                                .entry(pair.1)
                                .or_default()
                                .insert(pair);
                            total_minutes += minutes;
                        }
                    }
                }
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        let shift = &shifts[entity_index];
        let Some(emp_idx) = shift.employee_idx else {
            return HardSoftDecimalScore::ZERO;
        };

        self.employee_to_shifts
            .entry(emp_idx)
            .or_default()
            .insert(entity_index);

        let mut total_minutes = 0i64;
        if let Some(emp_shifts) = self.employee_to_shifts.get(&emp_idx) {
            for &other_idx in emp_shifts {
                if other_idx == entity_index {
                    continue;
                }
                let minutes = Self::gap_penalty_minutes(shift, &shifts[other_idx]);
                if minutes > 0 {
                    let pair = if entity_index < other_idx {
                        (entity_index, other_idx)
                    } else {
                        (other_idx, entity_index)
                    };
                    if let Entry::Vacant(e) = self.conflicts.entry(pair) {
                        e.insert(minutes);
                        self.entity_to_conflicts
                            .entry(pair.0)
                            .or_default()
                            .insert(pair);
                        self.entity_to_conflicts
                            .entry(pair.1)
                            .or_default()
                            .insert(pair);
                        total_minutes += minutes;
                    }
                }
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn on_retract(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        if let Some(emp_idx) = shifts[entity_index].employee_idx {
            if let Some(emp_shifts) = self.employee_to_shifts.get_mut(&emp_idx) {
                emp_shifts.remove(&entity_index);
                if emp_shifts.is_empty() {
                    self.employee_to_shifts.remove(&emp_idx);
                }
            }
        }

        let Some(pairs) = self.entity_to_conflicts.remove(&entity_index) else {
            return HardSoftDecimalScore::ZERO;
        };

        let mut total_minutes = 0i64;
        for pair in pairs {
            if let Some(minutes) = self.conflicts.remove(&pair) {
                total_minutes += minutes;
            }
            let other = if pair.0 == entity_index {
                pair.1
            } else {
                pair.0
            };
            if let Some(other_set) = self.entity_to_conflicts.get_mut(&other) {
                other_set.remove(&pair);
            }
        }
        HardSoftDecimalScore::of_hard_scaled(total_minutes * 1000)
    }

    fn reset(&mut self) {
        self.conflicts.clear();
        self.entity_to_conflicts.clear();
        self.employee_to_shifts.clear();
    }

    fn name(&self) -> &str {
        "At least 10 hours between 2 shifts"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// HARD: One Shift Per Day Constraint
// ============================================================================

/// Maximum one shift per employee per day.
pub struct OneShiftPerDayConstraint {
    /// Tracks conflicting pairs (low_idx, high_idx).
    conflicts: HashSet<(usize, usize)>,
    /// Reverse index: entity → conflicts involving it (O(k) retraction).
    entity_to_conflicts: HashMap<usize, HashSet<(usize, usize)>>,
    /// Reverse index: employee → shift indices (O(k) insertion).
    employee_to_shifts: HashMap<usize, HashSet<usize>>,
}

impl OneShiftPerDayConstraint {
    pub fn new() -> Self {
        Self {
            conflicts: HashSet::new(),
            entity_to_conflicts: HashMap::new(),
            employee_to_shifts: HashMap::new(),
        }
    }

    /// Checks if two shifts are on the same day (assumes same employee).
    #[inline]
    fn same_day(s1: &Shift, s2: &Shift) -> bool {
        s1.date() == s2.date()
    }

    fn shifts_conflict(s1: &Shift, s2: &Shift) -> bool {
        match (s1.employee_idx, s2.employee_idx) {
            (Some(e1), Some(e2)) if e1 == e2 => Self::same_day(s1, s2),
            _ => false,
        }
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> for OneShiftPerDayConstraint {
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        let mut count = 0i64;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                if Self::shifts_conflict(&shifts[i], &shifts[j]) {
                    count += 1;
                }
            }
        }
        HardSoftDecimalScore::of_hard(-count)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        let shifts = &solution.shifts;
        let mut count = 0;
        for i in 0..shifts.len() {
            for j in (i + 1)..shifts.len() {
                if Self::shifts_conflict(&shifts[i], &shifts[j]) {
                    count += 1;
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.reset();
        let shifts = &solution.shifts;

        // Build employee→shifts index
        for (idx, shift) in shifts.iter().enumerate() {
            if let Some(emp_idx) = shift.employee_idx {
                self.employee_to_shifts
                    .entry(emp_idx)
                    .or_default()
                    .insert(idx);
            }
        }

        // Find conflicts (only check within same employee's shifts)
        let mut score = HardSoftDecimalScore::ZERO;
        for emp_shifts in self.employee_to_shifts.values() {
            let indices: Vec<_> = emp_shifts.iter().copied().collect();
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let (a, b) = (indices[i], indices[j]);
                    if Self::same_day(&shifts[a], &shifts[b]) {
                        let pair = if a < b { (a, b) } else { (b, a) };
                        if self.conflicts.insert(pair) {
                            self.entity_to_conflicts
                                .entry(pair.0)
                                .or_default()
                                .insert(pair);
                            self.entity_to_conflicts
                                .entry(pair.1)
                                .or_default()
                                .insert(pair);
                            score = score + HardSoftDecimalScore::of_hard(-1);
                        }
                    }
                }
            }
        }
        score
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        let shift = &shifts[entity_index];
        let Some(emp_idx) = shift.employee_idx else {
            return HardSoftDecimalScore::ZERO;
        };

        // Add to employee→shifts index
        self.employee_to_shifts
            .entry(emp_idx)
            .or_default()
            .insert(entity_index);

        // Only check other shifts for THIS employee (O(k) not O(n))
        let mut delta = HardSoftDecimalScore::ZERO;
        if let Some(emp_shifts) = self.employee_to_shifts.get(&emp_idx) {
            for &other_idx in emp_shifts {
                if other_idx == entity_index {
                    continue;
                }
                if Self::same_day(shift, &shifts[other_idx]) {
                    let pair = if entity_index < other_idx {
                        (entity_index, other_idx)
                    } else {
                        (other_idx, entity_index)
                    };
                    if self.conflicts.insert(pair) {
                        self.entity_to_conflicts
                            .entry(pair.0)
                            .or_default()
                            .insert(pair);
                        self.entity_to_conflicts
                            .entry(pair.1)
                            .or_default()
                            .insert(pair);
                        delta = delta + HardSoftDecimalScore::of_hard(-1);
                    }
                }
            }
        }
        delta
    }

    fn on_retract(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        let shifts = &solution.shifts;
        if entity_index >= shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }

        // Remove from employee→shifts index
        if let Some(emp_idx) = shifts[entity_index].employee_idx {
            if let Some(emp_shifts) = self.employee_to_shifts.get_mut(&emp_idx) {
                emp_shifts.remove(&entity_index);
                if emp_shifts.is_empty() {
                    self.employee_to_shifts.remove(&emp_idx);
                }
            }
        }

        // Use reverse index for O(k) removal
        let Some(pairs) = self.entity_to_conflicts.remove(&entity_index) else {
            return HardSoftDecimalScore::ZERO;
        };

        let mut delta = HardSoftDecimalScore::ZERO;
        for pair in pairs {
            self.conflicts.remove(&pair);
            // Remove from other entity's reverse index
            let other = if pair.0 == entity_index {
                pair.1
            } else {
                pair.0
            };
            if let Some(other_set) = self.entity_to_conflicts.get_mut(&other) {
                other_set.remove(&pair);
            }
            delta = delta + HardSoftDecimalScore::of_hard(1);
        }
        delta
    }

    fn reset(&mut self) {
        self.conflicts.clear();
        self.entity_to_conflicts.clear();
        self.employee_to_shifts.clear();
    }

    fn name(&self) -> &str {
        "One shift per day"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// HARD: Unavailable Employee Constraint
// ============================================================================

/// Employee cannot work on unavailable dates.
/// Penalty = overlap duration in minutes (scaled by 1000 for HardSoftDecimalScore).
pub struct UnavailableEmployeeConstraint {
    /// shift_idx → overlap minutes with unavailable date
    violations: HashMap<usize, i64>,
}

impl UnavailableEmployeeConstraint {
    pub fn new() -> Self {
        Self {
            violations: HashMap::new(),
        }
    }

    /// Returns overlap minutes if shift is on an unavailable date for the employee.
    fn violation_minutes(&self, solution: &EmployeeSchedule, shift_idx: usize) -> i64 {
        let shift = &solution.shifts[shift_idx];
        if let Some(emp_idx) = shift.employee_idx {
            if let Some(employee) = get_employee(solution, emp_idx) {
                // Check each unavailable date for overlap
                for &date in &employee.unavailable_dates {
                    let minutes = shift_date_overlap_minutes(shift, date);
                    if minutes > 0 {
                        return minutes;
                    }
                }
            }
        }
        0
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore>
    for UnavailableEmployeeConstraint
{
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let mut total_minutes = 0i64;
        for idx in 0..solution.shifts.len() {
            total_minutes += self.violation_minutes(solution, idx);
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        (0..solution.shifts.len())
            .filter(|&idx| self.violation_minutes(solution, idx) > 0)
            .count()
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.violations.clear();
        let mut total_minutes = 0i64;
        for idx in 0..solution.shifts.len() {
            let minutes = self.violation_minutes(solution, idx);
            if minutes > 0 {
                self.violations.insert(idx, minutes);
                total_minutes += minutes;
            }
        }
        HardSoftDecimalScore::of_hard_scaled(-total_minutes * 1000)
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        let minutes = self.violation_minutes(solution, entity_index);
        if minutes > 0 {
            self.violations.insert(entity_index, minutes);
            return HardSoftDecimalScore::of_hard_scaled(-minutes * 1000);
        }
        HardSoftDecimalScore::ZERO
    }

    fn on_retract(
        &mut self,
        _solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if let Some(minutes) = self.violations.remove(&entity_index) {
            return HardSoftDecimalScore::of_hard_scaled(minutes * 1000);
        }
        HardSoftDecimalScore::ZERO
    }

    fn reset(&mut self) {
        self.violations.clear();
    }

    fn name(&self) -> &str {
        "Unavailable employee"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// SOFT: Undesired Day Constraint
// ============================================================================

/// Penalize scheduling on undesired dates.
/// Penalty = 1 soft per violation
pub struct UndesiredDayConstraint {
    /// Tracks which shifts currently violate this constraint.
    violations: HashSet<usize>,
}

impl UndesiredDayConstraint {
    pub fn new() -> Self {
        Self {
            violations: HashSet::new(),
        }
    }

    /// Returns true if shift is on an undesired date for the employee.
    fn is_violation(&self, solution: &EmployeeSchedule, shift_idx: usize) -> bool {
        let shift = &solution.shifts[shift_idx];
        if let Some(emp_idx) = shift.employee_idx {
            if let Some(employee) = get_employee(solution, emp_idx) {
                let shift_date = shift.date();
                return employee.undesired_dates.contains(&shift_date);
            }
        }
        false
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> for UndesiredDayConstraint {
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let mut count = 0i64;
        for idx in 0..solution.shifts.len() {
            if self.is_violation(solution, idx) {
                count += 1;
            }
        }
        HardSoftDecimalScore::of_soft(-count)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        (0..solution.shifts.len())
            .filter(|&idx| self.is_violation(solution, idx))
            .count()
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.violations.clear();
        let mut count = 0i64;
        for idx in 0..solution.shifts.len() {
            if self.is_violation(solution, idx) {
                self.violations.insert(idx);
                count += 1;
            }
        }
        HardSoftDecimalScore::of_soft(-count)
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        if self.is_violation(solution, entity_index) && self.violations.insert(entity_index) {
            return HardSoftDecimalScore::of_soft(-1);
        }
        HardSoftDecimalScore::ZERO
    }

    fn on_retract(
        &mut self,
        _solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if self.violations.remove(&entity_index) {
            return HardSoftDecimalScore::of_soft(1);
        }
        HardSoftDecimalScore::ZERO
    }

    fn reset(&mut self) {
        self.violations.clear();
    }

    fn name(&self) -> &str {
        "Undesired day for employee"
    }

    fn is_hard(&self) -> bool {
        false
    }
}

// ============================================================================
// SOFT: Desired Day Constraint
// ============================================================================

/// Reward scheduling on desired dates.
/// Reward = 1 soft per match
pub struct DesiredDayConstraint {
    /// Tracks which shifts match this constraint.
    matches: HashSet<usize>,
}

impl DesiredDayConstraint {
    pub fn new() -> Self {
        Self {
            matches: HashSet::new(),
        }
    }

    /// Returns true if shift is on a desired date for the employee.
    fn is_match(&self, solution: &EmployeeSchedule, shift_idx: usize) -> bool {
        let shift = &solution.shifts[shift_idx];
        if let Some(emp_idx) = shift.employee_idx {
            if let Some(employee) = get_employee(solution, emp_idx) {
                let shift_date = shift.date();
                return employee.desired_dates.contains(&shift_date);
            }
        }
        false
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> for DesiredDayConstraint {
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let mut count = 0i64;
        for idx in 0..solution.shifts.len() {
            if self.is_match(solution, idx) {
                count += 1;
            }
        }
        HardSoftDecimalScore::of_soft(count) // Reward
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        (0..solution.shifts.len())
            .filter(|&idx| self.is_match(solution, idx))
            .count()
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.matches.clear();
        let mut count = 0i64;
        for idx in 0..solution.shifts.len() {
            if self.is_match(solution, idx) {
                self.matches.insert(idx);
                count += 1;
            }
        }
        HardSoftDecimalScore::of_soft(count) // Reward
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        if self.is_match(solution, entity_index) && self.matches.insert(entity_index) {
            return HardSoftDecimalScore::of_soft(1); // Reward
        }
        HardSoftDecimalScore::ZERO
    }

    fn on_retract(
        &mut self,
        _solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if self.matches.remove(&entity_index) {
            return HardSoftDecimalScore::of_soft(-1); // Remove reward
        }
        HardSoftDecimalScore::ZERO
    }

    fn reset(&mut self) {
        self.matches.clear();
    }

    fn name(&self) -> &str {
        "Desired day for employee"
    }

    fn is_hard(&self) -> bool {
        false
    }
}

// ============================================================================
// SOFT: Balance Assignments Constraint
// ============================================================================

/// Fair distribution of shifts across employees.
/// Unfairness = standard deviation of shift counts
pub struct BalanceAssignmentsConstraint {
    /// employee_idx → shift count
    assignment_counts: HashMap<usize, i64>,
    /// Number of employees with at least one shift.
    employee_count: i64,
    /// Sum of all shift counts (for mean calculation).
    total_shifts: i64,
    /// Sum of squared counts (for variance calculation).
    sum_squared: i64,
}

impl BalanceAssignmentsConstraint {
    pub fn new() -> Self {
        Self {
            assignment_counts: HashMap::new(),
            employee_count: 0,
            total_shifts: 0,
            sum_squared: 0,
        }
    }

    /// Calculates unfairness as standard deviation, scaled by 1000 for HardSoftDecimalScore.
    /// unfairness = σ = sqrt(Σ(x_i - μ)²/n) = sqrt(E[X²] - E[X]²)
    ///
    /// Returns the scaled value directly (e.g., 6930 for σ=6.93).
    fn calculate_unfairness(&self) -> i64 {
        if self.employee_count == 0 {
            return 0;
        }
        let n = self.employee_count as f64;
        let mean = self.total_shifts as f64 / n;
        let variance = (self.sum_squared as f64 / n) - (mean * mean);
        if variance <= 0.0 {
            return 0;
        }
        let std_dev = variance.sqrt();
        // Return scaled by 1000 for HardSoftDecimalScore (use directly with of_soft_scaled)
        (std_dev * 1000.0).round() as i64
    }
}

impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore>
    for BalanceAssignmentsConstraint
{
    fn evaluate(&self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        let mut counts: HashMap<usize, i64> = HashMap::new();
        for shift in &solution.shifts {
            if let Some(emp_idx) = shift.employee_idx {
                *counts.entry(emp_idx).or_insert(0) += 1;
            }
        }
        if counts.is_empty() {
            return HardSoftDecimalScore::ZERO;
        }
        let n = counts.len() as f64;
        let total: i64 = counts.values().sum();
        let sum_sq: i64 = counts.values().map(|&c| c * c).sum();
        let mean = total as f64 / n;
        let variance = (sum_sq as f64 / n) - (mean * mean);
        let std_dev = if variance > 0.0 { variance.sqrt() } else { 0.0 };
        // unfairness = std_dev, scaled by 1000 for HardSoftDecimalScore
        HardSoftDecimalScore::of_soft_scaled(-(std_dev * 1000.0).round() as i64)
    }

    fn match_count(&self, solution: &EmployeeSchedule) -> usize {
        let mut counts: HashMap<usize, i64> = HashMap::new();
        for shift in &solution.shifts {
            if let Some(emp_idx) = shift.employee_idx {
                *counts.entry(emp_idx).or_insert(0) += 1;
            }
        }
        // Count employees that deviate from mean
        if counts.is_empty() {
            return 0;
        }
        let total: i64 = counts.values().sum();
        let mean = total as f64 / counts.len() as f64;
        counts
            .values()
            .filter(|&&c| ((c as f64) - mean).abs() > 0.5)
            .count()
    }

    fn initialize(&mut self, solution: &EmployeeSchedule) -> HardSoftDecimalScore {
        self.assignment_counts.clear();
        for shift in &solution.shifts {
            if let Some(emp_idx) = shift.employee_idx {
                *self.assignment_counts.entry(emp_idx).or_insert(0) += 1;
            }
        }
        self.employee_count = self.assignment_counts.len() as i64;
        self.total_shifts = self.assignment_counts.values().sum();
        self.sum_squared = self.assignment_counts.values().map(|&c| c * c).sum();
        let unfairness = self.calculate_unfairness();
        // unfairness is already scaled by 1000
        HardSoftDecimalScore::of_soft_scaled(-unfairness)
    }

    fn on_insert(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        let shift = &solution.shifts[entity_index];
        let Some(emp_idx) = shift.employee_idx else {
            return HardSoftDecimalScore::ZERO;
        };

        let old_unfairness = self.calculate_unfairness();

        let old_count = *self.assignment_counts.get(&emp_idx).unwrap_or(&0);
        let new_count = old_count + 1;
        self.assignment_counts.insert(emp_idx, new_count);

        // Update statistics
        if old_count == 0 {
            self.employee_count += 1;
        }
        self.total_shifts += 1;
        self.sum_squared += new_count * new_count - old_count * old_count;

        let new_unfairness = self.calculate_unfairness();
        let delta = new_unfairness - old_unfairness;
        // delta is already scaled by 1000
        HardSoftDecimalScore::of_soft_scaled(-delta)
    }

    fn on_retract(
        &mut self,
        solution: &EmployeeSchedule,
        entity_index: usize,
    ) -> HardSoftDecimalScore {
        if entity_index >= solution.shifts.len() {
            return HardSoftDecimalScore::ZERO;
        }
        let shift = &solution.shifts[entity_index];
        let Some(emp_idx) = shift.employee_idx else {
            return HardSoftDecimalScore::ZERO;
        };

        let old_count = *self.assignment_counts.get(&emp_idx).unwrap_or(&0);
        if old_count == 0 {
            return HardSoftDecimalScore::ZERO;
        }

        let old_unfairness = self.calculate_unfairness();

        let new_count = old_count - 1;
        if new_count == 0 {
            self.assignment_counts.remove(&emp_idx);
            self.employee_count -= 1;
        } else {
            self.assignment_counts.insert(emp_idx, new_count);
        }

        // Update statistics
        self.total_shifts -= 1;
        self.sum_squared += new_count * new_count - old_count * old_count;

        let new_unfairness = self.calculate_unfairness();
        let delta = new_unfairness - old_unfairness;
        // delta is already scaled by 1000
        HardSoftDecimalScore::of_soft_scaled(-delta)
    }

    fn reset(&mut self) {
        self.assignment_counts.clear();
        self.employee_count = 0;
        self.total_shifts = 0;
        self.sum_squared = 0;
    }

    fn name(&self) -> &str {
        "Balance employee assignments"
    }

    fn is_hard(&self) -> bool {
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use solverforge::ConstraintSet;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn datetime(year: i32, month: u32, day: u32, hour: u32) -> chrono::NaiveDateTime {
        date(year, month, day).and_hms_opt(hour, 0, 0).unwrap()
    }

    #[test]
    fn test_required_skill_constraint() {
        let employees = vec![
            Employee::new(0, "Alice").with_skill("Barista"),
            Employee::new(1, "Bob").with_skill("Waiter"),
        ];
        let mut shifts = vec![Shift::new(
            "1",
            datetime(2024, 1, 1, 8),
            datetime(2024, 1, 1, 16),
            "Cafe",
            "Barista",
        )];
        shifts[0].employee_idx = Some(1); // Bob (index 1) doesn't have Barista skill

        let solution = EmployeeSchedule::new(employees, shifts);
        let constraint = RequiredSkillConstraint::new();

        assert_eq!(
            constraint.evaluate(&solution),
            HardSoftDecimalScore::of_hard(-1)
        );
    }

    #[test]
    fn test_no_overlapping_shifts() {
        let employees = vec![Employee::new(0, "Alice")];
        let mut shifts = vec![
            // Shift 1: 8:00 - 16:00
            Shift::new(
                "1",
                datetime(2024, 1, 1, 8),
                datetime(2024, 1, 1, 16),
                "A",
                "Skill",
            ),
            // Shift 2: 12:00 - 20:00 (overlaps 12:00-16:00 = 4 hours = 240 minutes)
            Shift::new(
                "2",
                datetime(2024, 1, 1, 12),
                datetime(2024, 1, 1, 20),
                "B",
                "Skill",
            ),
        ];
        shifts[0].employee_idx = Some(0); // Alice
        shifts[1].employee_idx = Some(0); // Alice

        let solution = EmployeeSchedule::new(employees, shifts);
        let constraint = NoOverlappingShiftsConstraint::new();

        // Penalty = 240 minutes overlap (scaled by 1000)
        assert_eq!(
            constraint.evaluate(&solution),
            HardSoftDecimalScore::of_hard_scaled(-240 * 1000)
        );
    }

    #[test]
    fn test_incremental_update() {
        let employees = vec![
            Employee::new(0, "Alice").with_skill("Barista"),
            Employee::new(1, "Bob").with_skill("Barista"),
        ];
        let mut shifts = vec![Shift::new(
            "1",
            datetime(2024, 1, 1, 8),
            datetime(2024, 1, 1, 16),
            "Cafe",
            "Barista",
        )];
        shifts[0].employee_idx = Some(0); // Alice

        let mut solution = EmployeeSchedule::new(employees, shifts);
        let mut constraint = RequiredSkillConstraint::new();

        // Initialize
        let score = constraint.initialize(&solution);
        assert_eq!(score, HardSoftDecimalScore::ZERO); // Alice has Barista skill

        // Retract (before change)
        let retract_delta = constraint.on_retract(&solution, 0);
        assert_eq!(retract_delta, HardSoftDecimalScore::ZERO); // Was not a violation

        // Change to Bob (who also has Barista skill)
        solution.shifts[0].employee_idx = Some(1); // Bob

        // Insert (after change)
        let insert_delta = constraint.on_insert(&solution, 0);
        assert_eq!(insert_delta, HardSoftDecimalScore::ZERO); // Still not a violation
    }

    #[test]
    fn test_constraint_set_integration() {
        let employees = vec![Employee::new(0, "Alice").with_skill("Barista")];
        let mut shifts = vec![Shift::new(
            "1",
            datetime(2024, 1, 1, 8),
            datetime(2024, 1, 1, 16),
            "Cafe",
            "Barista",
        )];
        shifts[0].employee_idx = Some(0); // Alice

        let solution = EmployeeSchedule::new(employees, shifts);
        let constraints = create_constraints();

        // All constraints should pass (no violations)
        let score = constraints.evaluate_all(&solution);
        assert!(score.is_feasible());
    }
}
