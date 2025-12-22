//! Planning annotations for domain model fields.
//!
//! These annotations match the Timefold solver annotations 1:1 and are serialized
//! to JSON for the solver service using the exact format Java expects.

use serde::{Deserialize, Serialize};

/// Helper for skip_serializing_if on boolean fields
fn is_false(b: &bool) -> bool {
    !*b
}

/// Helper for skip_serializing_if on empty Vec fields
fn is_empty_vec(v: &[String]) -> bool {
    v.is_empty()
}

/// Helper for skip_serializing_if on None Option fields
fn is_none<T>(opt: &Option<T>) -> bool {
    opt.is_none()
}

/// Planning annotations for domain fields.
///
/// Matches Timefold annotations 1:1:
/// - PlanningVariable: valueRangeProviderRefs[], allowsUnassigned
/// - PlanningListVariable: valueRangeProviderRefs[], allowsUnassignedValues
/// - ValueRangeProvider: id
/// - PlanningScore: bendableHardLevelsSize, bendableSoftLevelsSize
/// - InverseRelationShadowVariable: sourceVariableName
///
/// Serialization matches the Java solver service expectations exactly:
/// - Tag: `"annotation"` (not `"type"`)
/// - Field names use camelCase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "annotation")]
pub enum PlanningAnnotation {
    /// Marks a field as the unique identifier for a planning entity.
    PlanningId,

    /// Marks a class as a planning entity that the solver can change.
    /// Used at class level in DomainClass.
    PlanningEntity,

    /// Marks a class as the planning solution containing entities and problem facts.
    /// Used at class level in DomainClass.
    PlanningSolution,

    /// Marks a field as a planning variable that the solver assigns.
    /// Matches Timefold's @PlanningVariable annotation.
    PlanningVariable {
        /// References to ValueRangeProvider ids that provide values for this variable.
        /// If empty, auto-detection by type is used.
        #[serde(
            default,
            rename = "valueRangeProviderRefs",
            skip_serializing_if = "is_empty_vec"
        )]
        value_range_provider_refs: Vec<String>,

        /// If true, null is a valid value (variable can be unassigned).
        #[serde(default, rename = "allowsUnassigned", skip_serializing_if = "is_false")]
        allows_unassigned: bool,
    },

    /// Marks a field as a list planning variable for list-based assignment.
    /// Matches Timefold's @PlanningListVariable annotation.
    PlanningListVariable {
        /// References to ValueRangeProvider ids that provide values for this variable.
        /// If empty, auto-detection by type is used.
        #[serde(
            default,
            rename = "valueRangeProviderRefs",
            skip_serializing_if = "is_empty_vec"
        )]
        value_range_provider_refs: Vec<String>,

        /// If true, elements can remain unassigned (not in any list).
        #[serde(
            default,
            rename = "allowsUnassignedValues",
            skip_serializing_if = "is_false"
        )]
        allows_unassigned_values: bool,
    },

    /// Marks a field as the score of the solution.
    /// Matches Timefold's @PlanningScore annotation.
    PlanningScore {
        /// Number of hard levels for bendable scores.
        #[serde(
            default,
            rename = "bendable_hard_levels",
            skip_serializing_if = "is_none"
        )]
        bendable_hard_levels: Option<i32>,

        /// Number of soft levels for bendable scores.
        #[serde(
            default,
            rename = "bendable_soft_levels",
            skip_serializing_if = "is_none"
        )]
        bendable_soft_levels: Option<i32>,
    },

    /// Marks a collection as providing values for planning variables.
    /// Matches Timefold's @ValueRangeProvider annotation.
    ValueRangeProvider {
        /// The id used by PlanningVariable.valueRangeProviderRefs to reference this provider.
        /// If empty, auto-detection by type is used.
        #[serde(default, skip_serializing_if = "is_none")]
        id: Option<String>,
    },

    /// Marks a field as a problem fact (immutable input data).
    ProblemFactProperty,

    /// Marks a collection field as containing problem facts.
    ProblemFactCollectionProperty,

    /// Marks a field as containing a single planning entity.
    PlanningEntityProperty,

    /// Marks a collection field as containing planning entities.
    PlanningEntityCollectionProperty,

    /// Marks a field as pinned (solver won't change it).
    PlanningPin,

    /// Shadow variable that tracks the inverse of a list variable relationship.
    /// Matches Timefold's @InverseRelationShadowVariable annotation.
    InverseRelationShadowVariable {
        /// The name of the source variable on the other side of the relationship.
        #[serde(rename = "source_variable_name")]
        source_variable_name: String,
    },
}

