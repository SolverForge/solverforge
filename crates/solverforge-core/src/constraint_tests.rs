// Tests for constraint types

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

use crate::constraint_weights::{
    ConstraintWeightError, ConstraintWeights, MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
};

#[test]
fn constraint_weights_default_to_multiplier_one() {
    let mut weights = ConstraintWeights::new();
    let travel = ConstraintRef::new("", "Travel");
    assert!(weights.is_empty());
    assert_eq!(weights.multiplier_for(&travel), 1);

    weights.set("Travel", 3).set("Overtime", 0);
    assert_eq!(weights.len(), 2);
    assert_eq!(weights.get("Travel"), Some(3));
    assert_eq!(weights.multiplier_for(&travel), 3);
    assert_eq!(
        weights.multiplier_for(&ConstraintRef::new("", "Overtime")),
        0
    );
    assert_eq!(
        weights.iter().collect::<Vec<_>>(),
        vec![("Overtime", 0), ("Travel", 3)]
    );
    assert_eq!(weights.remove("Travel"), Some(3));
    assert_eq!(weights.multiplier_for(&travel), 1);
}

#[test]
fn constraint_weights_key_on_full_name() {
    let mut weights = ConstraintWeights::new();
    weights.set("rostering/Overtime", 4);
    assert_eq!(
        weights.multiplier_for(&ConstraintRef::new("rostering", "Overtime")),
        4
    );
    assert_eq!(
        weights.multiplier_for(&ConstraintRef::new("", "Overtime")),
        1
    );
}

#[test]
fn constraint_weights_validate_rejects_unknown_names_with_suggestions() {
    let known = [
        ConstraintRef::new("", "Travel time"),
        ConstraintRef::new("", "Overtime"),
    ];
    let mut weights = ConstraintWeights::new();
    weights.set("Overtime", 2);
    assert_eq!(weights.validate(&known), Ok(()));

    weights.set("overtme", 1);
    let error = weights.validate(&known).unwrap_err();
    assert_eq!(
        error,
        ConstraintWeightError::UnknownConstraint {
            name: "overtme".to_string(),
            suggestions: vec!["Overtime".to_string()],
            known: vec!["Overtime".to_string(), "Travel time".to_string()],
        }
    );
    let message = error.to_string();
    assert!(message.contains("unknown constraint \"overtme\""));
    assert!(message.contains("did you mean \"Overtime\""));
    assert!(message.contains("\"Travel time\""));
}

#[test]
fn constraint_weights_validate_rejects_multipliers_above_cap() {
    let known = [ConstraintRef::new("", "Travel")];
    let mut weights = ConstraintWeights::new();
    weights.set("Travel", MAX_CONSTRAINT_WEIGHT_MULTIPLIER);
    assert_eq!(weights.validate(&known), Ok(()));

    weights.set("Travel", MAX_CONSTRAINT_WEIGHT_MULTIPLIER + 1);
    assert_eq!(
        weights.validate(&known),
        Err(ConstraintWeightError::MultiplierTooLarge {
            name: "Travel".to_string(),
            multiplier: MAX_CONSTRAINT_WEIGHT_MULTIPLIER + 1,
            max: MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
        })
    );
}

#[test]
fn constraint_weights_validate_reports_empty_known_set() {
    let mut weights = ConstraintWeights::new();
    weights.set("Anything", 1);
    let message = weights.validate(&[]).unwrap_err().to_string();
    assert!(message.ends_with("known constraints: none"), "{message}");
}
