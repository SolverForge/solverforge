//! Tests for descriptor types.
//!
//! Note: Entity extraction is now done through generated methods on solution types
//! (zero-erasure architecture). These tests cover descriptor metadata only.

use super::*;
use crate::domain::{ShadowVariableKind, VariableType};
use std::any::TypeId;

#[derive(Clone, Debug)]
struct TestEntity {
    id: i64,
    row: Option<i32>,
}

#[derive(Clone, Debug)]
struct TestSolution {
    entities: Vec<TestEntity>,
}

// EntityDescriptor metadata tests

#[test]
fn test_entity_descriptor_basic() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    assert_eq!(descriptor.type_name, "TestEntity");
    assert_eq!(descriptor.solution_field, "entities");
    assert!(descriptor.is_collection);
    assert!(descriptor.variable_descriptors.is_empty());
}

#[test]
fn test_entity_descriptor_with_variable() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"));

    assert_eq!(descriptor.variable_descriptors.len(), 1);
    assert_eq!(descriptor.variable_descriptors[0].name, "row");
}

#[test]
fn test_entity_descriptor_with_id_field() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_id_field("id");

    assert_eq!(descriptor.id_field, Some("id"));
}

#[test]
fn test_entity_descriptor_with_pin_field() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_pin_field("pinned");

    assert_eq!(descriptor.pin_field, Some("pinned"));
}

#[test]
fn test_entity_descriptor_find_variable() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_variable(VariableDescriptor::genuine("column"));

    let found = descriptor.find_variable("row");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "row");

    let not_found = descriptor.find_variable("nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_entity_descriptor_genuine_variables() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_variable(VariableDescriptor::shadow(
            "computed",
            ShadowVariableKind::Custom,
        ));

    let genuine: Vec<_> = descriptor.genuine_variable_descriptors().collect();
    assert_eq!(genuine.len(), 1);
    assert_eq!(genuine[0].name, "row");
}

#[test]
fn test_entity_descriptor_shadow_variables() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_variable(VariableDescriptor::shadow(
            "computed",
            ShadowVariableKind::Custom,
        ));

    let shadows: Vec<_> = descriptor.shadow_variable_descriptors().collect();
    assert_eq!(shadows.len(), 1);
    assert_eq!(shadows[0].name, "computed");
}

#[test]
fn test_entity_descriptor_has_genuine_variables() {
    let desc_with_genuine =
        EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
            .with_variable(VariableDescriptor::genuine("row"));
    assert!(desc_with_genuine.has_genuine_variables());

    let desc_shadow_only =
        EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities").with_variable(
            VariableDescriptor::shadow("computed", ShadowVariableKind::Custom),
        );
    assert!(!desc_shadow_only.has_genuine_variables());

    let desc_empty = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");
    assert!(!desc_empty.has_genuine_variables());
}

#[test]
fn test_entity_descriptor_clone() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_id_field("id");

    let cloned = descriptor.clone();
    assert_eq!(cloned.type_name, descriptor.type_name);
    assert_eq!(cloned.variable_descriptors.len(), 1);
    assert_eq!(cloned.id_field, Some("id"));
}

// ProblemFactDescriptor tests

#[test]
fn test_problem_fact_descriptor_basic() {
    let descriptor = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts");

    assert_eq!(descriptor.type_name, "SomeFact");
    assert_eq!(descriptor.solution_field, "facts");
    assert!(descriptor.is_collection);
}

#[test]
fn test_problem_fact_descriptor_single() {
    let descriptor = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "fact").single();

    assert!(!descriptor.is_collection);
}

#[test]
fn test_problem_fact_descriptor_with_id_field() {
    let descriptor =
        ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts").with_id_field("id");

    assert_eq!(descriptor.id_field, Some("id"));
}

#[test]
fn test_problem_fact_descriptor_clone() {
    let descriptor = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts")
        .with_id_field("id")
        .single();

    let cloned = descriptor.clone();
    assert_eq!(cloned.type_name, descriptor.type_name);
    assert_eq!(cloned.id_field, Some("id"));
    assert!(!cloned.is_collection);
}

// SolutionDescriptor tests

