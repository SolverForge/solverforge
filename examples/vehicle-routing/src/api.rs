//! REST API for Vehicle Routing Problem.
//!
//! Provides endpoints for:
//! - Demo data retrieval
//! - Route plan management (create, get, stop)
//! - Route geometry for map visualization
//! - Swagger UI at /q/swagger-ui

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::demo_data::{available_datasets, generate_by_name};
use crate::domain::{Vehicle, VehicleRoutePlan, Visit};
use crate::geometry::{encode_routes, EncodedSegment};
use crate::solver::{SolverConfig, SolverService, SolverStatus};
use solverforge::prelude::HardSoftScore;
use std::time::Duration;

// ============================================================================
// Date/Time Utilities
// ============================================================================

/// Reference date for time calculations (matches Python frontend).
const BASE_DATE: &str = "2025-01-05";

/// Converts seconds from midnight to ISO datetime string.
///
/// # Examples
///
/// ```
/// use vehicle_routing::api::seconds_to_iso;
///
/// assert_eq!(seconds_to_iso(0), "2025-01-05T00:00:00");
/// assert_eq!(seconds_to_iso(8 * 3600), "2025-01-05T08:00:00");
/// assert_eq!(seconds_to_iso(8 * 3600 + 30 * 60 + 45), "2025-01-05T08:30:45");
/// ```
pub fn seconds_to_iso(seconds: i64) -> String {
    let hours = (seconds / 3600) % 24;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{}T{:02}:{:02}:{:02}", BASE_DATE, hours, mins, secs)
}

/// Parses ISO datetime string to seconds from midnight.
///
/// # Examples
///
/// ```
/// use vehicle_routing::api::iso_to_seconds;
///
/// assert_eq!(iso_to_seconds("2025-01-05T08:00:00"), 8 * 3600);
/// assert_eq!(iso_to_seconds("2025-01-05T08:30:45"), 8 * 3600 + 30 * 60 + 45);
/// ```
pub fn iso_to_seconds(iso: &str) -> i64 {
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S") {
        let midnight = NaiveDateTime::new(dt.date(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        (dt - midnight).num_seconds()
    } else {
        0
    }
}

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

/// Creates the API router with CORS and Swagger UI enabled.
pub fn create_router() -> Router {
    let state = Arc::new(AppState::new());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health & Info
        .route("/health", get(health))
        .route("/info", get(info))
        // Demo data
        .route("/demo-data", get(list_demo_data))
        .route("/demo-data/{name}", get(get_demo_data))
        .route("/demo-data/{name}/stream", get(get_demo_data_stream))
        // Route plans
        .route("/route-plans", post(create_route_plan))
        .route("/route-plans", get(list_route_plans))
        .route("/route-plans/{id}", get(get_route_plan))
        .route("/route-plans/{id}/status", get(get_route_plan_status))
        .route("/route-plans/{id}", delete(stop_solving))
        .route("/route-plans/{id}/geometry", get(get_route_geometry))
        // Analysis and recommendations
        .route("/route-plans/analyze", put(analyze_route_plan))
        .route("/route-plans/recommendation", post(recommend_assignment))
        .route("/route-plans/recommendation/apply", post(apply_recommendation))
        // Swagger UI at /q/swagger-ui (Quarkus-style path)
        .merge(SwaggerUi::new("/q/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors)
        .with_state(state)
}

// ============================================================================
// Health & Info
// ============================================================================

/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Status indicator ("UP" when healthy).
    pub status: &'static str,
}

/// GET /health - Health check endpoint.
#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service is healthy", body = HealthResponse))
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "UP" })
}

/// Application info response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    /// Application name.
    pub name: &'static str,
    /// Application version.
    pub version: &'static str,
    /// Solver engine name.
    pub solver_engine: &'static str,
}

/// GET /info - Application info endpoint.
#[utoipa::path(
    get,
    path = "/info",
    responses((status = 200, description = "Application info", body = InfoResponse))
)]
async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        name: "Vehicle Routing",
        version: env!("CARGO_PKG_VERSION"),
        solver_engine: "SolverForge-RS",
    })
}

// ============================================================================
// Demo Data
// ============================================================================

