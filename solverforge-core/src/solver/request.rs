use crate::constraints::StreamComponent;
use crate::solver::TerminationConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveRequest {
    pub domain: HashMap<String, DomainObjectDto>,
    pub constraints: HashMap<String, Vec<StreamComponent>>,
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
        domain: HashMap<String, DomainObjectDto>,
        constraints: HashMap<String, Vec<StreamComponent>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainObjectDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_entity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_solution: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberDto>,
}

impl DomainObjectDto {
    pub fn new() -> Self {
        Self {
            planning_entity: None,
            planning_solution: None,
            members: Vec::new(),
        }
    }

    pub fn planning_entity() -> Self {
        Self {
            planning_entity: Some(true),
            planning_solution: None,
            members: Vec::new(),
        }
    }

    pub fn planning_solution() -> Self {
        Self {
            planning_entity: None,
            planning_solution: Some(true),
            members: Vec::new(),
        }
    }

    pub fn with_member(mut self, member: MemberDto) -> Self {
        self.members.push(member);
        self
    }

    pub fn with_members(mut self, members: Vec<MemberDto>) -> Self {
        self.members = members;
        self
    }
}

impl Default for DomainObjectDto {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberDto {
    pub name: String,
    pub getter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_variable: Option<PlanningVariableDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_list_variable: Option<PlanningListVariableDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_id: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_score: Option<PlanningScoreDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_range_provider: Option<ValueRangeProviderDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_fact_collection_property: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_entity_collection_property: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverse_relation_shadow_variable: Option<InverseRelationShadowDto>,
}

impl MemberDto {
    pub fn new(name: impl Into<String>, getter: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            getter: getter.into(),
            setter: None,
            planning_variable: None,
            planning_list_variable: None,
            planning_id: None,
            planning_score: None,
            value_range_provider: None,
            problem_fact_collection_property: None,
            planning_entity_collection_property: None,
            inverse_relation_shadow_variable: None,
        }
    }

    pub fn with_setter(mut self, setter: impl Into<String>) -> Self {
        self.setter = Some(setter.into());
        self
    }

    pub fn with_planning_variable(mut self, variable: PlanningVariableDto) -> Self {
        self.planning_variable = Some(variable);
        self
    }

    pub fn with_planning_list_variable(mut self, variable: PlanningListVariableDto) -> Self {
        self.planning_list_variable = Some(variable);
        self
    }

    pub fn with_planning_id(mut self) -> Self {
        self.planning_id = Some(true);
        self
    }

    pub fn with_planning_score(mut self, score: PlanningScoreDto) -> Self {
        self.planning_score = Some(score);
        self
    }

    pub fn with_value_range_provider(mut self, provider: ValueRangeProviderDto) -> Self {
        self.value_range_provider = Some(provider);
        self
    }

    pub fn with_problem_fact_collection_property(mut self) -> Self {
        self.problem_fact_collection_property = Some(true);
        self
    }

    pub fn with_planning_entity_collection_property(mut self) -> Self {
        self.planning_entity_collection_property = Some(true);
        self
    }

    pub fn with_inverse_relation_shadow_variable(
        mut self,
        shadow: InverseRelationShadowDto,
    ) -> Self {
        self.inverse_relation_shadow_variable = Some(shadow);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningVariableDto {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_range_provider_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_unassigned: Option<bool>,
}

impl PlanningVariableDto {
    pub fn new() -> Self {
        Self {
            value_range_provider_refs: Vec::new(),
            allows_unassigned: None,
        }
    }

    pub fn with_value_range_provider_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.value_range_provider_refs.push(ref_id.into());
        self
    }

    pub fn with_allows_unassigned(mut self, allows: bool) -> Self {
        self.allows_unassigned = Some(allows);
        self
    }
}

impl Default for PlanningVariableDto {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningListVariableDto {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_range_provider_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_unassigned_values: Option<bool>,
}

impl PlanningListVariableDto {
    pub fn new() -> Self {
        Self {
            value_range_provider_refs: Vec::new(),
            allows_unassigned_values: None,
        }
    }

    pub fn with_value_range_provider_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.value_range_provider_refs.push(ref_id.into());
        self
    }

    pub fn with_allows_unassigned_values(mut self, allows: bool) -> Self {
        self.allows_unassigned_values = Some(allows);
        self
    }
}

impl Default for PlanningListVariableDto {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningScoreDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bendable_hard_levels_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bendable_soft_levels_size: Option<usize>,
}

impl PlanningScoreDto {
    pub fn new() -> Self {
        Self {
            bendable_hard_levels_size: None,
            bendable_soft_levels_size: None,
        }
    }

