use super::DomainClass;
use crate::SolverForgeError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainModel {
    pub classes: IndexMap<String, DomainClass>,
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

    pub fn to_dto(&self) -> indexmap::IndexMap<String, crate::solver::DomainObjectDto> {
        use crate::domain::PlanningAnnotation as DomainAnnotation;
        use crate::solver::{
            DomainAccessor, DomainObjectDto, DomainObjectMapper, FieldDescriptor,
            PlanningAnnotation as SolverAnnotation,
        };

        let mut result = indexmap::IndexMap::new();

        for (name, class) in &self.classes {
            let mut dto = DomainObjectDto::new();

            for field in &class.fields {
                // Generate accessor names that match WasmModuleBuilder exports:
                // get_{Class}_{field} and set_{Class}_{field}
                let (getter, setter) = if let Some(a) = &field.accessor {
                    // Use explicitly defined accessor
                    (a.getter.clone(), Some(a.setter.clone()))
                } else {
                    // Generate defaults
                    let getter = format!("get_{}_{}", name, field.name);
                    // Generate setter for fields that need to be modified:
                    // - PlanningVariable: solver assigns values
                    // - PlanningListVariable: solver modifies lists
                    // - ProblemFactCollectionProperty: solution class collections
                    // - PlanningEntityCollectionProperty: solution class entity collections
                    let setter = if field.planning_annotations.iter().any(|a| {
                        matches!(
                            a,
                            DomainAnnotation::PlanningVariable { .. }
                                | DomainAnnotation::PlanningListVariable { .. }
                                | DomainAnnotation::ProblemFactCollectionProperty
                                | DomainAnnotation::PlanningEntityCollectionProperty
                        )
                    }) {
                        Some(format!("set_{}_{}", name, field.name))
                    } else {
                        None
                    };
                    (getter, setter)
                };

                let accessor = if let Some(s) = setter {
                    DomainAccessor::getter_setter(getter, s)
                } else {
                    DomainAccessor::new(getter)
                };

                // Convert domain annotations to solver annotations
                let mut annotations = Vec::new();
                for ann in &field.planning_annotations {
                    match ann {
                        DomainAnnotation::PlanningId => {
                            annotations.push(SolverAnnotation::PlanningId);
                        }
                        DomainAnnotation::PlanningVariable {
                            allows_unassigned, ..
                        } => {
                            annotations.push(SolverAnnotation::PlanningVariable {
                                allows_unassigned: *allows_unassigned,
                            });
                        }
                        DomainAnnotation::PlanningListVariable { .. } => {
                            // List variables use the same PlanningVariable annotation
                            annotations.push(SolverAnnotation::PlanningVariable {
                                allows_unassigned: false,
                            });
                        }
                        DomainAnnotation::PlanningScore { .. } => {
                            annotations.push(SolverAnnotation::PlanningScore);
                        }
                        DomainAnnotation::ValueRangeProvider { .. } => {
                            annotations.push(SolverAnnotation::ValueRangeProvider);
                        }
                        DomainAnnotation::ProblemFactCollectionProperty => {
                            annotations.push(SolverAnnotation::ProblemFactCollectionProperty);
                        }
                        DomainAnnotation::PlanningEntityCollectionProperty => {
                            annotations.push(SolverAnnotation::PlanningEntityCollectionProperty);
                        }
                        _ => {}
                    }
                }

                // Derive field type from domain type
                let field_type = field.field_type.to_type_string();

                let field_descriptor = FieldDescriptor::new(field_type)
                    .with_accessor(accessor)
                    .with_annotations(annotations);

                dto = dto.with_field(&field.name, field_descriptor);
            }

            // Add mapper for solution class (PlanningSolution)
            // Uses parseSchedule/scheduleString which are exported by WasmModuleBuilder
            if class.is_planning_solution() {
                dto = dto.with_mapper(DomainObjectMapper::new("parseSchedule", "scheduleString"));
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
    classes: IndexMap<String, DomainClass>,
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
