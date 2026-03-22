use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use super::dto::{AnalyzeResponse, ConstraintAnalysisDto, ConstraintMatchDto, PlanDto};
use super::sse;
use crate::data::{generate, DemoData};
use crate::solver::{SolverService, SolverStatus};

/// Shared application state.
pub struct AppState {
    pub solver: SolverService,
}

impl AppState {
    pub fn new() -> Self {
        Self { solver: SolverService::new() }
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

/// Creates the API router.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/info", get(info))
        .route("/demo-data", get(list_demo_data))
        .route("/demo-data/{id}", get(get_demo_data))
        .route("/schedules", post(create_schedule))
        .route("/schedules", get(list_schedules))
        .route("/schedules/analyze", put(analyze_schedule))
        .route("/schedules/{id}", get(get_schedule))
        .route("/schedules/{id}/status", get(get_schedule_status))
        .route("/schedules/{id}/events", get(sse::events))
        .route("/schedules/{id}/analyze", get(analyze_by_id))
        .route("/schedules/{id}", delete(stop_solving))
        .with_state(state)
}

// ============================================================================
// Handlers
// ============================================================================

#[derive(Serialize)]
struct HealthResponse { status: &'static str }

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "UP" })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InfoResponse {
    name: &'static str,
    version: &'static str,
    solver_engine: &'static str,
}

async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        solver_engine: "SolverForge",
    })
}

async fn list_demo_data() -> Json<Vec<&'static str>> {
    Json(vec!["STANDARD", "SMALL"])
}

async fn get_demo_data(Path(id): Path<String>) -> Result<Json<PlanDto>, StatusCode> {
    let demo = id.parse::<DemoData>().map_err(|_| StatusCode::NOT_FOUND)?;
    let plan = generate(demo);
    Ok(Json(PlanDto::from_plan(&plan, None)))
}

async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<PlanDto>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let plan = dto.to_domain();
    state.solver.start_solving(id.clone(), plan);
    id
}

async fn list_schedules(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.solver.list_jobs())
}

async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PlanDto>, StatusCode> {
    if !state.solver.has_job(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    match state.solver.with_snapshot(&id, |plan, _score, status| {
        PlanDto::from_plan(plan, Some(status))
    }) {
        Some(dto) => Ok(Json(dto)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    score: Option<String>,
    solver_status: SolverStatus,
}

async fn get_schedule_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    if !state.solver.has_job(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    match state.solver.with_snapshot(&id, |plan, _score, status| StatusResponse {
        score: plan.score.map(|s| s.to_string()),
        solver_status: status,
    }) {
        Some(resp) => Ok(Json(resp)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn stop_solving(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> StatusCode {
    state.solver.stop_solving(&id);
    if state.solver.remove_job(&id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn analyze_schedule(Json(dto): Json<PlanDto>) -> Json<AnalyzeResponse> {
    use crate::constraints::create_constraints;
    use solverforge::ConstraintSet;
    use solverforge::ScoreDirector;

    let plan = dto.to_domain();
    let constraints = create_constraints();
    let mut director = ScoreDirector::new(plan, constraints);
    let score = director.calculate_score();
    let analyses = director.constraints().evaluate_detailed(director.working_solution());

    let constraints_dto: Vec<ConstraintAnalysisDto> = analyses
        .into_iter()
        .map(|a| ConstraintAnalysisDto {
            name: a.constraint_ref.name.clone(),
            constraint_type: if a.is_hard { "hard" } else { "soft" }.to_string(),
            weight: format!("{}", a.weight),
            score: format!("{}", a.score),
            matches: a
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

async fn analyze_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    use crate::constraints::create_constraints;
    use solverforge::ConstraintSet;
    use solverforge::ScoreDirector;

    let plan = state
        .solver
        .with_snapshot(&id, |plan, _score, _status| plan.clone())
        .ok_or(StatusCode::NOT_FOUND)?;

    let constraints = create_constraints();
    let mut director = ScoreDirector::new(plan, constraints);
    let score = director.calculate_score();
    let analyses = director.constraints().evaluate_detailed(director.working_solution());

    let constraints_dto: Vec<ConstraintAnalysisDto> = analyses
        .into_iter()
        .map(|a| ConstraintAnalysisDto {
            name: a.constraint_ref.name.clone(),
            constraint_type: if a.is_hard { "hard" } else { "soft" }.to_string(),
            weight: format!("{}", a.weight),
            score: format!("{}", a.score),
            matches: a
                .matches
                .iter()
                .map(|m| ConstraintMatchDto {
                    score: format!("{}", m.score),
                    justification: m.justification.description.clone(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(AnalyzeResponse {
        score: format!("{}", score),
        constraints: constraints_dto,
    }))
}
