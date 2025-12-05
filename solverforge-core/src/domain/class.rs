use super::{PlanningAnnotation, ShadowAnnotation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainClass {
    pub name: String,
    #[serde(default)]
    pub annotations: Vec<PlanningAnnotation>,
    #[serde(default)]
    pub fields: Vec<FieldDescriptor>,
}

impl DomainClass {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            annotations: Vec::new(),
            fields: Vec::new(),
        }
    }

    pub fn with_annotation(mut self, annotation: PlanningAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    pub fn with_field(mut self, field: FieldDescriptor) -> Self {
        self.fields.push(field);
        self
    }

    pub fn is_planning_entity(&self) -> bool {
        self.annotations
            .iter()
            .any(|a| matches!(a, PlanningAnnotation::PlanningEntity))
    }

    pub fn is_planning_solution(&self) -> bool {
        self.annotations
            .iter()
            .any(|a| matches!(a, PlanningAnnotation::PlanningSolution))
    }

    pub fn get_planning_variables(&self) -> impl Iterator<Item = &FieldDescriptor> {
        self.fields.iter().filter(|f| f.is_planning_variable())
    }

    pub fn get_planning_id_field(&self) -> Option<&FieldDescriptor> {
        self.fields
            .iter()
            .find(|f| f.has_annotation(|a| matches!(a, PlanningAnnotation::PlanningId)))
    }

    pub fn get_score_field(&self) -> Option<&FieldDescriptor> {
        self.fields
            .iter()
            .find(|f| f.has_annotation(|a| matches!(a, PlanningAnnotation::PlanningScore { .. })))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDescriptor {
    pub name: String,
    pub field_type: FieldType,
    #[serde(default)]
    pub planning_annotations: Vec<PlanningAnnotation>,
    #[serde(default)]
    pub shadow_annotations: Vec<ShadowAnnotation>,
    #[serde(default)]
    pub accessor: Option<DomainAccessor>,
}

impl FieldDescriptor {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            planning_annotations: Vec::new(),
            shadow_annotations: Vec::new(),
            accessor: None,
        }
    }

    pub fn with_planning_annotation(mut self, annotation: PlanningAnnotation) -> Self {
        self.planning_annotations.push(annotation);
        self
    }

    pub fn with_shadow_annotation(mut self, annotation: ShadowAnnotation) -> Self {
        self.shadow_annotations.push(annotation);
        self
    }

    pub fn with_accessor(mut self, accessor: DomainAccessor) -> Self {
        self.accessor = Some(accessor);
        self
    }

    pub fn is_planning_variable(&self) -> bool {
        self.planning_annotations
            .iter()
            .any(|a| a.is_any_variable())
    }

    pub fn is_shadow_variable(&self) -> bool {
        !self.shadow_annotations.is_empty()
    }

    pub fn has_annotation<F>(&self, predicate: F) -> bool
    where
        F: Fn(&PlanningAnnotation) -> bool,
    {
        self.planning_annotations.iter().any(predicate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainAccessor {
    pub getter: String,
    pub setter: String,
}

impl DomainAccessor {
    pub fn new(getter: impl Into<String>, setter: impl Into<String>) -> Self {
        Self {
            getter: getter.into(),
            setter: setter.into(),
        }
    }

    pub fn from_field_name(field_name: &str) -> Self {
        let capitalized = capitalize_first(field_name);
        Self {
            getter: format!("get{}", capitalized),
            setter: format!("set{}", capitalized),
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FieldType {
    Primitive(PrimitiveType),
    Object {
        class_name: String,
    },
    Array {
        element_type: Box<FieldType>,
    },
    List {
        element_type: Box<FieldType>,
    },
    Set {
        element_type: Box<FieldType>,
    },
    Map {
        key_type: Box<FieldType>,
        value_type: Box<FieldType>,
    },
    Score(ScoreType),
}

impl FieldType {
    pub fn object(class_name: impl Into<String>) -> Self {
        FieldType::Object {
            class_name: class_name.into(),
        }
    }

    pub fn array(element_type: FieldType) -> Self {
        FieldType::Array {
            element_type: Box::new(element_type),
        }
    }

    pub fn list(element_type: FieldType) -> Self {
        FieldType::List {
            element_type: Box::new(element_type),
        }
    }

    pub fn set(element_type: FieldType) -> Self {
        FieldType::Set {
            element_type: Box::new(element_type),
        }
    }

    pub fn map(key_type: FieldType, value_type: FieldType) -> Self {
        FieldType::Map {
            key_type: Box::new(key_type),
            value_type: Box::new(value_type),
        }
    }

    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            FieldType::Array { .. } | FieldType::List { .. } | FieldType::Set { .. }
        )
    }

    /// Returns a Java-compatible type string for the field type
    pub fn to_type_string(&self) -> String {
        match self {
            FieldType::Primitive(p) => match p {
                PrimitiveType::Bool => "boolean".to_string(),
                PrimitiveType::Int => "int".to_string(),
                PrimitiveType::Long => "long".to_string(),
                PrimitiveType::Float => "float".to_string(),
                PrimitiveType::Double => "double".to_string(),
                PrimitiveType::String => "String".to_string(),
                PrimitiveType::Date => "LocalDate".to_string(),
                PrimitiveType::DateTime => "LocalDateTime".to_string(),
            },
            FieldType::Object { class_name } => class_name.clone(),
            FieldType::Array { element_type } => format!("{}[]", element_type.to_type_string()),
            FieldType::List { element_type } => format!("{}[]", element_type.to_type_string()),
            FieldType::Set { element_type } => format!("{}[]", element_type.to_type_string()),
            FieldType::Map {
                key_type,
                value_type,
            } => {
                format!(
                    "Map<{}, {}>",
                    key_type.to_type_string(),
                    value_type.to_type_string()
                )
            }
            FieldType::Score(s) => match s {
                ScoreType::Simple => "SimpleScore".to_string(),
                ScoreType::HardSoft => "HardSoftScore".to_string(),
                ScoreType::HardMediumSoft => "HardMediumSoftScore".to_string(),
                ScoreType::SimpleDecimal => "SimpleBigDecimalScore".to_string(),
                ScoreType::HardSoftDecimal => "HardSoftBigDecimalScore".to_string(),
                ScoreType::HardMediumSoftDecimal => "HardMediumSoftBigDecimalScore".to_string(),
                ScoreType::Bendable { .. } => "BendableScore".to_string(),
                ScoreType::BendableDecimal { .. } => "BendableBigDecimalScore".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Bool,
    Int,
    Long,
    Float,
    Double,
    String,
    Date,     // LocalDate - stored as epoch day (i64)
    DateTime, // LocalDateTime - stored as epoch second (i64)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreType {
    Simple,
    HardSoft,
    HardMediumSoft,
    SimpleDecimal,
    HardSoftDecimal,
    HardMediumSoftDecimal,
    Bendable {
        hard_levels: usize,
        soft_levels: usize,
    },
    BendableDecimal {
        hard_levels: usize,
        soft_levels: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_class_new() {
        let class = DomainClass::new("Lesson");
        assert_eq!(class.name, "Lesson");
        assert!(class.annotations.is_empty());
        assert!(class.fields.is_empty());
    }

    #[test]
    fn test_domain_class_builder() {
        let class = DomainClass::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(
                FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                    .with_planning_annotation(PlanningAnnotation::PlanningId),
            )
            .with_field(
                FieldDescriptor::new("room", FieldType::object("Room")).with_planning_annotation(
                    PlanningAnnotation::planning_variable(vec!["rooms".to_string()]),
                ),
            );

        assert!(class.is_planning_entity());
        assert!(!class.is_planning_solution());
        assert_eq!(class.get_planning_variables().count(), 1);
        assert!(class.get_planning_id_field().is_some());
    }

    #[test]
    fn test_field_descriptor() {
        let field = FieldDescriptor::new("room", FieldType::object("Room"))
            .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                "rooms".to_string()
            ]))
            .with_accessor(DomainAccessor::from_field_name("room"));

        assert!(field.is_planning_variable());
        assert!(!field.is_shadow_variable());
        assert!(field.accessor.is_some());

        let accessor = field.accessor.unwrap();
        assert_eq!(accessor.getter, "getRoom");
        assert_eq!(accessor.setter, "setRoom");
    }

    #[test]
    fn test_shadow_field() {
        let field = FieldDescriptor::new("vehicle", FieldType::object("Vehicle"))
            .with_shadow_annotation(ShadowAnnotation::inverse_relation("visits"));

        assert!(!field.is_planning_variable());
        assert!(field.is_shadow_variable());
    }

    #[test]
    fn test_field_type_object() {
        let ft = FieldType::object("Room");
        match ft {
            FieldType::Object { class_name } => assert_eq!(class_name, "Room"),
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_field_type_collection() {
        let list = FieldType::list(FieldType::object("Lesson"));
        assert!(list.is_collection());

        let obj = FieldType::object("Room");
        assert!(!obj.is_collection());
    }

    #[test]
    fn test_field_type_nested() {
        let map = FieldType::map(
            FieldType::Primitive(PrimitiveType::String),
            FieldType::list(FieldType::object("Lesson")),
        );

        match map {
            FieldType::Map {
                key_type,
                value_type,
            } => {
                assert!(matches!(
                    *key_type,
                    FieldType::Primitive(PrimitiveType::String)
                ));
                assert!(matches!(*value_type, FieldType::List { .. }));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_score_type() {
        let bendable = ScoreType::Bendable {
            hard_levels: 2,
            soft_levels: 3,
        };
        match bendable {
            ScoreType::Bendable {
                hard_levels,
                soft_levels,
            } => {
                assert_eq!(hard_levels, 2);
                assert_eq!(soft_levels, 3);
            }
            _ => panic!("Expected Bendable"),
        }
    }

    #[test]
    fn test_domain_accessor_from_field() {
        let accessor = DomainAccessor::from_field_name("timeslot");
        assert_eq!(accessor.getter, "getTimeslot");
        assert_eq!(accessor.setter, "setTimeslot");
    }

    #[test]
    fn test_solution_class() {
        let solution = DomainClass::new("Timetable")
            .with_annotation(PlanningAnnotation::PlanningSolution)
            .with_field(
                FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                    .with_planning_annotation(PlanningAnnotation::planning_score()),
            );

        assert!(solution.is_planning_solution());
        assert!(solution.get_score_field().is_some());
    }

    #[test]
    fn test_json_serialization() {
        let class = DomainClass::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(
                FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                    .with_planning_annotation(PlanningAnnotation::PlanningId),
            );

        let json = serde_json::to_string(&class).unwrap();
        let parsed: DomainClass = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, class.name);
        assert_eq!(parsed.fields.len(), class.fields.len());
    }
}
