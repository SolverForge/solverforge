use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlanningAnnotation {
    PlanningId,
    PlanningEntity,
    PlanningSolution,
    PlanningVariable {
        #[serde(default)]
        value_range_provider_refs: Vec<String>,
        #[serde(default)]
        allows_unassigned: bool,
    },
    PlanningListVariable {
        #[serde(default)]
        value_range_provider_refs: Vec<String>,
    },
    PlanningScore {
        #[serde(default)]
        bendable_hard_levels: Option<usize>,
        #[serde(default)]
        bendable_soft_levels: Option<usize>,
    },
    ValueRangeProvider {
        #[serde(default)]
        id: Option<String>,
    },
    ProblemFactProperty,
    ProblemFactCollectionProperty,
    PlanningEntityProperty,
    PlanningEntityCollectionProperty,
    PlanningPin,
    InverseRelationShadowVariable {
        source_variable_name: String,
    },
}

impl PlanningAnnotation {
    pub fn planning_variable(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningVariable {
            value_range_provider_refs,
            allows_unassigned: false,
        }
    }

    pub fn planning_variable_unassigned(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningVariable {
            value_range_provider_refs,
            allows_unassigned: true,
        }
    }

    pub fn planning_list_variable(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningListVariable {
            value_range_provider_refs,
        }
    }

    pub fn planning_score() -> Self {
        PlanningAnnotation::PlanningScore {
            bendable_hard_levels: None,
            bendable_soft_levels: None,
        }
    }

    pub fn planning_score_bendable(hard_levels: usize, soft_levels: usize) -> Self {
        PlanningAnnotation::PlanningScore {
            bendable_hard_levels: Some(hard_levels),
            bendable_soft_levels: Some(soft_levels),
        }
    }

    pub fn value_range_provider(id: impl Into<String>) -> Self {
        PlanningAnnotation::ValueRangeProvider {
            id: Some(id.into()),
        }
    }

    pub fn inverse_relation_shadow(source_variable_name: impl Into<String>) -> Self {
        PlanningAnnotation::InverseRelationShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn is_planning_variable(&self) -> bool {
        matches!(self, PlanningAnnotation::PlanningVariable { .. })
    }

    pub fn is_planning_list_variable(&self) -> bool {
        matches!(self, PlanningAnnotation::PlanningListVariable { .. })
    }

    pub fn is_any_variable(&self) -> bool {
        self.is_planning_variable() || self.is_planning_list_variable()
    }

    pub fn is_shadow_variable(&self) -> bool {
        matches!(
            self,
            PlanningAnnotation::InverseRelationShadowVariable { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_id() {
        let ann = PlanningAnnotation::PlanningId;
        assert_eq!(ann, PlanningAnnotation::PlanningId);
    }

    #[test]
    fn test_planning_variable() {
        let ann = PlanningAnnotation::planning_variable(vec!["rooms".to_string()]);
        match ann {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                allows_unassigned,
            } => {
                assert_eq!(value_range_provider_refs, vec!["rooms"]);
                assert!(!allows_unassigned);
            }
            _ => panic!("Expected PlanningVariable"),
        }
    }

    #[test]
    fn test_planning_variable_unassigned() {
        let ann = PlanningAnnotation::planning_variable_unassigned(vec!["slots".to_string()]);
        match ann {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                allows_unassigned,
            } => {
                assert_eq!(value_range_provider_refs, vec!["slots"]);
                assert!(allows_unassigned);
            }
            _ => panic!("Expected PlanningVariable"),
        }
    }

    #[test]
    fn test_planning_list_variable() {
        let ann = PlanningAnnotation::planning_list_variable(vec!["tasks".to_string()]);
        match ann {
            PlanningAnnotation::PlanningListVariable {
                value_range_provider_refs,
            } => {
                assert_eq!(value_range_provider_refs, vec!["tasks"]);
            }
            _ => panic!("Expected PlanningListVariable"),
        }
    }

    #[test]
    fn test_planning_score() {
        let ann = PlanningAnnotation::planning_score();
        match ann {
            PlanningAnnotation::PlanningScore {
                bendable_hard_levels,
                bendable_soft_levels,
            } => {
                assert!(bendable_hard_levels.is_none());
                assert!(bendable_soft_levels.is_none());
            }
            _ => panic!("Expected PlanningScore"),
        }
    }

    #[test]
    fn test_planning_score_bendable() {
        let ann = PlanningAnnotation::planning_score_bendable(2, 3);
        match ann {
            PlanningAnnotation::PlanningScore {
                bendable_hard_levels,
                bendable_soft_levels,
            } => {
                assert_eq!(bendable_hard_levels, Some(2));
                assert_eq!(bendable_soft_levels, Some(3));
            }
            _ => panic!("Expected PlanningScore"),
        }
    }

    #[test]
    fn test_value_range_provider() {
        let ann = PlanningAnnotation::value_range_provider("timeslots");
        match ann {
            PlanningAnnotation::ValueRangeProvider { id } => {
                assert_eq!(id, Some("timeslots".to_string()));
            }
            _ => panic!("Expected ValueRangeProvider"),
        }
    }

    #[test]
    fn test_inverse_relation_shadow() {
        let ann = PlanningAnnotation::inverse_relation_shadow("visits");
        match ann {
            PlanningAnnotation::InverseRelationShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "visits");
            }
            _ => panic!("Expected InverseRelationShadowVariable"),
        }
    }

