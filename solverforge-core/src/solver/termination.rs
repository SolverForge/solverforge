use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spent_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unimproved_spent_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unimproved_step_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score_feasible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_count_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_count_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_calculation_count_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diminished_returns: Option<DiminishedReturnsConfig>,
}

impl TerminationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_spent_limit(mut self, limit: impl Into<String>) -> Self {
        self.spent_limit = Some(limit.into());
        self
    }

    pub fn with_unimproved_spent_limit(mut self, limit: impl Into<String>) -> Self {
        self.unimproved_spent_limit = Some(limit.into());
        self
    }

    pub fn with_unimproved_step_count(mut self, count: u64) -> Self {
        self.unimproved_step_count = Some(count);
        self
    }

    pub fn with_best_score_limit(mut self, limit: impl Into<String>) -> Self {
        self.best_score_limit = Some(limit.into());
        self
    }

    pub fn with_best_score_feasible(mut self, feasible: bool) -> Self {
        self.best_score_feasible = Some(feasible);
        self
    }

    pub fn with_step_count_limit(mut self, count: u64) -> Self {
        self.step_count_limit = Some(count);
        self
    }

    pub fn with_move_count_limit(mut self, count: u64) -> Self {
        self.move_count_limit = Some(count);
        self
    }

    pub fn with_score_calculation_count_limit(mut self, count: u64) -> Self {
        self.score_calculation_count_limit = Some(count);
        self
    }

    pub fn with_diminished_returns(mut self, config: DiminishedReturnsConfig) -> Self {
        self.diminished_returns = Some(config);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiminishedReturnsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_improvement_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_improvement_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_improvement_spent_limit: Option<String>,
}

impl DiminishedReturnsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_minimum_improvement_ratio(mut self, ratio: impl Into<String>) -> Self {
        self.minimum_improvement_ratio = Some(ratio.into());
        self
    }

    pub fn with_slow_improvement_limit(mut self, limit: impl Into<String>) -> Self {
        self.slow_improvement_limit = Some(limit.into());
        self
    }

    pub fn with_slow_improvement_spent_limit(mut self, limit: impl Into<String>) -> Self {
        self.slow_improvement_spent_limit = Some(limit.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_termination_config_new() {
        let config = TerminationConfig::new();
        assert!(config.spent_limit.is_none());
        assert!(config.unimproved_spent_limit.is_none());
    }

    #[test]
    fn test_termination_config_spent_limit() {
        let config = TerminationConfig::new().with_spent_limit("PT5M");
        assert_eq!(config.spent_limit, Some("PT5M".to_string()));
    }

    #[test]
    fn test_termination_config_unimproved_spent_limit() {
        let config = TerminationConfig::new().with_unimproved_spent_limit("PT30S");
        assert_eq!(config.unimproved_spent_limit, Some("PT30S".to_string()));
    }

    #[test]
    fn test_termination_config_unimproved_step_count() {
        let config = TerminationConfig::new().with_unimproved_step_count(100);
        assert_eq!(config.unimproved_step_count, Some(100));
    }

    #[test]
    fn test_termination_config_best_score_limit() {
        let config = TerminationConfig::new().with_best_score_limit("0hard/-100soft");
        assert_eq!(config.best_score_limit, Some("0hard/-100soft".to_string()));
    }

    #[test]
    fn test_termination_config_best_score_feasible() {
        let config = TerminationConfig::new().with_best_score_feasible(true);
        assert_eq!(config.best_score_feasible, Some(true));
    }

    #[test]
    fn test_termination_config_step_count_limit() {
        let config = TerminationConfig::new().with_step_count_limit(1000);
        assert_eq!(config.step_count_limit, Some(1000));
    }

    #[test]
    fn test_termination_config_move_count_limit() {
        let config = TerminationConfig::new().with_move_count_limit(10000);
        assert_eq!(config.move_count_limit, Some(10000));
    }

    #[test]
    fn test_termination_config_score_calculation_count_limit() {
        let config = TerminationConfig::new().with_score_calculation_count_limit(1000000);
        assert_eq!(config.score_calculation_count_limit, Some(1000000));
    }

    #[test]
    fn test_termination_config_chained() {
        let config = TerminationConfig::new()
            .with_spent_limit("PT10M")
            .with_unimproved_spent_limit("PT1M")
            .with_best_score_feasible(true);

        assert_eq!(config.spent_limit, Some("PT10M".to_string()));
        assert_eq!(config.unimproved_spent_limit, Some("PT1M".to_string()));
        assert_eq!(config.best_score_feasible, Some(true));
    }

    #[test]
    fn test_diminished_returns_config_new() {
        let config = DiminishedReturnsConfig::new();
        assert!(config.minimum_improvement_ratio.is_none());
    }

    #[test]
    fn test_diminished_returns_config_with_ratio() {
        let config = DiminishedReturnsConfig::new().with_minimum_improvement_ratio("0.001");
        assert_eq!(config.minimum_improvement_ratio, Some("0.001".to_string()));
    }

    #[test]
    fn test_termination_config_with_diminished_returns() {
        let dr = DiminishedReturnsConfig::new().with_minimum_improvement_ratio("0.01");
        let config = TerminationConfig::new().with_diminished_returns(dr);
        assert!(config.diminished_returns.is_some());
    }

    #[test]
    fn test_termination_config_json_serialization() {
        let config = TerminationConfig::new()
            .with_spent_limit("PT5M")
            .with_best_score_feasible(true);

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"spentLimit\":\"PT5M\""));
        assert!(json.contains("\"bestScoreFeasible\":true"));

        let parsed: TerminationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_termination_config_json_omits_none() {
        let config = TerminationConfig::new().with_spent_limit("PT1H");
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("unimprovedSpentLimit"));
        assert!(!json.contains("bestScoreLimit"));
    }

    #[test]
    fn test_diminished_returns_json_serialization() {
        let config = DiminishedReturnsConfig::new()
            .with_minimum_improvement_ratio("0.001")
            .with_slow_improvement_limit("PT30S");

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"minimumImprovementRatio\":\"0.001\""));
        assert!(json.contains("\"slowImprovementLimit\":\"PT30S\""));

        let parsed: DiminishedReturnsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_termination_config_clone() {
        let config = TerminationConfig::new().with_spent_limit("PT5M");
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_termination_config_debug() {
        let config = TerminationConfig::new().with_spent_limit("PT5M");
        let debug = format!("{:?}", config);
        assert!(debug.contains("TerminationConfig"));
        assert!(debug.contains("PT5M"));
    }
}
