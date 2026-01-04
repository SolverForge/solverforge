//! Employee Scheduling Quickstart for SolverForge
//!
//! This example demonstrates how to build a constraint-based employee
//! scheduling application using SolverForge with an Axum REST API.
//!
//! Run with: cargo run -p employee-scheduling
//! Then open: http://localhost:7860

use employee_scheduling::{api, console};

use owo_colors::OwoColorize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing (logs from employee_scheduling at INFO level)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("employee_scheduling=info".parse().unwrap()),
        )
        .init();

    // Print colorful banner
    console::print_banner();

    // Create shared application state
    let state = Arc::new(api::AppState::new());

    // CORS for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Determine static files path (works from workspace root or example dir)
    let static_path = if PathBuf::from("examples/employee-scheduling/static").exists() {
        "examples/employee-scheduling/static"
    } else {
        "static"
    };

    // Build router
    let app = api::router(state)
        .fallback_service(ServeDir::new(static_path))
        .layer(cors);

    // Bind and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], 7860));
    println!(
        "{} Server listening on {}",
        "▸".bright_green(),
        format!("http://{}", addr).bright_cyan().underline()
    );
    println!(
        "{} Open {} in your browser\n",
        "▸".bright_green(),
        "http://localhost:7860".bright_cyan().underline()
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
