use std::any::TypeId;

use crate::domain::{ChainedVariableInfo, ShadowVariableKind, VariableType};

#[test]
fn test_variable_type_is_genuine() {
    assert!(VariableType::Genuine.is_genuine());
    assert!(VariableType::Chained.is_genuine());
    assert!(VariableType::List.is_genuine());
    assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_genuine());
}

#[test]
fn test_variable_type_is_shadow() {
    assert!(!VariableType::Genuine.is_shadow());
    assert!(!VariableType::Chained.is_shadow());
    assert!(!VariableType::List.is_shadow());
    assert!(VariableType::Shadow(ShadowVariableKind::Custom).is_shadow());
    assert!(VariableType::Shadow(ShadowVariableKind::InverseRelation).is_shadow());
}

#[test]
fn test_variable_type_is_chained() {
    assert!(!VariableType::Genuine.is_chained());
    assert!(VariableType::Chained.is_chained());
    assert!(!VariableType::List.is_chained());
    assert!(!VariableType::Shadow(ShadowVariableKind::Anchor).is_chained());
}

#[test]
fn test_variable_type_is_list() {
    assert!(!VariableType::Genuine.is_list());
    assert!(!VariableType::Chained.is_list());
    assert!(VariableType::List.is_list());
    assert!(!VariableType::Shadow(ShadowVariableKind::Index).is_list());
}

#[test]
fn test_variable_type_is_basic() {
    assert!(VariableType::Genuine.is_basic());
    assert!(!VariableType::Chained.is_basic());
    assert!(!VariableType::List.is_basic());
    assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_basic());
}

#[test]
fn test_shadow_variable_kind_requires_listener() {
    assert!(ShadowVariableKind::Custom.requires_listener());
    assert!(ShadowVariableKind::Cascading.requires_listener());
    assert!(!ShadowVariableKind::InverseRelation.requires_listener());
    assert!(!ShadowVariableKind::Index.requires_listener());
    assert!(!ShadowVariableKind::Anchor.requires_listener());
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
    assert!(ShadowVariableKind::Anchor.is_automatic());
    assert!(!ShadowVariableKind::Piggyback.is_automatic());
}

#[test]
fn test_shadow_variable_kind_is_piggyback() {
    assert!(ShadowVariableKind::Piggyback.is_piggyback());
    assert!(!ShadowVariableKind::Custom.is_piggyback());
    assert!(!ShadowVariableKind::Anchor.is_piggyback());
}

struct TestAnchor;
struct TestEntity;

#[test]
fn test_chained_variable_info_new() {
    let info = ChainedVariableInfo::new::<TestAnchor, TestEntity>();

    assert_eq!(info.anchor_type_id, TypeId::of::<TestAnchor>());
    assert_eq!(info.entity_type_id, TypeId::of::<TestEntity>());
    assert!(!info.has_anchor_shadow);
}

#[test]
fn test_chained_variable_info_with_anchor_shadow() {
    let info = ChainedVariableInfo::with_anchor_shadow::<TestAnchor, TestEntity>();

    assert_eq!(info.anchor_type_id, TypeId::of::<TestAnchor>());
    assert_eq!(info.entity_type_id, TypeId::of::<TestEntity>());
    assert!(info.has_anchor_shadow);
}

#[test]
fn test_chained_variable_info_type_checks() {
    let info = ChainedVariableInfo::new::<TestAnchor, TestEntity>();

    assert!(info.is_anchor_type(TypeId::of::<TestAnchor>()));
    assert!(!info.is_anchor_type(TypeId::of::<TestEntity>()));

    assert!(info.is_entity_type(TypeId::of::<TestEntity>()));
    assert!(!info.is_entity_type(TypeId::of::<TestAnchor>()));
}
