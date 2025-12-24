//! Python bindings for solver configuration and execution.
//!
//! Provides Python wrappers for the solver factory, solver, and configuration types.
//!
//! # Example
//!
//! ```python
//! from solverforge import (
//!     SolverConfig, TerminationConfig, SolverFactory,
//!     planning_solution, planning_entity
//! )
//!
//! config = SolverConfig() \
//!     .with_solution_class(Timetable) \
//!     .with_entity_class(Lesson) \
//!     .with_termination(
//!         TerminationConfig().with_spent_limit("PT5M")
//!     )
//!
//! factory = SolverFactory.create(config, constraint_provider)
//! solver = factory.build_solver()
//! solution = solver.solve(problem)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyType;
use solverforge_core::constraints::ConstraintSet;
use solverforge_core::domain::{DomainModel, DomainModelBuilder};
use solverforge_core::solver::{
    DiminishedReturnsConfig, EnvironmentMode, MoveThreadCount, ScoreDto, SolveHandle,
    SolveResponse, SolveState, SolveStatus, SolverConfig, SolverFactory, SolverService,
    TerminationConfig, DEFAULT_SERVICE_URL,
};
use solverforge_core::wasm::{HostFunctionRegistry, PredicateDefinition, WasmModuleBuilder};
use solverforge_core::LanguageBridge;
use std::sync::Arc;

use crate::bridge::PythonBridge;
use crate::decorators::PyConstraintProvider;
use crate::lambda_analyzer::LambdaInfo;
use crate::stream::PyConstraint;

/// Python wrapper for TerminationConfig.
#[pyclass(name = "TerminationConfig")]
#[derive(Clone)]
pub struct PyTerminationConfig {
    inner: TerminationConfig,
}

#[pymethods]
impl PyTerminationConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: TerminationConfig::new(),
        }
    }

    /// Set the spent time limit (ISO-8601 duration, e.g., "PT5M" for 5 minutes).
    fn with_spent_limit(&self, limit: &str) -> Self {
        Self {
            inner: self.inner.clone().with_spent_limit(limit),
        }
    }

    /// Set the unimproved spent time limit.
    fn with_unimproved_spent_limit(&self, limit: &str) -> Self {
        Self {
            inner: self.inner.clone().with_unimproved_spent_limit(limit),
        }
    }

    /// Set the unimproved step count limit.
    fn with_unimproved_step_count(&self, count: u64) -> Self {
        Self {
            inner: self.inner.clone().with_unimproved_step_count(count),
        }
    }

    /// Set the best score limit.
    fn with_best_score_limit(&self, limit: &str) -> Self {
        Self {
            inner: self.inner.clone().with_best_score_limit(limit),
        }
    }

    /// Set whether to terminate when a feasible solution is found.
    fn with_best_score_feasible(&self, feasible: bool) -> Self {
        Self {
            inner: self.inner.clone().with_best_score_feasible(feasible),
        }
    }

    /// Set the step count limit.
    fn with_step_count_limit(&self, count: u64) -> Self {
        Self {
            inner: self.inner.clone().with_step_count_limit(count),
        }
    }

    /// Set the move count limit.
    fn with_move_count_limit(&self, count: u64) -> Self {
        Self {
            inner: self.inner.clone().with_move_count_limit(count),
        }
    }

    /// Set the score calculation count limit.
    fn with_score_calculation_count_limit(&self, count: u64) -> Self {
        Self {
            inner: self.inner.clone().with_score_calculation_count_limit(count),
        }
    }

    #[getter]
    fn spent_limit(&self) -> Option<String> {
        self.inner.spent_limit.clone()
    }

    #[getter]
    fn unimproved_spent_limit(&self) -> Option<String> {
        self.inner.unimproved_spent_limit.clone()
    }

    #[getter]
    fn best_score_limit(&self) -> Option<String> {
        self.inner.best_score_limit.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "TerminationConfig(spent_limit={:?}, unimproved_spent_limit={:?})",
            self.inner.spent_limit, self.inner.unimproved_spent_limit
        )
    }
}

impl PyTerminationConfig {
    pub fn to_rust(&self) -> TerminationConfig {
        self.inner.clone()
    }
}

/// Python wrapper for DiminishedReturnsConfig.
#[pyclass(name = "DiminishedReturnsConfig")]
#[derive(Clone)]
pub struct PyDiminishedReturnsConfig {
    inner: DiminishedReturnsConfig,
}

#[pymethods]
impl PyDiminishedReturnsConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: DiminishedReturnsConfig::new(),
        }
    }

    fn with_minimum_improvement_ratio(&self, ratio: &str) -> Self {
        Self {
            inner: self.inner.clone().with_minimum_improvement_ratio(ratio),
        }
    }

    fn with_slow_improvement_limit(&self, limit: &str) -> Self {
        Self {
            inner: self.inner.clone().with_slow_improvement_limit(limit),
        }
    }

    fn with_slow_improvement_spent_limit(&self, limit: &str) -> Self {
        Self {
            inner: self.inner.clone().with_slow_improvement_spent_limit(limit),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "DiminishedReturnsConfig(minimum_improvement_ratio={:?})",
            self.inner.minimum_improvement_ratio
        )
    }
}

/// Environment mode for solver behavior.
#[pyclass(name = "EnvironmentMode")]
#[derive(Clone)]
pub struct PyEnvironmentMode {
    inner: EnvironmentMode,
}

#[pymethods]
impl PyEnvironmentMode {
    /// Non-reproducible mode for maximum performance.
    #[staticmethod]
    fn non_reproducible() -> Self {
        Self {
            inner: EnvironmentMode::NonReproducible,
        }
    }

    /// Reproducible mode (default) - same input produces same output.
    #[staticmethod]
    fn reproducible() -> Self {
        Self {
            inner: EnvironmentMode::Reproducible,
        }
    }

    /// No assertion mode - faster but less safe.
    #[staticmethod]
    fn no_assert() -> Self {
        Self {
            inner: EnvironmentMode::NoAssert,
        }
    }

    /// Phase-level assertions.
    #[staticmethod]
    fn phase_assert() -> Self {
        Self {
            inner: EnvironmentMode::PhaseAssert,
        }
    }

    /// Step-level assertions.
    #[staticmethod]
    fn step_assert() -> Self {
        Self {
            inner: EnvironmentMode::StepAssert,
        }
    }

    /// Full assertions (slowest but safest).
    #[staticmethod]
    fn full_assert() -> Self {
        Self {
            inner: EnvironmentMode::FullAssert,
        }
    }

    /// Full assertions with tracking.
    #[staticmethod]
    fn tracked_full_assert() -> Self {
        Self {
            inner: EnvironmentMode::TrackedFullAssert,
        }
    }

    fn is_asserted(&self) -> bool {
        self.inner.is_asserted()
    }

    fn is_reproducible(&self) -> bool {
        self.inner.is_reproducible()
    }

    fn is_tracked(&self) -> bool {
        self.inner.is_tracked()
    }

    fn __repr__(&self) -> String {
        format!("EnvironmentMode.{:?}", self.inner)
    }
}

impl PyEnvironmentMode {
    pub fn to_rust(&self) -> EnvironmentMode {
        self.inner
    }
}

/// Move thread count configuration.
#[pyclass(name = "MoveThreadCount")]
#[derive(Clone)]
pub struct PyMoveThreadCount {
    inner: MoveThreadCount,
}

