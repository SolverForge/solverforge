use crate::domain::descriptor::VariableDescriptor;
use crate::domain::variable::ValueRangeType;
use crate::domain::{ShadowVariableKind, VariableType};

#[test]
fn test_genuine() {
    let desc = VariableDescriptor::genuine("my_var");
    assert_eq!(desc.name, "my_var");
    assert_eq!(desc.variable_type, VariableType::Genuine);
    assert!(!desc.allows_unassigned);
    assert!(desc.variable_type.is_genuine());
    assert!(desc.variable_type.is_basic());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_chained() {
    let desc = VariableDescriptor::chained("previous");
    assert_eq!(desc.name, "previous");
    assert_eq!(desc.variable_type, VariableType::Chained);
    assert!(!desc.allows_unassigned); // Chained vars must point to something
    assert!(desc.variable_type.is_genuine()); // Chained is a genuine variable type
    assert!(!desc.variable_type.is_basic()); // But not basic
    assert!(desc.variable_type.is_chained());
}

#[test]
fn test_list() {
    let desc = VariableDescriptor::list("tasks");
    assert_eq!(desc.name, "tasks");
    assert_eq!(desc.variable_type, VariableType::List);
    assert!(desc.variable_type.is_list());
    assert!(desc.variable_type.is_genuine());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_piggyback() {
    let desc = VariableDescriptor::piggyback("arrival_time", "departure_time");
    assert_eq!(
        desc.variable_type,
        VariableType::Shadow(ShadowVariableKind::Piggyback)
    );
    assert!(desc.allows_unassigned);
    assert_eq!(desc.source_variable, Some("departure_time"));
    assert!(desc.variable_type.is_shadow());
    assert!(!desc.variable_type.is_genuine());
}

#[test]
fn test_with_value_range() {
    let desc = VariableDescriptor::genuine("var").with_value_range("range_provider");
    assert_eq!(desc.value_range_provider, Some("range_provider"));
}

#[test]
fn test_with_value_range_type() {
    let desc = VariableDescriptor::genuine("var")
        .with_value_range_type(ValueRangeType::CountableRange { from: 0, to: 100 });
    assert_eq!(
        desc.value_range_type,
        ValueRangeType::CountableRange { from: 0, to: 100 }
    );
}

#[test]
fn test_with_allows_unassigned() {
    let desc = VariableDescriptor::genuine("var").with_allows_unassigned(true);
    assert!(desc.allows_unassigned);
}
