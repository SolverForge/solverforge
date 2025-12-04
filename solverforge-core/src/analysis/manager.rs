use crate::analysis::ScoreExplanation;
use crate::bridge::LanguageBridge;
use crate::constraints::ConstraintSet;
use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::solver::{
    ListAccessorDto, ScoreDto, SolveRequest, SolveResponse, Solver, SolverConfig, SolverService,
};
use crate::ObjectHandle;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct SolutionManager<B: LanguageBridge> {
    config: SolverConfig,
    service: Arc<dyn SolverService>,
    domain_model: DomainModel,
    constraints: ConstraintSet,
    wasm_module: String,
    _bridge: PhantomData<B>,
}

impl<B: LanguageBridge> SolutionManager<B> {
    pub fn create(solver: &Solver<B>) -> Self {
        Self {
            config: solver.config().clone(),
            service: solver.service().clone(),
            domain_model: solver.domain_model().clone(),
            constraints: solver.constraints().clone(),
            wasm_module: solver.wasm_module().to_string(),
            _bridge: PhantomData,
        }
    }

    pub fn new(
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

    pub fn explain(
        &self,
        bridge: &B,
        solution: ObjectHandle,
    ) -> SolverForgeResult<ScoreExplanation> {
        let request = self.build_explain_request(bridge, solution)?;
        let response = self.service.solve(&request)?;
        self.parse_explanation(response)
    }

    pub fn update(&self, bridge: &B, solution: ObjectHandle) -> SolverForgeResult<ScoreDto> {
        let request = self.build_update_request(bridge, solution)?;
        let response = self.service.solve(&request)?;
        Ok(parse_score_string(&response.score))
    }

    fn build_explain_request(
        &self,
        bridge: &B,
        solution: ObjectHandle,
    ) -> SolverForgeResult<SolveRequest> {
        let solution_json = bridge.serialize_object(solution).map_err(|e| {
            SolverForgeError::Bridge(format!("Failed to serialize solution: {}", e))
        })?;

        let domain_dto = self.domain_model.to_dto();
        let constraints_dto = self.constraints.to_dto();

        let list_accessor = ListAccessorDto::new(
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
            solution_json,
        );

        // Set step limit to 0 for explain (just calculate score)
        if let Some(termination) = self.config.termination.as_ref() {
            request = request.with_termination(termination.clone());
        }

        if let Some(mode) = &self.config.environment_mode {
            request = request.with_environment_mode(format!("{:?}", mode).to_uppercase());
        }

        Ok(request)
    }

    fn build_update_request(
        &self,
        bridge: &B,
        solution: ObjectHandle,
    ) -> SolverForgeResult<SolveRequest> {
        self.build_explain_request(bridge, solution)
    }

    fn parse_explanation(&self, response: SolveResponse) -> SolverForgeResult<ScoreExplanation> {
        // Parse the score string into a ScoreDto
        // Score format: "0" for simple, "0hard/-5soft" for hard/soft
        let score = parse_score_string(&response.score);
        Ok(ScoreExplanation::new(score))
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
}

/// Parse a score string from Java into a ScoreDto.
fn parse_score_string(s: &str) -> ScoreDto {
    // Try to parse as hard/soft first: "0hard/-5soft"
    if let Some((hard_str, rest)) = s.split_once("hard/") {
        if let Some((soft_str, _)) = rest.split_once("soft") {
            if let (Ok(hard), Ok(soft)) = (hard_str.parse(), soft_str.parse()) {
                return ScoreDto::hard_soft(hard, soft);
            }
        }
        // Try hard/medium/soft: "0hard/-10medium/-5soft"
        if let Some((medium_str, rest2)) = rest.split_once("medium/") {
            if let Some((soft_str, _)) = rest2.split_once("soft") {
                if let (Ok(hard), Ok(medium), Ok(soft)) =
                    (hard_str.parse(), medium_str.parse(), soft_str.parse())
                {
                    return ScoreDto::hard_medium_soft(hard, medium, soft);
                }
            }
        }
    }
    // Try to parse as simple score
    if let Ok(score) = s.parse() {
        return ScoreDto::simple(score);
    }
    // Fallback to 0
    ScoreDto::simple(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{ConstraintMatch, Indictment};
    use crate::bridge::tests::MockBridge;
    use crate::constraints::Constraint;
    use crate::domain::{DomainModelBuilder, FieldDescriptor, FieldType, PrimitiveType, ScoreType};
    use crate::solver::HttpSolverService;

    fn create_test_config() -> SolverConfig {
        SolverConfig::new().with_solution_class("Timetable")
    }

    fn create_test_domain() -> DomainModel {
        use crate::domain::{DomainClass, PlanningAnnotation};

        DomainModelBuilder::new()
            .add_class(
                DomainClass::new("Timetable")
                    .with_annotation(PlanningAnnotation::PlanningSolution)
                    .with_field(
                        FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                            .with_planning_annotation(PlanningAnnotation::planning_score()),
                    ),
            )
            .add_class(
                DomainClass::new("Lesson")
                    .with_annotation(PlanningAnnotation::PlanningEntity)
                    .with_field(
                        FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                            .with_planning_annotation(PlanningAnnotation::PlanningId),
                    )
                    .with_field(
                        FieldDescriptor::new("room", FieldType::object("Room"))
                            .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                                "rooms".to_string(),
                            ])),
                    ),
            )
            .build()
    }