#[test]
fn test_solution_descriptor_basic() {
    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());

    assert_eq!(solution_desc.type_name, "TestSolution");
    assert_eq!(solution_desc.score_field, "score");
    assert!(solution_desc.entity_descriptors.is_empty());
    assert!(solution_desc.problem_fact_descriptors.is_empty());
}

#[test]
fn test_solution_descriptor_with_entity() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    assert_eq!(solution_desc.entity_descriptor_count(), 1);
}

#[test]
fn test_solution_descriptor_with_problem_fact() {
    let fact_desc = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_problem_fact(fact_desc);

    assert_eq!(solution_desc.problem_fact_descriptor_count(), 1);
}

#[test]
fn test_solution_descriptor_with_score_field() {
    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_score_field("custom_score");

    assert_eq!(solution_desc.score_field, "custom_score");
}

#[test]
fn test_solution_descriptor_find_entity_descriptor() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let found = solution_desc.find_entity_descriptor("TestEntity");
    assert!(found.is_some());
    assert_eq!(found.unwrap().type_name, "TestEntity");

    let not_found = solution_desc.find_entity_descriptor("NonExistent");
    assert!(not_found.is_none());
}

#[test]
fn test_solution_descriptor_find_entity_descriptor_by_type() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let found = solution_desc.find_entity_descriptor_by_type(TypeId::of::<TestEntity>());
    assert!(found.is_some());
    assert_eq!(found.unwrap().type_name, "TestEntity");

    let not_found = solution_desc.find_entity_descriptor_by_type(TypeId::of::<i32>());
    assert!(not_found.is_none());
}

#[test]
fn test_solution_descriptor_genuine_variable_descriptors() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_variable(VariableDescriptor::shadow(
            "computed",
            ShadowVariableKind::Custom,
        ));

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let genuine = solution_desc.genuine_variable_descriptors();
    assert_eq!(genuine.len(), 1);
    assert_eq!(genuine[0].name, "row");
}

#[test]
fn test_solution_descriptor_shadow_variable_descriptors() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_variable(VariableDescriptor::genuine("row"))
        .with_variable(VariableDescriptor::shadow(
            "computed",
            ShadowVariableKind::Custom,
        ));

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let shadows = solution_desc.shadow_variable_descriptors();
    assert_eq!(shadows.len(), 1);
    assert_eq!(shadows[0].name, "computed");
}

#[test]
fn test_solution_descriptor_counts() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");
    let fact_desc = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc)
        .with_problem_fact(fact_desc);

    assert_eq!(solution_desc.entity_descriptor_count(), 1);
    assert_eq!(solution_desc.problem_fact_descriptor_count(), 1);
}

#[test]
fn test_solution_descriptor_clone() {
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc)
        .with_score_field("score");

    let cloned = solution_desc.clone();
    assert_eq!(cloned.type_name, solution_desc.type_name);
    assert_eq!(cloned.entity_descriptor_count(), 1);
}

// ============ VariableDescriptor Tests ============

#[test]
fn test_variable_descriptor_genuine() {
    let desc = VariableDescriptor::genuine("my_var");
    assert_eq!(desc.name, "my_var");
    assert_eq!(desc.variable_type, VariableType::Genuine);
    assert!(!desc.allows_unassigned);
    assert!(desc.variable_type.is_genuine());
    assert!(desc.variable_type.is_basic());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_chained() {
    let desc = VariableDescriptor::chained("previous");
    assert_eq!(desc.name, "previous");
    assert_eq!(desc.variable_type, VariableType::Chained);
    assert!(!desc.allows_unassigned); // Chained vars must point to something
    assert!(desc.variable_type.is_genuine()); // Chained is a genuine variable type
    assert!(!desc.variable_type.is_basic()); // But not basic
    assert!(desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_list() {
    let desc = VariableDescriptor::list("tasks");
    assert_eq!(desc.name, "tasks");
    assert_eq!(desc.variable_type, VariableType::List);
    assert!(desc.variable_type.is_list());
    assert!(desc.variable_type.is_genuine());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_with_value_range() {
    let desc = VariableDescriptor::genuine("var").with_value_range("range_provider");
    assert_eq!(desc.value_range_provider, Some("range_provider"));
}

#[test]
fn test_variable_descriptor_with_allows_unassigned() {
    let desc = VariableDescriptor::genuine("var").with_allows_unassigned(true);
    assert!(desc.allows_unassigned);
}
