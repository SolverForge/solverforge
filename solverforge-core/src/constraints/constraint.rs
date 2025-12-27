use crate::constraints::{StreamComponent, WasmFunction};
use crate::wasm::PredicateDefinition;
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

    pub fn to_dto(&self) -> indexmap::IndexMap<String, Vec<StreamComponent>> {
        self.constraints
            .iter()
            .map(|c| (c.name.clone(), c.components.clone()))
            .collect()
    }

    /// Extracts all predicate definitions from the constraints.
    ///
    /// This walks through all constraint components and extracts `WasmFunction`s
    /// that have associated expressions, converting them to `PredicateDefinition`s
    /// for compilation into the WASM module.
    pub fn extract_predicates(&self) -> Vec<PredicateDefinition> {
        let mut predicates = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for constraint in &self.constraints {
            Self::collect_predicates_from_components(
                &constraint.components,
                &mut predicates,
                &mut seen,
            );
        }

        predicates
    }

    fn collect_predicates_from_components(
        components: &[StreamComponent],
        predicates: &mut Vec<PredicateDefinition>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for component in components {
            Self::collect_from_component(component, predicates, seen);
        }
    }

    fn collect_from_component(
        component: &StreamComponent,
        predicates: &mut Vec<PredicateDefinition>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match component {
            StreamComponent::Filter { predicate } => {
                Self::add_predicate_if_new(predicate, 1, predicates, seen);
            }
            StreamComponent::Penalize {
                scale_by: Some(scale_by),
                ..
            }
            | StreamComponent::Reward {
                scale_by: Some(scale_by),
                ..
            }
            | StreamComponent::Impact {
                scale_by: Some(scale_by),
                ..
            } => {
                // Scale functions can have different arities depending on stream type
                // Most commonly it's 1 (single entity) but could be 2+ for joins
                Self::add_predicate_if_new(scale_by, 1, predicates, seen);
            }
            StreamComponent::Map { mappers } | StreamComponent::Expand { mappers } => {
                for mapper in mappers {
                    Self::add_predicate_if_new(mapper, 1, predicates, seen);
                }
            }
            StreamComponent::GroupBy { keys, .. } => {
                for key in keys {
                    Self::add_predicate_if_new(key, 1, predicates, seen);
                }
            }
            StreamComponent::FlattenLast { map: Some(map) } => {
                Self::add_predicate_if_new(map, 1, predicates, seen);
            }
            StreamComponent::IndictWith {
                indicted_object_provider,
            } => {
                Self::add_predicate_if_new(indicted_object_provider, 1, predicates, seen);
            }
            StreamComponent::JustifyWith {
                justification_supplier,
            } => {
                Self::add_predicate_if_new(justification_supplier, 1, predicates, seen);
            }
            StreamComponent::Concat { other_components } => {
                Self::collect_predicates_from_components(other_components, predicates, seen);
            }
            StreamComponent::ForEachUniquePair { joiners, .. }
            | StreamComponent::Join { joiners, .. }
            | StreamComponent::IfExists { joiners, .. }
            | StreamComponent::IfNotExists { joiners, .. }
            | StreamComponent::IfExistsOther { joiners, .. }
            | StreamComponent::IfNotExistsOther { joiners, .. }
            | StreamComponent::IfExistsIncludingUnassigned { joiners, .. }
            | StreamComponent::IfNotExistsIncludingUnassigned { joiners, .. } => {
                Self::collect_from_joiners(joiners, predicates, seen);
            }
            // Components without functions
            StreamComponent::ForEach { .. }
            | StreamComponent::ForEachIncludingUnassigned { .. }
            | StreamComponent::Complement { .. }
            | StreamComponent::Distinct
            | StreamComponent::Penalize { scale_by: None, .. }
            | StreamComponent::Reward { scale_by: None, .. }
            | StreamComponent::Impact { scale_by: None, .. }
            | StreamComponent::FlattenLast { map: None } => {}
        }
    }

    fn collect_from_joiners(
        joiners: &[crate::constraints::Joiner],
        predicates: &mut Vec<PredicateDefinition>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        use crate::constraints::Joiner;
        for joiner in joiners {
            match joiner {
                Joiner::Equal {
                    map,
                    left_map,
                    right_map,
                    relation_predicate,
                    hasher,
                } => {
                    if let Some(f) = map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = left_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = right_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = relation_predicate {
                        Self::add_predicate_if_new(f, 2, predicates, seen);
                    }
                    if let Some(f) = hasher {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                }
                Joiner::LessThan {
                    map,
                    left_map,
                    right_map,
                    comparator,
                }
                | Joiner::LessThanOrEqual {
                    map,
                    left_map,
                    right_map,
                    comparator,
                }
                | Joiner::GreaterThan {
                    map,
                    left_map,
                    right_map,
                    comparator,
                }
                | Joiner::GreaterThanOrEqual {
                    map,
                    left_map,
                    right_map,
                    comparator,
                } => {
                    if let Some(f) = map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = left_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = right_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    Self::add_predicate_if_new(comparator, 2, predicates, seen);
                }
                Joiner::Overlapping {
                    start_map,
                    end_map,
                    left_start_map,
                    left_end_map,
                    right_start_map,
                    right_end_map,
                    comparator,
                } => {
                    if let Some(f) = start_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = end_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = left_start_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = left_end_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = right_start_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = right_end_map {
                        Self::add_predicate_if_new(f, 1, predicates, seen);
                    }
                    if let Some(f) = comparator {
                        Self::add_predicate_if_new(f, 2, predicates, seen);
                    }
                }
                Joiner::Filtering { filter } => {
                    Self::add_predicate_if_new(filter, 2, predicates, seen);
                }
            }
        }
    }

    fn add_predicate_if_new(
        func: &WasmFunction,
        arity: u32,
        predicates: &mut Vec<PredicateDefinition>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if let Some(expr) = func.expression() {
            if !seen.contains(func.name()) {
                seen.insert(func.name().to_string());
                predicates.push(PredicateDefinition::from_expression(
                    func.name(),
                    arity,
                    expr.clone(),
                ));
            }
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
}
