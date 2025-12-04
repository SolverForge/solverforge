use crate::error::{SolverForgeError, SolverForgeResult};
use crate::solver::{
    AsyncSolveResponse, SolveHandle, SolveRequest, SolveResponse, SolveState, SolveStatus,
};
use std::time::Duration;

pub trait SolverService: Send + Sync {
    fn solve(&self, request: &SolveRequest) -> SolverForgeResult<SolveResponse>;

    fn solve_async(&self, request: &SolveRequest) -> SolverForgeResult<SolveHandle>;

    fn get_status(&self, handle: &SolveHandle) -> SolverForgeResult<SolveStatus>;

    fn get_best_solution(&self, handle: &SolveHandle) -> SolverForgeResult<Option<SolveResponse>>;

    fn stop(&self, handle: &SolveHandle) -> SolverForgeResult<()>;

    fn is_available(&self) -> bool;
}

pub struct HttpSolverService {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl HttpSolverService {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.into(),
            client,
        }
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.into(),
            client,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> SolverForgeResult<R> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .map_err(|e| SolverForgeError::Http(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(SolverForgeError::Solver(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response.json().map_err(|e| {
            SolverForgeError::Serialization(format!("Failed to parse response: {}", e))
        })
    }

    fn get_json<R: serde::de::DeserializeOwned>(&self, path: &str) -> SolverForgeResult<R> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| SolverForgeError::Http(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(SolverForgeError::Solver(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response.json().map_err(|e| {
            SolverForgeError::Serialization(format!("Failed to parse response: {}", e))
        })
    }

    fn post_empty(&self, path: &str) -> SolverForgeResult<()> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .send()
            .map_err(|e| SolverForgeError::Http(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(SolverForgeError::Solver(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

impl SolverService for HttpSolverService {
    fn solve(&self, request: &SolveRequest) -> SolverForgeResult<SolveResponse> {
        self.post_json("/solve", request)
    }

    fn solve_async(&self, request: &SolveRequest) -> SolverForgeResult<SolveHandle> {
        let response: AsyncSolveResponse = self.post_json("/solve/async", request)?;
        Ok(SolveHandle::new(response.solve_id))
    }

    fn get_status(&self, handle: &SolveHandle) -> SolverForgeResult<SolveStatus> {
        self.get_json(&format!("/solve/{}/status", handle.id))
    }

    fn get_best_solution(&self, handle: &SolveHandle) -> SolverForgeResult<Option<SolveResponse>> {
        let status = self.get_status(handle)?;
        if status.state == SolveState::Pending {
            return Ok(None);
        }
        let response: SolveResponse = self.get_json(&format!("/solve/{}/best", handle.id))?;
        Ok(Some(response))
    }

    fn stop(&self, handle: &SolveHandle) -> SolverForgeResult<()> {
        self.post_empty(&format!("/solve/{}/stop", handle.id))
    }

    fn is_available(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_solver_service_new() {
        let service = HttpSolverService::new("http://localhost:8080");
        assert_eq!(service.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_http_solver_service_with_timeout() {
        let service =
            HttpSolverService::with_timeout("http://localhost:8080", Duration::from_secs(30));
        assert_eq!(service.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_solve_handle_new() {
        let handle = SolveHandle::new("test-solve-123");
        assert_eq!(handle.id, "test-solve-123");
    }

    #[test]
    fn test_http_solver_service_is_available_when_offline() {
        let service =
            HttpSolverService::with_timeout("http://localhost:19999", Duration::from_millis(100));
        assert!(!service.is_available());
    }
}
