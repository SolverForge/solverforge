//! Employee Scheduling Quickstart for SolverForge
//!
//! This example demonstrates how to build a constraint-based employee
//! scheduling application using SolverForge with an Axum REST API.
//!
//! Run with: cargo run -p employee-scheduling
//! Then open: http://localhost:8080

use employee_scheduling::{api, console};

use owo_colors::OwoColorize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // Print colorful banner
    console::print_banner();

    // Create shared application state
    let state = Arc::new(api::AppState::new());

    // CORS for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = api::router(state)
        .fallback_service(ServeDir::new("static"))
        .layer(cors);

    // Bind and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!(
        "{} Server listening on {}",
        "▸".bright_green(),
        format!("http://{}", addr).bright_cyan().underline()
    );
    println!(
        "{} Open {} in your browser\n",
        "▸".bright_green(),
        "http://localhost:8080".bright_cyan().underline()
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