    #[test]
    fn test_is_planning_variable() {
        let var = PlanningAnnotation::planning_variable(vec![]);
        assert!(var.is_planning_variable());
        assert!(var.is_any_variable());
        assert!(!var.is_planning_list_variable());
        assert!(!var.is_shadow_variable());
    }

    #[test]
    fn test_is_planning_list_variable() {
        let var = PlanningAnnotation::planning_list_variable(vec![]);
        assert!(!var.is_planning_variable());
        assert!(var.is_any_variable());
        assert!(var.is_planning_list_variable());
        assert!(!var.is_shadow_variable());
    }

    #[test]
    fn test_is_shadow_variable() {
        let shadow = PlanningAnnotation::inverse_relation_shadow("test");
        assert!(shadow.is_shadow_variable());
        assert!(!shadow.is_any_variable());
    }

    #[test]
    fn test_json_serialization_planning_id() {
        let ann = PlanningAnnotation::PlanningId;
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("\"type\":\"PlanningId\""));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_serialization_planning_variable() {
        let ann =
            PlanningAnnotation::planning_variable(vec!["rooms".to_string(), "slots".to_string()]);
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("\"type\":\"PlanningVariable\""));
        assert!(json.contains("\"value_range_provider_refs\""));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_serialization_planning_score_bendable() {
        let ann = PlanningAnnotation::planning_score_bendable(2, 3);
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("\"bendable_hard_levels\":2"));
        assert!(json.contains("\"bendable_soft_levels\":3"));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_deserialization_defaults() {
        let json = r#"{"type":"PlanningVariable"}"#;
        let parsed: PlanningAnnotation = serde_json::from_str(json).unwrap();
        match parsed {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                allows_unassigned,
            } => {
                assert!(value_range_provider_refs.is_empty());
                assert!(!allows_unassigned);
            }
            _ => panic!("Expected PlanningVariable"),
        }
    }

    #[test]
    fn test_simple_annotations() {
        let annotations = vec![
            PlanningAnnotation::PlanningEntity,
            PlanningAnnotation::PlanningSolution,
            PlanningAnnotation::ProblemFactProperty,
            PlanningAnnotation::ProblemFactCollectionProperty,
            PlanningAnnotation::PlanningEntityProperty,
            PlanningAnnotation::PlanningEntityCollectionProperty,
            PlanningAnnotation::PlanningPin,
        ];

        for ann in annotations {
            let json = serde_json::to_string(&ann).unwrap();
            let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, ann);
        }
    }
}
