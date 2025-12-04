use super::DomainClass;
use crate::SolverForgeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainModel {
    pub classes: HashMap<String, DomainClass>,
    pub solution_class: Option<String>,
    pub entity_classes: Vec<String>,
}

impl DomainModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> DomainModelBuilder {
        DomainModelBuilder::new()
    }

    pub fn get_class(&self, name: &str) -> Option<&DomainClass> {
        self.classes.get(name)
    }

    pub fn get_solution_class(&self) -> Option<&DomainClass> {
        self.solution_class
            .as_ref()
            .and_then(|name| self.classes.get(name))
    }

    pub fn get_entity_classes(&self) -> impl Iterator<Item = &DomainClass> {
        self.entity_classes
            .iter()
            .filter_map(|name| self.classes.get(name))
    }

    pub fn solution_class(&self) -> Option<&str> {
        self.solution_class.as_deref()
    }

    pub fn to_dto(&self) -> std::collections::HashMap<String, crate::solver::DomainObjectDto> {
        use crate::domain::PlanningAnnotation;
        use crate::solver::{
            DomainObjectDto, MemberDto, PlanningListVariableDto, PlanningScoreDto,
            PlanningVariableDto, ValueRangeProviderDto,
        };

        let mut result = std::collections::HashMap::new();

        for (name, class) in &self.classes {
            let mut dto = if class.is_planning_solution() {
                DomainObjectDto::planning_solution()
            } else if class.is_planning_entity() {
                DomainObjectDto::planning_entity()
            } else {
                DomainObjectDto::new()
            };

            for field in &class.fields {
                let getter = field
                    .accessor
                    .as_ref()
                    .map(|a| a.getter.clone())
                    .unwrap_or_else(|| format!("get_{}", field.name));
                let setter = field.accessor.as_ref().map(|a| a.setter.clone());

                let mut member = MemberDto::new(&field.name, getter);
                if let Some(s) = setter {
                    member = member.with_setter(s);
                }

                for ann in &field.planning_annotations {
                    match ann {
                        PlanningAnnotation::PlanningId => {
                            member = member.with_planning_id();
                        }
                        PlanningAnnotation::PlanningVariable {
                            value_range_provider_refs,
                            allows_unassigned,
                        } => {
                            let mut var_dto = PlanningVariableDto::new();
                            for ref_id in value_range_provider_refs {
                                var_dto = var_dto.with_value_range_provider_ref(ref_id);
                            }
                            if *allows_unassigned {
                                var_dto = var_dto.with_allows_unassigned(true);
                            }
                            member = member.with_planning_variable(var_dto);
                        }
                        PlanningAnnotation::PlanningListVariable {
                            value_range_provider_refs,
                        } => {
                            let mut var_dto = PlanningListVariableDto::new();
                            for ref_id in value_range_provider_refs {
                                var_dto = var_dto.with_value_range_provider_ref(ref_id);
                            }
                            member = member.with_planning_list_variable(var_dto);
                        }
                        PlanningAnnotation::PlanningScore {
                            bendable_hard_levels,
                            bendable_soft_levels,
                        } => {
                            let score_dto = if bendable_hard_levels.is_some()
                                || bendable_soft_levels.is_some()
                            {
                                PlanningScoreDto::bendable(
                                    bendable_hard_levels.unwrap_or(0),
                                    bendable_soft_levels.unwrap_or(0),
                                )
                            } else {
                                PlanningScoreDto::new()
                            };
                            member = member.with_planning_score(score_dto);
                        }
                        PlanningAnnotation::ValueRangeProvider { id } => {
                            let mut provider = ValueRangeProviderDto::new();
                            if let Some(id) = id {
                                provider = provider.with_id(id);
                            }
                            member = member.with_value_range_provider(provider);
                        }
                        PlanningAnnotation::ProblemFactCollectionProperty => {
                            member = member.with_problem_fact_collection_property();
                        }
                        PlanningAnnotation::PlanningEntityCollectionProperty => {
                            member = member.with_planning_entity_collection_property();
                        }
                        _ => {}
                    }
                }

                dto = dto.with_member(member);
            }

            result.insert(name.clone(), dto);
        }

        result
    }

    pub fn validate(&self) -> Result<(), SolverForgeError> {
        if self.solution_class.is_none() {
            return Err(SolverForgeError::Validation(
                "Domain model must have a solution class".to_string(),
            ));
        }

        let solution_name = self.solution_class.as_ref().unwrap();
        let solution = self.classes.get(solution_name).ok_or_else(|| {
            SolverForgeError::Validation(format!(
                "Solution class '{}' not found in domain model",
                solution_name
            ))
        })?;

        if !solution.is_planning_solution() {
            return Err(SolverForgeError::Validation(format!(
                "Class '{}' is marked as solution but lacks @PlanningSolution annotation",
                solution_name
            )));
        }

        if solution.get_score_field().is_none() {
            return Err(SolverForgeError::Validation(format!(
                "Solution class '{}' must have a @PlanningScore field",
                solution_name
            )));
        }

        if self.entity_classes.is_empty() {
            return Err(SolverForgeError::Validation(
                "Domain model must have at least one entity class".to_string(),
            ));
        }

        for entity_name in &self.entity_classes {
            let entity = self.classes.get(entity_name).ok_or_else(|| {
                SolverForgeError::Validation(format!(
                    "Entity class '{}' not found in domain model",
                    entity_name
                ))
            })?;

            if !entity.is_planning_entity() {
                return Err(SolverForgeError::Validation(format!(
                    "Class '{}' is marked as entity but lacks @PlanningEntity annotation",
                    entity_name
                )));
            }

            let has_variable = entity.get_planning_variables().next().is_some();
            if !has_variable {
                return Err(SolverForgeError::Validation(format!(
                    "Entity class '{}' must have at least one @PlanningVariable",
                    entity_name
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DomainModelBuilder {
    classes: HashMap<String, DomainClass>,
    solution_class: Option<String>,
    entity_classes: Vec<String>,
}

impl DomainModelBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_class(mut self, class: DomainClass) -> Self {
        let name = class.name.clone();

        if class.is_planning_solution() {
            self.solution_class = Some(name.clone());
        }

        if class.is_planning_entity() {
            self.entity_classes.push(name.clone());
        }

        self.classes.insert(name, class);
        self
    }

    pub fn with_solution(mut self, class_name: impl Into<String>) -> Self {
        self.solution_class = Some(class_name.into());
        self
    }

    pub fn with_entity(mut self, class_name: impl Into<String>) -> Self {
        self.entity_classes.push(class_name.into());
        self
    }

    pub fn solution_class(self, class_name: impl Into<String>) -> Self {
        self.with_solution(class_name)
    }

    pub fn entity_class(self, class_name: impl Into<String>) -> Self {
        self.with_entity(class_name)
    }

    pub fn build(self) -> DomainModel {
        DomainModel {
            classes: self.classes,
            solution_class: self.solution_class,
            entity_classes: self.entity_classes,
        }
    }

    pub fn build_validated(self) -> Result<DomainModel, SolverForgeError> {
        let model = self.build();
        model.validate()?;
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FieldDescriptor, FieldType, PlanningAnnotation, ScoreType};

    fn create_lesson_entity() -> DomainClass {
        DomainClass::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(
                FieldDescriptor::new(
                    "id",
                    FieldType::Primitive(crate::domain::PrimitiveType::String),
                )
                .with_planning_annotation(PlanningAnnotation::PlanningId),
            )
            .with_field(
                FieldDescriptor::new("room", FieldType::object("Room")).with_planning_annotation(
                    PlanningAnnotation::planning_variable(vec!["rooms".to_string()]),
                ),
            )
    }

    fn create_timetable_solution() -> DomainClass {
        DomainClass::new("Timetable")
            .with_annotation(PlanningAnnotation::PlanningSolution)
            .with_field(
                FieldDescriptor::new("lessons", FieldType::list(FieldType::object("Lesson")))
                    .with_planning_annotation(PlanningAnnotation::PlanningEntityCollectionProperty),
            )
            .with_field(
                FieldDescriptor::new("rooms", FieldType::list(FieldType::object("Room")))
                    .with_planning_annotation(PlanningAnnotation::value_range_provider("rooms")),
            )
            .with_field(
                FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                    .with_planning_annotation(PlanningAnnotation::planning_score()),
            )
    }

    #[test]
    fn test_builder_basic() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .add_class(create_timetable_solution())
            .build();

        assert_eq!(model.classes.len(), 2);
        assert_eq!(model.solution_class, Some("Timetable".to_string()));
        assert_eq!(model.entity_classes, vec!["Lesson"]);
    }

    #[test]
    fn test_get_class() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .build();

        assert!(model.get_class("Lesson").is_some());
        assert!(model.get_class("Unknown").is_none());
    }

    #[test]
    fn test_get_solution_class() {
        let model = DomainModel::builder()
            .add_class(create_timetable_solution())
            .build();

        let solution = model.get_solution_class().unwrap();
        assert_eq!(solution.name, "Timetable");
    }

    #[test]
    fn test_get_entity_classes() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .build();

        let entities: Vec<_> = model.get_entity_classes().collect();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Lesson");
    }

    #[test]
    fn test_validate_success() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .add_class(create_timetable_solution())
            .build();

        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_validate_no_solution() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .build();

        let err = model.validate().unwrap_err();
        assert!(err.to_string().contains("solution class"));
    }

    #[test]
    fn test_validate_no_entities() {
        let model = DomainModel::builder()
            .add_class(create_timetable_solution())
            .build();

        let err = model.validate().unwrap_err();
        assert!(err.to_string().contains("entity class"));
    }

    #[test]
    fn test_validate_solution_without_score() {
        let solution =
            DomainClass::new("Timetable").with_annotation(PlanningAnnotation::PlanningSolution);

        let model = DomainModel::builder()
            .add_class(solution)
            .add_class(create_lesson_entity())
            .build();

        let err = model.validate().unwrap_err();
        assert!(err.to_string().contains("@PlanningScore"));
    }

    #[test]
    fn test_validate_entity_without_variable() {
        let entity = DomainClass::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(
                FieldDescriptor::new(
                    "id",
                    FieldType::Primitive(crate::domain::PrimitiveType::String),
                )
                .with_planning_annotation(PlanningAnnotation::PlanningId),
            );

        let model = DomainModel::builder()
            .add_class(entity)
            .add_class(create_timetable_solution())
            .build();

        let err = model.validate().unwrap_err();
        assert!(err.to_string().contains("@PlanningVariable"));
    }

    #[test]
    fn test_build_validated() {
        let result = DomainModel::builder()
            .add_class(create_lesson_entity())
            .add_class(create_timetable_solution())
            .build_validated();

        assert!(result.is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let model = DomainModel::builder()
            .add_class(create_lesson_entity())
            .add_class(create_timetable_solution())
            .build();

        let json = serde_json::to_string(&model).unwrap();
        let parsed: DomainModel = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.classes.len(), model.classes.len());
        assert_eq!(parsed.solution_class, model.solution_class);
    }
}