#[pymethods]
impl PyMoveThreadCount {
    /// Automatically determine thread count.
    #[staticmethod]
    fn auto() -> Self {
        Self {
            inner: MoveThreadCount::Auto,
        }
    }

    /// Single-threaded (no parallelism).
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: MoveThreadCount::None,
        }
    }

    /// Fixed thread count.
    #[staticmethod]
    fn count(n: u32) -> Self {
        Self {
            inner: MoveThreadCount::Count(n),
        }
    }

    fn is_parallel(&self) -> bool {
        self.inner.is_parallel()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MoveThreadCount::Auto => "MoveThreadCount.auto()".to_string(),
            MoveThreadCount::None => "MoveThreadCount.none()".to_string(),
            MoveThreadCount::Count(n) => format!("MoveThreadCount.count({})", n),
        }
    }
}

impl PyMoveThreadCount {
    pub fn to_rust(&self) -> MoveThreadCount {
        self.inner.clone()
    }
}

/// Python wrapper for SolverConfig.
#[pyclass(name = "SolverConfig")]
pub struct PySolverConfig {
    inner: SolverConfig,
    /// Stored Python class object for solution class (for domain model extraction).
    solution_class_obj: Option<Py<PyType>>,
    /// Stored Python class objects for entity classes.
    entity_class_objs: Vec<Py<PyType>>,
}

impl Clone for PySolverConfig {
    fn clone(&self) -> Self {
        Python::attach(|py| Self {
            inner: self.inner.clone(),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        })
    }
}

#[pymethods]
impl PySolverConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: SolverConfig::new(),
            solution_class_obj: None,
            entity_class_objs: Vec::new(),
        }
    }

    /// Set the solution class.
    fn with_solution_class(&self, py: Python<'_>, cls: &Bound<'_, PyType>) -> PyResult<Self> {
        let class_name: String = cls.getattr("__name__")?.extract()?;
        Ok(Self {
            inner: self.inner.clone().with_solution_class(class_name),
            solution_class_obj: Some(cls.clone().unbind()),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        })
    }

    /// Add an entity class.
    fn with_entity_class(&self, py: Python<'_>, cls: &Bound<'_, PyType>) -> PyResult<Self> {
        let class_name: String = cls.getattr("__name__")?.extract()?;
        let mut entity_objs: Vec<Py<PyType>> = self
            .entity_class_objs
            .iter()
            .map(|c| c.clone_ref(py))
            .collect();
        entity_objs.push(cls.clone().unbind());
        Ok(Self {
            inner: self.inner.clone().with_entity_class(class_name),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: entity_objs,
        })
    }

    /// Set the environment mode.
    fn with_environment_mode(&self, py: Python<'_>, mode: &PyEnvironmentMode) -> Self {
        Self {
            inner: self.inner.clone().with_environment_mode(mode.to_rust()),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        }
    }

    /// Set the random seed for reproducibility.
    fn with_random_seed(&self, py: Python<'_>, seed: u64) -> Self {
        Self {
            inner: self.inner.clone().with_random_seed(seed),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        }
    }

    /// Set the move thread count.
    fn with_move_thread_count(&self, py: Python<'_>, count: &PyMoveThreadCount) -> Self {
        Self {
            inner: self.inner.clone().with_move_thread_count(count.to_rust()),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        }
    }

    /// Set the termination configuration.
    fn with_termination(&self, py: Python<'_>, termination: &PyTerminationConfig) -> Self {
        Self {
            inner: self.inner.clone().with_termination(termination.to_rust()),
            solution_class_obj: self.solution_class_obj.as_ref().map(|c| c.clone_ref(py)),
            entity_class_objs: self
                .entity_class_objs
                .iter()
                .map(|c| c.clone_ref(py))
                .collect(),
        }
    }

    #[getter]
    fn solution_class(&self) -> Option<String> {
        self.inner.solution_class.clone()
    }

    #[getter]
    fn entity_class_list(&self) -> Vec<String> {
        self.inner.entity_class_list.clone()
    }

    #[getter]
    fn random_seed(&self) -> Option<u64> {
        self.inner.random_seed
    }

    fn __repr__(&self) -> String {
        format!(
            "SolverConfig(solution_class={:?}, entity_classes={:?})",
            self.inner.solution_class, self.inner.entity_class_list
        )
    }
}

impl PySolverConfig {
    pub fn to_rust(&self) -> SolverConfig {
        self.inner.clone()
    }

    /// Get the solution class object if set.
    pub fn solution_class_obj(&self) -> Option<&Py<PyType>> {
        self.solution_class_obj.as_ref()
    }

    /// Get the entity class objects.
    pub fn entity_class_objs(&self) -> &[Py<PyType>] {
        &self.entity_class_objs
    }
}

/// Python wrapper for SolveHandle.
#[pyclass(name = "SolveHandle")]
#[derive(Clone)]
pub struct PySolveHandle {
    inner: SolveHandle,
}

#[pymethods]
impl PySolveHandle {
    /// Get the solve handle ID.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    fn __repr__(&self) -> String {
        format!("SolveHandle(id='{}')", self.inner.id)
    }
}

impl PySolveHandle {
    pub fn from_rust(handle: SolveHandle) -> Self {
        Self { inner: handle }
    }

    pub fn to_rust(&self) -> &SolveHandle {
        &self.inner
    }
}

/// Python wrapper for SolveState.
#[pyclass(name = "SolveState")]
#[derive(Clone)]
pub struct PySolveState {
    inner: SolveState,
}

#[pymethods]
impl PySolveState {
    /// Whether the solve is in a terminal state.
    fn is_terminal(&self) -> bool {
        self.inner.is_terminal()
    }

    /// Whether the solve is currently running.
    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn __repr__(&self) -> String {
        format!("SolveState.{:?}", self.inner)
    }
}

impl PySolveState {
    pub fn from_rust(state: SolveState) -> Self {
        Self { inner: state }
    }
}

/// Python wrapper for ScoreDto.
#[pyclass(name = "ScoreDto")]
#[derive(Clone)]
pub struct PyScoreDto {
    inner: ScoreDto,
}

#[pymethods]
impl PyScoreDto {
    #[getter]
    fn score_string(&self) -> &str {
        &self.inner.score_string
    }

    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score
    }

    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score
    }

    #[getter]
    fn medium_score(&self) -> Option<i64> {
        self.inner.medium_score
    }

    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible
    }

    fn __repr__(&self) -> String {
        format!(
            "ScoreDto(score='{}', feasible={})",
            self.inner.score_string, self.inner.is_feasible
        )
    }
}

impl PyScoreDto {
    pub fn from_rust(score: ScoreDto) -> Self {
        Self { inner: score }
    }
}

/// Python wrapper for SolveStatus.
#[pyclass(name = "SolveStatus")]
#[derive(Clone)]
pub struct PySolveStatus {
    inner: SolveStatus,
}

#[pymethods]
impl PySolveStatus {
    /// The current state of the solve.
    #[getter]
    fn state(&self) -> PySolveState {
        PySolveState::from_rust(self.inner.state)
    }

    /// Time spent solving in milliseconds.
    #[getter]
    fn time_spent_ms(&self) -> u64 {
        self.inner.time_spent_ms
    }

    /// The current best score, if available.
    #[getter]
    fn best_score(&self) -> Option<PyScoreDto> {
        self.inner.best_score.clone().map(PyScoreDto::from_rust)
    }

    /// Error message if the solve failed.
    #[getter]
    fn error(&self) -> Option<String> {
        self.inner.error.clone()
    }

    /// Whether the solver is still solving.
    fn is_solving(&self) -> bool {
        self.inner.state.is_running()
    }

