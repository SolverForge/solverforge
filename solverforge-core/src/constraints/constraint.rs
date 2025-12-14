use crate::constraints::{ConstraintAnalysis, StreamComponent, WasmFunction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub components: Vec<StreamComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indictment: Option<WasmFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<WasmFunction>,
    #[serde(
        rename = "incrementalMetadata",
        skip_serializing_if = "Option::is_none"
    )]
    pub incremental_metadata: Option<ConstraintAnalysis>,
}

impl Constraint {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            package: None,
            description: None,
            group: None,
            components: Vec::new(),
            indictment: None,
            justification: None,
            incremental_metadata: None,
        }
    }

    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn with_component(mut self, component: StreamComponent) -> Self {
        self.components.push(component);
        self
    }

    pub fn with_components(mut self, components: Vec<StreamComponent>) -> Self {
        self.components = components;
        self
    }

    pub fn with_indictment(mut self, indictment: WasmFunction) -> Self {
        self.indictment = Some(indictment);
        self
    }

    pub fn with_justification(mut self, justification: WasmFunction) -> Self {
        self.justification = Some(justification);
        self
    }

    pub fn with_incremental_metadata(mut self, metadata: ConstraintAnalysis) -> Self {
        self.incremental_metadata = Some(metadata);
        self
    }

    pub fn full_name(&self) -> String {
        match &self.package {
            Some(pkg) => format!("{}/{}", pkg, self.name),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConstraintSet {
    pub constraints: Vec<Constraint>,
}

impl ConstraintSet {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.iter()
    }

    pub fn to_dto(&self) -> indexmap::IndexMap<String, crate::solver::ConstraintDto> {
        self.constraints
            .iter()
            .map(|c| {
                let dto = crate::solver::ConstraintDto::new(c.components.clone());
                let dto = if let Some(ref metadata) = c.incremental_metadata {
                    dto.with_incremental_metadata(metadata.clone())
                } else {
                    dto
                };
                (c.full_name(), dto)
            })
            .collect()
    }

    /// Analyze all constraints in this set for incremental scoring eligibility.
    ///
    /// This method analyzes each constraint's stream components to determine
    /// whether it supports incremental scoring. The analysis result is stored
    /// in each constraint's `incremental_metadata` field.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_core::constraints::ConstraintSet;
    ///
    /// let mut constraint_set = ConstraintSet::new();
    /// // ... add constraints ...
    /// constraint_set.analyze_incrementality();
    /// ```
    pub fn analyze_incrementality(&mut self) {
        use crate::constraints::ConstraintAnalyzer;

        for constraint in &mut self.constraints {
            let analysis = ConstraintAnalyzer::analyze(constraint);
            constraint.incremental_metadata = Some(analysis);
        }
    }
}

impl FromIterator<Constraint> for ConstraintSet {
    fn from_iter<I: IntoIterator<Item = Constraint>>(iter: I) -> Self {
        ConstraintSet {
            constraints: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Joiner;

    #[test]
    fn test_constraint_new() {
        let constraint = Constraint::new("Room conflict");
        assert_eq!(constraint.name, "Room conflict");
        assert!(constraint.package.is_none());
        assert!(constraint.components.is_empty());
    }

    #[test]
    fn test_constraint_with_package() {
        let constraint = Constraint::new("Room conflict").with_package("timetabling");
        assert_eq!(constraint.package, Some("timetabling".to_string()));
    }

    #[test]
    fn test_constraint_with_description() {
        let constraint =
            Constraint::new("Room conflict").with_description("Two lessons in same room");
        assert_eq!(
            constraint.description,
            Some("Two lessons in same room".to_string())
        );
    }

    #[test]
    fn test_constraint_with_group() {
        let constraint = Constraint::new("Room conflict").with_group("Hard constraints");
        assert_eq!(constraint.group, Some("Hard constraints".to_string()));
    }

    #[test]
    fn test_constraint_with_component() {
        let constraint = Constraint::new("Room conflict")
            .with_component(StreamComponent::for_each("Lesson"))
            .with_component(StreamComponent::penalize("1hard"));
        assert_eq!(constraint.components.len(), 2);
    }

    #[test]
    fn test_constraint_with_components() {
        let components = vec![
            StreamComponent::for_each("Lesson"),
            StreamComponent::penalize("1hard"),
        ];
        let constraint = Constraint::new("Room conflict").with_components(components);
        assert_eq!(constraint.components.len(), 2);
    }

    #[test]
    fn test_constraint_with_indictment() {
        let constraint =
            Constraint::new("Room conflict").with_indictment(WasmFunction::new("get_room"));
        assert!(constraint.indictment.is_some());
    }

    #[test]
    fn test_constraint_with_justification() {
        let constraint = Constraint::new("Room conflict")
            .with_justification(WasmFunction::new("create_justification"));
        assert!(constraint.justification.is_some());
    }

    #[test]
    fn test_constraint_full_name() {
        let constraint1 = Constraint::new("Room conflict");
        assert_eq!(constraint1.full_name(), "Room conflict");

        let constraint2 = Constraint::new("Room conflict").with_package("timetabling");
        assert_eq!(constraint2.full_name(), "timetabling/Room conflict");
    }

    #[test]
    fn test_constraint_set_new() {
        let set = ConstraintSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_constraint_set_with_constraint() {
        let set = ConstraintSet::new()
            .with_constraint(Constraint::new("Constraint 1"))
            .with_constraint(Constraint::new("Constraint 2"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_constraint_set_add_constraint() {
        let mut set = ConstraintSet::new();
        set.add_constraint(Constraint::new("Constraint 1"));
        set.add_constraint(Constraint::new("Constraint 2"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_constraint_set_iter() {
        let set = ConstraintSet::new()
            .with_constraint(Constraint::new("C1"))
            .with_constraint(Constraint::new("C2"));

        let names: Vec<_> = set.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["C1", "C2"]);
    }

    #[test]
    fn test_constraint_set_from_iter() {
        let constraints = vec![Constraint::new("C1"), Constraint::new("C2")];
        let set: ConstraintSet = constraints.into_iter().collect();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_constraint_json_serialization() {
        let constraint = Constraint::new("Room conflict")
            .with_package("timetabling")
            .with_component(StreamComponent::for_each_unique_pair_with_joiners(
                "Lesson",
                vec![Joiner::equal(WasmFunction::new("get_timeslot"))],
            ))
            .with_component(StreamComponent::filter(WasmFunction::new("same_room")))
            .with_component(StreamComponent::penalize("1hard"));

        let json = serde_json::to_string(&constraint).unwrap();
        assert!(json.contains("\"name\":\"Room conflict\""));
        assert!(json.contains("\"package\":\"timetabling\""));
        assert!(json.contains("\"components\""));

        let parsed: Constraint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, constraint);
    }

    #[test]
    fn test_constraint_set_json_serialization() {
        let set = ConstraintSet::new()
            .with_constraint(
                Constraint::new("C1")
                    .with_component(StreamComponent::for_each("Lesson"))
                    .with_component(StreamComponent::penalize("1hard")),
            )
            .with_constraint(
                Constraint::new("C2")
                    .with_component(StreamComponent::for_each("Room"))
                    .with_component(StreamComponent::reward("1soft")),
            );

        let json = serde_json::to_string(&set).unwrap();
        let parsed: ConstraintSet = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_realistic_room_conflict_constraint() {
        let constraint = Constraint::new("Room conflict")
            .with_package("school.timetabling")
            .with_description("A room can accommodate at most one lesson at the same time.")
            .with_group("Hard constraints")
            .with_component(StreamComponent::for_each_unique_pair_with_joiners(
                "Lesson",
                vec![
                    Joiner::equal(WasmFunction::new("get_timeslot")),
                    Joiner::equal(WasmFunction::new("get_room")),
                ],
            ))
            .with_component(StreamComponent::penalize("1hard"));

        assert_eq!(constraint.components.len(), 2);
        assert_eq!(constraint.full_name(), "school.timetabling/Room conflict");
    }

    #[test]
    fn test_constraint_clone() {
        let constraint = Constraint::new("Test")
            .with_package("pkg")
            .with_component(StreamComponent::for_each("Entity"));
        let cloned = constraint.clone();
        assert_eq!(constraint, cloned);
    }

    #[test]
    fn test_constraint_debug() {
        let constraint = Constraint::new("Test");
        let debug = format!("{:?}", constraint);
        assert!(debug.contains("Constraint"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn test_constraint_with_incremental_metadata() {
        use crate::constraints::{ConstraintAnalysis, IncrementalSupport};

        let metadata = ConstraintAnalysis {
            name: "testConstraint".to_string(),
            support: IncrementalSupport::FullyIncremental,
            affected_entities: vec!["Shift".to_string()],
            reason: None,
        };

        let constraint =
            Constraint::new("testConstraint").with_incremental_metadata(metadata.clone());

        assert!(constraint.incremental_metadata.is_some());
        assert_eq!(constraint.incremental_metadata.unwrap(), metadata);
    }

    #[test]
    fn test_constraint_set_analyze_incrementality() {
        let mut set = ConstraintSet::new()
            .with_constraint(
                Constraint::new("simpleConstraint")
                    .with_component(StreamComponent::for_each("Shift"))
                    .with_component(StreamComponent::filter(WasmFunction::new("predicate")))
                    .with_component(StreamComponent::penalize("1hard")),
            )
            .with_constraint(
                Constraint::new("complementConstraint")
                    .with_component(StreamComponent::for_each("Shift"))
                    .with_component(StreamComponent::complement("Employee"))
                    .with_component(StreamComponent::penalize("1hard")),
            );

        // Before analysis, metadata should be None
        assert!(set.constraints[0].incremental_metadata.is_none());
        assert!(set.constraints[1].incremental_metadata.is_none());

        // Analyze incrementality
        set.analyze_incrementality();

        // After analysis, metadata should be populated
        assert!(set.constraints[0].incremental_metadata.is_some());
        assert!(set.constraints[1].incremental_metadata.is_some());

        // Simple constraint should be incremental
        let metadata0 = set.constraints[0].incremental_metadata.as_ref().unwrap();
        assert_eq!(
            metadata0.support,
            crate::constraints::IncrementalSupport::FullyIncremental
        );

        // Complement constraint should be non-incremental
        let metadata1 = set.constraints[1].incremental_metadata.as_ref().unwrap();
        assert_eq!(
            metadata1.support,
            crate::constraints::IncrementalSupport::NonIncremental
        );
    }

    #[test]
    fn test_incremental_metadata_serialization() {
        use crate::constraints::{ConstraintAnalysis, IncrementalSupport};

        let metadata = ConstraintAnalysis {
            name: "testConstraint".to_string(),
            support: IncrementalSupport::FullyIncremental,
            affected_entities: vec!["Shift".to_string()],
            reason: None,
        };

        let constraint = Constraint::new("testConstraint")
            .with_component(StreamComponent::for_each("Shift"))
            .with_component(StreamComponent::penalize("1hard"))
            .with_incremental_metadata(metadata);

        // Serialize to JSON
        let json = serde_json::to_string(&constraint).unwrap();

        // Should contain incremental metadata
        assert!(json.contains("incrementalMetadata"));
        assert!(json.contains("FULLY_INCREMENTAL"));

        // Deserialize back
        let parsed: Constraint = serde_json::from_str(&json).unwrap();
        assert!(parsed.incremental_metadata.is_some());
        assert_eq!(
            parsed.incremental_metadata.as_ref().unwrap().support,
            IncrementalSupport::FullyIncremental
        );
    }

    #[test]
    fn test_incremental_metadata_omitted_when_none() {
        let constraint = Constraint::new("testConstraint")
            .with_component(StreamComponent::for_each("Shift"))
            .with_component(StreamComponent::penalize("1hard"));

        // Serialize to JSON
        let json = serde_json::to_string(&constraint).unwrap();

        // Should NOT contain incremental metadata when None
        assert!(!json.contains("incrementalMetadata"));

        // Deserialize back
        let parsed: Constraint = serde_json::from_str(&json).unwrap();
        assert!(parsed.incremental_metadata.is_none());
    }
}
