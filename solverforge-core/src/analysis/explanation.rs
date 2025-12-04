use crate::solver::ScoreDto;
use crate::{ObjectHandle, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreExplanation {
    pub score: ScoreDto,
    pub constraint_matches: Vec<ConstraintMatch>,
    pub indictments: Vec<Indictment>,
}

impl ScoreExplanation {
    pub fn new(score: ScoreDto) -> Self {
        Self {
            score,
            constraint_matches: Vec::new(),
            indictments: Vec::new(),
        }
    }

    pub fn with_constraint_match(mut self, constraint_match: ConstraintMatch) -> Self {
        self.constraint_matches.push(constraint_match);
        self
    }

    pub fn with_indictment(mut self, indictment: Indictment) -> Self {
        self.indictments.push(indictment);
        self
    }

    pub fn is_feasible(&self) -> bool {
        self.score.is_feasible
    }

    pub fn constraint_count(&self) -> usize {
        self.constraint_matches.len()
    }

    pub fn get_constraint_matches_by_name(&self, name: &str) -> Vec<&ConstraintMatch> {
        self.constraint_matches
            .iter()
            .filter(|m| m.constraint_name == name)
            .collect()
    }

    pub fn get_indictment_for_object(&self, object: ObjectHandle) -> Option<&Indictment> {
        self.indictments
            .iter()
            .find(|i| i.indicted_object == object)
    }

    pub fn hard_score(&self) -> i64 {
        self.score.hard_score
    }

    pub fn soft_score(&self) -> i64 {
        self.score.soft_score
    }

    pub fn medium_score(&self) -> Option<i64> {
        self.score.medium_score
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintMatch {
    pub constraint_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_package: Option<String>,
    pub score: ScoreDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<Value>,
    pub indicted_objects: Vec<ObjectHandle>,
}

impl ConstraintMatch {
    pub fn new(constraint_name: impl Into<String>, score: ScoreDto) -> Self {
        Self {
            constraint_name: constraint_name.into(),
            constraint_package: None,
            score,
            justification: None,
            indicted_objects: Vec::new(),
        }
    }

    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.constraint_package = Some(package.into());
        self
    }

    pub fn with_justification(mut self, justification: Value) -> Self {
        self.justification = Some(justification);
        self
    }

    pub fn with_indicted_object(mut self, object: ObjectHandle) -> Self {
        self.indicted_objects.push(object);
        self
    }

    pub fn with_indicted_objects(mut self, objects: Vec<ObjectHandle>) -> Self {
        self.indicted_objects = objects;
        self
    }

    pub fn full_constraint_name(&self) -> String {
        match &self.constraint_package {
            Some(pkg) => format!("{}.{}", pkg, self.constraint_name),
            None => self.constraint_name.clone(),
        }
    }

    pub fn is_feasible(&self) -> bool {
        self.score.is_feasible
    }

    pub fn hard_score(&self) -> i64 {
        self.score.hard_score
    }

    pub fn soft_score(&self) -> i64 {
        self.score.soft_score
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Indictment {
    pub indicted_object: ObjectHandle,
    pub constraint_matches: Vec<ConstraintMatch>,
    pub score: ScoreDto,
}

impl Indictment {
    pub fn new(object: ObjectHandle, score: ScoreDto) -> Self {
        Self {
            indicted_object: object,
            constraint_matches: Vec::new(),
            score,
        }
    }

    pub fn with_constraint_match(mut self, constraint_match: ConstraintMatch) -> Self {
        self.constraint_matches.push(constraint_match);
        self
    }

    pub fn constraint_count(&self) -> usize {
        self.constraint_matches.len()
    }

    pub fn is_feasible(&self) -> bool {
        self.score.is_feasible
    }

    pub fn hard_score(&self) -> i64 {
        self.score.hard_score
    }

    pub fn soft_score(&self) -> i64 {
        self.score.soft_score
    }

    pub fn get_constraint_matches_by_name(&self, name: &str) -> Vec<&ConstraintMatch> {
        self.constraint_matches
            .iter()
            .filter(|m| m.constraint_name == name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_score() -> ScoreDto {
        ScoreDto::hard_soft(-1, -10)
    }

    fn create_feasible_score() -> ScoreDto {
        ScoreDto::hard_soft(0, -5)
    }

    #[test]
    fn test_score_explanation_new() {
        let explanation = ScoreExplanation::new(create_test_score());

        assert_eq!(explanation.hard_score(), -1);
        assert_eq!(explanation.soft_score(), -10);
        assert!(!explanation.is_feasible());
        assert!(explanation.constraint_matches.is_empty());
        assert!(explanation.indictments.is_empty());
    }

    #[test]
    fn test_score_explanation_builder() {
        let obj = ObjectHandle::new(1);
        let constraint_match = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0))
            .with_indicted_object(obj);

        let indictment = Indictment::new(obj, ScoreDto::hard_soft(-1, 0))
            .with_constraint_match(constraint_match.clone());

        let explanation = ScoreExplanation::new(create_test_score())
            .with_constraint_match(constraint_match)
            .with_indictment(indictment);

        assert_eq!(explanation.constraint_count(), 1);
        assert_eq!(explanation.indictments.len(), 1);
    }

    #[test]
    fn test_score_explanation_get_by_name() {
        let m1 = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));
        let m2 = ConstraintMatch::new("teacherConflict", ScoreDto::hard_soft(-1, 0));
        let m3 = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));

        let explanation = ScoreExplanation::new(create_test_score())
            .with_constraint_match(m1)
            .with_constraint_match(m2)
            .with_constraint_match(m3);

        let room_matches = explanation.get_constraint_matches_by_name("roomConflict");
        assert_eq!(room_matches.len(), 2);

        let teacher_matches = explanation.get_constraint_matches_by_name("teacherConflict");
        assert_eq!(teacher_matches.len(), 1);
    }

    #[test]
    fn test_score_explanation_get_indictment() {
        let obj1 = ObjectHandle::new(1);
        let obj2 = ObjectHandle::new(2);

        let indictment1 = Indictment::new(obj1, ScoreDto::hard_soft(-1, 0));
        let indictment2 = Indictment::new(obj2, ScoreDto::hard_soft(0, -5));

        let explanation = ScoreExplanation::new(create_test_score())
            .with_indictment(indictment1)
            .with_indictment(indictment2);

        let found = explanation.get_indictment_for_object(obj1);
        assert!(found.is_some());
        assert!(!found.unwrap().is_feasible());

        let found2 = explanation.get_indictment_for_object(obj2);
        assert!(found2.is_some());
        assert!(found2.unwrap().is_feasible());

        let not_found = explanation.get_indictment_for_object(ObjectHandle::new(99));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_score_explanation_medium_score() {
        let score = ScoreDto::hard_medium_soft(0, -3, -10);
        let explanation = ScoreExplanation::new(score);

        assert_eq!(explanation.medium_score(), Some(-3));
    }

    #[test]
    fn test_constraint_match_new() {
        let cm = ConstraintMatch::new("testConstraint", create_test_score());

        assert_eq!(cm.constraint_name, "testConstraint");
        assert!(cm.constraint_package.is_none());
        assert_eq!(cm.hard_score(), -1);
        assert_eq!(cm.soft_score(), -10);
        assert!(!cm.is_feasible());
    }

    #[test]
    fn test_constraint_match_builder() {
        let obj1 = ObjectHandle::new(1);
        let obj2 = ObjectHandle::new(2);

        let cm = ConstraintMatch::new("roomConflict", create_feasible_score())
            .with_package("com.example.constraints")
            .with_justification(Value::String("Room A is overbooked".into()))
            .with_indicted_objects(vec![obj1, obj2]);

        assert_eq!(
            cm.constraint_package,
            Some("com.example.constraints".into())
        );
        assert!(cm.justification.is_some());
        assert_eq!(cm.indicted_objects.len(), 2);
        assert!(cm.is_feasible());
    }

    #[test]
    fn test_constraint_match_full_name() {
        let cm_no_pkg = ConstraintMatch::new("testConstraint", create_test_score());
        assert_eq!(cm_no_pkg.full_constraint_name(), "testConstraint");

        let cm_with_pkg =
            ConstraintMatch::new("roomConflict", create_test_score()).with_package("com.example");
        assert_eq!(
            cm_with_pkg.full_constraint_name(),
            "com.example.roomConflict"
        );
    }

    #[test]
    fn test_constraint_match_add_single_indicted() {
        let obj = ObjectHandle::new(1);
        let cm = ConstraintMatch::new("test", create_test_score()).with_indicted_object(obj);

        assert_eq!(cm.indicted_objects.len(), 1);
        assert_eq!(cm.indicted_objects[0], obj);
    }

    #[test]
    fn test_indictment_new() {
        let obj = ObjectHandle::new(42);
        let indictment = Indictment::new(obj, create_test_score());

        assert_eq!(indictment.indicted_object, obj);
        assert_eq!(indictment.hard_score(), -1);
        assert_eq!(indictment.soft_score(), -10);
        assert!(!indictment.is_feasible());
        assert_eq!(indictment.constraint_count(), 0);
    }

    #[test]
    fn test_indictment_with_matches() {
        let obj = ObjectHandle::new(1);
        let cm1 = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));
        let cm2 = ConstraintMatch::new("teacherConflict", ScoreDto::hard_soft(-1, 0));

        let indictment = Indictment::new(obj, ScoreDto::hard_soft(-2, 0))
            .with_constraint_match(cm1)
            .with_constraint_match(cm2);

        assert_eq!(indictment.constraint_count(), 2);
    }

    #[test]
    fn test_indictment_get_by_name() {
        let obj = ObjectHandle::new(1);
        let cm1 = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));
        let cm2 = ConstraintMatch::new("teacherConflict", ScoreDto::hard_soft(-1, 0));
        let cm3 = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));

        let indictment = Indictment::new(obj, ScoreDto::hard_soft(-3, 0))
            .with_constraint_match(cm1)
            .with_constraint_match(cm2)
            .with_constraint_match(cm3);

        let room_matches = indictment.get_constraint_matches_by_name("roomConflict");
        assert_eq!(room_matches.len(), 2);
    }

    #[test]
    fn test_score_explanation_json_serialization() {
        let explanation = ScoreExplanation::new(create_feasible_score())
            .with_constraint_match(ConstraintMatch::new("test", ScoreDto::hard_soft(0, -5)));

        let json = serde_json::to_string(&explanation).unwrap();
        assert!(json.contains("\"score\""));
        assert!(json.contains("\"constraintMatches\""));
        assert!(json.contains("\"indictments\""));

        let parsed: ScoreExplanation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.constraint_count(), 1);
    }

    #[test]
    fn test_constraint_match_json_omits_optional() {
        let cm = ConstraintMatch::new("test", create_test_score());
        let json = serde_json::to_string(&cm).unwrap();

        assert!(!json.contains("constraintPackage"));
        assert!(!json.contains("justification"));
    }

    #[test]
    fn test_indictment_json_serialization() {
        let obj = ObjectHandle::new(1);
        let indictment = Indictment::new(obj, create_test_score());

        let json = serde_json::to_string(&indictment).unwrap();
        assert!(json.contains("\"indictedObject\""));
        assert!(json.contains("\"constraintMatches\""));
        assert!(json.contains("\"score\""));

        let parsed: Indictment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.indicted_object, obj);
    }

    #[test]
    fn test_feasible_explanation() {
        let explanation = ScoreExplanation::new(create_feasible_score());
        assert!(explanation.is_feasible());
    }

    #[test]
    fn test_infeasible_explanation() {
        let explanation = ScoreExplanation::new(create_test_score());
        assert!(!explanation.is_feasible());
    }
}
