use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShadowAnnotation {
    ShadowVariable {
        source_variable_name: String,
        #[serde(default)]
        source_entity_class: Option<String>,
    },
    InverseRelationShadowVariable {
        source_variable_name: String,
    },
    IndexShadowVariable {
        source_variable_name: String,
    },
    PreviousElementShadowVariable {
        source_variable_name: String,
    },
    NextElementShadowVariable {
        source_variable_name: String,
    },
    AnchorShadowVariable {
        source_variable_name: String,
    },
    PiggybackShadowVariable {
        shadow_variable_name: String,
    },
    CascadingUpdateShadowVariable {
        target_method_name: String,
    },
}

impl ShadowAnnotation {
    pub fn shadow_variable(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::ShadowVariable {
            source_variable_name: source_variable_name.into(),
            source_entity_class: None,
        }
    }

    pub fn shadow_variable_with_class(
        source_variable_name: impl Into<String>,
        source_entity_class: impl Into<String>,
    ) -> Self {
        ShadowAnnotation::ShadowVariable {
            source_variable_name: source_variable_name.into(),
            source_entity_class: Some(source_entity_class.into()),
        }
    }

    pub fn inverse_relation(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::InverseRelationShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn index(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::IndexShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn previous_element(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::PreviousElementShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn next_element(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::NextElementShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn anchor(source_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::AnchorShadowVariable {
            source_variable_name: source_variable_name.into(),
        }
    }

    pub fn piggyback(shadow_variable_name: impl Into<String>) -> Self {
        ShadowAnnotation::PiggybackShadowVariable {
            shadow_variable_name: shadow_variable_name.into(),
        }
    }

    pub fn cascading_update(target_method_name: impl Into<String>) -> Self {
        ShadowAnnotation::CascadingUpdateShadowVariable {
            target_method_name: target_method_name.into(),
        }
    }

    pub fn source_variable_name(&self) -> Option<&str> {
        match self {
            ShadowAnnotation::ShadowVariable {
                source_variable_name,
                ..
            } => Some(source_variable_name),
            ShadowAnnotation::InverseRelationShadowVariable {
                source_variable_name,
            } => Some(source_variable_name),
            ShadowAnnotation::IndexShadowVariable {
                source_variable_name,
            } => Some(source_variable_name),
            ShadowAnnotation::PreviousElementShadowVariable {
                source_variable_name,
            } => Some(source_variable_name),
            ShadowAnnotation::NextElementShadowVariable {
                source_variable_name,
            } => Some(source_variable_name),
            ShadowAnnotation::AnchorShadowVariable {
                source_variable_name,
            } => Some(source_variable_name),
            ShadowAnnotation::PiggybackShadowVariable { .. } => None,
            ShadowAnnotation::CascadingUpdateShadowVariable { .. } => None,
        }
    }

    pub fn is_list_variable_shadow(&self) -> bool {
        matches!(
            self,
            ShadowAnnotation::IndexShadowVariable { .. }
                | ShadowAnnotation::PreviousElementShadowVariable { .. }
                | ShadowAnnotation::NextElementShadowVariable { .. }
                | ShadowAnnotation::AnchorShadowVariable { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_variable() {
        let ann = ShadowAnnotation::shadow_variable("room");
        match ann {
            ShadowAnnotation::ShadowVariable {
                source_variable_name,
                source_entity_class,
            } => {
                assert_eq!(source_variable_name, "room");
                assert!(source_entity_class.is_none());
            }
            _ => panic!("Expected ShadowVariable"),
        }
    }

    #[test]
    fn test_shadow_variable_with_class() {
        let ann = ShadowAnnotation::shadow_variable_with_class("room", "Lesson");
        match ann {
            ShadowAnnotation::ShadowVariable {
                source_variable_name,
                source_entity_class,
            } => {
                assert_eq!(source_variable_name, "room");
                assert_eq!(source_entity_class, Some("Lesson".to_string()));
            }
            _ => panic!("Expected ShadowVariable"),
        }
    }

    #[test]
    fn test_inverse_relation() {
        let ann = ShadowAnnotation::inverse_relation("visits");
        match ann {
            ShadowAnnotation::InverseRelationShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "visits");
            }
            _ => panic!("Expected InverseRelationShadowVariable"),
        }
    }

    #[test]
    fn test_index() {
        let ann = ShadowAnnotation::index("taskList");
        match ann {
            ShadowAnnotation::IndexShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "taskList");
            }
            _ => panic!("Expected IndexShadowVariable"),
        }
    }

    #[test]
    fn test_previous_element() {
        let ann = ShadowAnnotation::previous_element("taskList");
        match ann {
            ShadowAnnotation::PreviousElementShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "taskList");
            }
            _ => panic!("Expected PreviousElementShadowVariable"),
        }
    }

    #[test]
    fn test_next_element() {
        let ann = ShadowAnnotation::next_element("taskList");
        match ann {
            ShadowAnnotation::NextElementShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "taskList");
            }
            _ => panic!("Expected NextElementShadowVariable"),
        }
    }

    #[test]
    fn test_anchor() {
        let ann = ShadowAnnotation::anchor("taskList");
        match ann {
            ShadowAnnotation::AnchorShadowVariable {
                source_variable_name,
            } => {
                assert_eq!(source_variable_name, "taskList");
            }
            _ => panic!("Expected AnchorShadowVariable"),
        }
    }

    #[test]
    fn test_piggyback() {
        let ann = ShadowAnnotation::piggyback("arrivalTime");
        match ann {
            ShadowAnnotation::PiggybackShadowVariable {
                shadow_variable_name,
            } => {
                assert_eq!(shadow_variable_name, "arrivalTime");
            }
            _ => panic!("Expected PiggybackShadowVariable"),
        }
    }

    #[test]
    fn test_cascading_update() {
        let ann = ShadowAnnotation::cascading_update("updateArrivalTime");
        match ann {
            ShadowAnnotation::CascadingUpdateShadowVariable { target_method_name } => {
                assert_eq!(target_method_name, "updateArrivalTime");
            }
            _ => panic!("Expected CascadingUpdateShadowVariable"),
        }
    }

    #[test]
    fn test_source_variable_name() {
        assert_eq!(
            ShadowAnnotation::shadow_variable("room").source_variable_name(),
            Some("room")
        );
        assert_eq!(
            ShadowAnnotation::inverse_relation("visits").source_variable_name(),
            Some("visits")
        );
        assert_eq!(
            ShadowAnnotation::index("tasks").source_variable_name(),
            Some("tasks")
        );
        assert_eq!(
            ShadowAnnotation::previous_element("tasks").source_variable_name(),
            Some("tasks")
        );
        assert_eq!(
            ShadowAnnotation::next_element("tasks").source_variable_name(),
            Some("tasks")
        );
        assert_eq!(
            ShadowAnnotation::anchor("tasks").source_variable_name(),
            Some("tasks")
        );
        assert_eq!(
            ShadowAnnotation::piggyback("time").source_variable_name(),
            None
        );
        assert_eq!(
            ShadowAnnotation::cascading_update("update").source_variable_name(),
            None
        );
    }

    #[test]
    fn test_is_list_variable_shadow() {
        assert!(!ShadowAnnotation::shadow_variable("room").is_list_variable_shadow());
        assert!(!ShadowAnnotation::inverse_relation("visits").is_list_variable_shadow());
        assert!(ShadowAnnotation::index("tasks").is_list_variable_shadow());
        assert!(ShadowAnnotation::previous_element("tasks").is_list_variable_shadow());
        assert!(ShadowAnnotation::next_element("tasks").is_list_variable_shadow());
        assert!(ShadowAnnotation::anchor("tasks").is_list_variable_shadow());
        assert!(!ShadowAnnotation::piggyback("time").is_list_variable_shadow());
        assert!(!ShadowAnnotation::cascading_update("update").is_list_variable_shadow());
    }

    #[test]
    fn test_json_serialization() {
        let annotations = vec![
            ShadowAnnotation::shadow_variable("room"),
            ShadowAnnotation::shadow_variable_with_class("room", "Lesson"),
            ShadowAnnotation::inverse_relation("visits"),
            ShadowAnnotation::index("tasks"),
            ShadowAnnotation::previous_element("tasks"),
            ShadowAnnotation::next_element("tasks"),
            ShadowAnnotation::anchor("tasks"),
            ShadowAnnotation::piggyback("arrivalTime"),
            ShadowAnnotation::cascading_update("updateTime"),
        ];

        for ann in annotations {
            let json = serde_json::to_string(&ann).unwrap();
            let parsed: ShadowAnnotation = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, ann);
        }
    }

    #[test]
    fn test_json_format() {
        let ann = ShadowAnnotation::shadow_variable_with_class("room", "Lesson");
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("\"type\":\"ShadowVariable\""));
        assert!(json.contains("\"source_variable_name\":\"room\""));
        assert!(json.contains("\"source_entity_class\":\"Lesson\""));
    }
}
