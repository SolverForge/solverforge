use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use solverforge::prelude::*;
use std::collections::HashSet;

/// An employee who can be assigned to shifts.
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Employee {
    /// Index into `EmployeeSchedule.employees` for O(1) joins.
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

    /// Populates sorted Vec fields from HashSets.
    /// Must be called after all dates have been added.
    pub fn finalize(&mut self) {
        self.unavailable_days = self.unavailable_dates.iter().copied().collect();
        self.unavailable_days.sort();
        self.undesired_days = self.undesired_dates.iter().copied().collect();
        self.undesired_days.sort();
        self.desired_days = self.desired_dates.iter().copied().collect();
        self.desired_days.sort();
    }

    pub fn with_skills(mut self, skills: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for skill in skills {
            self.skills.insert(skill.into());
        }
        self
    }
}
