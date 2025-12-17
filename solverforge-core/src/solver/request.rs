use crate::constraints::StreamComponent;
use crate::solver::TerminationConfig;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Solve request matching solverforge-wasm-service's PlanningProblem schema
/// Uses IndexMap for domain and constraints to preserve insertion order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveRequest {
    pub domain: IndexMap<String, DomainObjectDto>,
    pub constraints: IndexMap<String, Vec<StreamComponent>>,
    pub wasm: String,
    pub allocator: String,
    pub deallocator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_deallocator: Option<String>,
    pub list_accessor: ListAccessorDto,
    pub problem: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfig>,
}

impl SolveRequest {
    pub fn new(
        domain: IndexMap<String, DomainObjectDto>,
        constraints: IndexMap<String, Vec<StreamComponent>>,
        wasm: String,
        allocator: String,
        deallocator: String,
        list_accessor: ListAccessorDto,
        problem: String,
    ) -> Self {
        Self {
            domain,
            constraints,
            wasm,
            allocator,
            deallocator,
            solution_deallocator: None,
            list_accessor,
            problem,
            environment_mode: None,
            termination: None,
        }
    }

    pub fn with_solution_deallocator(mut self, deallocator: impl Into<String>) -> Self {
        self.solution_deallocator = Some(deallocator.into());
        self
    }

    pub fn with_environment_mode(mut self, mode: impl Into<String>) -> Self {
        self.environment_mode = Some(mode.into());
        self
    }

    pub fn with_termination(mut self, termination: TerminationConfig) -> Self {
        self.termination = Some(termination);
        self
    }
}

/// Domain object definition with fields and optional mapper
/// Fields use IndexMap to preserve insertion order, which is critical
/// for correct WASM memory layout alignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainObjectDto {
    pub fields: IndexMap<String, FieldDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapper: Option<DomainObjectMapper>,
}

impl DomainObjectDto {
    pub fn new() -> Self {
        Self {
            fields: IndexMap::new(),
            mapper: None,
        }
    }

    pub fn with_field(mut self, name: impl Into<String>, field: FieldDescriptor) -> Self {
        self.fields.insert(name.into(), field);
        self
    }

    pub fn with_mapper(mut self, mapper: DomainObjectMapper) -> Self {
        self.mapper = Some(mapper);
        self
    }
}

impl Default for DomainObjectDto {
    fn default() -> Self {
        Self::new()
    }
}

/// Field descriptor with type, accessor, and annotations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDescriptor {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessor: Option<DomainAccessor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<PlanningAnnotation>,
}

impl FieldDescriptor {
    pub fn new(field_type: impl Into<String>) -> Self {
        Self {
            field_type: field_type.into(),
            accessor: None,
            annotations: Vec::new(),
        }
    }

    pub fn with_accessor(mut self, accessor: DomainAccessor) -> Self {
        self.accessor = Some(accessor);
        self
    }

    pub fn with_annotation(mut self, annotation: PlanningAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    pub fn with_annotations(mut self, annotations: Vec<PlanningAnnotation>) -> Self {
        self.annotations = annotations;
        self
    }
}

/// Getter/setter accessor for a field
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainAccessor {
    pub getter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setter: Option<String>,
}

impl DomainAccessor {
    pub fn new(getter: impl Into<String>) -> Self {
        Self {
            getter: getter.into(),
            setter: None,
        }
    }

    pub fn with_setter(mut self, setter: impl Into<String>) -> Self {
        self.setter = Some(setter.into());
        self
    }

