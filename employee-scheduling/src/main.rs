//! Employee Scheduling Quickstart - Axum Server
//!
//! Run with: cargo run -p employee-scheduling
//! Then open: http://localhost:7860

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use employee_scheduling::api;

#[tokio::main]
async fn main() {
    solverforge::console::init();

    let state = Arc::new(api::AppState::new());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let static_path = if PathBuf::from("examples/employee-scheduling/static").exists() {
        "examples/employee-scheduling/static"
    } else {
        "static"
    };

    let app = api::router(state)
        .fallback_service(ServeDir::new(static_path))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 7860));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Server running at http://localhost:{}", addr.port());
    axum::serve(listener, app).await.unwrap();
}
