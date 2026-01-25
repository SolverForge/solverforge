//! DTOs for REST API requests/responses.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::{Employee, EmployeeSchedule, Shift};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeDto {
    pub name: String,
    pub skills: Vec<String>,
    #[serde(default)]
    pub unavailable_dates: Vec<NaiveDate>,
    #[serde(default)]
    pub undesired_dates: Vec<NaiveDate>,
    #[serde(default)]
    pub desired_dates: Vec<NaiveDate>,
}

impl From<&Employee> for EmployeeDto {
    fn from(e: &Employee) -> Self {
        Self {
            name: e.name.clone(),
            skills: e.skills.iter().cloned().collect(),
            unavailable_dates: e.unavailable_dates.iter().cloned().collect(),
            undesired_dates: e.undesired_dates.iter().cloned().collect(),
            desired_dates: e.desired_dates.iter().cloned().collect(),
        }
    }
}

impl EmployeeDto {
    pub fn to_employee(&self, index: usize) -> Employee {
        let unavailable_dates: HashSet<NaiveDate> =
            self.unavailable_dates.iter().cloned().collect();
        let undesired_dates: HashSet<NaiveDate> =
            self.undesired_dates.iter().cloned().collect();
        let desired_dates: HashSet<NaiveDate> =
            self.desired_dates.iter().cloned().collect();

        let mut unavailable_days: Vec<NaiveDate> = unavailable_dates.iter().copied().collect();
        unavailable_days.sort();
        let mut undesired_days: Vec<NaiveDate> = undesired_dates.iter().copied().collect();
        undesired_days.sort();
        let mut desired_days: Vec<NaiveDate> = desired_dates.iter().copied().collect();
        desired_days.sort();

        Employee {
            index,
            name: self.name.clone(),
            skills: self.skills.iter().cloned().collect(),
            unavailable_dates,
            undesired_dates,
            desired_dates,
            unavailable_days,
            undesired_days,
            desired_days,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftDto {
    pub id: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub location: String,
    pub required_skill: String,
    pub employee: Option<EmployeeDto>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDto {
    pub employees: Vec<EmployeeDto>,
    pub shifts: Vec<ShiftDto>,
    #[serde(default)]
    pub score: Option<String>,
    #[serde(default, skip_deserializing)]
    pub solver_status: Option<String>,
}

impl ScheduleDto {
    pub fn from_schedule(schedule: &EmployeeSchedule, solver_status: Option<String>) -> Self {
        let employees: Vec<EmployeeDto> = schedule.employees.iter().map(EmployeeDto::from).collect();

        let shifts: Vec<ShiftDto> = schedule
            .shifts
            .iter()
            .map(|s| ShiftDto {
                id: s.id.clone(),
                start: s.start,
                end: s.end,
                location: s.location.clone(),
                required_skill: s.required_skill.clone(),
                employee: s.employee_idx
                    .and_then(|idx| schedule.employees.get(idx))
                    .map(EmployeeDto::from),
            })
            .collect();

        Self {
            employees,
            shifts,
            score: schedule.score.map(|s| format!("{}", s)),
            solver_status,
        }
    }

    pub fn to_domain(&self) -> EmployeeSchedule {
        let employees: Vec<Employee> = self
            .employees
            .iter()
            .enumerate()
            .map(|(i, dto)| dto.to_employee(i))
            .collect();
        let name_to_idx: std::collections::HashMap<&str, usize> = employees
            .iter()
            .map(|e| (e.name.as_str(), e.index))
            .collect();

        let shifts: Vec<Shift> = self
            .shifts
            .iter()
            .map(|s| Shift {
                id: s.id.clone(),
                start: s.start,
                end: s.end,
                location: s.location.clone(),
                required_skill: s.required_skill.clone(),
                employee_idx: s.employee.as_ref().and_then(|e| name_to_idx.get(e.name.as_str()).copied()),
            })
            .collect();

        EmployeeSchedule::new(employees, shifts)
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub solver_engine: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub score: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysisDto {
    pub name: String,
    #[serde(rename = "type")]
    pub constraint_type: String,
    pub weight: String,
    pub score: String,
    pub matches: Vec<ConstraintMatchDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintMatchDto {
    pub score: String,
    pub justification: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub score: String,
    pub constraints: Vec<ConstraintAnalysisDto>,
}
