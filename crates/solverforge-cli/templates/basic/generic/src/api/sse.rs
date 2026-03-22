use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::routes::AppState;

pub async fn events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    let rx = state.solver.subscribe(&id).ok_or(StatusCode::NOT_FOUND)?;

    let bootstrap_json = state.solver.sse_snapshot(&id).unwrap_or_else(||
        r#"{"solverStatus":"SOLVING"}"#.to_string()
    );
    let bootstrap = tokio_stream::iter(std::iter::once(Ok::<_, std::convert::Infallible>(
        format!("data: {}\n\n", bootstrap_json).into_bytes(),
    )));

    let live = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(json) => Some(Ok::<_, std::convert::Infallible>(
            format!("data: {}\n\n", json).into_bytes(),
        )),
        Err(_) => None, // Lagged — skip missed messages
    });

    let stream = bootstrap.chain(live);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(stream))
        .unwrap())
}