/// GET /demo-data - List available demo datasets.
#[utoipa::path(
    get,
    path = "/demo-data",
    responses((status = 200, description = "List of demo dataset names", body = Vec<String>))
)]
async fn list_demo_data() -> Json<Vec<&'static str>> {
    Json(available_datasets().to_vec())
}

/// GET /demo-data/{name} - Get a specific demo dataset.
#[utoipa::path(
    get,
    path = "/demo-data/{name}",
    params(("name" = String, Path, description = "Demo dataset name")),
    responses(
        (status = 200, description = "Demo data retrieved", body = RoutePlanDto),
        (status = 404, description = "Dataset not found")
    )
)]
async fn get_demo_data(Path(name): Path<String>) -> Result<Json<RoutePlanDto>, StatusCode> {
    match generate_by_name(&name) {
        Some(plan) => Ok(Json(RoutePlanDto::from_plan(&plan, None))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /demo-data/{name}/stream - Get demo data with SSE progress updates.
///
/// Returns Server-Sent Events (SSE) stream with progress and final solution.
/// Downloads OSM road network and computes real driving times.
/// Compatible with frontend's EventSource API.
async fn get_demo_data_stream(Path(name): Path<String>) -> impl IntoResponse {
    // Generate the demo data
    let mut plan = match generate_by_name(&name) {
        Some(p) => p,
        None => {
            let error = r#"data: {"event":"error","message":"Demo data not found"}"#;
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(format!("{}\n\n", error)))
                .unwrap();
        }
    };

    // Build SSE stream with async routing
    let stream = async_stream::stream! {
        // Progress: downloading
        yield Ok::<_, std::convert::Infallible>(
            "data: {\"event\":\"progress\",\"phase\":\"downloading\",\"message\":\"Downloading road network...\",\"percent\":20}\n\n".to_string()
        );

        // Initialize routing (downloads OSM, builds graph, computes matrix)
        let routing_result = plan.init_routing().await;

        if let Err(e) = routing_result {
            // Routing failed, fall back to haversine
            tracing::warn!("Road routing failed, using haversine: {}", e);
            plan.finalize();
            yield Ok("data: {\"event\":\"progress\",\"phase\":\"fallback\",\"message\":\"Using straight-line distances\",\"percent\":80}\n\n".to_string());
        } else {
            yield Ok("data: {\"event\":\"progress\",\"phase\":\"computing\",\"message\":\"Computing travel times...\",\"percent\":80}\n\n".to_string());
        }

        // Build response DTO
        let dto = RoutePlanDto::from_plan(&plan, None);
        let solution_json = serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string());

        // Complete
        yield Ok(format!(
            "data: {{\"event\":\"progress\",\"phase\":\"complete\",\"message\":\"Ready!\",\"percent\":100}}\n\n\
             data: {{\"event\":\"complete\",\"solution\":{}}}\n\n",
            solution_json
        ));
    };

    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(body)
        .unwrap()
}

// ============================================================================
// DTOs (Python API Compatible)
// ============================================================================

/// Visit DTO matching Python API structure.
///
/// All times are ISO datetime strings (e.g., "2025-01-05T08:30:00").
/// Location is `[latitude, longitude]` array.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VisitDto {
    /// Unique visit identifier.
    pub id: String,
    /// Customer name.
    pub name: String,
    /// Location as `[latitude, longitude]`.
    pub location: [f64; 2],
    /// Quantity demanded.
    pub demand: i32,
    /// Earliest service start time (ISO datetime).
    pub min_start_time: String,
    /// Latest service end time (ISO datetime).
    pub max_end_time: String,
    /// Service duration in seconds.
    pub service_duration: i32,
    /// Assigned vehicle ID (null if unassigned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<String>,
    /// Previous visit in route (null if first or unassigned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_visit: Option<String>,
    /// Next visit in route (null if last or unassigned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_visit: Option<String>,
    /// Arrival time at visit (ISO datetime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrival_time: Option<String>,
    /// Service start time (ISO datetime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_service_time: Option<String>,
    /// Departure time from visit (ISO datetime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_time: Option<String>,
    /// Driving time from previous stop in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driving_time_seconds_from_previous_standstill: Option<i32>,
}

