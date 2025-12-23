//! Python bindings for the embedded solver service.
//!
//! Provides automatic lifecycle management for the solver service,
//! eliminating the need for users to manually start the Java service.

use pyo3::prelude::*;
use solverforge_service::{EmbeddedService, ServiceConfig};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Global singleton for the embedded service.
/// This ensures only one service instance runs across the entire Python process.
static GLOBAL_SERVICE: OnceLock<Arc<Mutex<Option<EmbeddedService>>>> = OnceLock::new();

fn get_global_service() -> &'static Arc<Mutex<Option<EmbeddedService>>> {
    GLOBAL_SERVICE.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Python wrapper for ServiceConfig.
#[pyclass(name = "ServiceConfig")]
#[derive(Clone)]
pub struct PyServiceConfig {
    inner: ServiceConfig,
}

#[pymethods]
impl PyServiceConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: ServiceConfig::new(),
        }
    }

    /// Set the port for the service (0 for auto-select).
    fn with_port(&self, port: u16) -> Self {
        Self {
            inner: self.inner.clone().with_port(port),
        }
    }

    /// Set the startup timeout in seconds.
    fn with_startup_timeout_secs(&self, secs: u64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .with_startup_timeout(Duration::from_secs(secs)),
        }
    }

    /// Set the JAVA_HOME path.
    fn with_java_home(&self, path: &str) -> Self {
        Self {
            inner: self.inner.clone().with_java_home(PathBuf::from(path)),
        }
    }

    /// Add a JVM option (e.g., "-Xmx2g").
    fn with_java_opt(&self, opt: &str) -> Self {
        Self {
            inner: self.inner.clone().with_java_opt(opt),
        }
    }

    /// Set the path to the solver service submodule.
    fn with_submodule_dir(&self, path: &str) -> Self {
        Self {
            inner: self.inner.clone().with_submodule_dir(PathBuf::from(path)),
        }
    }

    /// Set the cache directory for the JAR file.
    fn with_cache_dir(&self, path: &str) -> Self {
        Self {
            inner: self.inner.clone().with_cache_dir(PathBuf::from(path)),
        }
    }

    #[getter]
    fn port(&self) -> u16 {
        self.inner.port
    }

    fn __repr__(&self) -> String {
        format!(
            "ServiceConfig(port={}, java_home={:?})",
            self.inner.port, self.inner.java_home
        )
    }
}

impl PyServiceConfig {
    pub fn to_rust(&self) -> ServiceConfig {
        self.inner.clone()
    }
}

/// Python wrapper for EmbeddedService.
///
/// Manages the lifecycle of the solver service Java process.
/// The service is automatically started when created and stopped when dropped.
#[pyclass(name = "EmbeddedService")]
pub struct PyEmbeddedService {
    /// URL of the running service (e.g., "http://localhost:8080")
    url: String,
}

#[pymethods]
impl PyEmbeddedService {
    /// Start a new embedded service with the given configuration.
    ///
    /// If a service is already running from a previous call, this will
    /// return a reference to the existing service.
    #[staticmethod]
    #[pyo3(signature = (config=None))]
    fn start(config: Option<&PyServiceConfig>) -> PyResult<Self> {
        let service_config = config.map(|c| c.to_rust()).unwrap_or_default();

        let global = get_global_service();
        let mut guard = global
            .lock()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Check if service is already running
        if let Some(ref mut service) = *guard {
            if service.is_running() {
                return Ok(Self { url: service.url() });
            }
        }

        // Start new service
        let service = EmbeddedService::start(service_config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let url = service.url();
        *guard = Some(service);

        Ok(Self { url })
    }

    /// Get the URL of the running service.
    #[getter]
    fn url(&self) -> &str {
        &self.url
    }

    /// Get the port the service is running on.
    #[getter]
    fn port(&self) -> u16 {
        self.url
            .rsplit(':')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Check if the service is currently running.
    fn is_running(&self) -> bool {
        let global = get_global_service();
        if let Ok(mut guard) = global.lock() {
            if let Some(ref mut service) = *guard {
                return service.is_running();
            }
        }
        false
    }

    /// Stop the embedded service.
    fn stop(&mut self) -> PyResult<()> {
        let global = get_global_service();
        let mut guard = global
            .lock()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        if let Some(ref mut service) = *guard {
            service
                .stop()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            *guard = None;
        }

        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "EmbeddedService(url='{}', running={})",
            self.url,
            self.is_running()
        )
    }
}

/// Ensure the solver service is running, starting it if necessary.
///
/// This is the primary entry point for autonomous service management.
/// Call this before creating a SolverFactory to ensure the service is available.
///
/// Returns the URL of the running service.
#[pyfunction]
#[pyo3(signature = (config=None))]
pub fn ensure_service(config: Option<&PyServiceConfig>) -> PyResult<String> {
    let service = PyEmbeddedService::start(config)?;
    Ok(service.url)
}

/// Check if the solver service is currently available at the given URL.
#[pyfunction]
#[pyo3(signature = (url=None))]
pub fn is_service_available(url: Option<&str>) -> bool {
    use solverforge_core::solver::HttpSolverService;
    use solverforge_core::SolverService;

    let url = url.unwrap_or("http://localhost:8080");
    let service = HttpSolverService::new(url);
    service.is_available()
}

/// Get the URL of the currently running embedded service, if any.
#[pyfunction]
pub fn get_service_url() -> Option<String> {
    let global = get_global_service();
    if let Ok(guard) = global.lock() {
        if let Some(ref service) = *guard {
            return Some(service.url());
        }
    }
    None
}

/// Stop the global embedded service if running.
#[pyfunction]
pub fn stop_service() -> PyResult<()> {
    let global = get_global_service();
    let mut guard = global
        .lock()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    if let Some(ref mut service) = *guard {
        service
            .stop()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    }
    *guard = None;

    Ok(())
}

/// Register service classes with the Python module.
pub fn register_service(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyServiceConfig>()?;
    m.add_class::<PyEmbeddedService>()?;
    m.add_function(wrap_pyfunction!(ensure_service, m)?)?;
    m.add_function(wrap_pyfunction!(is_service_available, m)?)?;
    m.add_function(wrap_pyfunction!(get_service_url, m)?)?;
    m.add_function(wrap_pyfunction!(stop_service, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_new() {
        let config = PyServiceConfig::new();
        assert_eq!(config.inner.port, 0);
    }

    #[test]
    fn test_service_config_with_port() {
        let config = PyServiceConfig::new().with_port(8080);
        assert_eq!(config.inner.port, 8080);
    }

    #[test]
    fn test_service_config_with_java_home() {
        let config = PyServiceConfig::new().with_java_home("/usr/lib/jvm/java-24");
        assert_eq!(
            config.inner.java_home,
            Some(PathBuf::from("/usr/lib/jvm/java-24"))
        );
    }

    #[test]
    fn test_service_config_chained() {
        let config = PyServiceConfig::new()
            .with_port(9999)
            .with_startup_timeout_secs(120)
            .with_java_opt("-Xmx4g")
            .with_java_opt("-Xms1g");

        assert_eq!(config.inner.port, 9999);
        assert_eq!(config.inner.startup_timeout, Duration::from_secs(120));
        assert_eq!(config.inner.java_opts.len(), 2);
    }
}
