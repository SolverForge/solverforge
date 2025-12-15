//! High-level builder API for creating solvers from `PlanningSolution` types.
//!
//! The `SolverBuilder` provides an ergonomic way to create solvers by automatically
//! extracting domain models, constraints, and generating WASM modules from types
//! that implement the `PlanningSolution` trait.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_core::{SolverBuilder, TerminationConfig, PlanningSolution};
//!
//! // Given a type implementing PlanningSolution (usually via derive macro)
//! let solver = SolverBuilder::<Timetable>::new()
//!     .with_service_url("http://localhost:8080")
//!     .with_termination(TerminationConfig::new().with_spent_limit("PT5M"))
//!     .build()?;
//!
//! let solution = solver.solve(problem)?;
//! ```

use crate::bridge::LanguageBridge;
use crate::constraints::ConstraintSet;
use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::solver::{
    EnvironmentMode, HttpSolverService, MoveThreadCount, SolverConfig, SolverService,
    TerminationConfig,
};
use crate::traits::PlanningSolution;
use crate::wasm::WasmModuleBuilder;
use std::marker::PhantomData;
use std::sync::Arc;

/// Default service URL for the solver service.
pub const DEFAULT_SERVICE_URL: &str = "http://localhost:8080";

/// Builder for creating solvers from `PlanningSolution` types.
///
/// This builder automatically extracts domain models and constraints from the
/// solution type and generates the required WASM module.
///
/// # Type Parameters
///
/// - `S`: The solution type that implements `PlanningSolution`
///
/// # Example
///
/// ```ignore
/// let solver = SolverBuilder::<Timetable>::new()
///     .with_termination(TerminationConfig::new().with_spent_limit("PT5M"))
///     .build()?;
/// ```
pub struct SolverBuilder<S: PlanningSolution> {
    service_url: String,
    termination: Option<TerminationConfig>,
    environment_mode: Option<EnvironmentMode>,
    random_seed: Option<u64>,
    move_thread_count: Option<MoveThreadCount>,
    custom_service: Option<Arc<dyn SolverService>>,
    _phantom: PhantomData<S>,
}