/// Vehicle DTO matching Python API structure.
///
/// Visits are referenced by ID only; full visit data is in the plan's `visits` array.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDto {
    /// Unique vehicle identifier.
    pub id: String,
    /// Vehicle name for display.
    pub name: String,
    /// Maximum capacity.
    pub capacity: i32,
    /// Home depot location as `[latitude, longitude]`.
    pub home_location: [f64; 2],
    /// Departure time from depot (ISO datetime).
    pub departure_time: String,
    /// Visit IDs in route order.
    pub visits: Vec<String>,
    /// Total demand of assigned visits.
    pub total_demand: i32,
    /// Total driving time in seconds.
    pub total_driving_time_seconds: i32,
    /// Arrival time back at depot (ISO datetime).
    pub arrival_time: String,
}

/// Termination configuration for the solver.
///
/// Supports multiple termination conditions that combine with OR logic.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct TerminationConfigDto {
    /// Stop after this many seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds_spent_limit: Option<u64>,
    /// Stop after this many seconds without improvement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unimproved_seconds_spent_limit: Option<u64>,
    /// Stop after this many steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_count_limit: Option<u64>,
    /// Stop after this many steps without improvement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unimproved_step_count_limit: Option<u64>,
}

/// Full route plan DTO matching Python API structure.
///
/// Contains ALL visits in a flat list; assignment is indicated by `vehicle` field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanDto {
    /// Problem name.
    pub name: String,
    /// South-west corner of bounding box as `[latitude, longitude]`.
    pub south_west_corner: [f64; 2],
    /// North-east corner of bounding box as `[latitude, longitude]`.
    pub north_east_corner: [f64; 2],
    /// Earliest vehicle departure time (ISO datetime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date_time: Option<String>,
    /// Latest vehicle arrival time (ISO datetime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date_time: Option<String>,
    /// Total driving time across all vehicles in seconds.
    pub total_driving_time_seconds: i32,
    /// All vehicles.
    pub vehicles: Vec<VehicleDto>,
    /// All visits (assigned and unassigned).
    pub visits: Vec<VisitDto>,
    /// Current score (e.g., "0hard/-14400soft").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<String>,
    /// Solver status ("NOT_SOLVING", "SOLVING_ACTIVE", etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_status: Option<String>,
    /// Termination configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfigDto>,
    /// Precomputed travel time matrix (optional, from real roads).
    /// Row/column order: depot locations first, then visit locations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub travel_time_matrix: Option<Vec<Vec<i64>>>,
}

