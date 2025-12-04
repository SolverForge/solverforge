use crate::solver::{EnvironmentMode, MoveThreadCount, TerminationConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_class_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_mode: Option<EnvironmentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_thread_count: Option<MoveThreadCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfig>,
}

impl SolverConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_solution_class(mut self, class_name: impl Into<String>) -> Self {
        self.solution_class = Some(class_name.into());
        self
    }

    pub fn with_entity_class(mut self, class_name: impl Into<String>) -> Self {
        self.entity_class_list.push(class_name.into());
        self
    }

    pub fn with_entity_classes(mut self, classes: Vec<String>) -> Self {
        self.entity_class_list = classes;
        self
    }

    pub fn with_environment_mode(mut self, mode: EnvironmentMode) -> Self {
        self.environment_mode = Some(mode);
        self
    }

    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn with_move_thread_count(mut self, count: MoveThreadCount) -> Self {
        self.move_thread_count = Some(count);
        self
    }

    pub fn with_termination(mut self, termination: TerminationConfig) -> Self {
        self.termination = Some(termination);
        self
    }

    pub fn environment_mode_or_default(&self) -> EnvironmentMode {
        self.environment_mode.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_config_new() {
        let config = SolverConfig::new();
        assert!(config.solution_class.is_none());
        assert!(config.entity_class_list.is_empty());
        assert!(config.environment_mode.is_none());
    }

    #[test]
    fn test_solver_config_with_solution_class() {
        let config = SolverConfig::new().with_solution_class("Timetable");
        assert_eq!(config.solution_class, Some("Timetable".to_string()));
    }

    #[test]
    fn test_solver_config_with_entity_class() {
        let config = SolverConfig::new()
            .with_entity_class("Lesson")
            .with_entity_class("Room");
        assert_eq!(config.entity_class_list.len(), 2);
        assert!(config.entity_class_list.contains(&"Lesson".to_string()));
        assert!(config.entity_class_list.contains(&"Room".to_string()));
    }

    #[test]
    fn test_solver_config_with_entity_classes() {
        let config =
            SolverConfig::new().with_entity_classes(vec!["Lesson".to_string(), "Room".to_string()]);
        assert_eq!(config.entity_class_list.len(), 2);
    }

    #[test]
    fn test_solver_config_with_environment_mode() {
        let config = SolverConfig::new().with_environment_mode(EnvironmentMode::FullAssert);
        assert_eq!(config.environment_mode, Some(EnvironmentMode::FullAssert));
    }

    #[test]
    fn test_solver_config_with_random_seed() {
        let config = SolverConfig::new().with_random_seed(42);
        assert_eq!(config.random_seed, Some(42));
    }

    #[test]
    fn test_solver_config_with_move_thread_count() {
        let config = SolverConfig::new().with_move_thread_count(MoveThreadCount::Auto);
        assert_eq!(config.move_thread_count, Some(MoveThreadCount::Auto));
    }

    #[test]
    fn test_solver_config_with_termination() {
        let termination = TerminationConfig::new().with_spent_limit("PT5M");
        let config = SolverConfig::new().with_termination(termination.clone());
        assert_eq!(config.termination, Some(termination));
    }

    #[test]
    fn test_solver_config_environment_mode_or_default() {
        let config = SolverConfig::new();
        assert_eq!(
            config.environment_mode_or_default(),
            EnvironmentMode::Reproducible
        );

        let config = SolverConfig::new().with_environment_mode(EnvironmentMode::FullAssert);
        assert_eq!(
            config.environment_mode_or_default(),
            EnvironmentMode::FullAssert
        );
    }

    #[test]
    fn test_solver_config_chained() {
        let config = SolverConfig::new()
            .with_solution_class("Timetable")
            .with_entity_class("Lesson")
            .with_environment_mode(EnvironmentMode::NoAssert)
            .with_random_seed(12345)
            .with_termination(TerminationConfig::new().with_spent_limit("PT10M"));

        assert_eq!(config.solution_class, Some("Timetable".to_string()));
        assert_eq!(config.entity_class_list, vec!["Lesson".to_string()]);
        assert_eq!(config.environment_mode, Some(EnvironmentMode::NoAssert));
        assert_eq!(config.random_seed, Some(12345));
        assert!(config.termination.is_some());
    }

    #[test]
    fn test_solver_config_json_serialization() {
        let config = SolverConfig::new()
            .with_solution_class("Timetable")
            .with_entity_class("Lesson")
            .with_environment_mode(EnvironmentMode::PhaseAssert);

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"solutionClass\":\"Timetable\""));
        assert!(json.contains("\"entityClassList\":[\"Lesson\"]"));
        assert!(json.contains("\"environmentMode\":\"PHASE_ASSERT\""));

        let parsed: SolverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_solver_config_json_omits_none() {
        let config = SolverConfig::new().with_solution_class("Timetable");
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("randomSeed"));
        assert!(!json.contains("termination"));
    }

    #[test]
    fn test_solver_config_full_json() {
        let config = SolverConfig::new()
            .with_solution_class("Timetable")
            .with_entity_class("Lesson")
            .with_environment_mode(EnvironmentMode::FullAssert)
            .with_random_seed(42)
            .with_move_thread_count(MoveThreadCount::Count(4))
            .with_termination(
                TerminationConfig::new()
                    .with_spent_limit("PT5M")
                    .with_best_score_feasible(true),
            );

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: SolverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_solver_config_clone() {
        let config = SolverConfig::new()
            .with_solution_class("Timetable")
            .with_entity_class("Lesson");
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_solver_config_debug() {
        let config = SolverConfig::new().with_solution_class("Timetable");
        let debug = format!("{:?}", config);
        assert!(debug.contains("SolverConfig"));
        assert!(debug.contains("Timetable"));
    }
}