impl<S: PlanningSolution> Default for SolverBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> SolverBuilder<S> {
    /// Creates a new `SolverBuilder` with default settings.
    ///
    /// The default service URL is `http://localhost:8080`.
    pub fn new() -> Self {
        Self {
            service_url: DEFAULT_SERVICE_URL.to_string(),
            termination: None,
            environment_mode: None,
            random_seed: None,
            move_thread_count: None,
            custom_service: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the URL of the solver service.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = SolverBuilder::<Timetable>::new()
    ///     .with_service_url("http://solver.example.com:8080");
    /// ```
    pub fn with_service_url(mut self, url: impl Into<String>) -> Self {
        self.service_url = url.into();
        self
    }

    /// Sets the termination configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = SolverBuilder::<Timetable>::new()
    ///     .with_termination(
    ///         TerminationConfig::new()
    ///             .with_spent_limit("PT5M")
    ///             .with_best_score_feasible(true)
    ///     );
    /// ```
    pub fn with_termination(mut self, termination: TerminationConfig) -> Self {
        self.termination = Some(termination);
        self
    }

    /// Sets the environment mode for the solver.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = SolverBuilder::<Timetable>::new()
    ///     .with_environment_mode(EnvironmentMode::Reproducible);
    /// ```
    pub fn with_environment_mode(mut self, mode: EnvironmentMode) -> Self {
        self.environment_mode = Some(mode);
        self
    }

    /// Sets the random seed for reproducible solving.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = SolverBuilder::<Timetable>::new()
    ///     .with_random_seed(42);
    /// ```
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// Sets the move thread count for parallel solving.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = SolverBuilder::<Timetable>::new()
    ///     .with_move_thread_count(MoveThreadCount::Auto);
    /// ```
    pub fn with_move_thread_count(mut self, count: MoveThreadCount) -> Self {
        self.move_thread_count = Some(count);
        self
    }

    /// Uses a custom solver service instead of the default HTTP service.
    ///
    /// This is useful for testing or when using a different transport.
    pub fn with_service(mut self, service: Arc<dyn SolverService>) -> Self {
        self.custom_service = Some(service);
        self
    }

    /// Returns the domain model for the solution type.
    ///
    /// This is extracted from the `PlanningSolution::domain_model()` method.
    pub fn domain_model() -> DomainModel {
        S::domain_model()
    }

    /// Returns the constraint set for the solution type.
    ///
    /// This is extracted from the `PlanningSolution::constraints()` method.
    pub fn constraints() -> ConstraintSet {
        S::constraints()
    }

    /// Builds the solver configuration.
    fn build_config(&self) -> SolverConfig {
        let domain_model = S::domain_model();
        let solution_class = domain_model.solution_class().map(|s| s.to_string());

        let entity_classes: Vec<String> = domain_model
            .classes
            .values()
            .filter(|c| c.is_planning_entity())
            .map(|c| c.name.clone())
            .collect();

        let mut config = SolverConfig::new().with_entity_classes(entity_classes);

        if let Some(solution_class) = solution_class {
            config = config.with_solution_class(solution_class);
        }

        if let Some(termination) = &self.termination {
            config = config.with_termination(termination.clone());
        }

        if let Some(mode) = &self.environment_mode {
            config = config.with_environment_mode(*mode);
        }

        if let Some(seed) = self.random_seed {
            config = config.with_random_seed(seed);
        }

        if let Some(count) = &self.move_thread_count {
            config = config.with_move_thread_count(count.clone());
        }

        config
    }

    /// Generates the WASM module from the domain model.
    ///
    /// This builds a base64-encoded WASM module containing:
    /// - Memory allocation functions
    /// - Getters/setters for all domain class fields
    /// - Predicate functions from constraints
    fn generate_wasm_module(&self) -> SolverForgeResult<String> {
        let domain_model = S::domain_model();

        WasmModuleBuilder::new()
            .with_domain_model(domain_model)
            .build_base64()
    }

    /// Builds a `TypedSolver` that can solve instances of the solution type.
    ///
    /// # Errors
    ///
    /// Returns an error if WASM module generation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let solver = SolverBuilder::<Timetable>::new()
    ///     .with_termination(TerminationConfig::new().with_spent_limit("PT5M"))
    ///     .build()?;
    /// ```
    pub fn build<B: LanguageBridge>(self) -> SolverForgeResult<TypedSolver<S, B>> {
        let config = self.build_config();
        let domain_model = S::domain_model();
        let constraints = S::constraints();
        let wasm_module = self.generate_wasm_module()?;

        let service: Arc<dyn SolverService> = match self.custom_service {
            Some(service) => service,
            None => Arc::new(HttpSolverService::new(&self.service_url)),
        };

        Ok(TypedSolver {
            config,
            domain_model,
            constraints,
            wasm_module,
            service,
            service_url: self.service_url,
            _phantom: PhantomData,
        })
    }

    /// Builds a `TypedSolver` with a specific bridge instance.
    ///
    /// This is a convenience method that creates a solver ready to use with
    /// the provided bridge.
    pub fn build_with_bridge<B: LanguageBridge>(
        self,
        _bridge: Arc<B>,
    ) -> SolverForgeResult<TypedSolver<S, B>> {
        self.build()
    }
}

/// A solver that is typed to a specific `PlanningSolution` type.
///
/// This provides a type-safe interface for solving problems and extracting
/// solutions of the correct type.
pub struct TypedSolver<S: PlanningSolution, B: LanguageBridge> {
    config: SolverConfig,
    domain_model: DomainModel,
    constraints: ConstraintSet,
    wasm_module: String,
    service: Arc<dyn SolverService>,
    service_url: String,
    _phantom: PhantomData<(S, B)>,
}

impl<S: PlanningSolution, B: LanguageBridge> TypedSolver<S, B> {
    /// Returns the solver configuration.
    pub fn config(&self) -> &SolverConfig {
        &self.config
    }

    /// Returns the domain model.
    pub fn domain_model(&self) -> &DomainModel {
        &self.domain_model
    }

    /// Returns the constraint set.
    pub fn constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    /// Returns the generated WASM module as a base64-encoded string.
    pub fn wasm_module(&self) -> &str {
        &self.wasm_module
    }

    /// Returns the service URL.
    pub fn service_url(&self) -> &str {
        &self.service_url
    }

    /// Checks if the solver service is available.
    pub fn is_service_available(&self) -> bool {
        self.service.is_available()
    }

    /// Solves the given problem and returns the solution.
    ///
    /// # Arguments
    ///
    /// * `problem` - The initial solution with unassigned planning variables
    ///
    /// # Returns
    ///
    /// The solved solution with planning variables assigned, or an error
    /// if solving fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let problem = Timetable {
    ///     timeslots: vec![...],
    ///     rooms: vec![...],
    ///     lessons: vec![...],
    ///     score: None,
    /// };
    ///
    /// let solution = solver.solve(problem)?;
    /// println!("Score: {:?}", solution.score());
    /// ```
    pub fn solve(&self, problem: S) -> SolverForgeResult<S> {
        // Serialize the problem to JSON
        let problem_json = problem.to_json()?;

        // Build the solve request
        let domain_dto = self.domain_model.to_dto();
        let constraints_dto = self.constraints.to_dto();

        let list_accessor = crate::solver::ListAccessorDto::new(
            "create_list",
            "get_item",
            "set_item",
            "get_size",
            "append",
            "insert",
            "remove",
            "deallocate_list",
        );

        let mut request = crate::solver::SolveRequest::new(
            domain_dto,
            constraints_dto,
            self.wasm_module.clone(),
            "alloc".to_string(),
            "dealloc".to_string(),
            list_accessor,
            problem_json,
        );

        if let Some(mode) = &self.config.environment_mode {
            request = request.with_environment_mode(format!("{:?}", mode).to_uppercase());
        }

        if let Some(termination) = &self.config.termination {
            request = request.with_termination(termination.clone());
        }

        // Solve
        let response = self.service.solve(&request)?;

        // Parse the solution from JSON
        S::from_json(&response.solution)
    }

    /// Starts an asynchronous solve and returns a handle.
    ///
    /// Use `get_best_solution()` to retrieve intermediate solutions and
    /// `stop()` to terminate early.
    pub fn solve_async(&self, problem: S) -> SolverForgeResult<crate::solver::SolveHandle> {
        let problem_json = problem.to_json()?;

        let domain_dto = self.domain_model.to_dto();
        let constraints_dto = self.constraints.to_dto();

        let list_accessor = crate::solver::ListAccessorDto::new(
            "create_list",
            "get_item",
            "set_item",
            "get_size",
            "append",
            "insert",
            "remove",
            "deallocate_list",
        );

        let mut request = crate::solver::SolveRequest::new(
            domain_dto,
            constraints_dto,
            self.wasm_module.clone(),
            "alloc".to_string(),
            "dealloc".to_string(),
            list_accessor,
            problem_json,
        );

        if let Some(mode) = &self.config.environment_mode {
            request = request.with_environment_mode(format!("{:?}", mode).to_uppercase());
        }

        if let Some(termination) = &self.config.termination {
            request = request.with_termination(termination.clone());
        }

        self.service.solve_async(&request)
    }

    /// Gets the status of an asynchronous solve.
    pub fn get_status(
        &self,
        handle: &crate::solver::SolveHandle,
    ) -> SolverForgeResult<crate::solver::SolveStatus> {
        self.service.get_status(handle)
    }

    /// Gets the best solution found so far in an asynchronous solve.
    pub fn get_best_solution(
        &self,
        handle: &crate::solver::SolveHandle,
    ) -> SolverForgeResult<Option<S>> {
        let response = self.service.get_best_solution(handle)?;
        match response {
            Some(r) => Ok(Some(S::from_json(&r.solution)?)),
            None => Ok(None),
        }
    }

    /// Stops an asynchronous solve.
    pub fn stop(&self, handle: &crate::solver::SolveHandle) -> SolverForgeResult<()> {
        self.service.stop(handle)
    }
}

/// Error type for solver builder operations.
#[derive(Debug, Clone)]
pub enum SolverBuilderError {
    /// WASM module generation failed
    WasmGeneration(String),
    /// Configuration is invalid
    InvalidConfiguration(String),
}

impl std::fmt::Display for SolverBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverBuilderError::WasmGeneration(msg) => {
                write!(f, "WASM module generation failed: {}", msg)
            }
            SolverBuilderError::InvalidConfiguration(msg) => {
                write!(f, "Invalid solver configuration: {}", msg)
            }
        }
    }
}

