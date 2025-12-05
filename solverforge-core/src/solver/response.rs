use serde::{Deserialize, Deserializer, Serialize};

/// Performance statistics from a solver run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverStats {
    pub time_spent_millis: u64,
    pub score_calculation_count: u64,
    pub score_calculation_speed: u64,
    pub move_evaluation_count: u64,
    pub move_evaluation_speed: u64,
}

impl SolverStats {
    pub fn new(
        time_spent_millis: u64,
        score_calculation_count: u64,
        score_calculation_speed: u64,
        move_evaluation_count: u64,
        move_evaluation_speed: u64,
    ) -> Self {
        Self {
            time_spent_millis,
            score_calculation_count,
            score_calculation_speed,
            move_evaluation_count,
            move_evaluation_speed,
        }
    }

    /// Returns a formatted summary of the solver statistics.
    pub fn summary(&self) -> String {
        format!(
            "Time: {}ms | Moves: {} ({}/sec) | Score calcs: {} ({}/sec)",
            self.time_spent_millis,
            self.move_evaluation_count,
            self.move_evaluation_speed,
            self.score_calculation_count,
            self.score_calculation_speed
        )
    }
}

/// Custom deserializer for score that handles both string format (e.g., "-8")
/// and object format (e.g., {"SimpleScore": "-8"})
fn deserialize_score<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{Error, Visitor};

    struct ScoreVisitor;

    impl<'de> Visitor<'de> for ScoreVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, null, or an object with score type key")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(value.to_string())
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            // Handle null as an uninitialized score
            Ok("uninitialized".to_string())
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            // Expect a single entry like {"SimpleScore": "-8"} or {"HardSoftScore": "0hard/-5soft"}
            if let Some((_score_type, score_value)) = map.next_entry::<String, String>()? {
                // For SimpleScore, HardSoftScore, etc., return just the value
                Ok(score_value)
            } else {
                Err(Error::custom("expected score object to have one entry"))
            }
        }
    }

    deserializer.deserialize_any(ScoreVisitor)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveResponse {
    pub solution: String,
    /// The score as a string (e.g., "-8" for SimpleScore, "0hard/-5soft" for HardSoftScore)
    /// Handles both string and object formats from the server.
    #[serde(deserialize_with = "deserialize_score")]
    pub score: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<SolverStats>,
}

impl SolveResponse {
    pub fn new(solution: String, score: impl Into<String>) -> Self {
        Self {
            solution,
            score: score.into(),
            stats: None,
        }
    }