impl RoutePlanDto {
    /// Converts domain model to DTO for API responses.
    ///
    /// Builds flat visit list with vehicle assignments and timing info.
    pub fn from_plan(plan: &VehicleRoutePlan, status: Option<SolverStatus>) -> Self {
        // Build vehicle ID lookup: visit_idx -> (vehicle_id, position in route)
        let mut visit_vehicle: HashMap<usize, (String, usize)> = HashMap::new();
        for v in &plan.vehicles {
            for (pos, &visit_idx) in v.visits.iter().enumerate() {
                visit_vehicle.insert(visit_idx, (v.id.to_string(), pos));
            }
        }

        // Build visit ID lookup for next/previous references
        let visit_id = |idx: usize| -> String { format!("v{}", idx) };

        // Calculate timing for all vehicles
        let mut visit_timings: HashMap<usize, (i64, i64, i64, i32)> = HashMap::new(); // (arrival, service_start, departure, driving_time)
        for v in &plan.vehicles {
            let timings = plan.calculate_route_times(v);
            let mut prev_loc = v.home_location_idx;

            for timing in timings.iter() {
                let driving_time = plan.travel_time(prev_loc, plan.visits[timing.visit_idx].location_idx);
                let service_start = timing.arrival.max(plan.visits[timing.visit_idx].min_start_time);
                visit_timings.insert(
                    timing.visit_idx,
                    (timing.arrival, service_start, timing.departure, driving_time as i32),
                );
                prev_loc = plan.visits[timing.visit_idx].location_idx;
            }
        }

        // Build ALL visits with assignment info
        let visits: Vec<VisitDto> = plan
            .visits
            .iter()
            .filter_map(|visit| {
                let loc = plan.locations.get(visit.location_idx)?;
                let (vehicle_id, vehicle_pos) = visit_vehicle.get(&visit.index).cloned().unzip();
                let vehicle_for_visit = vehicle_id.as_ref().and_then(|vid| {
                    plan.vehicles.iter().find(|v| v.id.to_string() == *vid)
                });

                // Get previous/next visit IDs
                let (prev_visit, next_visit) = if let (Some(v), Some(pos)) = (vehicle_for_visit, vehicle_pos) {
                    let prev = if pos > 0 { Some(visit_id(v.visits[pos - 1])) } else { None };
                    let next = if pos + 1 < v.visits.len() { Some(visit_id(v.visits[pos + 1])) } else { None };
                    (prev, next)
                } else {
                    (None, None)
                };

                let timing = visit_timings.get(&visit.index);

                Some(VisitDto {
                    id: visit_id(visit.index),
                    name: visit.name.clone(),
                    location: [loc.latitude, loc.longitude],
                    demand: visit.demand,
                    min_start_time: seconds_to_iso(visit.min_start_time),
                    max_end_time: seconds_to_iso(visit.max_end_time),
                    service_duration: visit.service_duration as i32,
                    vehicle: vehicle_id,
                    previous_visit: prev_visit,
                    next_visit,
                    arrival_time: timing.map(|t| seconds_to_iso(t.0)),
                    start_service_time: timing.map(|t| seconds_to_iso(t.1)),
                    departure_time: timing.map(|t| seconds_to_iso(t.2)),
                    driving_time_seconds_from_previous_standstill: timing.map(|t| t.3),
                })
            })
            .collect();

        // Build vehicles with visit ID references
        let vehicles: Vec<VehicleDto> = plan
            .vehicles
            .iter()
            .map(|v| {
                let home_loc = plan
                    .locations
                    .get(v.home_location_idx)
                    .map(|l| [l.latitude, l.longitude])
                    .unwrap_or([0.0, 0.0]);

                let total_driving = plan.total_driving_time(v);
                let route_times = plan.calculate_route_times(v);

                // Calculate arrival time back at depot
                let arrival = if v.visits.is_empty() {
                    v.departure_time
                } else if let Some(last_timing) = route_times.last() {
                    let last_visit = &plan.visits[last_timing.visit_idx];
                    let return_travel = plan.travel_time(last_visit.location_idx, v.home_location_idx);
                    last_timing.departure + return_travel
                } else {
                    v.departure_time
                };

                VehicleDto {
                    id: v.id.to_string(),
                    name: v.name.clone(),
                    capacity: v.capacity,
                    home_location: home_loc,
                    departure_time: seconds_to_iso(v.departure_time),
                    visits: v.visits.iter().map(|&idx| visit_id(idx)).collect(),
                    total_demand: v.total_demand(plan),
                    total_driving_time_seconds: total_driving as i32,
                    arrival_time: seconds_to_iso(arrival),
                }
            })
            .collect();

        // Calculate plan-level times
        let start_dt = plan.vehicles.iter().map(|v| v.departure_time).min();
        let end_dt = vehicles.iter().map(|v| iso_to_seconds(&v.arrival_time)).max();

        Self {
            name: plan.name.clone(),
            south_west_corner: plan.south_west_corner,
            north_east_corner: plan.north_east_corner,
            start_date_time: start_dt.map(seconds_to_iso),
            end_date_time: end_dt.map(seconds_to_iso),
            total_driving_time_seconds: plan.total_driving_time_all() as i32,
            vehicles,
            visits,
            score: plan.score.map(|s| format!("{}", s)),
            solver_status: status.map(|s| s.as_str().to_string()),
            termination: None,
            travel_time_matrix: if plan.travel_time_matrix.is_empty() {
                None
            } else {
                Some(plan.travel_time_matrix.clone())
            },
        }
    }

