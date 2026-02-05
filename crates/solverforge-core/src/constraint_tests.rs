//! Tests for constraint types

use super::constraint::*;

#[test]
fn test_constraint_ref_full_name() {
    let cr = ConstraintRef::new("my.package", "TestConstraint");
    assert_eq!(cr.full_name(), "my.package/TestConstraint");
}

#[test]
fn test_constraint_ref_empty_package() {
    let cr = ConstraintRef::new("", "Simple");
    assert_eq!(cr.full_name(), "Simple");
}

#[test]
fn test_impact_type() {
    assert_ne!(ImpactType::Penalty, ImpactType::Reward);
}
