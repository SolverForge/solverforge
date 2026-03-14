use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use super::dto::{InstanceDto, SolutionDto};
use crate::data::demo_instance;
use crate::domain::{ProblemData, Vehicle, VrpPlan};
use crate::solver::{SolverService, SolverStatus};

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

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/demo-data", get(get_demo))
        .route("/solutions", post(create_solution))
        .route("/solutions", get(list_solutions))
        .route("/solutions/{id}", get(get_solution))
        .route("/solutions/{id}", delete(stop_solving))
        .with_state(state)
}

async fn health() -> &'static str { "OK" }

async fn get_demo() -> Json<InstanceDto> {
    let (data, n_vehicles) = demo_instance();
    Json(InstanceDto {
        capacity: data.capacity,
        depot: data.depot,
        demands: data.demands.iter().map(|&d| d as i32).collect(),
        distance_matrix: data.distance_matrix,
        n_vehicles,
        time_limit_secs: 60,
    })
}

async fn create_solution(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<InstanceDto>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let n_nodes = dto.demands.len();
    let depot = dto.depot;

    let problem = Box::new(ProblemData {
        capacity: dto.capacity,
        depot,
        demands: dto.demands.iter().map(|&d| d as i32).collect(),
        distance_matrix: dto.distance_matrix.clone(),
        time_windows: vec![(0, i64::MAX); n_nodes],
        service_durations: vec![0; n_nodes],
        travel_times: dto.distance_matrix,
        vehicle_departure_time: 0,
    });

    let data_ptr: *const ProblemData = &*problem;
    let all_visits: Vec<usize> = (0..n_nodes).filter(|&i| i != depot).collect();
    let vehicles: Vec<Vehicle> = (0..dto.n_vehicles)
        .map(|vid| Vehicle { id: vid, visits: Vec::new(), data: data_ptr })
        .collect();

    let plan = VrpPlan {
        vehicles,
        all_visits,
        score: None,
        problem_data: Some(problem),
    };

    state.solver.start_solving(id.clone(), plan);
    id
}

async fn list_solutions(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.solver.list_jobs())
}

async fn get_solution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SolutionDto>, StatusCode> {
    // Return NOT_FOUND if the job doesn't exist; ACCEPTED (202) if solving but no solution yet.
    match state.solver.get_status(&id) {
        None => return Err(StatusCode::NOT_FOUND),
        Some(status) => {
            let result = state.solver.with_snapshot(&id, |plan, score, st| {
                let routes: Vec<Vec<usize>> = plan
                    .vehicles
                    .iter()
                    .filter(|v| !v.visits.is_empty())
                    .map(|v| v.visits.clone())
                    .collect();
                let cost = plan.score.map(|s| -s.soft()).unwrap_or(0);
                Json(SolutionDto {
                    routes,
                    cost,
                    score: score.map(|s| s.to_string()),
                    solver_status: st,
                })
            });
            match result {
                Some(dto) => Ok(dto),
                None => {
                    // Still solving, no best solution yet.
                    Ok(Json(SolutionDto {
                        routes: vec![],
                        cost: 0,
                        score: None,
                        solver_status: status,
                    }))
                }
            }
        }
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