    /// Converts DTO to domain model for solving.
    pub fn to_domain(&self) -> VehicleRoutePlan {
        use crate::domain::Location;

        // Build locations (depots first, then visit locations)
        let mut locations = Vec::new();
        let mut depot_indices: HashMap<(i64, i64), usize> = HashMap::new();

        // Add unique depot locations
        for vdto in &self.vehicles {
            let key = (
                (vdto.home_location[0] * 1e6) as i64,
                (vdto.home_location[1] * 1e6) as i64,
            );
            depot_indices.entry(key).or_insert_with(|| {
                let idx = locations.len();
                locations.push(Location::new(idx, vdto.home_location[0], vdto.home_location[1]));
                idx
            });
        }

        // Build visit ID to index mapping
        let visit_id_to_idx: HashMap<&str, usize> = self
            .visits
            .iter()
            .enumerate()
            .map(|(i, v)| (v.id.as_str(), i))
            .collect();

        // Add visit locations
        let visit_start_idx = locations.len();
        for (i, vdto) in self.visits.iter().enumerate() {
            locations.push(Location::new(
                visit_start_idx + i,
                vdto.location[0],
                vdto.location[1],
            ));
        }

        // Build visits
        let visits: Vec<Visit> = self
            .visits
            .iter()
            .enumerate()
            .map(|(i, vdto)| {
                Visit::new(i, &vdto.name, visit_start_idx + i)
                    .with_demand(vdto.demand)
                    .with_time_window(
                        iso_to_seconds(&vdto.min_start_time),
                        iso_to_seconds(&vdto.max_end_time),
                    )
                    .with_service_duration(vdto.service_duration as i64)
            })
            .collect();

        // Build vehicles
        let vehicles: Vec<Vehicle> = self
            .vehicles
            .iter()
            .enumerate()
            .map(|(i, vdto)| {
                let key = (
                    (vdto.home_location[0] * 1e6) as i64,
                    (vdto.home_location[1] * 1e6) as i64,
                );
                let home_idx = depot_indices[&key];

                // Map visit IDs to indices
                let visit_indices: Vec<usize> = vdto
                    .visits
                    .iter()
                    .filter_map(|vid| visit_id_to_idx.get(vid.as_str()).copied())
                    .collect();

                let mut v = Vehicle::new(i, &vdto.name, vdto.capacity, home_idx);
                v.departure_time = iso_to_seconds(&vdto.departure_time);
                v.visits = visit_indices;
                v
            })
            .collect();

        let mut plan = VehicleRoutePlan::new(&self.name, locations, visits, vehicles);
        plan.south_west_corner = self.south_west_corner;
        plan.north_east_corner = self.north_east_corner;

        // Use provided matrix (from real roads) if available, otherwise compute haversine
        if let Some(matrix) = &self.travel_time_matrix {
            plan.travel_time_matrix = matrix.clone();
        } else {
            plan.finalize();
        }
        plan
    }
}

// ============================================================================
// Route Plan Handlers
// ============================================================================

/// POST /route-plans - Create and start solving a route plan.
#[utoipa::path(
    post,
    path = "/route-plans",
    request_body = RoutePlanDto,
    responses((status = 200, description = "Job ID", body = String))
)]
async fn create_route_plan(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<RoutePlanDto>,
) -> Result<String, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let mut plan = dto.to_domain();

    // Initialize road routing (uses cached network - instant after first download)
    if let Err(e) = plan.init_routing().await {
        tracing::error!("Road routing initialization failed: {}", e);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Convert termination config from DTO
    let config = if let Some(term) = &dto.termination {
        SolverConfig {
            time_limit: term.seconds_spent_limit.map(Duration::from_secs),
            unimproved_time_limit: term.unimproved_seconds_spent_limit.map(Duration::from_secs),
            step_limit: term.step_count_limit,
            unimproved_step_limit: term.unimproved_step_count_limit,
        }
    } else {
        SolverConfig::default_config()
    };

    let job = state.solver.create_job_with_config(id.clone(), plan, config);
    state.solver.start_solving(job);
    Ok(id)
}

/// GET /route-plans - List all route plan IDs.
#[utoipa::path(
    get,
    path = "/route-plans",
    responses((status = 200, description = "List of job IDs", body = Vec<String>))
)]
async fn list_route_plans(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.solver.list_jobs())
}

