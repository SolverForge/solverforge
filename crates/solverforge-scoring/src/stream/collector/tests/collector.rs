// Tests for collector module.
//
// Tests extracted from:
// - load_balance.rs (6 tests)

use super::super::load_balance::load_balance;
use super::super::{Accumulator, UniCollector};

// ============================================================================
// LoadBalance collector tests
// ============================================================================

#[test]
fn test_perfectly_balanced() {
    let collector = load_balance(|x: &i32| *x, |_| 1i64);
    let mut acc = collector.create_accumulator();

    // Two items with equal load
    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&1));

    let result = acc.finish();
    assert_eq!(result.unfairness(), 0);
}

#[test]
fn test_unbalanced() {
    let collector = load_balance(|x: &i32| *x, |_| 1i64);
    let mut acc = collector.create_accumulator();

    // Item 0 has 2 load, Item 1 has 1 load
    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&1));

    let result = acc.finish();
    // Timefold formula: sqrt((fraction/n) + integral)
    // For loads [2, 1], mean = 1.5
    // Squared deviations: (2-1.5)^2 + (1-1.5)^2 = 0.25 + 0.25 = 0.5
    // unfairness = sqrt(0.5) ~ 0.707 -> rounds to 1
    assert_eq!(result.unfairness(), 1);
}

#[test]
fn test_retract() {
    let collector = load_balance(|x: &i32| *x, |_| 1i64);
    let mut acc = collector.create_accumulator();

    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&1));

    // Remove one from item 0 -> now balanced
    acc.retract(&collector.extract(&0));

    let result = acc.finish();
    assert_eq!(result.unfairness(), 0);
}

#[test]
fn test_empty() {
    let collector = load_balance(|x: &i32| *x, |_| 1i64);
    let acc = collector.create_accumulator();

    let result = acc.finish();
    assert_eq!(result.unfairness(), 0);
}

#[test]
fn test_single_item() {
    let collector = load_balance(|x: &i32| *x, |_| 1i64);
    let mut acc = collector.create_accumulator();

    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&0));
    acc.accumulate(&collector.extract(&0));

    let result = acc.finish();
    // Single item always has 0 variance from mean (it IS the mean)
    // But Timefold returns sqrt(squared_deviation) for n=1
    assert_eq!(result.loads().get(&0), Some(&3));
}

// Matches Timefold's InnerUniConstraintCollectorsTest.loadBalance() pattern.
// Note: We return i64 (rounded) instead of BigDecimal, so 0.707 -> 1.
#[test]
fn test_timefold_parity() {
    struct LoadBalanced {
        value: &'static str,
        metric: i64,
    }

    let collector = load_balance(|lb: &LoadBalanced| lb.value, |lb: &LoadBalanced| lb.metric);
    let mut acc = collector.create_accumulator();

    // Default state
    assert_eq!(acc.finish().unfairness(), 0);

    // Add A with metric 2
    let a = LoadBalanced {
        value: "A",
        metric: 2,
    };
    acc.accumulate(&collector.extract(&a));
    assert_eq!(acc.finish().unfairness(), 0); // Single item

    // Add B with metric 1 -> A=2, B=1
    let b = LoadBalanced {
        value: "B",
        metric: 1,
    };
    acc.accumulate(&collector.extract(&b));
    // sqrt((2-1.5)^2 + (1-1.5)^2) = sqrt(0.5) ~ 0.707 -> rounds to 1
    assert_eq!(acc.finish().unfairness(), 1);

    // Add another B -> A=2, B=2 -> perfectly balanced
    acc.accumulate(&collector.extract(&b));
    assert_eq!(acc.finish().unfairness(), 0);

    // Retract B -> A=2, B=1 again
    acc.retract(&collector.extract(&b));
    assert_eq!(acc.finish().unfairness(), 1);

    // Retract B completely -> only A left
    acc.retract(&collector.extract(&b));
    assert_eq!(acc.finish().unfairness(), 0);

    // Retract A -> empty
    acc.retract(&collector.extract(&a));
    assert_eq!(acc.finish().unfairness(), 0);
}