    /// Whether the solver has terminated.
    fn is_terminated(&self) -> bool {
        self.inner.state.is_terminal()
    }

    fn __repr__(&self) -> String {
        format!(
            "SolveStatus(state={:?}, time_spent_ms={}, best_score={:?})",
            self.inner.state,
            self.inner.time_spent_ms,
            self.inner.best_score.as_ref().map(|s| &s.score_string)
        )
    }
}

impl PySolveStatus {
    pub fn from_rust(status: SolveStatus) -> Self {
        Self { inner: status }
    }
}

/// Python wrapper for SolveResponse.
#[pyclass(name = "SolveResponse")]
pub struct PySolveResponse {
    inner: SolveResponse,
    /// Cached Python solution object.
    solution_obj: Option<Py<PyAny>>,
}

#[pymethods]
impl PySolveResponse {
    /// Get the solution object (if deserialized from Python).
    #[getter]
    fn solution(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.solution_obj.as_ref().map(|s| s.clone_ref(py))
    }

    /// Get the raw solution JSON string.
    #[getter]
    fn solution_json(&self) -> &str {
        &self.inner.solution
    }

    /// Get the score as a string.
    #[getter]
    fn score(&self) -> &str {
        &self.inner.score
    }

    /// Whether the solution is feasible (based on score parsing).
    fn is_feasible(&self) -> bool {
        // Parse the score string to determine feasibility
        // For HardSoftScore format like "0hard/-10soft", check if hard is 0
        if self.inner.score.contains("hard") {
            self.inner
                .score
                .split("hard")
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .map(|h| h >= 0)
                .unwrap_or(false)
        } else {
            // For SimpleScore, any score is feasible
            true
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SolveResponse(score='{}', feasible={})",
            self.inner.score,
            self.is_feasible()
        )
    }
}

impl PySolveResponse {
    pub fn from_rust(response: SolveResponse, solution_obj: Option<Py<PyAny>>) -> Self {
        Self {
            inner: response,
            solution_obj,
        }
    }
}

/// Python wrapper for SolverFactory.
#[pyclass(name = "SolverFactory")]
pub struct PySolverFactory {
    config: SolverConfig,
    constraints: ConstraintSet,
    domain_model: DomainModel,
    service_url: String,
    wasm_module: String,
}

#[pymethods]
impl PySolverFactory {
    /// Create a new solver factory.
    ///
    /// # Arguments
    /// * `config` - The solver configuration
    /// * `constraint_provider` - The constraint provider function/decorator
    /// * `service_url` - Optional URL for the solver service (auto-starts embedded if not specified and not available)
    #[staticmethod]
    #[pyo3(signature = (config, constraint_provider, service_url=None))]
    fn create(
        py: Python<'_>,
        config: &PySolverConfig,
        constraint_provider: &PyConstraintProvider,
        service_url: Option<&str>,
    ) -> PyResult<Self> {
        // Get constraints from the provider
        let constraints = constraint_provider.get_constraints(py)?;
        let (constraint_set, predicates) = build_constraint_set(&constraints);

        // Build domain model from decorated classes
        let domain_model = if let Some(solution_cls) = config.solution_class_obj() {
            let solution_bound = solution_cls.bind(py);
            let entity_bounds: Vec<Bound<'_, PyType>> = config
                .entity_class_objs()
                .iter()
                .map(|c| c.bind(py).clone())
                .collect();
            let py_domain_model =
                crate::decorators::build_domain_model(solution_bound, entity_bounds)?;
            py_domain_model.to_rust()
        } else {
            // Fallback: build basic domain model from class names
            let mut builder = DomainModelBuilder::new();
            if let Some(solution_class) = &config.to_rust().solution_class {
                builder = builder.solution_class(solution_class);
            }
            for entity_class in &config.to_rust().entity_class_list {
                builder = builder.entity_class(entity_class);
            }
            builder.build()
        };

        // Determine service URL, auto-starting embedded service if needed
        let final_url = {
            use solverforge_core::solver::HttpSolverService;
            use solverforge_core::SolverService;

            let requested_url = service_url.unwrap_or(DEFAULT_SERVICE_URL);
            let service = HttpSolverService::new(requested_url);

            if service.is_available() {
                // Use existing service
                requested_url.to_string()
            } else {
                // Auto-start embedded service
                log::info!(
                    "Solver service not available at {}, starting embedded service...",
                    requested_url
                );
                let embedded_url = crate::service::ensure_service(None)?;
                log::info!("Embedded service started at {}", embedded_url);
                embedded_url
            }
        };

        // Generate WASM module from domain model with host functions and predicates
        let host_registry = HostFunctionRegistry::with_standard_functions();
        let mut wasm_builder = WasmModuleBuilder::new()
            .with_domain_model(domain_model.clone())
            .with_host_functions(host_registry);

        // Add all predicates from constraints to the WASM module
        for lambda in &predicates {
            if let Some(predicate_def) = lambda_to_predicate(lambda) {
                wasm_builder = wasm_builder.add_predicate(predicate_def);
            }
        }

        let wasm_module = wasm_builder.build_base64().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to generate WASM module: {}",
                e
            ))
        })?;

        Ok(Self {
            config: config.to_rust(),
            constraints: constraint_set,
            domain_model,
            service_url: final_url,
            wasm_module,
        })
    }

    /// Build a solver instance.
    fn build_solver(&self, _py: Python<'_>) -> PySolver {
        let bridge = Arc::new(PythonBridge::new());
        PySolver {
            config: self.config.clone(),
            constraints: self.constraints.clone(),
            domain_model: self.domain_model.clone(),
            service_url: self.service_url.clone(),
            wasm_module: self.wasm_module.clone(),
            bridge,
        }
    }

    /// Check if the solver service is available.
    fn is_service_available(&self) -> bool {
        use solverforge_core::solver::HttpSolverService;
        let service = HttpSolverService::new(&self.service_url);
        service.is_available()
    }

    #[getter]
    fn config(&self) -> PySolverConfig {
        PySolverConfig {
            inner: self.config.clone(),
            solution_class_obj: None,
            entity_class_objs: Vec::new(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SolverFactory(solution_class={:?}, service_url='{}')",
            self.config.solution_class, self.service_url
        )
    }
}

impl PySolverFactory {
    /// Create a SolutionManager from this factory.
    /// Used internally by PySolutionManager::create.
    pub fn create_solution_manager(
        &self,
    ) -> solverforge_core::analysis::SolutionManager<PythonBridge> {
        use solverforge_core::analysis::SolutionManager;
        use solverforge_core::solver::HttpSolverService;

        let service = Arc::new(HttpSolverService::new(&self.service_url));

        SolutionManager::new(
            self.config.clone(),
            service,
            self.domain_model.clone(),
            self.constraints.clone(),
            self.wasm_module.clone(),
        )
    }

    /// Get the config (for internal use).
    pub fn get_config(&self) -> &SolverConfig {
        &self.config
    }

    /// Get the constraints (for internal use).
    pub fn get_constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    /// Get the domain model (for internal use).
    pub fn get_domain_model(&self) -> &DomainModel {
        &self.domain_model
    }

    /// Get the service URL (for internal use).
    pub fn get_service_url(&self) -> &str {
        &self.service_url
    }

    /// Get the WASM module (for internal use).
    pub fn get_wasm_module(&self) -> &str {
        &self.wasm_module
    }
}

