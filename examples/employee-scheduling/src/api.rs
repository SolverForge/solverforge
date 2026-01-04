//! REST API handlers for Employee Scheduling.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::demo_data::{self, DemoData};
use crate::domain::{Employee, EmployeeSchedule, Shift};
use crate::solver::{SolverService, SolverStatus};

/// Application state shared across handlers.
pub struct AppState {
    pub solver: SolverService,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            solver: SolverService::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DTOs
// ============================================================================

/// Employee DTO for API requests/responses.
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
    fn to_employee(&self, index: usize) -> Employee {
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

/// Shift DTO with embedded Employee object.
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

/// Full schedule DTO for request/response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDto {
    pub employees: Vec<EmployeeDto>,
    pub shifts: Vec<ShiftDto>,
    #[serde(default)]
    pub score: Option<String>,
    #[serde(default)]
    pub solver_status: Option<SolverStatus>,
}

impl ScheduleDto {
    pub fn from_schedule(schedule: &EmployeeSchedule, status: Option<SolverStatus>) -> Self {
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
            solver_status: status,
        }
    }

    pub fn to_domain(&self) -> EmployeeSchedule {
        // Build employees with their indices set correctly
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

// ============================================================================
// Router and Handlers
// ============================================================================

/// Creates the API router.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health & Info
        .route("/health", get(health))
        .route("/info", get(info))
        // Demo data
        .route("/demo-data", get(list_demo_data))
        .route("/demo-data/{id}", get(get_demo_data))
        // Schedules
        .route("/schedules", post(create_schedule))
        .route("/schedules", get(list_schedules))
        .route("/schedules/analyze", put(analyze_schedule))
        .route("/schedules/{id}", get(get_schedule))
        .route("/schedules/{id}/status", get(get_schedule_status))
        .route("/schedules/{id}", delete(stop_solving))
        .with_state(state)
}

// ============================================================================
// Health & Info
// ============================================================================

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// GET /health - Health check endpoint.
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "UP" })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub solver_engine: &'static str,
}

/// GET /info - Application info endpoint.
async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        name: "Employee Scheduling",
        version: env!("CARGO_PKG_VERSION"),
        solver_engine: "SolverForge-RS",
    })
}

/// GET /demo-data - List available demo data sets.
async fn list_demo_data() -> Json<Vec<&'static str>> {
    Json(demo_data::list_demo_data())
}

/// GET /demo-data/{id} - Get a specific demo data set.
async fn get_demo_data(Path(id): Path<String>) -> Result<Json<ScheduleDto>, StatusCode> {
    match id.parse::<DemoData>() {
        Ok(demo) => {
            let schedule = demo_data::generate(demo);
            Ok(Json(ScheduleDto::from_schedule(&schedule, None)))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /schedules - Create and start solving a schedule.
/// Returns the job ID as plain text.
async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<ScheduleDto>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let schedule = dto.to_domain();
    let job = state.solver.create_job(id.clone(), schedule);
    state.solver.start_solving(job);
    id
}

/// GET /schedules - List all schedule IDs.
async fn list_schedules(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.solver.list_jobs())
}

/// GET /schedules/{id} - Get a schedule's current state.
async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ScheduleDto>, StatusCode> {
    match state.solver.get_job(&id) {
        Some(job) => {
            let job_guard = job.read();
            Ok(Json(ScheduleDto::from_schedule(
                &job_guard.schedule,
                Some(job_guard.status.clone()),
            )))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Response for schedule status only.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub score: Option<String>,
    pub solver_status: SolverStatus,
}

/// GET /schedules/{id}/status - Get a schedule's status.
async fn get_schedule_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.solver.get_job(&id) {
        Some(job) => {
            let job_guard = job.read();
            Ok(Json(StatusResponse {
                score: job_guard.schedule.score.map(|s| format!("{}", s)),
                solver_status: job_guard.status.clone(),
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /schedules/{id} - Stop solving and remove a schedule.
async fn stop_solving(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> StatusCode {
    state.solver.stop_solving(&id);
    if state.solver.remove_job(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Constraint analysis result.
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

/// A single constraint match.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintMatchDto {
    pub score: String,
    pub justification: String,
}

/// Response for constraint analysis.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub score: String,
    pub constraints: Vec<ConstraintAnalysisDto>,
}

/// PUT /schedules/analyze - Analyze constraints for a schedule.
///
/// Uses TypedScoreDirector for incremental scoring.
async fn analyze_schedule(
    Json(dto): Json<ScheduleDto>,
) -> Json<AnalyzeResponse> {
    use crate::constraints::create_fluent_constraints;
    use solverforge::{ConstraintSet, TypedScoreDirector};

    let schedule = dto.to_domain();

    // Use fluent API constraints for zero-erasure scoring
    let constraints = create_fluent_constraints();
    let mut director = TypedScoreDirector::new(schedule, constraints);

    let score = director.calculate_score();

    // Get per-constraint breakdown with detailed matches
    let analyses = director.constraints().evaluate_detailed(director.working_solution());

    let constraints_dto: Vec<ConstraintAnalysisDto> = analyses
        .into_iter()
        .map(|analysis| {
            ConstraintAnalysisDto {
                name: analysis.constraint_ref.name.clone(),
                constraint_type: if analysis.is_hard { "hard" } else { "soft" }.to_string(),
                weight: format!("{}", analysis.weight),
                score: format!("{}", analysis.score),
                matches: analysis
                    .matches
                    .iter()
                    .map(|m| ConstraintMatchDto {
                        score: format!("{}", m.score),
                        justification: m.justification.description.clone(),
                    })
                    .collect(),
            }
        })
        .collect();

    Json(AnalyzeResponse {
        score: format!("{}", score),
        constraints: constraints_dto,
    })
}