    pub fn getter_setter(getter: impl Into<String>, setter: impl Into<String>) -> Self {
        Self {
            getter: getter.into(),
            setter: Some(setter.into()),
        }
    }
}

/// Mapper for parsing/serializing solution objects
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainObjectMapper {
    #[serde(rename = "fromString")]
    pub from_string: String,
    #[serde(rename = "toString")]
    pub to_string: String,
}

impl DomainObjectMapper {
    pub fn new(from_string: impl Into<String>, to_string: impl Into<String>) -> Self {
        Self {
            from_string: from_string.into(),
            to_string: to_string.into(),
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Planning annotation types matching Java's PlanningAnnotation hierarchy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "annotation")]
pub enum PlanningAnnotation {
    PlanningVariable {
        #[serde(default, rename = "allowsUnassigned", skip_serializing_if = "is_false")]
        allows_unassigned: bool,
    },
    PlanningId,
    PlanningScore,
    ValueRangeProvider,
    ProblemFactCollectionProperty,
    PlanningEntityCollectionProperty,
}

impl PlanningAnnotation {
    pub fn planning_variable() -> Self {
        PlanningAnnotation::PlanningVariable {
            allows_unassigned: false,
        }
    }

    pub fn planning_variable_allows_unassigned() -> Self {
        PlanningAnnotation::PlanningVariable {
            allows_unassigned: true,
        }
    }

    pub fn planning_id() -> Self {
        PlanningAnnotation::PlanningId
    }

    pub fn planning_score() -> Self {
        PlanningAnnotation::PlanningScore
    }

    pub fn value_range_provider() -> Self {
        PlanningAnnotation::ValueRangeProvider
    }

    pub fn problem_fact_collection_property() -> Self {
        PlanningAnnotation::ProblemFactCollectionProperty
    }

    pub fn planning_entity_collection_property() -> Self {
        PlanningAnnotation::PlanningEntityCollectionProperty
    }
}

/// List accessor for WASM list operations
/// JSON field names match Java's DomainListAccessor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListAccessorDto {
    #[serde(rename = "new")]
    pub create: String,
    #[serde(rename = "get")]
    pub get_item: String,
    #[serde(rename = "set")]
    pub set_item: String,
    #[serde(rename = "length")]
    pub get_size: String,
    pub append: String,
    pub insert: String,
    pub remove: String,
    pub deallocator: String,
}

impl ListAccessorDto {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create: impl Into<String>,
        get_item: impl Into<String>,
        set_item: impl Into<String>,
        get_size: impl Into<String>,
        append: impl Into<String>,
        insert: impl Into<String>,
        remove: impl Into<String>,
        deallocator: impl Into<String>,
    ) -> Self {
        Self {
            create: create.into(),
            get_item: get_item.into(),
            set_item: set_item.into(),
            get_size: get_size.into(),
            append: append.into(),
            insert: insert.into(),
            remove: remove.into(),
            deallocator: deallocator.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_request_new() {
        let request = SolveRequest::new(
            IndexMap::new(),
            IndexMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
            ),
            "{}".to_string(),
        );

        assert_eq!(request.wasm, "AGFzbQ==");
        assert_eq!(request.allocator, "allocate");
        assert!(request.environment_mode.is_none());
    }

    #[test]
    fn test_solve_request_with_options() {
        let termination = TerminationConfig::new().with_spent_limit("PT5M");
        let request = SolveRequest::new(
            IndexMap::new(),
            IndexMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
            ),
            "{}".to_string(),
        )
        .with_environment_mode("FULL_ASSERT")
        .with_termination(termination);

        assert_eq!(request.environment_mode, Some("FULL_ASSERT".to_string()));
        assert!(request.termination.is_some());
    }

    #[test]
    fn test_domain_object_dto_with_fields() {
        let dto = DomainObjectDto::new()
            .with_field(
                "id",
                FieldDescriptor::new("int")
                    .with_accessor(DomainAccessor::new("getId"))
                    .with_annotation(PlanningAnnotation::planning_id()),
            )
            .with_field(
                "employee",
                FieldDescriptor::new("Employee")
                    .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
                    .with_annotation(PlanningAnnotation::planning_variable()),
            );

        assert_eq!(dto.fields.len(), 2);
        assert!(dto.fields.contains_key("id"));
        assert!(dto.fields.contains_key("employee"));
    }

    #[test]
    fn test_domain_object_dto_with_mapper() {
        let dto = DomainObjectDto::new()
            .with_mapper(DomainObjectMapper::new("parseSchedule", "scheduleString"));

        assert!(dto.mapper.is_some());
        let mapper = dto.mapper.unwrap();
        assert_eq!(mapper.from_string, "parseSchedule");
        assert_eq!(mapper.to_string, "scheduleString");
    }

    #[test]
    fn test_field_descriptor() {
        let field = FieldDescriptor::new("Employee")
            .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
            .with_annotation(PlanningAnnotation::PlanningVariable {
                allows_unassigned: false,
            });

        assert_eq!(field.field_type, "Employee");
        assert!(field.accessor.is_some());
        assert_eq!(field.annotations.len(), 1);
    }

    #[test]
    fn test_domain_accessor() {
        let accessor = DomainAccessor::new("getRoom").with_setter("setRoom");
        assert_eq!(accessor.getter, "getRoom");
        assert_eq!(accessor.setter, Some("setRoom".to_string()));

        let accessor2 = DomainAccessor::getter_setter("getRoom", "setRoom");
        assert_eq!(accessor2.getter, "getRoom");
        assert_eq!(accessor2.setter, Some("setRoom".to_string()));
    }

    #[test]
    fn test_planning_annotation_constructors() {
        assert!(matches!(
            PlanningAnnotation::planning_variable(),
            PlanningAnnotation::PlanningVariable {
                allows_unassigned: false
            }
        ));
        assert!(matches!(
            PlanningAnnotation::planning_variable_allows_unassigned(),
            PlanningAnnotation::PlanningVariable {
                allows_unassigned: true
            }
        ));
        assert!(matches!(
            PlanningAnnotation::planning_id(),
            PlanningAnnotation::PlanningId
        ));
        assert!(matches!(
            PlanningAnnotation::planning_score(),
            PlanningAnnotation::PlanningScore
        ));
        assert!(matches!(
            PlanningAnnotation::value_range_provider(),
            PlanningAnnotation::ValueRangeProvider
        ));
    }

    #[test]
    fn test_list_accessor_dto() {
        let accessor = ListAccessorDto::new(
            "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
        );

        assert_eq!(accessor.create, "newList");
        assert_eq!(accessor.get_item, "getItem");
        assert_eq!(accessor.set_item, "setItem");
        assert_eq!(accessor.get_size, "size");
        assert_eq!(accessor.deallocator, "dealloc");
    }

    #[test]
    fn test_solve_request_json_serialization() {
        let mut domain = IndexMap::new();
        domain.insert(
            "Employee".to_string(),
            DomainObjectDto::new().with_field(
                "id",
                FieldDescriptor::new("int")
                    .with_accessor(DomainAccessor::new("getEmployeeId"))
                    .with_annotation(PlanningAnnotation::planning_id()),
            ),
        );

        let request = SolveRequest::new(
            domain,
            IndexMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
            ),
            r#"{"employees": []}"#.to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"domain\""));
        assert!(json.contains("\"listAccessor\""));
        assert!(json.contains("\"type\":\"int\""));
        assert!(json.contains("\"annotation\":\"PlanningId\""));

        let parsed: SolveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn test_solve_request_omits_none_fields() {
        let request = SolveRequest::new(
            IndexMap::new(),
            IndexMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
            ),
            "{}".to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("environmentMode"));
        assert!(!json.contains("termination"));
        assert!(!json.contains("solutionDeallocator"));
    }

    #[test]
    fn test_planning_annotation_json_serialization() {
        let variable = PlanningAnnotation::PlanningVariable {
            allows_unassigned: false,
        };
        let json = serde_json::to_string(&variable).unwrap();
        assert!(json.contains("\"annotation\":\"PlanningVariable\""));
        // allows_unassigned: false should be omitted
        assert!(!json.contains("allowsUnassigned"));

        let variable_unassigned = PlanningAnnotation::PlanningVariable {
            allows_unassigned: true,
        };
        let json2 = serde_json::to_string(&variable_unassigned).unwrap();
        assert!(json2.contains("\"allowsUnassigned\":true"));

        let planning_id = PlanningAnnotation::PlanningId;
        let json3 = serde_json::to_string(&planning_id).unwrap();
        assert!(json3.contains("\"annotation\":\"PlanningId\""));
    }

    #[test]
    fn test_list_accessor_json_field_names() {
        let accessor = ListAccessorDto::new(
            "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
        );

        let json = serde_json::to_string(&accessor).unwrap();
        // Verify Java-compatible field names
        assert!(json.contains("\"new\":\"newList\""));
        assert!(json.contains("\"get\":\"getItem\""));
        assert!(json.contains("\"set\":\"setItem\""));
        assert!(json.contains("\"length\":\"size\""));
        assert!(json.contains("\"append\":\"append\""));
    }

    #[test]
    fn test_domain_object_mapper_json_serialization() {
        let mapper = DomainObjectMapper::new("parseSchedule", "scheduleString");
        let json = serde_json::to_string(&mapper).unwrap();
        assert!(json.contains("\"fromString\":\"parseSchedule\""));
        assert!(json.contains("\"toString\":\"scheduleString\""));
    }

    #[test]
    fn test_field_descriptor_json_serialization() {
        let field = FieldDescriptor::new("Employee")
            .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
            .with_annotation(PlanningAnnotation::planning_variable());

        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("\"type\":\"Employee\""));
        assert!(json.contains("\"getter\":\"getEmployee\""));
        assert!(json.contains("\"setter\":\"setEmployee\""));
        assert!(json.contains("\"annotation\":\"PlanningVariable\""));

        let parsed: FieldDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, field);
    }

