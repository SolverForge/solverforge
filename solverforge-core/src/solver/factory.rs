use crate::bridge::LanguageBridge;
use crate::constraints::ConstraintSet;
use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::solver::{
    HttpSolverService, SolveHandle, SolveRequest, SolveResponse, SolveStatus, SolverConfig,
    SolverService,
};
use crate::ObjectHandle;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct SolverFactory<B: LanguageBridge> {
    config: SolverConfig,
    service: Arc<dyn SolverService>,
    domain_model: DomainModel,
    constraints: ConstraintSet,
    wasm_module: String,
    _bridge: PhantomData<B>,
}

impl<B: LanguageBridge> SolverFactory<B> {
    pub fn create(
        config: SolverConfig,
        service_url: impl Into<String>,
        domain_model: DomainModel,
        constraints: ConstraintSet,
        wasm_module: String,
    ) -> Self {
        let service = Arc::new(HttpSolverService::new(service_url));
        Self {
            config,
            service,
            domain_model,
            constraints,
            wasm_module,
            _bridge: PhantomData,
        }
    }

    pub fn with_service(
        config: SolverConfig,
        service: Arc<dyn SolverService>,
        domain_model: DomainModel,
        constraints: ConstraintSet,
        wasm_module: String,
    ) -> Self {
        Self {
            config,
            service,
            domain_model,
            constraints,
            wasm_module,
            _bridge: PhantomData,
        }
    }

    pub fn build_solver(&self, bridge: Arc<B>) -> Solver<B> {
        Solver {
            config: self.config.clone(),
            service: self.service.clone(),
            domain_model: self.domain_model.clone(),
            constraints: self.constraints.clone(),
            wasm_module: self.wasm_module.clone(),
            bridge,
        }
    }

    pub fn config(&self) -> &SolverConfig {
        &self.config
    }

    pub fn domain_model(&self) -> &DomainModel {
        &self.domain_model
    }

    pub fn constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    pub fn is_service_available(&self) -> bool {
        self.service.is_available()
    }
}

pub struct Solver<B: LanguageBridge> {
    config: SolverConfig,
    service: Arc<dyn SolverService>,
    domain_model: DomainModel,
    constraints: ConstraintSet,
    wasm_module: String,
    bridge: Arc<B>,
}

impl<B: LanguageBridge> Solver<B> {
    pub fn solve(&self, problem: ObjectHandle) -> SolverForgeResult<SolveResponse> {
        let request = self.build_request(problem)?;
        self.service.solve(&request)
    }

    pub fn solve_async(&self, problem: ObjectHandle) -> SolverForgeResult<SolveHandle> {
        let request = self.build_request(problem)?;
        self.service.solve_async(&request)
    }

    pub fn get_status(&self, handle: &SolveHandle) -> SolverForgeResult<SolveStatus> {
        self.service.get_status(handle)
    }

    pub fn get_best_solution(
        &self,
        handle: &SolveHandle,
    ) -> SolverForgeResult<Option<SolveResponse>> {
        self.service.get_best_solution(handle)
    }

    pub fn stop(&self, handle: &SolveHandle) -> SolverForgeResult<()> {
        self.service.stop(handle)
    }

    pub fn config(&self) -> &SolverConfig {
        &self.config
    }

    pub fn bridge(&self) -> &Arc<B> {
        &self.bridge
    }

    pub fn service(&self) -> &Arc<dyn SolverService> {
        &self.service
    }

    pub fn domain_model(&self) -> &DomainModel {
        &self.domain_model
    }

    pub fn constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    pub fn wasm_module(&self) -> &str {
        &self.wasm_module
    }

    fn build_request(&self, problem: ObjectHandle) -> SolverForgeResult<SolveRequest> {
        let problem_json = self
            .bridge
            .serialize_object(problem)
            .map_err(|e| SolverForgeError::Bridge(format!("Failed to serialize problem: {}", e)))?;

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

        let mut request = SolveRequest::new(
            domain_dto,
            constraints_dto,
            self.wasm_module.clone(),
            "allocate".to_string(),
            "deallocate".to_string(),
            list_accessor,
            problem_json,
        );

        if let Some(mode) = &self.config.environment_mode {
            request = request.with_environment_mode(format!("{:?}", mode).to_uppercase());
        }

        if let Some(termination) = &self.config.termination {
            request = request.with_termination(termination.clone());
        }

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::tests::MockBridge;
    use crate::constraints::Constraint;
    use crate::domain::DomainModelBuilder;

    fn create_test_domain() -> DomainModel {
        DomainModelBuilder::new()
            .solution_class("Timetable")
            .entity_class("Lesson")
            .build()
    }

    fn create_test_constraints() -> ConstraintSet {
        ConstraintSet::new().with_constraint(Constraint::new("testConstraint"))
    }

    #[test]
    fn test_solver_factory_create() {
        let factory = SolverFactory::<MockBridge>::create(
            SolverConfig::new(),
            "http://localhost:8080",
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        assert!(factory.config().solution_class.is_none());
        assert_eq!(factory.domain_model().solution_class(), Some("Timetable"));
        assert_eq!(factory.constraints().len(), 1);
    }

    #[test]
    fn test_solver_factory_build_solver() {
        let factory = SolverFactory::create(
            SolverConfig::new().with_solution_class("Timetable"),
            "http://localhost:8080",
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        let bridge = Arc::new(MockBridge::new());
        let solver = factory.build_solver(bridge);

        assert_eq!(
            solver.config().solution_class,
            Some("Timetable".to_string())
        );
    }

    #[test]
    fn test_solver_factory_is_service_available_offline() {
        let factory = SolverFactory::<MockBridge>::create(
            SolverConfig::new(),
            "http://localhost:19999",
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        assert!(!factory.is_service_available());
    }

    #[test]
    fn test_solver_config_access() {
        let config = SolverConfig::new()
            .with_solution_class("Timetable")
            .with_random_seed(42);

        let factory = SolverFactory::<MockBridge>::create(
            config,
            "http://localhost:8080",
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        assert_eq!(factory.config().random_seed, Some(42));
    }

    #[test]
    fn test_solver_bridge_access() {
        let factory = SolverFactory::create(
            SolverConfig::new(),
            "http://localhost:8080",
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        let bridge = Arc::new(MockBridge::new());
        let solver = factory.build_solver(bridge.clone());

        assert!(Arc::ptr_eq(solver.bridge(), &bridge));
    }
}