impl PlanningAnnotation {
    // === PlanningId ===

    /// Creates a PlanningId annotation.
    pub fn planning_id() -> Self {
        PlanningAnnotation::PlanningId
    }

    // === PlanningVariable ===

    /// Creates a PlanningVariable annotation with default settings.
    pub fn planning_variable(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningVariable {
            value_range_provider_refs,
            allows_unassigned: false,
        }
    }

    /// Creates a PlanningVariable annotation that allows unassigned values.
    pub fn planning_variable_unassigned(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningVariable {
            value_range_provider_refs,
            allows_unassigned: true,
        }
    }

    /// Creates a PlanningVariable with full control over all fields.
    pub fn planning_variable_full(
        value_range_provider_refs: Vec<String>,
        allows_unassigned: bool,
    ) -> Self {
        PlanningAnnotation::PlanningVariable {
            value_range_provider_refs,
            allows_unassigned,
        }
    }

    // === PlanningListVariable ===

    /// Creates a PlanningListVariable annotation with default settings.
    pub fn planning_list_variable(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningListVariable {
            value_range_provider_refs,
            allows_unassigned_values: false,
        }
    }

    /// Creates a PlanningListVariable annotation that allows unassigned values.
    pub fn planning_list_variable_unassigned(value_range_provider_refs: Vec<String>) -> Self {
        PlanningAnnotation::PlanningListVariable {
            value_range_provider_refs,
            allows_unassigned_values: true,
        }
    }

    /// Creates a PlanningListVariable with full control over all fields.
    pub fn planning_list_variable_full(
        value_range_provider_refs: Vec<String>,
        allows_unassigned_values: bool,
    ) -> Self {
        PlanningAnnotation::PlanningListVariable {
            value_range_provider_refs,
            allows_unassigned_values,
        }
    }

    // === PlanningScore ===

    /// Creates a PlanningScore annotation for non-bendable scores.
    pub fn planning_score() -> Self {
        PlanningAnnotation::PlanningScore {
            bendable_hard_levels: None,
            bendable_soft_levels: None,
        }
    }

    /// Creates a PlanningScore annotation for bendable scores.
    pub fn planning_score_bendable(hard_levels: i32, soft_levels: i32) -> Self {
        PlanningAnnotation::PlanningScore {
            bendable_hard_levels: Some(hard_levels),
            bendable_soft_levels: Some(soft_levels),
        }
    }

    // === ValueRangeProvider ===

    /// Creates a ValueRangeProvider annotation without an explicit id.
    pub fn value_range_provider() -> Self {
        PlanningAnnotation::ValueRangeProvider { id: None }
    }

    /// Creates a ValueRangeProvider annotation with an explicit id.
    pub fn value_range_provider_with_id(id: impl Into<String>) -> Self {
        PlanningAnnotation::ValueRangeProvider {
            id: Some(id.into()),
        }
    }

    // === Shadow Variables ===