    #[test]
    fn test_full_domain_json_structure() {
        // Build a domain matching Java's test case
        let mut domain = IndexMap::new();

        // Employee with PlanningId
        domain.insert(
            "Employee".to_string(),
            DomainObjectDto::new().with_field(
                "id",
                FieldDescriptor::new("int")
                    .with_accessor(DomainAccessor::new("getEmployeeId"))
                    .with_annotation(PlanningAnnotation::planning_id()),
            ),
        );

        // Shift with PlanningVariable
        domain.insert(
            "Shift".to_string(),
            DomainObjectDto::new().with_field(
                "employee",
                FieldDescriptor::new("Employee")
                    .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
                    .with_annotation(PlanningAnnotation::planning_variable()),
            ),
        );

        // Schedule (solution) with collections and score
        domain.insert(
            "Schedule".to_string(),
            DomainObjectDto::new()
                .with_field(
                    "employees",
                    FieldDescriptor::new("Employee[]")
                        .with_accessor(DomainAccessor::getter_setter(
                            "getEmployees",
                            "setEmployees",
                        ))
                        .with_annotation(PlanningAnnotation::problem_fact_collection_property())
                        .with_annotation(PlanningAnnotation::value_range_provider()),
                )
                .with_field(
                    "shifts",
                    FieldDescriptor::new("Shift[]")
                        .with_accessor(DomainAccessor::getter_setter("getShifts", "setShifts"))
                        .with_annotation(PlanningAnnotation::planning_entity_collection_property()),
                )
                .with_field(
                    "score",
                    FieldDescriptor::new("SimpleScore")
                        .with_annotation(PlanningAnnotation::planning_score()),
                )
                .with_mapper(DomainObjectMapper::new("parseSchedule", "scheduleString")),
        );

        let json = serde_json::to_string_pretty(&domain).unwrap();

        // Verify structure
        assert!(json.contains("\"Employee\""));
        assert!(json.contains("\"Shift\""));
        assert!(json.contains("\"Schedule\""));
        // Type is serialized under "type" field
        assert!(json.contains("\"type\": \"int\"") || json.contains("\"type\":\"int\""));
        assert!(json.contains("\"Employee\""));
        assert!(json.contains("\"Employee[]\""));
        assert!(json.contains("\"SimpleScore\""));
        // Annotations use PascalCase for variant names
        assert!(
            json.contains("\"annotation\": \"PlanningId\"")
                || json.contains("\"annotation\":\"PlanningId\"")
        );
        assert!(
            json.contains("\"annotation\": \"PlanningVariable\"")
                || json.contains("\"annotation\":\"PlanningVariable\"")
        );
        assert!(
            json.contains("\"annotation\": \"PlanningScore\"")
                || json.contains("\"annotation\":\"PlanningScore\"")
        );
        assert!(
            json.contains("\"annotation\": \"ValueRangeProvider\"")
                || json.contains("\"annotation\":\"ValueRangeProvider\"")
        );
        assert!(
            json.contains("\"fromString\": \"parseSchedule\"")
                || json.contains("\"fromString\":\"parseSchedule\"")
        );
    }

    #[test]
    fn test_domain_object_dto_clone() {
        let dto = DomainObjectDto::new().with_field(
            "id",
            FieldDescriptor::new("int").with_annotation(PlanningAnnotation::planning_id()),
        );
        let cloned = dto.clone();
        assert_eq!(dto, cloned);
    }

    #[test]
    fn test_domain_object_dto_debug() {
        let dto = DomainObjectDto::new();
        let debug = format!("{:?}", dto);
        assert!(debug.contains("DomainObjectDto"));
    }
}