    pub fn bendable(hard_levels: usize, soft_levels: usize) -> Self {
        Self {
            bendable_hard_levels_size: Some(hard_levels),
            bendable_soft_levels_size: Some(soft_levels),
        }
    }
}

impl Default for PlanningScoreDto {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueRangeProviderDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl ValueRangeProviderDto {
    pub fn new() -> Self {
        Self { id: None }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl Default for ValueRangeProviderDto {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InverseRelationShadowDto {
    pub source_variable_name: String,
}

impl InverseRelationShadowDto {
    pub fn new(source_variable_name: impl Into<String>) -> Self {
        Self {
            source_variable_name: source_variable_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAccessorDto {
    pub create: String,
    pub get_item: String,
    pub set_item: String,
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
            HashMap::new(),
            HashMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "create_list",
                "get_item",
                "set_item",
                "get_size",
                "append",
                "insert",
                "remove",
                "deallocate_list",
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
            HashMap::new(),
            HashMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "create", "get", "set", "size", "append", "insert", "remove", "free",
            ),
            "{}".to_string(),
        )
        .with_environment_mode("FULL_ASSERT")
        .with_termination(termination);

        assert_eq!(request.environment_mode, Some("FULL_ASSERT".to_string()));
        assert!(request.termination.is_some());
    }

    #[test]
    fn test_domain_object_dto_planning_entity() {
        let dto = DomainObjectDto::planning_entity().with_member(
            MemberDto::new("room", "getRoom").with_planning_variable(
                PlanningVariableDto::new().with_value_range_provider_ref("roomRange"),
            ),
        );

        assert_eq!(dto.planning_entity, Some(true));
        assert!(dto.planning_solution.is_none());
        assert_eq!(dto.members.len(), 1);
    }

    #[test]
    fn test_domain_object_dto_planning_solution() {
        let dto = DomainObjectDto::planning_solution();
        assert!(dto.planning_entity.is_none());
        assert_eq!(dto.planning_solution, Some(true));
    }

    #[test]
    fn test_member_dto_builder() {
        let member = MemberDto::new("id", "getId")
            .with_setter("setId")
            .with_planning_id();

        assert_eq!(member.name, "id");
        assert_eq!(member.getter, "getId");
        assert_eq!(member.setter, Some("setId".to_string()));
        assert_eq!(member.planning_id, Some(true));
    }

    #[test]
    fn test_planning_variable_dto() {
        let variable = PlanningVariableDto::new()
            .with_value_range_provider_ref("roomRange")
            .with_value_range_provider_ref("timeslotRange")
            .with_allows_unassigned(true);

        assert_eq!(variable.value_range_provider_refs.len(), 2);
        assert_eq!(variable.allows_unassigned, Some(true));
    }

    #[test]
    fn test_planning_score_dto_bendable() {
        let score = PlanningScoreDto::bendable(3, 2);
        assert_eq!(score.bendable_hard_levels_size, Some(3));
        assert_eq!(score.bendable_soft_levels_size, Some(2));
    }

    #[test]
    fn test_value_range_provider_dto() {
        let provider = ValueRangeProviderDto::new().with_id("roomRange");
        assert_eq!(provider.id, Some("roomRange".to_string()));
    }

    #[test]
    fn test_inverse_relation_shadow_dto() {
        let shadow = InverseRelationShadowDto::new("lessons");
        assert_eq!(shadow.source_variable_name, "lessons");
    }

    #[test]
    fn test_list_accessor_dto() {
        let accessor = ListAccessorDto::new(
            "create_list",
            "get_item",
            "set_item",
            "get_size",
            "append",
            "insert",
            "remove",
            "deallocate_list",
        );

        assert_eq!(accessor.create, "create_list");
        assert_eq!(accessor.get_item, "get_item");
        assert_eq!(accessor.deallocator, "deallocate_list");
    }

    #[test]
    fn test_solve_request_json_serialization() {
        let mut domain = HashMap::new();
        domain.insert(
            "Lesson".to_string(),
            DomainObjectDto::planning_entity()
                .with_member(MemberDto::new("id", "getId").with_planning_id()),
        );

        let request = SolveRequest::new(
            domain,
            HashMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "create", "get", "set", "size", "append", "insert", "remove", "free",
            ),
            r#"{"lessons": []}"#.to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"domain\""));
        assert!(json.contains("\"listAccessor\""));
        assert!(json.contains("\"planningEntity\":true"));

        let parsed: SolveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn test_solve_request_omits_none_fields() {
        let request = SolveRequest::new(
            HashMap::new(),
            HashMap::new(),
            "AGFzbQ==".to_string(),
            "allocate".to_string(),
            "deallocate".to_string(),
            ListAccessorDto::new(
                "create", "get", "set", "size", "append", "insert", "remove", "free",
            ),
            "{}".to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("environmentMode"));
        assert!(!json.contains("termination"));
        assert!(!json.contains("solutionDeallocator"));
    }

    #[test]
    fn test_member_dto_json_serialization() {
        let member = MemberDto::new("room", "getRoom")
            .with_setter("setRoom")
            .with_planning_variable(
                PlanningVariableDto::new()
                    .with_value_range_provider_ref("roomRange")
                    .with_allows_unassigned(false),
            );

        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("\"name\":\"room\""));
        assert!(json.contains("\"getter\":\"getRoom\""));
        assert!(json.contains("\"setter\":\"setRoom\""));
        assert!(json.contains("\"planningVariable\""));
        assert!(json.contains("\"valueRangeProviderRefs\":[\"roomRange\"]"));

        let parsed: MemberDto = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, member);
    }

    #[test]
    fn test_domain_object_dto_clone() {
        let dto = DomainObjectDto::planning_entity()
            .with_member(MemberDto::new("id", "getId").with_planning_id());
        let cloned = dto.clone();
        assert_eq!(dto, cloned);
    }

    #[test]
    fn test_domain_object_dto_debug() {
        let dto = DomainObjectDto::planning_entity();
        let debug = format!("{:?}", dto);
        assert!(debug.contains("DomainObjectDto"));
    }
}
