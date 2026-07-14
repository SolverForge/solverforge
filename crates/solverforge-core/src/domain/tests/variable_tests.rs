use crate::domain::{ShadowVariableKind, VariableType};

#[test]
fn test_variable_type_is_genuine() {
    assert!(VariableType::Genuine.is_genuine());
    assert!(VariableType::List.is_genuine());
    assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_genuine());
}

#[test]
fn test_variable_type_is_shadow() {
    assert!(!VariableType::Genuine.is_shadow());
    assert!(!VariableType::List.is_shadow());
    assert!(VariableType::Shadow(ShadowVariableKind::Custom).is_shadow());
    assert!(VariableType::Shadow(ShadowVariableKind::InverseRelation).is_shadow());
}

#[test]
fn test_variable_type_is_list() {
    assert!(!VariableType::Genuine.is_list());
    assert!(VariableType::List.is_list());
    assert!(!VariableType::Shadow(ShadowVariableKind::Index).is_list());
}

#[test]
fn test_variable_type_is_basic() {
    assert!(VariableType::Genuine.is_basic());
    assert!(!VariableType::List.is_basic());
    assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_basic());
}

#[test]
fn test_shadow_variable_kind_requires_listener() {
    assert!(ShadowVariableKind::Custom.requires_listener());
    assert!(ShadowVariableKind::Cascading.requires_listener());
    assert!(!ShadowVariableKind::InverseRelation.requires_listener());
    assert!(!ShadowVariableKind::Index.requires_listener());
    assert!(!ShadowVariableKind::Piggyback.requires_listener());
}

#[test]
fn test_shadow_variable_kind_is_automatic() {
    assert!(!ShadowVariableKind::Custom.is_automatic());
    assert!(!ShadowVariableKind::Cascading.is_automatic());
    assert!(ShadowVariableKind::InverseRelation.is_automatic());
    assert!(ShadowVariableKind::Index.is_automatic());
    assert!(ShadowVariableKind::NextElement.is_automatic());
    assert!(ShadowVariableKind::PreviousElement.is_automatic());
    assert!(!ShadowVariableKind::Piggyback.is_automatic());
}

#[test]
fn test_shadow_variable_kind_is_piggyback() {
    assert!(ShadowVariableKind::Piggyback.is_piggyback());
    assert!(!ShadowVariableKind::Custom.is_piggyback());
}
