use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvironmentMode {
    NonReproducible,
    #[default]
    Reproducible,
    NoAssert,
    PhaseAssert,
    StepAssert,
    FullAssert,
    TrackedFullAssert,
}

impl EnvironmentMode {
    pub fn is_asserted(&self) -> bool {
        !matches!(
            self,
            EnvironmentMode::NonReproducible
                | EnvironmentMode::Reproducible
                | EnvironmentMode::NoAssert
        )
    }

    pub fn is_reproducible(&self) -> bool {
        !matches!(self, EnvironmentMode::NonReproducible)
    }

    pub fn is_tracked(&self) -> bool {
        matches!(self, EnvironmentMode::TrackedFullAssert)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoveThreadCount {
    Auto,
    #[default]
    None,
    #[serde(untagged)]
    Count(u32),
}

impl MoveThreadCount {
    pub fn auto() -> Self {
        MoveThreadCount::Auto
    }

    pub fn none() -> Self {
        MoveThreadCount::None
    }

    pub fn count(n: u32) -> Self {
        MoveThreadCount::Count(n)
    }

    pub fn is_parallel(&self) -> bool {
        match self {
            MoveThreadCount::Auto => true,
            MoveThreadCount::None => false,
            MoveThreadCount::Count(n) => *n > 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_mode_default() {
        let mode: EnvironmentMode = Default::default();
        assert_eq!(mode, EnvironmentMode::Reproducible);
    }

    #[test]
    fn test_environment_mode_is_asserted() {
        assert!(!EnvironmentMode::NonReproducible.is_asserted());
        assert!(!EnvironmentMode::Reproducible.is_asserted());
        assert!(!EnvironmentMode::NoAssert.is_asserted());
        assert!(EnvironmentMode::PhaseAssert.is_asserted());
        assert!(EnvironmentMode::StepAssert.is_asserted());
        assert!(EnvironmentMode::FullAssert.is_asserted());
        assert!(EnvironmentMode::TrackedFullAssert.is_asserted());
    }

    #[test]
    fn test_environment_mode_is_reproducible() {
        assert!(!EnvironmentMode::NonReproducible.is_reproducible());
        assert!(EnvironmentMode::Reproducible.is_reproducible());
        assert!(EnvironmentMode::NoAssert.is_reproducible());
        assert!(EnvironmentMode::PhaseAssert.is_reproducible());
        assert!(EnvironmentMode::StepAssert.is_reproducible());
        assert!(EnvironmentMode::FullAssert.is_reproducible());
        assert!(EnvironmentMode::TrackedFullAssert.is_reproducible());
    }

    #[test]
    fn test_environment_mode_is_tracked() {
        assert!(!EnvironmentMode::NonReproducible.is_tracked());
        assert!(!EnvironmentMode::Reproducible.is_tracked());
        assert!(!EnvironmentMode::PhaseAssert.is_tracked());
        assert!(EnvironmentMode::TrackedFullAssert.is_tracked());
    }

    #[test]
    fn test_environment_mode_json_serialization() {
        assert_eq!(
            serde_json::to_string(&EnvironmentMode::NonReproducible).unwrap(),
            "\"NON_REPRODUCIBLE\""
        );
        assert_eq!(
            serde_json::to_string(&EnvironmentMode::PhaseAssert).unwrap(),
            "\"PHASE_ASSERT\""
        );
        assert_eq!(
            serde_json::to_string(&EnvironmentMode::TrackedFullAssert).unwrap(),
            "\"TRACKED_FULL_ASSERT\""
        );
    }

    #[test]
    fn test_environment_mode_json_deserialization() {
        let mode: EnvironmentMode = serde_json::from_str("\"FULL_ASSERT\"").unwrap();
        assert_eq!(mode, EnvironmentMode::FullAssert);
    }

    #[test]
    fn test_move_thread_count_default() {
        let count: MoveThreadCount = Default::default();
        assert_eq!(count, MoveThreadCount::None);
    }

    #[test]
    fn test_move_thread_count_constructors() {
        assert_eq!(MoveThreadCount::auto(), MoveThreadCount::Auto);
        assert_eq!(MoveThreadCount::none(), MoveThreadCount::None);
        assert_eq!(MoveThreadCount::count(4), MoveThreadCount::Count(4));
    }

    #[test]
    fn test_move_thread_count_is_parallel() {
        assert!(MoveThreadCount::Auto.is_parallel());
        assert!(!MoveThreadCount::None.is_parallel());
        assert!(!MoveThreadCount::Count(1).is_parallel());
        assert!(MoveThreadCount::Count(2).is_parallel());
        assert!(MoveThreadCount::Count(8).is_parallel());
    }

    #[test]
    fn test_move_thread_count_json_serialization() {
        assert_eq!(
            serde_json::to_string(&MoveThreadCount::Auto).unwrap(),
            "\"AUTO\""
        );
        assert_eq!(
            serde_json::to_string(&MoveThreadCount::None).unwrap(),
            "\"NONE\""
        );
        assert_eq!(
            serde_json::to_string(&MoveThreadCount::Count(4)).unwrap(),
            "4"
        );
    }

    #[test]
    fn test_move_thread_count_json_deserialization() {
        let auto: MoveThreadCount = serde_json::from_str("\"AUTO\"").unwrap();
        assert_eq!(auto, MoveThreadCount::Auto);

        let count: MoveThreadCount = serde_json::from_str("8").unwrap();
        assert_eq!(count, MoveThreadCount::Count(8));
    }

    #[test]
    fn test_environment_mode_clone() {
        let mode = EnvironmentMode::FullAssert;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_environment_mode_debug() {
        let debug = format!("{:?}", EnvironmentMode::PhaseAssert);
        assert!(debug.contains("PhaseAssert"));
    }

    #[test]
    fn test_move_thread_count_clone() {
        let count = MoveThreadCount::Count(4);
        let cloned = count.clone();
        assert_eq!(count, cloned);
    }
}
