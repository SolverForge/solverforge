use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintWeight {
    pub constraint_name: String,
    #[serde(default)]
    pub constraint_package: Option<String>,
}

impl ConstraintWeight {
    pub fn new(constraint_name: impl Into<String>) -> Self {
        Self {
            constraint_name: constraint_name.into(),
            constraint_package: None,
        }
    }

    pub fn with_package(constraint_name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            constraint_name: constraint_name.into(),
            constraint_package: Some(package.into()),
        }
    }

    pub fn full_name(&self) -> String {
        match &self.constraint_package {
            Some(pkg) => format!("{}/{}", pkg, self.constraint_name),
            None => self.constraint_name.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConstraintConfiguration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeepPlanningClone;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_weight_new() {
        let weight = ConstraintWeight::new("Room conflict");
        assert_eq!(weight.constraint_name, "Room conflict");
        assert!(weight.constraint_package.is_none());
    }

    #[test]
    fn test_constraint_weight_with_package() {
        let weight = ConstraintWeight::with_package("Room conflict", "timetabling");
        assert_eq!(weight.constraint_name, "Room conflict");
        assert_eq!(weight.constraint_package, Some("timetabling".to_string()));
    }

    #[test]
    fn test_full_name() {
        let weight1 = ConstraintWeight::new("Room conflict");
        assert_eq!(weight1.full_name(), "Room conflict");

        let weight2 = ConstraintWeight::with_package("Room conflict", "timetabling");
        assert_eq!(weight2.full_name(), "timetabling/Room conflict");
    }

    #[test]
    fn test_constraint_weight_json() {
        let weight = ConstraintWeight::with_package("Room conflict", "timetabling");
        let json = serde_json::to_string(&weight).unwrap();
        assert!(json.contains("\"constraint_name\":\"Room conflict\""));
        assert!(json.contains("\"constraint_package\":\"timetabling\""));

        let parsed: ConstraintWeight = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, weight);
    }

    #[test]
    fn test_constraint_weight_json_without_package() {
        let json = r#"{"constraint_name":"Room conflict"}"#;
        let parsed: ConstraintWeight = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.constraint_name, "Room conflict");
        assert!(parsed.constraint_package.is_none());
    }

    #[test]
    fn test_constraint_configuration() {
        let config = ConstraintConfiguration;
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ConstraintConfiguration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_deep_planning_clone() {
        let clone = DeepPlanningClone;
        let json = serde_json::to_string(&clone).unwrap();
        let parsed: DeepPlanningClone = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, clone);
    }
}