impl std::error::Error for SolverBuilderError {}

impl From<SolverBuilderError> for SolverForgeError {
    fn from(err: SolverBuilderError) -> Self {
        match err {
            SolverBuilderError::WasmGeneration(msg) => SolverForgeError::WasmGeneration(msg),
            SolverBuilderError::InvalidConfiguration(msg) => SolverForgeError::Configuration(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::tests::MockBridge;
    use crate::constraints::Constraint;
    use crate::domain::{
        DomainClass, FieldDescriptor, FieldType, PlanningAnnotation, PrimitiveType, ScoreType,
    };
    use crate::traits::PlanningEntity;
    use crate::HardSoftScore;
    use std::collections::HashMap;

    // Test entity
    #[derive(Clone, Debug)]
    struct TestLesson {
        id: String,
        subject: String,
        room: Option<String>,
    }

    impl crate::traits::PlanningEntity for TestLesson {
        fn domain_class() -> crate::domain::DomainClass {
            DomainClass::new("TestLesson")
                .with_annotation(PlanningAnnotation::PlanningEntity)
                .with_field(
                    FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                        .with_planning_annotation(PlanningAnnotation::PlanningId),
                )
                .with_field(FieldDescriptor::new(
                    "subject",
                    FieldType::Primitive(PrimitiveType::String),
                ))
                .with_field(
                    FieldDescriptor::new("room", FieldType::Primitive(PrimitiveType::String))
                        .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                            "rooms".to_string(),
                        ])),
                )
        }

        fn planning_id(&self) -> crate::Value {
            crate::Value::String(self.id.clone())
        }

        fn to_value(&self) -> crate::Value {
            let mut map = HashMap::new();
            map.insert("id".to_string(), crate::Value::String(self.id.clone()));
            map.insert(
                "subject".to_string(),
                crate::Value::String(self.subject.clone()),
            );
            map.insert(
                "room".to_string(),
                self.room
                    .clone()
                    .map(crate::Value::String)
                    .unwrap_or(crate::Value::Null),
            );
            crate::Value::Object(map)
        }

        fn from_value(value: &crate::Value) -> SolverForgeResult<Self> {
            match value {
                crate::Value::Object(map) => {
                    let id = map
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| SolverForgeError::Serialization("Missing id".to_string()))?
                        .to_string();
                    let subject = map
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SolverForgeError::Serialization("Missing subject".to_string())
                        })?
                        .to_string();
                    let room = map.get("room").and_then(|v| v.as_str()).map(String::from);
                    Ok(TestLesson { id, subject, room })
                }
                _ => Err(SolverForgeError::Serialization(
                    "Expected object".to_string(),
                )),
            }
        }
    }

    // Test solution
    #[derive(Clone, Debug)]
    struct TestTimetable {
        rooms: Vec<String>,
        lessons: Vec<TestLesson>,
        score: Option<HardSoftScore>,
    }

    impl PlanningSolution for TestTimetable {
        type Score = HardSoftScore;

        fn domain_model() -> DomainModel {
            DomainModel::builder()
                .add_class(TestLesson::domain_class())
                .add_class(
                    DomainClass::new("TestTimetable")
                        .with_annotation(PlanningAnnotation::PlanningSolution)
                        .with_field(
                            FieldDescriptor::new(
                                "rooms",
                                FieldType::list(FieldType::Primitive(PrimitiveType::String)),
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::ProblemFactCollectionProperty,
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::value_range_provider("rooms"),
                            ),
                        )
                        .with_field(
                            FieldDescriptor::new(
                                "lessons",
                                FieldType::list(FieldType::object("TestLesson")),
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::PlanningEntityCollectionProperty,
                            ),
                        )
                        .with_field(
                            FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                                .with_planning_annotation(PlanningAnnotation::planning_score()),
                        ),
                )
                .build()
        }

        fn constraints() -> ConstraintSet {
            ConstraintSet::new().with_constraint(Constraint::new("Test constraint"))
        }

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Self::Score) {
            self.score = Some(score);
        }

        fn to_json(&self) -> SolverForgeResult<String> {
            let mut map = HashMap::new();

            let rooms: Vec<crate::Value> = self
                .rooms
                .iter()
                .map(|r| crate::Value::String(r.clone()))
                .collect();
            map.insert("rooms".to_string(), crate::Value::Array(rooms));

            let lessons: Vec<crate::Value> = self.lessons.iter().map(|l| l.to_value()).collect();
            map.insert("lessons".to_string(), crate::Value::Array(lessons));

            if let Some(score) = &self.score {
                map.insert(
                    "score".to_string(),
                    crate::Value::String(format!("{}", score)),
                );
            }

            serde_json::to_string(&crate::Value::Object(map))
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))
        }

        fn from_json(json: &str) -> SolverForgeResult<Self> {
            let value: crate::Value = serde_json::from_str(json)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

            match value {
                crate::Value::Object(map) => {
                    let rooms = match map.get("rooms") {
                        Some(crate::Value::Array(arr)) => arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect(),
                        _ => Vec::new(),
                    };

                    let lessons = match map.get("lessons") {
                        Some(crate::Value::Array(arr)) => arr
                            .iter()
                            .filter_map(|v| TestLesson::from_value(v).ok())
                            .collect(),
                        _ => Vec::new(),
                    };

                    Ok(TestTimetable {
                        rooms,
                        lessons,
                        score: None,
                    })
                }
                _ => Err(SolverForgeError::Serialization(
                    "Expected object".to_string(),
                )),
            }
        }
    }

    #[test]
    fn test_solver_builder_new() {
        let builder = SolverBuilder::<TestTimetable>::new();
        assert_eq!(builder.service_url, DEFAULT_SERVICE_URL);
        assert!(builder.termination.is_none());
        assert!(builder.environment_mode.is_none());
    }

    #[test]
    fn test_solver_builder_default() {
        let builder = SolverBuilder::<TestTimetable>::default();
        assert_eq!(builder.service_url, DEFAULT_SERVICE_URL);
    }

    #[test]
    fn test_solver_builder_with_service_url() {
        let builder = SolverBuilder::<TestTimetable>::new().with_service_url("http://custom:9000");
        assert_eq!(builder.service_url, "http://custom:9000");
    }

    #[test]
    fn test_solver_builder_with_termination() {
        let termination = TerminationConfig::new().with_spent_limit("PT5M");
        let builder = SolverBuilder::<TestTimetable>::new().with_termination(termination.clone());
        assert_eq!(builder.termination, Some(termination));
    }

    #[test]
    fn test_solver_builder_with_environment_mode() {
        let builder = SolverBuilder::<TestTimetable>::new()
            .with_environment_mode(EnvironmentMode::FullAssert);
        assert_eq!(builder.environment_mode, Some(EnvironmentMode::FullAssert));
    }

    #[test]
    fn test_solver_builder_with_random_seed() {
        let builder = SolverBuilder::<TestTimetable>::new().with_random_seed(42);
        assert_eq!(builder.random_seed, Some(42));
    }

    #[test]
    fn test_solver_builder_with_move_thread_count() {
        let builder =
            SolverBuilder::<TestTimetable>::new().with_move_thread_count(MoveThreadCount::Auto);
        assert_eq!(builder.move_thread_count, Some(MoveThreadCount::Auto));
    }

    #[test]
    fn test_solver_builder_domain_model() {
        let model = SolverBuilder::<TestTimetable>::domain_model();
        assert!(model.get_solution_class().is_some());
        assert_eq!(model.get_solution_class().unwrap().name, "TestTimetable");
        assert!(model.get_class("TestLesson").is_some());
    }

    #[test]
    fn test_solver_builder_constraints() {
        let constraints = SolverBuilder::<TestTimetable>::constraints();
        assert!(!constraints.is_empty());
    }

    #[test]
    fn test_solver_builder_build_config() {
        let builder = SolverBuilder::<TestTimetable>::new()
            .with_termination(TerminationConfig::new().with_spent_limit("PT5M"))
            .with_environment_mode(EnvironmentMode::Reproducible)
            .with_random_seed(42);

        let config = builder.build_config();
        assert_eq!(config.solution_class, Some("TestTimetable".to_string()));
        assert!(config.entity_class_list.contains(&"TestLesson".to_string()));
        assert_eq!(config.environment_mode, Some(EnvironmentMode::Reproducible));
        assert_eq!(config.random_seed, Some(42));
        assert!(config.termination.is_some());
    }

    #[test]
    fn test_solver_builder_generate_wasm() {
        let builder = SolverBuilder::<TestTimetable>::new();
        let wasm = builder.generate_wasm_module().unwrap();
        assert!(wasm.starts_with("AGFzbQ")); // Base64 of "\0asm"
    }

    #[test]
    fn test_solver_builder_build() {
        let solver = SolverBuilder::<TestTimetable>::new()
            .with_service_url("http://localhost:19999")
            .with_termination(TerminationConfig::new().with_spent_limit("PT1M"))
            .build::<MockBridge>()
            .unwrap();

        assert_eq!(
            solver.config().solution_class,
            Some("TestTimetable".to_string())
        );
        assert!(solver.wasm_module().starts_with("AGFzbQ"));
        assert_eq!(solver.service_url(), "http://localhost:19999");
    }

    #[test]
    fn test_solver_builder_chained() {
        let solver = SolverBuilder::<TestTimetable>::new()
            .with_service_url("http://test:8080")
            .with_termination(TerminationConfig::new().with_spent_limit("PT10M"))
            .with_environment_mode(EnvironmentMode::NoAssert)
            .with_random_seed(123)
            .with_move_thread_count(MoveThreadCount::Count(4))
            .build::<MockBridge>()
            .unwrap();

        let config = solver.config();
        assert_eq!(config.environment_mode, Some(EnvironmentMode::NoAssert));
        assert_eq!(config.random_seed, Some(123));
        assert_eq!(config.move_thread_count, Some(MoveThreadCount::Count(4)));
    }

    #[test]
    fn test_typed_solver_domain_model() {
        let solver = SolverBuilder::<TestTimetable>::new()
            .build::<MockBridge>()
            .unwrap();

        let model = solver.domain_model();
        assert!(model.get_solution_class().is_some());
    }

    #[test]
    fn test_typed_solver_constraints() {
        let solver = SolverBuilder::<TestTimetable>::new()
            .build::<MockBridge>()
            .unwrap();

        let constraints = solver.constraints();
        assert!(!constraints.is_empty());
    }

    #[test]
    fn test_typed_solver_is_service_available_offline() {
        let solver = SolverBuilder::<TestTimetable>::new()
            .with_service_url("http://localhost:19999")
            .build::<MockBridge>()
            .unwrap();

        // Service at port 19999 should not be available
        assert!(!solver.is_service_available());
    }

    #[test]
    fn test_solver_builder_error_display() {
        let err = SolverBuilderError::WasmGeneration("test error".to_string());
        assert!(err.to_string().contains("WASM module generation failed"));

        let err = SolverBuilderError::InvalidConfiguration("invalid".to_string());
        assert!(err.to_string().contains("Invalid solver configuration"));
    }

    #[test]
    fn test_solver_builder_error_conversion() {
        let err = SolverBuilderError::WasmGeneration("test".to_string());
        let forge_err: SolverForgeError = err.into();
        assert!(matches!(forge_err, SolverForgeError::WasmGeneration(_)));

        let err = SolverBuilderError::InvalidConfiguration("test".to_string());
        let forge_err: SolverForgeError = err.into();
        assert!(matches!(forge_err, SolverForgeError::Configuration(_)));
    }
}
