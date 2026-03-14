use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

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
    /// Index into `EmployeeSchedule.employees`. `None` means unassigned.
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

    pub fn date(&self) -> NaiveDate {
        self.start.date()
    }
}
