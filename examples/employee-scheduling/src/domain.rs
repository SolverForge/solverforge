//! Domain model for Employee Scheduling Problem.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use solverforge::prelude::*;
use std::collections::HashSet;

/// An employee who can be assigned to shifts.
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Employee {
    /// Index of this employee in `EmployeeSchedule.employees` for O(1) join matching.
    pub index: usize,
    pub name: String,
    pub skills: HashSet<String>,
    #[serde(rename = "unavailableDates", default)]
    pub unavailable_dates: HashSet<NaiveDate>,
    #[serde(rename = "undesiredDates", default)]
    pub undesired_dates: HashSet<NaiveDate>,
    #[serde(rename = "desiredDates", default)]
    pub desired_dates: HashSet<NaiveDate>,
    /// Sorted unavailable dates for `flatten_last` compatibility.
    /// Populated by `finalize()` from `unavailable_dates` HashSet.
    #[serde(skip)]
    pub unavailable_days: Vec<NaiveDate>,
    /// Sorted undesired dates for `flatten_last` compatibility.
    #[serde(skip)]
    pub undesired_days: Vec<NaiveDate>,
    /// Sorted desired dates for `flatten_last` compatibility.
    #[serde(skip)]
    pub desired_days: Vec<NaiveDate>,
}

impl Employee {
    pub fn new(index: usize, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
            skills: HashSet::new(),
            unavailable_dates: HashSet::new(),
            undesired_dates: HashSet::new(),
            desired_dates: HashSet::new(),
            unavailable_days: Vec::new(),
            undesired_days: Vec::new(),
            desired_days: Vec::new(),
        }
    }

    /// Populates derived Vec fields from HashSets for zero-erasure stream compatibility.
    /// Must be called after all dates have been added to HashSets.
    pub fn finalize(&mut self) {
        self.unavailable_days = self.unavailable_dates.iter().copied().collect();
        self.unavailable_days.sort();
        self.undesired_days = self.undesired_dates.iter().copied().collect();
        self.undesired_days.sort();
        self.desired_days = self.desired_dates.iter().copied().collect();
        self.desired_days.sort();
    }

    pub fn with_skill(mut self, skill: impl Into<String>) -> Self {
        self.skills.insert(skill.into());
        self
    }

    pub fn with_skills(mut self, skills: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for skill in skills {
            self.skills.insert(skill.into());
        }
        self
    }

    pub fn with_unavailable_date(mut self, date: NaiveDate) -> Self {
        self.unavailable_dates.insert(date);
        self
    }

    pub fn with_undesired_date(mut self, date: NaiveDate) -> Self {
        self.undesired_dates.insert(date);
        self
    }

    pub fn with_desired_date(mut self, date: NaiveDate) -> Self {
        self.desired_dates.insert(date);
        self
    }
}

/// A shift that needs to be staffed by an employee.
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct Shift {
    #[planning_id]
    pub id: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub location: String,
    #[serde(rename = "requiredSkill")]
    pub required_skill: String,
    /// Index into `EmployeeSchedule.employees` (O(1) lookup, no String cloning).
    #[planning_variable(allows_unassigned = true)]
    pub employee_idx: Option<usize>,
}

impl Shift {
    pub fn new(
        id: impl Into<String>,
        start: NaiveDateTime,
        end: NaiveDateTime,
        location: impl Into<String>,
        required_skill: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            start,
            end,
            location: location.into(),
            required_skill: required_skill.into(),
            employee_idx: None,
        }
    }

    /// Returns the date of the shift start.
    pub fn date(&self) -> NaiveDate {
        self.start.date()
    }

    /// Returns the duration in hours.
    pub fn duration_hours(&self) -> f64 {
        (self.end - self.start).num_minutes() as f64 / 60.0
    }
}

/// The employee scheduling solution.
#[planning_solution]
#[derive(Serialize, Deserialize)]
pub struct EmployeeSchedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,
    #[planning_score]
    pub score: Option<HardSoftDecimalScore>,
    #[serde(rename = "solverStatus", skip_serializing_if = "Option::is_none")]
    pub solver_status: Option<String>,
}

impl EmployeeSchedule {
    pub fn new(employees: Vec<Employee>, shifts: Vec<Shift>) -> Self {
        Self {
            employees,
            shifts,
            score: None,
            solver_status: None,
        }
    }

    /// Gets an Employee by index (O(1)).
    #[inline]
    pub fn get_employee(&self, idx: usize) -> Option<&Employee> {
        self.employees.get(idx)
    }

    /// Returns the number of employees.
    #[inline]
    pub fn employee_count(&self) -> usize {
        self.employees.len()
    }
}
