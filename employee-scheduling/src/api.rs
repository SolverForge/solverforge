//! REST API handlers for Employee Scheduling.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use parking_lot::RwLock;
use solverforge::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::constraints::create_fluent_constraints;
use crate::demo_data::{self, DemoData};
use crate::domain::EmployeeSchedule;
use crate::dto::*;

struct SolveJob {
    solution: EmployeeSchedule,
    solver_status: String,
}

pub struct AppState {
    jobs: RwLock<HashMap<String, SolveJob>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/healthz", get(health))
        .route("/info", get(info))
        .route("/demo-data", get(list_demo_data))
        .route("/demo-data/{id}", get(get_demo_data))
        .route("/schedules", post(create_schedule))
        .route("/schedules", get(list_schedules))
        .route("/schedules/analyze", put(analyze_schedule))
        .route("/schedules/{id}", get(get_schedule))
        .route("/schedules/{id}/status", get(get_schedule_status))
        .route("/schedules/{id}", delete(stop_solving))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "UP" })
}

async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        name: "Employee Scheduling",
        version: env!("CARGO_PKG_VERSION"),
        solver_engine: "SolverForge",
    })
}

async fn list_demo_data() -> Json<Vec<&'static str>> {
    Json(demo_data::list_demo_data())
}

async fn get_demo_data(Path(id): Path<String>) -> Result<Json<ScheduleDto>, StatusCode> {
    match id.parse::<DemoData>() {
        Ok(demo) => {
            let schedule = demo_data::generate(demo);
            Ok(Json(ScheduleDto::from_schedule(&schedule, None)))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<ScheduleDto>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let schedule = dto.to_domain();

    {
        let mut jobs = state.jobs.write();
        jobs.insert(
            id.clone(),
            SolveJob {
                solution: schedule.clone(),
                solver_status: "SOLVING".to_string(),
            },
        );
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let job_id = id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        while let Some((solution, _score)) = rx.recv().await {
            let mut jobs = state_clone.jobs.write();
            if let Some(job) = jobs.get_mut(&job_id) {
                job.solution = solution;
            }
        }
        let mut jobs = state_clone.jobs.write();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.solver_status = "NOT_SOLVING".to_string();
        }
    });

    rayon::spawn(move || {
        schedule.solve(None, tx);
    });

    id
}

async fn list_schedules(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.jobs.read().keys().cloned().collect())
}

async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ScheduleDto>, StatusCode> {
    match state.jobs.read().get(&id) {
        Some(job) => Ok(Json(ScheduleDto::from_schedule(
            &job.solution,
            Some(job.solver_status.clone()),
        ))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_schedule_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.jobs.read().get(&id) {
        Some(job) => Ok(Json(StatusResponse {
            score: job.solution.score.map(|s| format!("{}", s)),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn stop_solving(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> StatusCode {
    if state.jobs.write().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn analyze_schedule(Json(dto): Json<ScheduleDto>) -> Json<AnalyzeResponse> {
    let schedule = dto.to_domain();
    let constraints = create_fluent_constraints();
    let director = ScoreDirector::new(schedule, constraints);
    let score = director.get_score();
    let analyses = director
        .constraints()
        .evaluate_detailed(director.working_solution());

    let constraints_dto: Vec<ConstraintAnalysisDto> = analyses
        .into_iter()
        .map(|analysis| ConstraintAnalysisDto {
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
        })
        .collect();

    Json(AnalyzeResponse {
        score: format!("{}", score),
        constraints: constraints_dto,
    })
}