/// Python wrapper for Solver.
#[pyclass(name = "Solver")]
pub struct PySolver {
    config: SolverConfig,
    constraints: ConstraintSet,
    domain_model: DomainModel,
    service_url: String,
    wasm_module: String,
    bridge: Arc<PythonBridge>,
}

#[pymethods]
impl PySolver {
    /// Solve the problem synchronously.
    ///
    /// # Arguments
    /// * `problem` - The problem instance to solve
    ///
    /// # Returns
    /// The solution with the best score found.
    fn solve(&self, py: Python<'_>, problem: Py<PyAny>) -> PyResult<PySolveResponse> {
        // Register the problem object with the bridge
        let handle = self.bridge.register_object(problem.clone_ref(py));

        // Create the core solver using the pre-built domain model
        let factory = SolverFactory::<PythonBridge>::create(
            self.config.clone(),
            &self.service_url,
            self.domain_model.clone(),
            self.constraints.clone(),
            self.wasm_module.clone(),
        );
        let solver = factory.build_solver(self.bridge.clone());

        // Solve
        let response = solver
            .solve(handle)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Deserialize the solution back to Python
        let solution_class = self.config.solution_class.as_deref().unwrap_or("Solution");
        let solution_obj = self
            .bridge
            .deserialize_object(&response.solution, solution_class)
            .ok()
            .and_then(|h| self.bridge.get_py_object(h));

        Ok(PySolveResponse::from_rust(response, solution_obj))
    }

    /// Start solving asynchronously.
    ///
    /// # Arguments
    /// * `problem` - The problem instance to solve
    ///
    /// # Returns
    /// A handle that can be used to check status and get results.
    fn solve_async(&self, py: Python<'_>, problem: Py<PyAny>) -> PyResult<PySolveHandle> {
        let handle = self.bridge.register_object(problem.clone_ref(py));

        let factory = SolverFactory::<PythonBridge>::create(
            self.config.clone(),
            &self.service_url,
            self.domain_model.clone(),
            self.constraints.clone(),
            self.wasm_module.clone(),
        );
        let solver = factory.build_solver(self.bridge.clone());

        let solve_handle = solver
            .solve_async(handle)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PySolveHandle::from_rust(solve_handle))
    }

    /// Get the status of an async solve.
    fn get_status(&self, handle: &PySolveHandle) -> PyResult<PySolveStatus> {
        let factory = SolverFactory::<PythonBridge>::create(
            self.config.clone(),
            &self.service_url,
            self.domain_model.clone(),
            self.constraints.clone(),
            self.wasm_module.clone(),
        );
        let solver = factory.build_solver(self.bridge.clone());

        let status = solver
            .get_status(handle.to_rust())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PySolveStatus::from_rust(status))
    }

    /// Get the best solution from an async solve.
    fn get_best_solution(
        &self,
        _py: Python<'_>,
        handle: &PySolveHandle,
    ) -> PyResult<Option<PySolveResponse>> {
        let factory = SolverFactory::<PythonBridge>::create(
            self.config.clone(),
            &self.service_url,
            self.domain_model.clone(),
            self.constraints.clone(),
            self.wasm_module.clone(),
        );
        let solver = factory.build_solver(self.bridge.clone());

        let response = solver
            .get_best_solution(handle.to_rust())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        match response {
            Some(resp) => {
                let solution_class = self.config.solution_class.as_deref().unwrap_or("Solution");
                let solution_obj = self
                    .bridge
                    .deserialize_object(&resp.solution, solution_class)
                    .ok()
                    .and_then(|h| self.bridge.get_py_object(h));
                Ok(Some(PySolveResponse::from_rust(resp, solution_obj)))
            }
            None => Ok(None),
        }
    }

    /// Stop an async solve early.
    fn stop(&self, handle: &PySolveHandle) -> PyResult<()> {
        let mut builder = DomainModelBuilder::new();
        if let Some(solution_class) = &self.config.solution_class {
            builder = builder.solution_class(solution_class);
        }
        for entity_class in &self.config.entity_class_list {
            builder = builder.entity_class(entity_class);
        }
        let domain_model = builder.build();

        let factory = SolverFactory::<PythonBridge>::create(
            self.config.clone(),
            &self.service_url,
            domain_model,
            self.constraints.clone(),
            self.wasm_module.clone(),
        );
        let solver = factory.build_solver(self.bridge.clone());

        solver
            .stop(handle.to_rust())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(())
    }

    #[getter]
    fn config(&self) -> PySolverConfig {
        PySolverConfig {
            inner: self.config.clone(),
            solution_class_obj: None,
            entity_class_objs: Vec::new(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Solver(solution_class={:?}, service_url='{}')",
            self.config.solution_class, self.service_url
        )
    }
}

/// Build a ConstraintSet and collect predicates from Python constraints.
fn build_constraint_set(constraints: &[PyConstraint]) -> (ConstraintSet, Vec<LambdaInfo>) {
    let mut set = ConstraintSet::new();
    let mut all_predicates = Vec::new();
    for constraint in constraints {
        set = set.with_constraint(constraint.to_rust());
        all_predicates.extend(constraint.predicates().iter().cloned());
    }
    (set, all_predicates)
}

/// Convert LambdaInfo to PredicateDefinition for WASM generation.
fn lambda_to_predicate(lambda: &LambdaInfo) -> Option<PredicateDefinition> {
    lambda.expression.as_ref().map(|expr| {
        PredicateDefinition::from_expression(&lambda.name, lambda.param_count as u32, expr.clone())
    })
}

/// Status of a solver job.
#[pyclass(name = "SolverJobStatus", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PySolverJobStatus {
    /// The solver job has not started solving yet.
    NotSolving,
    /// The solver job is scheduled to solve, but hasn't started yet.
    SolvingScheduled,
    /// The solver is actively solving the problem.
    SolvingActive,
}

#[pymethods]
impl PySolverJobStatus {
    fn __repr__(&self) -> String {
        match self {
            PySolverJobStatus::NotSolving => "SolverJobStatus.NOT_SOLVING".to_string(),
            PySolverJobStatus::SolvingScheduled => "SolverJobStatus.SOLVING_SCHEDULED".to_string(),
            PySolverJobStatus::SolvingActive => "SolverJobStatus.SOLVING_ACTIVE".to_string(),
        }
    }

    #[classattr]
    #[pyo3(name = "NOT_SOLVING")]
    fn not_solving() -> Self {
        PySolverJobStatus::NotSolving
    }

    #[classattr]
    #[pyo3(name = "SOLVING_SCHEDULED")]
    fn solving_scheduled() -> Self {
        PySolverJobStatus::SolvingScheduled
    }

    #[classattr]
    #[pyo3(name = "SOLVING_ACTIVE")]
    fn solving_active() -> Self {
        PySolverJobStatus::SolvingActive
    }
}

/// Internal state for a solver job.
struct SolverJobState {
    status: PySolverJobStatus,
    best_solution_json: Option<String>,
    best_score: Option<String>,
    final_solution_json: Option<String>,
    final_score: Option<String>,
    error: Option<String>,
    terminated_early: bool,
    last_notified_score: Option<String>,
}

/// A SolverJob is a handle to a solve operation that's running or scheduled.
#[pyclass(name = "SolverJob")]
pub struct PySolverJob {
    problem_id: Py<PyAny>,
    state: Arc<std::sync::Mutex<SolverJobState>>,
    solution_class: String,
    bridge: Arc<PythonBridge>,
    /// Thread handle for the background solver (if running).
    #[allow(dead_code)]
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

#[pymethods]
impl PySolverJob {
    /// Get the problem ID.
    fn get_problem_id(&self, py: Python<'_>) -> Py<PyAny> {
        self.problem_id.clone_ref(py)
    }