/// GET /route-plans/{id} - Get current route plan state.
#[utoipa::path(
    get,
    path = "/route-plans/{id}",
    params(("id" = String, Path, description = "Route plan ID")),
    responses(
        (status = 200, description = "Route plan retrieved", body = RoutePlanDto),
        (status = 404, description = "Not found")
    )
)]
async fn get_route_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RoutePlanDto>, StatusCode> {
    match state.solver.get_job(&id) {
        Some(job) => {
            let guard = job.read();
            Ok(Json(RoutePlanDto::from_plan(
                &guard.plan,
                Some(guard.status),
            )))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Status response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    /// Current score.
    pub score: Option<String>,
    /// Solver status.
    pub solver_status: String,
}

/// GET /route-plans/{id}/status - Get route plan status only.
#[utoipa::path(
    get,
    path = "/route-plans/{id}/status",
    params(("id" = String, Path, description = "Route plan ID")),
    responses(
        (status = 200, description = "Status retrieved", body = StatusResponse),
        (status = 404, description = "Not found")
    )
)]
async fn get_route_plan_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.solver.get_job(&id) {
        Some(job) => {
            let guard = job.read();
            Ok(Json(StatusResponse {
                score: guard.plan.score.map(|s| format!("{}", s)),
                solver_status: guard.status.as_str().to_string(),
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /route-plans/{id} - Stop solving and get final solution.
#[utoipa::path(
    delete,
    path = "/route-plans/{id}",
    params(("id" = String, Path, description = "Route plan ID")),
    responses(
        (status = 200, description = "Solving stopped", body = RoutePlanDto),
        (status = 404, description = "Not found")
    )
)]
async fn stop_solving(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RoutePlanDto>, StatusCode> {
    state.solver.stop_solving(&id);
    match state.solver.remove_job(&id) {
        Some(job) => {
            let guard = job.read();
            Ok(Json(RoutePlanDto::from_plan(
                &guard.plan,
                Some(SolverStatus::NotSolving),
            )))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Geometry response with encoded polylines for map rendering.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeometryResponse {
    /// Encoded route segments per vehicle.
    pub segments: Vec<EncodedSegment>,
}

/// GET /route-plans/{id}/geometry - Get encoded polylines for routes.
#[utoipa::path(
    get,
    path = "/route-plans/{id}/geometry",
    params(("id" = String, Path, description = "Route plan ID")),
    responses(
        (status = 200, description = "Geometry retrieved", body = GeometryResponse),
        (status = 404, description = "Not found")
    )
)]
async fn get_route_geometry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GeometryResponse>, StatusCode> {
    match state.solver.get_job(&id) {
        Some(job) => {
            let guard = job.read();
            let segments = encode_routes(&guard.plan);
            Ok(Json(GeometryResponse { segments }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// Score Analysis
// ============================================================================

/// Match analysis for a constraint violation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MatchAnalysisDto {
    /// Constraint name.
    pub name: String,
    /// Score impact of this match.
    pub score: String,
    /// Description of the match.
    pub justification: String,
}

/// Constraint analysis showing all matches.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConstraintAnalysisDto {
    /// Constraint name.
    pub name: String,
    /// Constraint weight (score per violation).
    pub weight: String,
    /// Total score from this constraint.
    pub score: String,
    /// Individual matches.
    pub matches: Vec<MatchAnalysisDto>,
}

/// Response from score analysis endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyzeResponse {
    /// Per-constraint breakdown.
    pub constraints: Vec<ConstraintAnalysisDto>,
}

/// PUT /route-plans/analyze - Analyze constraint violations.
#[utoipa::path(
    put,
    path = "/route-plans/analyze",
    request_body = RoutePlanDto,
    responses((status = 200, description = "Constraint analysis", body = AnalyzeResponse))
)]
async fn analyze_route_plan(Json(dto): Json<RoutePlanDto>) -> Json<AnalyzeResponse> {
    use crate::constraints::{VehicleCapacityConstraint, TimeWindowConstraint, MinimizeTravelTimeConstraint};
    use solverforge::IncrementalConstraint;

    let plan = dto.to_domain();

    // Evaluate each constraint
    let cap = VehicleCapacityConstraint::new();
    let tw = TimeWindowConstraint::new();
    let travel = MinimizeTravelTimeConstraint::new();

    let cap_score = cap.evaluate(&plan);
    let tw_score = tw.evaluate(&plan);
    let travel_score = travel.evaluate(&plan);

    // Build detailed matches for capacity constraint
    let cap_matches: Vec<MatchAnalysisDto> = plan.vehicles.iter()
        .filter(|v| v.total_demand(&plan) > v.capacity)
        .map(|v| {
            let excess = v.total_demand(&plan) - v.capacity;
            MatchAnalysisDto {
                name: "Vehicle capacity".to_string(),
                score: format!("{}hard/0soft", -excess),
                justification: format!("{} is over capacity by {} (demand {} > capacity {})",
                    v.name, excess, v.total_demand(&plan), v.capacity),
            }
        })
        .collect();

    // Build detailed matches for time window constraint
    let mut tw_matches: Vec<MatchAnalysisDto> = Vec::new();
    for vehicle in &plan.vehicles {
        let timings = plan.calculate_route_times(vehicle);
        for timing in &timings {
            if let Some(visit) = plan.get_visit(timing.visit_idx) {
                if timing.departure > visit.max_end_time {
                    let late_secs = timing.departure - visit.max_end_time;
                    let late_mins = (late_secs + 59) / 60;
                    tw_matches.push(MatchAnalysisDto {
                        name: "Service finished after max end time".to_string(),
                        score: format!("{}hard/0soft", -late_mins),
                        justification: format!("{} finishes {} mins late (ends at {}, max {})",
                            visit.name, late_mins,
                            seconds_to_iso(timing.departure),
                            seconds_to_iso(visit.max_end_time)),
                    });
                }
            }
        }
    }

    // Build matches for travel time
    let travel_matches: Vec<MatchAnalysisDto> = plan.vehicles.iter()
        .filter(|v| !v.visits.is_empty())
        .map(|v| {
            let time = plan.total_driving_time(v);
            MatchAnalysisDto {
                name: "Minimize travel time".to_string(),
                score: format!("0hard/{}soft", -time),
                justification: format!("{} drives {} seconds", v.name, time),
            }
        })
        .collect();

    let constraints = vec![
        ConstraintAnalysisDto {
            name: "Vehicle capacity".to_string(),
            weight: "1hard/0soft".to_string(),
            score: format!("{}", cap_score),
            matches: cap_matches,
        },
        ConstraintAnalysisDto {
            name: "Service finished after max end time".to_string(),
            weight: "1hard/0soft".to_string(),
            score: format!("{}", tw_score),
            matches: tw_matches,
        },
        ConstraintAnalysisDto {
            name: "Minimize travel time".to_string(),
            weight: "0hard/1soft".to_string(),
            score: format!("{}", travel_score),
            matches: travel_matches,
        },
    ];

    Json(AnalyzeResponse { constraints })
}

// ============================================================================
// Recommendation
// ============================================================================

/// Recommended assignment for a visit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VehicleRecommendation {
    /// Vehicle ID to assign to.
    pub vehicle_id: String,
    /// Position in vehicle's route.
    pub index: usize,
}

/// Recommendation response with score impact.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedAssignment {
    /// The recommendation.
    pub proposition: VehicleRecommendation,
    /// Score difference if applied.
    pub score_diff: String,
}

/// Request for visit recommendations.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationRequest {
    /// Current solution.
    pub solution: RoutePlanDto,
    /// Visit ID to find recommendations for.
    pub visit_id: String,
}

/// Request to apply a recommendation.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRecommendationRequest {
    /// Current solution.
    pub solution: RoutePlanDto,
    /// Visit ID to assign.
    pub visit_id: String,
    /// Vehicle ID to assign to.
    pub vehicle_id: String,
    /// Position in vehicle's route.
    pub index: usize,
}

/// POST /route-plans/recommendation - Get recommendations for assigning a visit.
#[utoipa::path(
    post,
    path = "/route-plans/recommendation",
    request_body = RecommendationRequest,
    responses((status = 200, description = "Recommendations", body = Vec<RecommendedAssignment>))
)]
async fn recommend_assignment(Json(request): Json<RecommendationRequest>) -> Json<Vec<RecommendedAssignment>> {
    use crate::constraints::create_constraints;
    use solverforge::ConstraintSet;

    let mut plan = request.solution.to_domain();
    let constraints = create_constraints();

    // Find the visit index by ID
    let visit_id_num: usize = request.visit_id.trim_start_matches('v').parse().unwrap_or(usize::MAX);
    if visit_id_num >= plan.visits.len() {
        return Json(vec![]);
    }

    // Remove visit from any current assignment
    for vehicle in &mut plan.vehicles {
        vehicle.visits.retain(|&v| v != visit_id_num);
    }
    plan.finalize();

    // Get baseline score
    let baseline = constraints.evaluate_all(&plan);

    // Try inserting at each position in each vehicle
    let mut recommendations: Vec<(RecommendedAssignment, HardSoftScore)> = Vec::new();

    for (v_idx, vehicle) in plan.vehicles.iter().enumerate() {
        for insert_pos in 0..=vehicle.visits.len() {
            // Clone and insert
            let mut test_plan = plan.clone();
            test_plan.vehicles[v_idx].visits.insert(insert_pos, visit_id_num);
            test_plan.finalize();

            let new_score = constraints.evaluate_all(&test_plan);
            let diff = new_score - baseline;

            recommendations.push((
                RecommendedAssignment {
                    proposition: VehicleRecommendation {
                        vehicle_id: vehicle.id.to_string(),
                        index: insert_pos,
                    },
                    score_diff: format!("{}", diff),
                },
                diff,
            ));
        }
    }

    // Sort by score (best first) and take top 5
    recommendations.sort_by(|a, b| b.1.cmp(&a.1));
    let top5: Vec<RecommendedAssignment> = recommendations.into_iter().take(5).map(|(r, _)| r).collect();

    Json(top5)
}

