//! SolverManager for managing multiple concurrent solves.
//!
//! Provides a high-level API for managing multiple planning problems simultaneously,
//! similar to Timefold's SolverManager.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use crate::error::{SolverForgeError, SolverForgeResult};
use crate::solver::{
    ListAccessorDto, SolveHandle, SolveRequest, SolveState, SolverConfig, SolverService,
    TerminationConfig,
};
use crate::traits::PlanningSolution;
use crate::wasm::{Expression, HostFunctionRegistry, WasmModuleBuilder};

/// Manages multiple concurrent solves for planning problems.
///
/// Each solve is identified by a unique `ProblemId` that allows tracking
/// and managing individual solves independently.
///
/// # Example
///
/// ```ignore
/// let service = Arc::new(HttpSolverService::new("http://localhost:8080"));
/// let mut manager = SolverManager::<Timetable, String>::new(service)
///     .with_termination(TerminationConfig::new().with_spent_limit("PT5M"));
///
/// manager.solve("problem-1".to_string(), problem1)?;
/// manager.solve("problem-2".to_string(), problem2)?;
///
/// // Check solutions later
/// if let Some(solution) = manager.get_best_solution(&"problem-1".to_string())? {
///     println!("Best score: {:?}", solution.score());
/// }
///
/// manager.terminate_all();
/// ```
pub struct SolverManager<S: PlanningSolution, ProblemId: Eq + Hash + Clone> {
    active_solves: HashMap<ProblemId, ManagedSolve<S>>,
    service: Arc<dyn SolverService>,
    config: SolverConfig,
    /// Cascading update expressions: (class_name, field_name, expression)
    cascading_expressions: Vec<(String, String, Expression)>,
}

struct ManagedSolve<S: PlanningSolution> {
    handle: SolveHandle,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, I> SolverManager<S, I>
where
    S: PlanningSolution + Clone,
    I: Eq + Hash + Clone,
{
    /// Creates a new SolverManager with the given solver service.
    pub fn new(service: Arc<dyn SolverService>) -> Self {
        Self {
            active_solves: HashMap::new(),
            service,
            config: SolverConfig::default(),
            cascading_expressions: Vec::new(),
        }
    }

    /// Sets the solver configuration.
    pub fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the termination configuration.
    pub fn with_termination(mut self, termination: TerminationConfig) -> Self {
        self.config.termination = Some(termination);
        self
    }

    /// Registers a cascading update expression for a shadow variable.
    ///
    /// This expression will be compiled to WASM and called by the solver
    /// when the shadow variable needs to be recomputed.
    ///
    /// # Arguments
    /// * `class_name` - The entity class name (e.g., "Visit")
    /// * `field_name` - The field with the cascading update shadow variable
    /// * `expression` - The expression to compute the shadow value
    pub fn with_cascading_expression(
        mut self,
        class_name: impl Into<String>,
        field_name: impl Into<String>,
        expression: Expression,
    ) -> Self {
        self.cascading_expressions
            .push((class_name.into(), field_name.into(), expression));
        self
    }

    /// Starts solving a problem with the given ID.
    ///
    /// Returns an error if a solve with this ID is already in progress.
    pub fn solve(&mut self, id: I, problem: S) -> SolverForgeResult<()> {
        if self.active_solves.contains_key(&id) {
            return Err(SolverForgeError::Solver(
                "Solve already in progress for this problem ID".into(),
            ));
        }

        // Build request using the same logic as TypedSolver
        let mut domain_model = S::domain_model();
        let constraints = S::constraints();

        // Apply registered cascading update expressions to the domain model
        for (class_name, field_name, expression) in &self.cascading_expressions {
            domain_model.set_cascading_expression(class_name, field_name, expression.clone())?;
        }

        // Extract predicates from constraints (expressions that need to be compiled to WASM)
        let predicates = constraints.extract_predicates();

        // Build WASM module (with standard host functions for list operations)
        let mut builder = WasmModuleBuilder::new()
            .with_host_functions(HostFunctionRegistry::with_standard_functions())
            .with_domain_model(domain_model.clone());

        // Add all constraint predicates to the WASM module
        for predicate in predicates {
            builder = builder.add_predicate(predicate);
        }

        let wasm_base64 = builder.build_base64()?;

        // Build the solve request (same as TypedSolver::solve)
        let domain_dto = domain_model.to_dto();
        let constraints_dto = constraints.to_dto();
        let problem_json = problem.to_json()?;

        let list_accessor = ListAccessorDto::new(
            "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
        );

        let mut request = SolveRequest::new(
            domain_dto,
            constraints_dto,
            wasm_base64,
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

        // Start async solve
        let handle = self.service.solve_async(&request)?;
        self.active_solves.insert(
            id,
            ManagedSolve {
                handle,
                _phantom: std::marker::PhantomData,
            },
        );

        Ok(())
    }

    /// Gets the best solution for a problem, if available.
    pub fn get_best_solution(&self, id: &I) -> SolverForgeResult<Option<S>> {
        let managed = self
            .active_solves
            .get(id)
            .ok_or_else(|| SolverForgeError::Solver("No solve found for this problem ID".into()))?;

        // Try to get the best solution from the handle
        if let Ok(Some(response)) = self.service.get_best_solution(&managed.handle) {
            let parsed = S::from_json(&response.solution)?;
            return Ok(Some(parsed));
        }

        // No solution available yet
        Ok(None)
    }

    /// Terminates a solve early.
    pub fn terminate(&mut self, id: &I) -> SolverForgeResult<()> {
        let managed = self
            .active_solves
            .get(id)
            .ok_or_else(|| SolverForgeError::Solver("No solve found for this problem ID".into()))?;

        self.service.stop(&managed.handle)?;
        self.active_solves.remove(id);

        Ok(())
    }

    /// Terminates all active solves.
    pub fn terminate_all(&mut self) {
        let ids: Vec<_> = self.active_solves.keys().cloned().collect();
        for id in ids {
            let _ = self.terminate(&id);
        }
    }

    /// Checks if a solve is currently in progress for the given ID.
    pub fn is_solving(&self, id: &I) -> bool {
        if let Some(managed) = self.active_solves.get(id) {
            if let Ok(status) = self.service.get_status(&managed.handle) {
                return status.state == SolveState::Running;
            }
        }
        false
    }

    /// Gets the number of active solves.
    pub fn active_solve_count(&self) -> usize {
        self.active_solves.len()
    }

    /// Removes completed solves from tracking.
    pub fn cleanup_completed(&mut self) -> SolverForgeResult<()> {
        let mut completed = Vec::new();

        for (id, managed) in &self.active_solves {
            let status = self.service.get_status(&managed.handle)?;
            if status.state != SolveState::Running {
                completed.push(id.clone());
            }
        }

        for id in completed {
            self.active_solves.remove(&id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_solver_manager_types_compile() {
        // Verify the generic types work correctly
        // Actual tests require a mock SolverService
    }
}