    /// Get the current solver status.
    fn get_solver_status(&self) -> PySolverJobStatus {
        let state = self.state.lock().unwrap();
        state.status
    }

    /// Check if the solver was terminated early.
    fn is_terminated_early(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.terminated_early
    }

    /// Request early termination of the solver.
    fn terminate_early(&self) {
        let mut state = self.state.lock().unwrap();
        state.terminated_early = true;
    }

    /// Get the final best solution. Blocks until solving completes.
    fn get_final_best_solution(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        // Poll until done
        loop {
            {
                let state = self.state.lock().unwrap();
                match state.status {
                    PySolverJobStatus::NotSolving => {
                        // Completed - return solution
                        if let Some(ref error) = state.error {
                            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                                error.clone(),
                            ));
                        }
                        if let Some(ref json) = state.final_solution_json {
                            // Deserialize the solution
                            let solution_obj = self
                                .bridge
                                .deserialize_object(json, &self.solution_class)
                                .map_err(|e| {
                                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                                })?;
                            return self.bridge.get_py_object(solution_obj).ok_or_else(|| {
                                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                                    "Failed to get solution object",
                                )
                            });
                        }
                        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "No solution available",
                        ));
                    }
                    _ => {
                        // Still running, continue polling
                    }
                }
            }
            // Release GIL while sleeping
            #[allow(deprecated)]
            py.allow_threads(|| {
                std::thread::sleep(std::time::Duration::from_millis(100));
            });
        }
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "SolverJob(problem_id={:?}, status={:?})",
            self.problem_id.bind(py),
            self.get_solver_status()
        )
    }
}

/// SolverManager manages multiple solver jobs and provides async solving with callbacks.
///
/// # Example
///
/// ```python
/// from solverforge import SolverManager, SolverFactory, SolverConfig
///
/// solver_manager = SolverManager.create(solver_factory)
///
/// def on_best_solution(solution):
///     print(f"New best: {solution.score}")
///
/// job = solver_manager.solve_and_listen("problem1", problem, on_best_solution)
/// final_solution = job.get_final_best_solution()
/// ```
#[pyclass(name = "SolverManager")]
pub struct PySolverManager {
    factory: Arc<PySolverFactoryData>,
    jobs: Arc<
        std::sync::Mutex<std::collections::HashMap<String, Arc<std::sync::Mutex<SolverJobState>>>>,
    >,
}

/// Internal factory data needed for solver creation.
struct PySolverFactoryData {
    config: SolverConfig,
    constraints: ConstraintSet,
    domain_model: DomainModel,
    service_url: String,
    wasm_module: String,
}