    /// Creates an InverseRelationShadowVariable annotation.
    pub fn inverse_relation_shadow(source_variable_name: impl Into<String>) -> Self {
        PlanningAnnotation::InverseRelationShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    // === Collection Properties ===

    /// Creates a ProblemFactProperty annotation.
    pub fn problem_fact_property() -> Self {
        PlanningAnnotation::ProblemFactProperty
    }

    /// Creates a ProblemFactCollectionProperty annotation.
    pub fn problem_fact_collection_property() -> Self {
        PlanningAnnotation::ProblemFactCollectionProperty
    }

    /// Creates a PlanningEntityProperty annotation.
    pub fn planning_entity_property() -> Self {
        PlanningAnnotation::PlanningEntityProperty
    }

    /// Creates a PlanningEntityCollectionProperty annotation.
    pub fn planning_entity_collection_property() -> Self {
        PlanningAnnotation::PlanningEntityCollectionProperty
    }

    // === Query Methods ===

    /// Returns true if this is a PlanningVariable annotation.
    pub fn is_planning_variable(&self) -> bool {
        matches!(self, PlanningAnnotation::PlanningVariable { .. })
    }

    /// Returns true if this is a PlanningListVariable annotation.
    pub fn is_planning_list_variable(&self) -> bool {
        matches!(self, PlanningAnnotation::PlanningListVariable { .. })
    }

    /// Returns true if this is any kind of planning variable (regular or list).
    pub fn is_any_variable(&self) -> bool {
        self.is_planning_variable() || self.is_planning_list_variable()
    }

    /// Returns true if this is a shadow variable annotation.
    pub fn is_shadow_variable(&self) -> bool {
        matches!(
            self,
            PlanningAnnotation::InverseRelationShadowVariable { .. }
        )
    }

    /// Returns the value_range_provider_refs if this is a planning variable.
    pub fn value_range_provider_refs(&self) -> Option<&Vec<String>> {
        match self {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                ..
            } => Some(value_range_provider_refs),
            PlanningAnnotation::PlanningListVariable {
                value_range_provider_refs,
                ..
            } => Some(value_range_provider_refs),
            _ => None,
        }
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
        let ann = PlanningAnnotation::planning_variable_unassigned(vec!["rooms".to_string()]);
        match ann {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                allows_unassigned,
            } => {
                assert_eq!(value_range_provider_refs, vec!["rooms"]);
                assert!(allows_unassigned);
            }
            _ => panic!("Expected PlanningVariable"),
        }
    }

    #[test]
    fn test_planning_list_variable() {
        let ann = PlanningAnnotation::planning_list_variable(vec!["visits".to_string()]);
        match ann {
            PlanningAnnotation::PlanningListVariable {
                value_range_provider_refs,
                allows_unassigned_values,
            } => {
                assert_eq!(value_range_provider_refs, vec!["visits"]);
                assert!(!allows_unassigned_values);
            }
            _ => panic!("Expected PlanningListVariable"),
        }
    }

    #[test]
    fn test_planning_list_variable_unassigned() {
        let ann = PlanningAnnotation::planning_list_variable_unassigned(vec!["visits".to_string()]);
        match ann {
            PlanningAnnotation::PlanningListVariable {
                value_range_provider_refs,
                allows_unassigned_values,
            } => {
                assert_eq!(value_range_provider_refs, vec!["visits"]);
                assert!(allows_unassigned_values);
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
        let ann = PlanningAnnotation::value_range_provider();
        match ann {
            PlanningAnnotation::ValueRangeProvider { id } => {
                assert!(id.is_none());
            }
            _ => panic!("Expected ValueRangeProvider"),
        }
    }

    #[test]
    fn test_value_range_provider_with_id() {
        let ann = PlanningAnnotation::value_range_provider_with_id("rooms");
        match ann {
            PlanningAnnotation::ValueRangeProvider { id } => {
                assert_eq!(id, Some("rooms".to_string()));
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
    fn test_value_range_provider_refs_getter() {
        let var = PlanningAnnotation::planning_variable(vec!["rooms".to_string()]);
        assert_eq!(
            var.value_range_provider_refs(),
            Some(&vec!["rooms".to_string()])
        );

        let list_var = PlanningAnnotation::planning_list_variable(vec!["visits".to_string()]);
        assert_eq!(
            list_var.value_range_provider_refs(),
            Some(&vec!["visits".to_string()])
        );

        let score = PlanningAnnotation::planning_score();
        assert!(score.value_range_provider_refs().is_none());
    }

    // JSON serialization tests - must match Java solver service expectations

    #[test]
    fn test_json_planning_id() {
        let ann = PlanningAnnotation::PlanningId;
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(json, r#"{"annotation":"PlanningId"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_variable_default() {
        let ann = PlanningAnnotation::planning_variable(vec![]);
        let json = serde_json::to_string(&ann).unwrap();
        // Empty refs and allows_unassigned=false should both be omitted
        assert_eq!(json, r#"{"annotation":"PlanningVariable"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_variable_with_refs() {
        let ann = PlanningAnnotation::planning_variable(vec!["rooms".to_string()]);
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(
            json,
            r#"{"annotation":"PlanningVariable","valueRangeProviderRefs":["rooms"]}"#
        );

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_variable_allows_unassigned() {
        let ann = PlanningAnnotation::planning_variable_unassigned(vec![]);
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(
            json,
            r#"{"annotation":"PlanningVariable","allowsUnassigned":true}"#
        );

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_variable_full() {
        let ann = PlanningAnnotation::planning_variable_full(
            vec!["rooms".to_string(), "timeslots".to_string()],
            true,
        );
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains(r#""valueRangeProviderRefs":["rooms","timeslots"]"#));
        assert!(json.contains(r#""allowsUnassigned":true"#));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_list_variable_default() {
        let ann = PlanningAnnotation::planning_list_variable(vec![]);
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(json, r#"{"annotation":"PlanningListVariable"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_list_variable_with_refs() {
        let ann = PlanningAnnotation::planning_list_variable(vec!["visits".to_string()]);
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(
            json,
            r#"{"annotation":"PlanningListVariable","valueRangeProviderRefs":["visits"]}"#
        );

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_list_variable_allows_unassigned() {
        let ann = PlanningAnnotation::planning_list_variable_unassigned(vec!["visits".to_string()]);
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains(r#""valueRangeProviderRefs":["visits"]"#));
        assert!(json.contains(r#""allowsUnassignedValues":true"#));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_score() {
        let ann = PlanningAnnotation::PlanningScore {
            bendable_hard_levels: None,
            bendable_soft_levels: None,
        };
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(json, r#"{"annotation":"PlanningScore"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_planning_score_bendable() {
        let ann = PlanningAnnotation::planning_score_bendable(2, 3);
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains(r#""bendable_hard_levels":2"#));
        assert!(json.contains(r#""bendable_soft_levels":3"#));

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_value_range_provider() {
        let ann = PlanningAnnotation::ValueRangeProvider { id: None };
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(json, r#"{"annotation":"ValueRangeProvider"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_value_range_provider_with_id() {
        let ann = PlanningAnnotation::value_range_provider_with_id("rooms");
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(json, r#"{"annotation":"ValueRangeProvider","id":"rooms"}"#);

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_inverse_relation_shadow() {
        let ann = PlanningAnnotation::inverse_relation_shadow("visits");
        let json = serde_json::to_string(&ann).unwrap();
        assert_eq!(
            json,
            r#"{"annotation":"InverseRelationShadowVariable","source_variable_name":"visits"}"#
        );

        let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ann);
    }

    #[test]
    fn test_json_simple_annotations() {
        let annotations = vec![
            (
                PlanningAnnotation::PlanningEntity,
                r#"{"annotation":"PlanningEntity"}"#,
            ),
            (
                PlanningAnnotation::PlanningSolution,
                r#"{"annotation":"PlanningSolution"}"#,
            ),
            (
                PlanningAnnotation::ProblemFactProperty,
                r#"{"annotation":"ProblemFactProperty"}"#,
            ),
            (
                PlanningAnnotation::ProblemFactCollectionProperty,
                r#"{"annotation":"ProblemFactCollectionProperty"}"#,
            ),
            (
                PlanningAnnotation::PlanningEntityProperty,
                r#"{"annotation":"PlanningEntityProperty"}"#,
            ),
            (
                PlanningAnnotation::PlanningEntityCollectionProperty,
                r#"{"annotation":"PlanningEntityCollectionProperty"}"#,
            ),
            (
                PlanningAnnotation::PlanningPin,
                r#"{"annotation":"PlanningPin"}"#,
            ),
        ];

        for (ann, expected_json) in annotations {
            let json = serde_json::to_string(&ann).unwrap();
            assert_eq!(json, expected_json);

            let parsed: PlanningAnnotation = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, ann);
        }
    }

    #[test]
    fn test_json_deserialization_defaults() {
        // Test that missing optional fields get defaults
        let json = r#"{"annotation":"PlanningVariable"}"#;
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
    fn test_json_deserialization_with_refs() {
        let json = r#"{"annotation":"PlanningVariable","valueRangeProviderRefs":["a","b"]}"#;
        let parsed: PlanningAnnotation = serde_json::from_str(json).unwrap();
        match parsed {
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                allows_unassigned,
            } => {
                assert_eq!(
                    value_range_provider_refs,
                    vec!["a".to_string(), "b".to_string()]
                );
                assert!(!allows_unassigned);
            }
            _ => panic!("Expected PlanningVariable"),
        }
    }
}