/// POST /route-plans/recommendation/apply - Apply a recommendation.
#[utoipa::path(
    post,
    path = "/route-plans/recommendation/apply",
    request_body = ApplyRecommendationRequest,
    responses((status = 200, description = "Updated solution", body = RoutePlanDto))
)]
async fn apply_recommendation(Json(request): Json<ApplyRecommendationRequest>) -> Json<RoutePlanDto> {
    let mut plan = request.solution.to_domain();

    // Find the visit index by ID
    let visit_id_num: usize = request.visit_id.trim_start_matches('v').parse().unwrap_or(usize::MAX);
    let vehicle_id_num: usize = request.vehicle_id.parse().unwrap_or(usize::MAX);

    // Remove visit from any current assignment
    for vehicle in &mut plan.vehicles {
        vehicle.visits.retain(|&v| v != visit_id_num);
    }

    // Insert at specified position
    if let Some(vehicle) = plan.vehicles.iter_mut().find(|v| v.id == vehicle_id_num) {
        let insert_idx = request.index.min(vehicle.visits.len());
        vehicle.visits.insert(insert_idx, visit_id_num);
    }

    plan.finalize();

    // Recalculate score
    use crate::constraints::create_constraints;
    use solverforge::ConstraintSet;
    let constraints = create_constraints();
    plan.score = Some(constraints.evaluate_all(&plan));

    Json(RoutePlanDto::from_plan(&plan, None))
}

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        info,
        list_demo_data,
        get_demo_data,
        create_route_plan,
        list_route_plans,
        get_route_plan,
        get_route_plan_status,
        stop_solving,
        get_route_geometry,
        analyze_route_plan,
        recommend_assignment,
        apply_recommendation,
    ),
    components(schemas(
        HealthResponse,
        InfoResponse,
        VisitDto,
        VehicleDto,
        RoutePlanDto,
        TerminationConfigDto,
        StatusResponse,
        GeometryResponse,
        MatchAnalysisDto,
        ConstraintAnalysisDto,
        AnalyzeResponse,
        VehicleRecommendation,
        RecommendedAssignment,
        RecommendationRequest,
        ApplyRecommendationRequest,
    ))
)]
struct ApiDoc;
