// Consolidated tests for api module.
//
// Tests extracted from:
// - weight_overrides.rs (8 tests)

use std::sync::Arc;

use solverforge_core::score::{HardSoftScore, SoftScore};

use crate::api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// ============================================================================
// ConstraintWeightOverrides tests
// ============================================================================

#[test]
fn test_new_is_empty() {
    let overrides = ConstraintWeightOverrides::<SoftScore>::new();
    assert!(overrides.is_empty());
    assert_eq!(overrides.len(), 0);
}

#[test]
fn test_put_and_get() {
    let mut overrides = ConstraintWeightOverrides::<SoftScore>::new();
    overrides.put("test", SoftScore::of(5));

    assert!(overrides.contains("test"));
    assert_eq!(overrides.get("test"), Some(&SoftScore::of(5)));
}

#[test]
fn test_get_or_default_with_override() {
    let mut overrides = ConstraintWeightOverrides::<SoftScore>::new();
    overrides.put("test", SoftScore::of(5));

    let weight = overrides.get_or_default("test", SoftScore::of(1));
    assert_eq!(weight, SoftScore::of(5));
}

#[test]
fn test_get_or_default_without_override() {
    let overrides = ConstraintWeightOverrides::<SoftScore>::new();

    let weight = overrides.get_or_default("test", SoftScore::of(1));
    assert_eq!(weight, SoftScore::of(1));
}

#[test]
fn test_remove() {
    let mut overrides = ConstraintWeightOverrides::<SoftScore>::new();
    overrides.put("test", SoftScore::of(5));

    let removed = overrides.remove("test");
    assert_eq!(removed, Some(SoftScore::of(5)));
    assert!(!overrides.contains("test"));
}

#[test]
fn test_from_pairs() {
    let overrides = ConstraintWeightOverrides::<HardSoftScore>::from_pairs([
        ("hard_constraint", HardSoftScore::of_hard(1)),
        ("soft_constraint", HardSoftScore::of_soft(10)),
    ]);

    assert_eq!(overrides.len(), 2);
    assert_eq!(
        overrides.get("hard_constraint"),
        Some(&HardSoftScore::of_hard(1))
    );
    assert_eq!(
        overrides.get("soft_constraint"),
        Some(&HardSoftScore::of_soft(10))
    );
}

#[test]
fn test_weight_provider_trait() {
    let mut overrides = ConstraintWeightOverrides::<SoftScore>::new();
    overrides.put("test", SoftScore::of(5));

    let provider: &dyn WeightProvider<SoftScore> = &overrides;
    assert_eq!(provider.weight("test"), Some(SoftScore::of(5)));
    assert_eq!(provider.weight("other"), None);
    assert_eq!(
        provider.weight_or_default("other", SoftScore::of(1)),
        SoftScore::of(1)
    );
}

#[test]
fn test_arc_weight_provider() {
    let mut overrides = ConstraintWeightOverrides::<SoftScore>::new();
    overrides.put("test", SoftScore::of(5));
    let arc_overrides: Arc<ConstraintWeightOverrides<SoftScore>> = overrides.into_arc();

    let provider: &dyn WeightProvider<SoftScore> = &arc_overrides;
    assert_eq!(provider.weight("test"), Some(SoftScore::of(5)));
}