#[pymethods]
impl PySolverManager {
    /// Create a new SolverManager from a SolverFactory.
    #[staticmethod]
    fn create(solver_factory: &PySolverFactory) -> Self {
        Self {
            factory: Arc::new(PySolverFactoryData {
                config: solver_factory.get_config().clone(),
                constraints: solver_factory.get_constraints().clone(),
                domain_model: solver_factory.get_domain_model().clone(),
                service_url: solver_factory.get_service_url().to_string(),
                wasm_module: solver_factory.get_wasm_module().to_string(),
            }),
            jobs: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Solve a problem asynchronously and call the listener when a new best solution is found.
    ///
    /// # Arguments
    /// * `problem_id` - A unique identifier for this problem
    /// * `problem` - The problem to solve
    /// * `listener` - A callback called with each new best solution
    ///
    /// # Returns
    /// A SolverJob handle to track and control the solve.
    #[pyo3(signature = (problem_id, problem, listener))]
    fn solve_and_listen(
        &self,
        py: Python<'_>,
        problem_id: Py<PyAny>,
        problem: Py<PyAny>,
        listener: Py<PyAny>,
    ) -> PyResult<PySolverJob> {
        // Create initial state
        let state = Arc::new(std::sync::Mutex::new(SolverJobState {
            status: PySolverJobStatus::SolvingScheduled,
            best_solution_json: None,
            best_score: None,
            final_solution_json: None,
            final_score: None,
            error: None,
            terminated_early: false,
            last_notified_score: None,
        }));

        // Store problem_id as string key for hashmap
        let problem_id_str: String = problem_id.bind(py).str()?.extract()?;
        {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.insert(problem_id_str.clone(), state.clone());
        }

        // Create bridge and register problem
        let bridge = Arc::new(PythonBridge::new());
        let handle = bridge.register_object(problem.clone_ref(py));

        // Get solution class name
        let solution_class = self
            .factory
            .config
            .solution_class
            .clone()
            .unwrap_or_else(|| "Solution".to_string());

        // Clone data for thread
        let factory_data = self.factory.clone();
        let state_clone = state.clone();
        let bridge_clone = bridge.clone();
        let listener_clone = listener.clone_ref(py);
        let solution_class_clone = solution_class.clone();

        // Spawn background thread
        let thread_handle = std::thread::spawn(move || {
            // Mark as active
            {
                let mut s = state_clone.lock().unwrap();
                s.status = PySolverJobStatus::SolvingActive;
            }

            // Create the solver
            let rust_factory = SolverFactory::<PythonBridge>::create(
                factory_data.config.clone(),
                &factory_data.service_url,
                factory_data.domain_model.clone(),
                factory_data.constraints.clone(),
                factory_data.wasm_module.clone(),
            );
            let solver = rust_factory.build_solver(bridge_clone.clone());

            // Start async solve
            let solve_handle = match solver.solve_async(handle) {
                Ok(h) => h,
                Err(e) => {
                    let mut s = state_clone.lock().unwrap();
                    s.error = Some(e.to_string());
                    s.status = PySolverJobStatus::NotSolving;
                    return;
                }
            };

            // Poll for updates
            loop {
                // Check for early termination
                {
                    let s = state_clone.lock().unwrap();
                    if s.terminated_early {
                        let _ = solver.stop(&solve_handle);
                        break;
                    }
                }

                // Get status
                let status = match solver.get_status(&solve_handle) {
                    Ok(s) => s,
                    Err(e) => {
                        let mut s = state_clone.lock().unwrap();
                        s.error = Some(e.to_string());
                        s.status = PySolverJobStatus::NotSolving;
                        return;
                    }
                };

                // Check if we have a new best solution
                if let Some(ref score) = status.best_score {
                    let score_string = score.score_string.clone();
                    let should_notify = {
                        let s = state_clone.lock().unwrap();
                        s.last_notified_score.as_ref() != Some(&score_string)
                    };

                    if should_notify {
                        // Get the best solution
                        if let Ok(Some(response)) = solver.get_best_solution(&solve_handle) {
                            // Update state
                            {
                                let mut s = state_clone.lock().unwrap();
                                s.best_solution_json = Some(response.solution.clone());
                                s.best_score = Some(response.score.clone());
                                s.last_notified_score = Some(score_string);
                            }

                            // Call the listener with the new best solution
                            Python::attach(|py| {
                                // Deserialize solution
                                if let Ok(obj_handle) = bridge_clone
                                    .deserialize_object(&response.solution, &solution_class_clone)
                                {
                                    if let Some(solution_obj) =
                                        bridge_clone.get_py_object(obj_handle)
                                    {
                                        // Call the listener
                                        if let Err(e) =
                                            listener_clone.call1(py, (solution_obj.bind(py),))
                                        {
                                            log::error!(
                                                "Error calling best solution listener: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    }
                }

                // Check if done
                if status.state.is_terminal() {
                    // Get final solution
                    if let Ok(Some(response)) = solver.get_best_solution(&solve_handle) {
                        let mut s = state_clone.lock().unwrap();
                        s.final_solution_json = Some(response.solution);
                        s.final_score = Some(response.score);
                    }
                    break;
                }

                // Sleep before next poll
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Mark as done
            {
                let mut s = state_clone.lock().unwrap();
                s.status = PySolverJobStatus::NotSolving;
            }
        });

        Ok(PySolverJob {
            problem_id: problem_id.clone_ref(py),
            state,
            solution_class,
            bridge,
            thread_handle: Some(thread_handle),
        })
    }

    /// Get the solver status for a problem.
    fn get_solver_status(
        &self,
        py: Python<'_>,
        problem_id: Py<PyAny>,
    ) -> PyResult<PySolverJobStatus> {
        let problem_id_str: String = problem_id.bind(py).str()?.extract()?;
        let jobs = self.jobs.lock().unwrap();
        if let Some(state) = jobs.get(&problem_id_str) {
            let s = state.lock().unwrap();
            Ok(s.status)
        } else {
            Ok(PySolverJobStatus::NotSolving)
        }
    }

    /// Terminate a problem's solver early.
    fn terminate_early(&self, py: Python<'_>, problem_id: Py<PyAny>) -> PyResult<()> {
        let problem_id_str: String = problem_id.bind(py).str()?.extract()?;
        let jobs = self.jobs.lock().unwrap();
        if let Some(state) = jobs.get(&problem_id_str) {
            let mut s = state.lock().unwrap();
            s.terminated_early = true;
        }
        Ok(())
    }

    /// Close the solver manager and release resources.
    fn close(&self) {
        // Mark all jobs as terminated
        let jobs = self.jobs.lock().unwrap();
        for state in jobs.values() {
            let mut s = state.lock().unwrap();
            s.terminated_early = true;
        }
    }

    fn __repr__(&self) -> String {
        let jobs = self.jobs.lock().unwrap();
        format!("SolverManager(active_jobs={})", jobs.len())
    }
}

/// Register solver classes with the Python module.
pub fn register_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTerminationConfig>()?;
    m.add_class::<PyDiminishedReturnsConfig>()?;
    m.add_class::<PyEnvironmentMode>()?;
    m.add_class::<PyMoveThreadCount>()?;
    m.add_class::<PySolverConfig>()?;
    m.add_class::<PySolveHandle>()?;
    m.add_class::<PySolveState>()?;
    m.add_class::<PyScoreDto>()?;
    m.add_class::<PySolveStatus>()?;
    m.add_class::<PySolveResponse>()?;
    m.add_class::<PySolverFactory>()?;
    m.add_class::<PySolver>()?;
    m.add_class::<PySolverJobStatus>()?;
    m.add_class::<PySolverJob>()?;
    m.add_class::<PySolverManager>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_python() {
        pyo3::Python::initialize();
    }

    #[test]
    fn test_termination_config_new() {
        let config = PyTerminationConfig::new();
        assert!(config.inner.spent_limit.is_none());
    }

    #[test]
    fn test_termination_config_spent_limit() {
        let config = PyTerminationConfig::new().with_spent_limit("PT5M");
        assert_eq!(config.inner.spent_limit, Some("PT5M".to_string()));
    }

    #[test]
    fn test_termination_config_chained() {
        let config = PyTerminationConfig::new()
            .with_spent_limit("PT10M")
            .with_unimproved_spent_limit("PT1M")
            .with_best_score_feasible(true);

        assert_eq!(config.inner.spent_limit, Some("PT10M".to_string()));
        assert_eq!(
            config.inner.unimproved_spent_limit,
            Some("PT1M".to_string())
        );
        assert_eq!(config.inner.best_score_feasible, Some(true));
    }

    #[test]
    fn test_termination_config_repr() {
        let config = PyTerminationConfig::new().with_spent_limit("PT5M");
        let repr = config.__repr__();
        assert!(repr.contains("TerminationConfig"));
        assert!(repr.contains("PT5M"));
    }

    #[test]
    fn test_environment_mode_constructors() {
        assert!(matches!(
            PyEnvironmentMode::reproducible().inner,
            EnvironmentMode::Reproducible
        ));
        assert!(matches!(
            PyEnvironmentMode::full_assert().inner,
            EnvironmentMode::FullAssert
        ));
    }

    #[test]
    fn test_environment_mode_is_asserted() {
        assert!(!PyEnvironmentMode::reproducible().is_asserted());
        assert!(PyEnvironmentMode::full_assert().is_asserted());
    }

    #[test]
    fn test_environment_mode_repr() {
        let mode = PyEnvironmentMode::reproducible();
        let repr = mode.__repr__();
        assert!(repr.contains("EnvironmentMode"));
        assert!(repr.contains("Reproducible"));
    }

    #[test]
    fn test_move_thread_count_constructors() {
        assert!(matches!(
            PyMoveThreadCount::auto().inner,
            MoveThreadCount::Auto
        ));
        assert!(matches!(
            PyMoveThreadCount::none().inner,
            MoveThreadCount::None
        ));
        assert!(matches!(
            PyMoveThreadCount::count(4).inner,
            MoveThreadCount::Count(4)
        ));
    }

    #[test]
    fn test_move_thread_count_is_parallel() {
        assert!(PyMoveThreadCount::auto().is_parallel());
        assert!(!PyMoveThreadCount::none().is_parallel());
        assert!(!PyMoveThreadCount::count(1).is_parallel());
        assert!(PyMoveThreadCount::count(4).is_parallel());
    }

    #[test]
    fn test_move_thread_count_repr() {
        assert_eq!(
            PyMoveThreadCount::auto().__repr__(),
            "MoveThreadCount.auto()"
        );
        assert_eq!(
            PyMoveThreadCount::none().__repr__(),
            "MoveThreadCount.none()"
        );
        assert_eq!(
            PyMoveThreadCount::count(4).__repr__(),
            "MoveThreadCount.count(4)"
        );
    }

    #[test]
    fn test_solver_config_new() {
        let config = PySolverConfig::new();
        assert!(config.inner.solution_class.is_none());
        assert!(config.inner.entity_class_list.is_empty());
    }

    #[test]
    fn test_solver_config_with_environment_mode() {
        init_python();
        Python::attach(|py| {
            let config =
                PySolverConfig::new().with_environment_mode(py, &PyEnvironmentMode::full_assert());
            assert_eq!(
                config.inner.environment_mode,
                Some(EnvironmentMode::FullAssert)
            );
        });
    }

    #[test]
    fn test_solver_config_with_random_seed() {
        init_python();
        Python::attach(|py| {
            let config = PySolverConfig::new().with_random_seed(py, 42);
            assert_eq!(config.inner.random_seed, Some(42));
        });
    }

    #[test]
    fn test_solver_config_with_termination() {
        init_python();
        Python::attach(|py| {
            let termination = PyTerminationConfig::new().with_spent_limit("PT5M");
            let config = PySolverConfig::new().with_termination(py, &termination);
            assert!(config.inner.termination.is_some());
        });
    }

    #[test]
    fn test_solver_config_repr() {
        let config = PySolverConfig::new();
        let repr = config.__repr__();
        assert!(repr.contains("SolverConfig"));
    }

    #[test]
    fn test_solve_handle_repr() {
        let handle = PySolveHandle {
            inner: SolveHandle {
                id: "test-123".to_string(),
            },
        };
        assert_eq!(handle.__repr__(), "SolveHandle(id='test-123')");
    }

    #[test]
    fn test_solve_state_methods() {
        let running = PySolveState::from_rust(SolveState::Running);
        assert!(running.is_running());
        assert!(!running.is_terminal());

        let completed = PySolveState::from_rust(SolveState::Completed);
        assert!(!completed.is_running());
        assert!(completed.is_terminal());
    }

    #[test]
    fn test_solve_status_from_rust() {
        let status = PySolveStatus::from_rust(SolveStatus::running(
            10000,
            Some(ScoreDto::hard_soft(-2, -100)),
        ));
        assert!(status.is_solving());
        assert!(!status.is_terminated());
        assert_eq!(status.time_spent_ms(), 10000);
        assert!(status.best_score().is_some());
    }

    #[test]
    fn test_score_dto_from_rust() {
        let score = PyScoreDto::from_rust(ScoreDto::hard_soft(0, -50));
        assert_eq!(score.hard_score(), 0);
        assert_eq!(score.soft_score(), -50);
        assert!(score.is_feasible());
        assert_eq!(score.score_string(), "0hard/-50soft");
    }

    #[test]
    fn test_solve_response_is_feasible() {
        let response =
            PySolveResponse::from_rust(SolveResponse::new("{}".to_string(), "0hard/-10soft"), None);
        assert!(response.is_feasible());

        let response2 = PySolveResponse::from_rust(
            SolveResponse::new("{}".to_string(), "-5hard/-10soft"),
            None,
        );
        assert!(!response2.is_feasible());

        let response3 =
            PySolveResponse::from_rust(SolveResponse::new("{}".to_string(), "42"), None);
        assert!(response3.is_feasible());
    }

    // ============================================================
    // SolverManager tests
    // ============================================================

    #[test]
    fn test_solver_job_status_repr() {
        assert_eq!(
            PySolverJobStatus::NotSolving.__repr__(),
            "SolverJobStatus.NOT_SOLVING"
        );
        assert_eq!(
            PySolverJobStatus::SolvingScheduled.__repr__(),
            "SolverJobStatus.SOLVING_SCHEDULED"
        );
        assert_eq!(
            PySolverJobStatus::SolvingActive.__repr__(),
            "SolverJobStatus.SOLVING_ACTIVE"
        );
    }

    #[test]
    fn test_solver_job_status_equality() {
        assert_eq!(PySolverJobStatus::NotSolving, PySolverJobStatus::NotSolving);
        assert_eq!(
            PySolverJobStatus::SolvingActive,
            PySolverJobStatus::SolvingActive
        );
        assert_ne!(
            PySolverJobStatus::NotSolving,
            PySolverJobStatus::SolvingActive
        );
    }

    #[test]
    fn test_solver_job_status_class_attrs() {
        assert_eq!(
            PySolverJobStatus::not_solving(),
            PySolverJobStatus::NotSolving
        );
        assert_eq!(
            PySolverJobStatus::solving_scheduled(),
            PySolverJobStatus::SolvingScheduled
        );
        assert_eq!(
            PySolverJobStatus::solving_active(),
            PySolverJobStatus::SolvingActive
        );
    }

    #[test]
    fn test_solver_job_state_initial() {
        let state = SolverJobState {
            status: PySolverJobStatus::SolvingScheduled,
            best_solution_json: None,
            best_score: None,
            final_solution_json: None,
            final_score: None,
            error: None,
            terminated_early: false,
            last_notified_score: None,
        };
        assert_eq!(state.status, PySolverJobStatus::SolvingScheduled);
        assert!(!state.terminated_early);
    }

    #[test]
    fn test_solver_manager_repr() {
        init_python();
        Python::attach(|_py| {
            // Create factory data directly
            let factory_data = Arc::new(PySolverFactoryData {
                config: SolverConfig::new(),
                constraints: ConstraintSet::new(),
                domain_model: solverforge_core::domain::DomainModelBuilder::new().build(),
                service_url: "http://localhost:8080".to_string(),
            });

            let manager = PySolverManager {
                factory: factory_data,
                jobs: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            };

            let repr = manager.__repr__();
            assert!(repr.contains("SolverManager"));
            assert!(repr.contains("active_jobs=0"));

            // Add a mock job
            {
                let mut jobs = manager.jobs.lock().unwrap();
                jobs.insert(
                    "test".to_string(),
                    Arc::new(std::sync::Mutex::new(SolverJobState {
                        status: PySolverJobStatus::SolvingActive,
                        best_solution_json: None,
                        best_score: None,
                        final_solution_json: None,
                        final_score: None,
                        error: None,
                        terminated_early: false,
                        last_notified_score: None,
                    })),
                );
            }

            let repr2 = manager.__repr__();
            assert!(repr2.contains("active_jobs=1"));
        });
    }

    #[test]
    fn test_solver_manager_close() {
        init_python();
        Python::attach(|_py| {
            let factory_data = Arc::new(PySolverFactoryData {
                config: SolverConfig::new(),
                constraints: ConstraintSet::new(),
                domain_model: solverforge_core::domain::DomainModelBuilder::new().build(),
                service_url: "http://localhost:8080".to_string(),
            });

            let manager = PySolverManager {
                factory: factory_data,
                jobs: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            };

            // Add a job
            {
                let mut jobs = manager.jobs.lock().unwrap();
                jobs.insert(
                    "test".to_string(),
                    Arc::new(std::sync::Mutex::new(SolverJobState {
                        status: PySolverJobStatus::SolvingActive,
                        best_solution_json: None,
                        best_score: None,
                        final_solution_json: None,
                        final_score: None,
                        error: None,
                        terminated_early: false,
                        last_notified_score: None,
                    })),
                );
            }

            // Close should mark job as terminated
            manager.close();

            let jobs = manager.jobs.lock().unwrap();
            let state = jobs.get("test").unwrap().lock().unwrap();
            assert!(state.terminated_early);
        });
    }

    // ============================================================
    // Integration tests for timetabling domain
    // These tests execute Python code to exercise the Python API
    // ============================================================

    mod timetabling_integration {
        use pyo3::prelude::*;
        use pyo3::types::PyDict;

        fn init_python() {
            pyo3::Python::initialize();
        }

        /// Test the full timetabling setup from Python
        #[test]
        fn test_timetabling_domain_setup() {
            init_python();
            Python::attach(|py| {
                let locals = PyDict::new(py);

                // Execute Python code that uses our bindings
                let result = py.run(
                    c"
# Import would be: from solverforge import *
# For testing, we work with raw Python classes

class Room:
    def __init__(self, name):
        self.name = name

class Timeslot:
    def __init__(self, day, start_time, end_time):
        self.day = day
        self.start_time = start_time
        self.end_time = end_time

class Lesson:
    def __init__(self, id, subject, teacher, student_group, timeslot=None, room=None):
        self.id = id
        self.subject = subject
        self.teacher = teacher
        self.student_group = student_group
        self.timeslot = timeslot
        self.room = room

class Timetable:
    def __init__(self, rooms, timeslots, lessons):
        self.rooms = rooms
        self.timeslots = timeslots
        self.lessons = lessons

# Create test data
room1 = Room('Room A')
room2 = Room('Room B')
timeslot1 = Timeslot('MONDAY', '08:30', '09:30')
timeslot2 = Timeslot('MONDAY', '09:30', '10:30')

lesson1 = Lesson(1, 'Math', 'Mr. Smith', '9th grade')
lesson2 = Lesson(2, 'English', 'Mrs. Jones', '10th grade')

timetable = Timetable(
    rooms=[room1, room2],
    timeslots=[timeslot1, timeslot2],
    lessons=[lesson1, lesson2]
)

# Verify structure
assert len(timetable.rooms) == 2
assert len(timetable.timeslots) == 2
assert len(timetable.lessons) == 2
assert lesson1.room is None  # Not yet assigned
result = 'success'
",
                    None,
                    Some(&locals),
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
                let result_val: String = locals
                    .get_item("result")
                    .unwrap()
                    .unwrap()
                    .extract()
                    .unwrap();
                assert_eq!(result_val, "success");
            });
        }

        /// Test constraint lambdas can be defined in Python
        #[test]
        fn test_constraint_lambda_patterns() {
            init_python();
            Python::attach(|py| {
                let locals = PyDict::new(py);

                let result = py.run(
                    c"
# Test various lambda patterns that constraints will use

class Lesson:
    def __init__(self, room=None, timeslot=None, teacher=None):
        self.room = room
        self.timeslot = timeslot
        self.teacher = teacher

# Pattern 1: Field access
get_room = lambda lesson: lesson.room
get_teacher = lambda lesson: lesson.teacher

# Pattern 2: Is not None check
has_room = lambda lesson: lesson.room is not None

# Pattern 3: Comparison
same_room = lambda a, b: a.room == b.room
same_teacher = lambda a, b: a.teacher == b.teacher

# Pattern 4: Combined
room_conflict = lambda a, b: a.room == b.room and a.timeslot == b.timeslot

# Test the patterns
lesson1 = Lesson(room='A', timeslot='T1', teacher='Smith')
lesson2 = Lesson(room='A', timeslot='T1', teacher='Jones')
lesson3 = Lesson(room=None)

assert get_room(lesson1) == 'A'
assert has_room(lesson1) == True
assert has_room(lesson3) == False
assert same_room(lesson1, lesson2) == True
assert same_teacher(lesson1, lesson2) == False
assert room_conflict(lesson1, lesson2) == True

result = 'success'
",
                    None,
                    Some(&locals),
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
            });
        }

        /// Test solver configuration can be built
        #[test]
        fn test_solver_config_from_python() {
            init_python();
            Python::attach(|py| {
                // Inject our classes
                let locals = PyDict::new(py);
                locals
                    .set_item(
                        "TerminationConfig",
                        py.get_type::<super::PyTerminationConfig>(),
                    )
                    .unwrap();
                locals
                    .set_item("SolverConfig", py.get_type::<super::PySolverConfig>())
                    .unwrap();
                locals
                    .set_item("EnvironmentMode", py.get_type::<super::PyEnvironmentMode>())
                    .unwrap();

                let result = py.run(
                    c"
class Timetable:
    pass

class Lesson:
    pass

# Build termination config
termination = TerminationConfig() \
    .with_spent_limit('PT5M') \
    .with_unimproved_spent_limit('PT1M')

assert termination.spent_limit == 'PT5M'
assert termination.unimproved_spent_limit == 'PT1M'

# Build solver config
config = SolverConfig() \
    .with_solution_class(Timetable) \
    .with_entity_class(Lesson) \
    .with_termination(termination) \
    .with_environment_mode(EnvironmentMode.reproducible()) \
    .with_random_seed(42)

result = 'success'
",
                    None,
                    Some(&locals),
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
            });
        }

        /// Test constraint factory and streams from Python
        #[test]
        fn test_constraint_streams_from_python() {
            init_python();
            Python::attach(|py| {
                let locals = PyDict::new(py);
                locals
                    .set_item(
                        "ConstraintFactory",
                        py.get_type::<crate::stream::PyConstraintFactory>(),
                    )
                    .unwrap();
                locals
                    .set_item(
                        "HardSoftScore",
                        py.get_type::<crate::score::PyHardSoftScore>(),
                    )
                    .unwrap();

                let result = py.run(
                    c"
class Lesson:
    pass

# Create factory and stream
factory = ConstraintFactory()

# Test for_each
stream = factory.for_each(Lesson)

# Test filter
filtered = stream.filter(lambda lesson: lesson.room is not None)

# Test penalize with score
score = HardSoftScore.of_hard(1)
builder = filtered.penalize(score)

# Test as_constraint
constraint = builder.as_constraint('Room required')
assert constraint.name == 'Room required'

result = 'success'
",
                    None,
                    Some(&locals),
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
            });
        }

        /// Test constraint provider decorator from Python
        #[test]
        fn test_constraint_provider_from_python() {
            init_python();
            Python::attach(|py| {
                let locals = PyDict::new(py);
                locals
                    .set_item(
                        "constraint_provider",
                        py.eval(
                            c"lambda f: __import__('types').SimpleNamespace(func=f, name=f.__name__)",
                            None,
                            None,
                        )
                        .unwrap(),
                    )
                    .unwrap();

                let result = py.run(
                    c"
# Test that constraint provider pattern works
@constraint_provider
def define_constraints(factory):
    return [
        # Would return constraint objects
    ]

assert define_constraints.name == 'define_constraints'
assert callable(define_constraints.func)

result = 'success'
",
                    None,
                    Some(&locals),
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
            });
        }

        /// Test full timetabling constraint provider
        #[test]
        fn test_full_timetabling_constraints_from_python() {
            init_python();
            Python::attach(|py| {
                // Use globals dict so class definitions are accessible from lambdas
                let globals = PyDict::new(py);
                globals
                    .set_item(
                        "ConstraintFactory",
                        py.get_type::<crate::stream::PyConstraintFactory>(),
                    )
                    .unwrap();
                globals
                    .set_item(
                        "HardSoftScore",
                        py.get_type::<crate::score::PyHardSoftScore>(),
                    )
                    .unwrap();
                globals
                    .set_item("Joiners", py.get_type::<crate::joiners::PyJoiners>())
                    .unwrap();

                let result = py.run(
                    c"
class Lesson:
    pass

def define_constraints(factory):
    return [
        # Room conflict: two lessons in the same room at the same time
        factory.for_each_unique_pair(
            Lesson,
            Joiners.equal(lambda lesson: lesson.room),
            Joiners.equal(lambda lesson: lesson.timeslot)
        )
        .penalize(HardSoftScore.of_hard(1))
        .as_constraint('Room conflict'),

        # Teacher conflict: teacher teaching two lessons at the same time
        factory.for_each_unique_pair(
            Lesson,
            Joiners.equal(lambda lesson: lesson.teacher),
            Joiners.equal(lambda lesson: lesson.timeslot)
        )
        .penalize(HardSoftScore.of_hard(1))
        .as_constraint('Teacher conflict'),

        # Student group conflict
        factory.for_each_unique_pair(
            Lesson,
            Joiners.equal(lambda lesson: lesson.student_group),
            Joiners.equal(lambda lesson: lesson.timeslot)
        )
        .penalize(HardSoftScore.of_hard(1))
        .as_constraint('Student group conflict'),
    ]

# Create constraints
factory = ConstraintFactory()
constraints = define_constraints(factory)

assert len(constraints) == 3
assert constraints[0].name == 'Room conflict'
assert constraints[1].name == 'Teacher conflict'
assert constraints[2].name == 'Student group conflict'

result = 'success'
",
                    Some(&globals),
                    None,
                );

                assert!(result.is_ok(), "Python code failed: {:?}", result.err());
            });
        }
    }
}
