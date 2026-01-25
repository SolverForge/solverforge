//! REST API handlers for Employee Scheduling.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, post, put},
    Json, Router,
};
use futures::stream::Stream;
use parking_lot::RwLock;
use solverforge::prelude::*;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::constraints::create_fluent_constraints;
use crate::demo_data::{self, DemoData};
use crate::domain::EmployeeSchedule;
use crate::dto::*;

struct SolveJob {
    solution: EmployeeSchedule,
    solver_status: String,
    broadcast_tx: broadcast::Sender<String>,
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
        .route("/schedules/{id}/events", get(subscribe_schedule))
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

    let (broadcast_tx, _) = broadcast::channel::<String>(16);

    {
        let mut jobs = state.jobs.write();
        jobs.insert(
            id.clone(),
            SolveJob {
                solution: schedule.clone(),
                solver_status: "SOLVING".to_string(),
                broadcast_tx: broadcast_tx.clone(),
            },
        );
    }

    let (tx, mut rx) =
        tokio::sync::mpsc::unbounded_channel::<(EmployeeSchedule, HardSoftDecimalScore)>();
    let job_id = id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        while let Some((solution, _score)) = rx.recv().await {
            let mut jobs = state_clone.jobs.write();
            if let Some(job) = jobs.get_mut(&job_id) {
                job.solution = solution.clone();
                let dto = ScheduleDto::from_schedule(&solution, Some("SOLVING".to_string()));
                if let Ok(json) = serde_json::to_string(&dto) {
                    let _ = job.broadcast_tx.send(json);
                }
            }
        }
        let mut jobs = state_clone.jobs.write();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.solver_status = "NOT_SOLVING".to_string();
            let dto = ScheduleDto::from_schedule(&job.solution, Some("NOT_SOLVING".to_string()));
            if let Ok(json) = serde_json::to_string(&dto) {
                let _ = job.broadcast_tx.send(json);
            }
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

async fn subscribe_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let (rx, initial_json) = {
        let jobs = state.jobs.read();
        match jobs.get(&id) {
            Some(job) => {
                // Get the current state to send immediately
                let dto =
                    ScheduleDto::from_schedule(&job.solution, Some(job.solver_status.clone()));
                let json = serde_json::to_string(&dto).unwrap_or_default();
                (job.broadcast_tx.subscribe(), json)
            }
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    // Send current state immediately, then stream updates
    let initial = futures::stream::once(async move { Ok(Event::default().data(initial_json)) });

    let updates = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(json) => Some(Ok(Event::default().data(json))),
        Err(_) => None,
    });

    let stream = initial.chain(updates);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
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