    fn create_test_constraints() -> ConstraintSet {
        ConstraintSet::new().with_constraint(Constraint::new("testConstraint"))
    }

    #[test]
    fn test_solution_manager_new() {
        let service = Arc::new(HttpSolverService::new("http://localhost:8080"));

        let manager = SolutionManager::<MockBridge>::new(
            create_test_config(),
            service,
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        assert_eq!(
            manager.config().solution_class,
            Some("Timetable".to_string())
        );
        assert_eq!(manager.constraints().len(), 1);
    }

    #[test]
    fn test_solution_manager_accessors() {
        let service = Arc::new(HttpSolverService::new("http://localhost:8080"));

        let manager = SolutionManager::<MockBridge>::new(
            create_test_config(),
            service,
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        assert!(manager.domain_model().classes.contains_key("Timetable"));
        assert!(manager.domain_model().classes.contains_key("Lesson"));
    }

    #[test]
    fn test_parse_explanation() {
        let service = Arc::new(HttpSolverService::new("http://localhost:8080"));

        let manager = SolutionManager::<MockBridge>::new(
            create_test_config(),
            service,
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        let response = SolveResponse::new("{}".to_string(), "-2hard/-15soft");

        let explanation = manager.parse_explanation(response).unwrap();

        assert_eq!(explanation.hard_score(), -2);
        assert_eq!(explanation.soft_score(), -15);
        assert!(!explanation.is_feasible());
    }

    #[test]
    fn test_parse_feasible_explanation() {
        let service = Arc::new(HttpSolverService::new("http://localhost:8080"));

        let manager = SolutionManager::<MockBridge>::new(
            create_test_config(),
            service,
            create_test_domain(),
            create_test_constraints(),
            "AGFzbQ==".to_string(),
        );

        let response = SolveResponse::new("{}".to_string(), "0hard/-5soft");

        let explanation = manager.parse_explanation(response).unwrap();

        assert!(explanation.is_feasible());
    }

    #[test]
    fn test_constraint_match_creation() {
        let cm = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0))
            .with_package("com.example")
            .with_indicted_object(ObjectHandle::new(1));

        assert_eq!(cm.full_constraint_name(), "com.example.roomConflict");
        assert_eq!(cm.indicted_objects.len(), 1);
    }

    #[test]
    fn test_indictment_creation() {
        let obj = ObjectHandle::new(42);
        let cm = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));

        let indictment = Indictment::new(obj, ScoreDto::hard_soft(-1, 0)).with_constraint_match(cm);

        assert_eq!(indictment.indicted_object, obj);
        assert_eq!(indictment.constraint_count(), 1);
    }

    #[test]
    fn test_score_explanation_builder() {
        let explanation = ScoreExplanation::new(ScoreDto::hard_soft(-3, -20))
            .with_constraint_match(ConstraintMatch::new(
                "conflict1",
                ScoreDto::hard_soft(-2, 0),
            ))
            .with_constraint_match(ConstraintMatch::new(
                "conflict2",
                ScoreDto::hard_soft(-1, -20),
            ))
            .with_indictment(Indictment::new(
                ObjectHandle::new(1),
                ScoreDto::hard_soft(-2, 0),
            ));

        assert_eq!(explanation.constraint_count(), 2);
        assert_eq!(explanation.indictments.len(), 1);
    }
}