    pub fn with_stats(solution: String, score: impl Into<String>, stats: SolverStats) -> Self {
        Self {
            solution,
            score: score.into(),
            stats: Some(stats),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDto {
    pub score_string: String,
    pub hard_score: i64,
    pub soft_score: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium_score: Option<i64>,
    pub is_feasible: bool,
}

impl ScoreDto {
    pub fn hard_soft(hard: i64, soft: i64) -> Self {
        Self {
            score_string: format!("{}hard/{}soft", hard, soft),
            hard_score: hard,
            soft_score: soft,
            medium_score: None,
            is_feasible: hard >= 0,
        }
    }

    pub fn hard_medium_soft(hard: i64, medium: i64, soft: i64) -> Self {
        Self {
            score_string: format!("{}hard/{}medium/{}soft", hard, medium, soft),
            hard_score: hard,
            soft_score: soft,
            medium_score: Some(medium),
            is_feasible: hard >= 0,
        }
    }

    pub fn simple(score: i64) -> Self {
        Self {
            score_string: score.to_string(),
            hard_score: score,
            soft_score: 0,
            medium_score: None,
            is_feasible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolveState {
    Pending,
    Running,
    Completed,
    Failed,
    Stopped,
}

impl SolveState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SolveState::Completed | SolveState::Failed | SolveState::Stopped
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(self, SolveState::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveStatus {
    pub state: SolveState,
    pub time_spent_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score: Option<ScoreDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SolveStatus {
    pub fn pending() -> Self {
        Self {
            state: SolveState::Pending,
            time_spent_ms: 0,
            best_score: None,
            error: None,
        }
    }

    pub fn running(time_spent_ms: u64, best_score: Option<ScoreDto>) -> Self {
        Self {
            state: SolveState::Running,
            time_spent_ms,
            best_score,
            error: None,
        }
    }

    pub fn completed(time_spent_ms: u64, score: ScoreDto) -> Self {
        Self {
            state: SolveState::Completed,
            time_spent_ms,
            best_score: Some(score),
            error: None,
        }
    }

    pub fn failed(time_spent_ms: u64, error: impl Into<String>) -> Self {
        Self {
            state: SolveState::Failed,
            time_spent_ms,
            best_score: None,
            error: Some(error.into()),
        }
    }

    pub fn stopped(time_spent_ms: u64, best_score: Option<ScoreDto>) -> Self {
        Self {
            state: SolveState::Stopped,
            time_spent_ms,
            best_score,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncSolveResponse {
    pub solve_id: String,
}

impl AsyncSolveResponse {
    pub fn new(solve_id: impl Into<String>) -> Self {
        Self {
            solve_id: solve_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SolveHandle {
    pub id: String,
}

impl SolveHandle {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_response_new() {
        let response = SolveResponse::new(r#"{"lessons": []}"#.to_string(), "0hard/-10soft");

        assert_eq!(response.solution, r#"{"lessons": []}"#);
        assert_eq!(response.score, "0hard/-10soft");
    }

    #[test]
    fn test_score_dto_hard_soft() {
        let score = ScoreDto::hard_soft(-5, -100);

        assert_eq!(score.hard_score, -5);
        assert_eq!(score.soft_score, -100);
        assert!(score.medium_score.is_none());
        assert!(!score.is_feasible);
        assert_eq!(score.score_string, "-5hard/-100soft");
    }

    #[test]
    fn test_score_dto_hard_medium_soft() {
        let score = ScoreDto::hard_medium_soft(0, -10, -50);

        assert_eq!(score.hard_score, 0);
        assert_eq!(score.medium_score, Some(-10));
        assert_eq!(score.soft_score, -50);
        assert!(score.is_feasible);
        assert_eq!(score.score_string, "0hard/-10medium/-50soft");
    }

    #[test]
    fn test_score_dto_simple() {
        let score = ScoreDto::simple(42);

        assert_eq!(score.hard_score, 42);
        assert_eq!(score.soft_score, 0);
        assert!(score.is_feasible);
        assert_eq!(score.score_string, "42");
    }

    #[test]
    fn test_solve_state_is_terminal() {
        assert!(!SolveState::Pending.is_terminal());
        assert!(!SolveState::Running.is_terminal());
        assert!(SolveState::Completed.is_terminal());
        assert!(SolveState::Failed.is_terminal());
        assert!(SolveState::Stopped.is_terminal());
    }

    #[test]
    fn test_solve_state_is_running() {
        assert!(!SolveState::Pending.is_running());
        assert!(SolveState::Running.is_running());
        assert!(!SolveState::Completed.is_running());
    }

    #[test]
    fn test_solve_status_pending() {
        let status = SolveStatus::pending();

        assert_eq!(status.state, SolveState::Pending);
        assert_eq!(status.time_spent_ms, 0);
        assert!(status.best_score.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_solve_status_running() {
        let score = ScoreDto::hard_soft(-10, -50);
        let status = SolveStatus::running(5000, Some(score));

        assert_eq!(status.state, SolveState::Running);
        assert_eq!(status.time_spent_ms, 5000);
        assert!(status.best_score.is_some());
    }

    #[test]
    fn test_solve_status_completed() {
        let score = ScoreDto::hard_soft(0, -20);
        let status = SolveStatus::completed(30000, score);

        assert_eq!(status.state, SolveState::Completed);
        assert_eq!(status.time_spent_ms, 30000);
        assert!(status.best_score.is_some());
        assert!(status.best_score.as_ref().unwrap().is_feasible);
    }

    #[test]
    fn test_solve_status_failed() {
        let status = SolveStatus::failed(1000, "Timeout exceeded");

        assert_eq!(status.state, SolveState::Failed);
        assert_eq!(status.error, Some("Timeout exceeded".to_string()));
    }

    #[test]
    fn test_solve_status_stopped() {
        let score = ScoreDto::hard_soft(-5, -30);
        let status = SolveStatus::stopped(15000, Some(score));

        assert_eq!(status.state, SolveState::Stopped);
        assert!(status.best_score.is_some());
    }

    #[test]
    fn test_async_solve_response() {
        let response = AsyncSolveResponse::new("solve-12345");
        assert_eq!(response.solve_id, "solve-12345");
    }

    #[test]
    fn test_solve_handle() {
        let handle = SolveHandle::new("solve-12345");
        assert_eq!(handle.id, "solve-12345");
    }

    #[test]
    fn test_solve_response_json_serialization() {
        let response = SolveResponse::new(r#"{"data": "test"}"#.to_string(), "0hard/-15soft");

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"solution\""));
        assert!(json.contains("\"score\":\"0hard/-15soft\""));

        let parsed: SolveResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_score_dto_json_omits_medium_when_none() {
        let score = ScoreDto::hard_soft(0, -10);
        let json = serde_json::to_string(&score).unwrap();
        assert!(!json.contains("mediumScore"));
    }

    #[test]
    fn test_solve_status_json_serialization() {
        let status = SolveStatus::running(10000, Some(ScoreDto::hard_soft(-2, -100)));

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"state\":\"RUNNING\""));
        assert!(json.contains("\"timeSpentMs\":10000"));

        let parsed: SolveStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_solve_state_json_serialization() {
        assert_eq!(
            serde_json::to_string(&SolveState::Pending).unwrap(),
            "\"PENDING\""
        );
        assert_eq!(
            serde_json::to_string(&SolveState::Running).unwrap(),
            "\"RUNNING\""
        );
        assert_eq!(
            serde_json::to_string(&SolveState::Completed).unwrap(),
            "\"COMPLETED\""
        );
        assert_eq!(
            serde_json::to_string(&SolveState::Failed).unwrap(),
            "\"FAILED\""
        );
        assert_eq!(
            serde_json::to_string(&SolveState::Stopped).unwrap(),
            "\"STOPPED\""
        );
    }

    #[test]
    fn test_score_dto_clone() {
        let score = ScoreDto::hard_soft(0, -10);
        let cloned = score.clone();
        assert_eq!(score, cloned);
    }

    #[test]
    fn test_solve_response_debug() {
        let response = SolveResponse::new("{}".to_string(), "0");
        let debug = format!("{:?}", response);
        assert!(debug.contains("SolveResponse"));
    }

    #[test]
    fn test_solve_status_debug() {
        let status = SolveStatus::pending();
        let debug = format!("{:?}", status);
        assert!(debug.contains("SolveStatus"));
    }
}
