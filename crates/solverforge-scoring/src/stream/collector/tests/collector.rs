/* Tests for collector module.

Tests extracted from:
- load_balance.rs (6 tests)
*/

use super::super::load_balance::load_balance;
use super::super::runs::consecutive_runs;
use super::super::{Accumulator, UniCollector};

/* ============================================================================
LoadBalance collector tests
============================================================================
*/

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
    /* For loads [2, 1], mean = 1.5
    Squared deviations: (2-1.5)^2 + (1-1.5)^2 = 0.25 + 0.25 = 0.5
    unfairness = sqrt(0.5) ~ 0.707 -> rounds to 1
    */
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
    assert_eq!(result.loads().get(&0), Some(&3));
}

// Baseline load balance test: checks incremental add/retract correctness.
// Note: We return i64 (rounded) so 0.707 -> 1.
#[test]
fn test_load_balance_standard_deviation() {
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

/* ============================================================================
Consecutive run collector tests
============================================================================
*/

#[test]
fn test_consecutive_runs_empty() {
    let collector = consecutive_runs(|value: &i64| *value);
    let acc = collector.create_accumulator();

    let result = acc.finish();
    assert!(result.is_empty());
    assert_eq!(result.point_count(), 0);
    assert_eq!(result.item_count(), 0);
}

#[test]
fn test_consecutive_runs_one_run() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [3, 1, 2] {
        acc.accumulate(&collector.extract(&value));
    }

    let result = acc.finish();
    assert_eq!(result.len(), 1);
    assert_eq!(result.runs()[0].start(), 1);
    assert_eq!(result.runs()[0].end(), 3);
    assert_eq!(result.runs()[0].point_count(), 3);
    assert_eq!(result.runs()[0].item_count(), 3);
}

#[test]
fn test_consecutive_runs_multiple_runs() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [8, 1, 2, 4, 5, 10] {
        acc.accumulate(&collector.extract(&value));
    }

    let result = acc.finish();
    assert_eq!(result.len(), 4);
    assert_eq!(result.runs()[0].start(), 1);
    assert_eq!(result.runs()[0].end(), 2);
    assert_eq!(result.runs()[1].start(), 4);
    assert_eq!(result.runs()[1].end(), 5);
    assert_eq!(result.runs()[2].start(), 8);
    assert_eq!(result.runs()[2].end(), 8);
    assert_eq!(result.runs()[3].start(), 10);
    assert_eq!(result.runs()[3].end(), 10);
}

#[test]
fn test_consecutive_runs_duplicates_count_items_not_points() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [1, 1, 2, 4, 4, 4] {
        acc.accumulate(&collector.extract(&value));
    }

    let result = acc.finish();
    assert_eq!(result.point_count(), 3);
    assert_eq!(result.item_count(), 6);
    assert_eq!(result.runs()[0].point_count(), 2);
    assert_eq!(result.runs()[0].item_count(), 3);
    assert_eq!(result.runs()[1].point_count(), 1);
    assert_eq!(result.runs()[1].item_count(), 3);
}

#[test]
fn test_consecutive_runs_negative_indexes() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [-3, -2, -1, 1] {
        acc.accumulate(&collector.extract(&value));
    }

    let result = acc.finish();
    assert_eq!(result.len(), 2);
    assert_eq!(result.runs()[0].start(), -3);
    assert_eq!(result.runs()[0].end(), -1);
    assert_eq!(result.runs()[1].start(), 1);
    assert_eq!(result.runs()[1].end(), 1);
}

#[test]
fn test_consecutive_runs_i64_max_boundary() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [i64::MIN, i64::MAX - 1, i64::MAX] {
        acc.accumulate(&collector.extract(&value));
    }

    let result = acc.finish();
    assert_eq!(result.len(), 2);
    assert_eq!(result.runs()[0].start(), i64::MIN);
    assert_eq!(result.runs()[0].end(), i64::MIN);
    assert_eq!(result.runs()[1].start(), i64::MAX - 1);
    assert_eq!(result.runs()[1].end(), i64::MAX);
}

#[test]
fn test_consecutive_runs_insert_retract_parity() {
    let collector = consecutive_runs(|value: &i64| *value);
    let mut acc = collector.create_accumulator();

    for value in [1, 2, 2, 3, 7] {
        acc.accumulate(&collector.extract(&value));
    }

    acc.retract(&collector.extract(&2));
    let result = acc.finish();
    assert_eq!(result.len(), 2);
    assert_eq!(result.runs()[0].item_count(), 3);
    assert_eq!(result.item_count(), 4);

    acc.retract(&collector.extract(&2));
    let result = acc.finish();
    assert_eq!(result.len(), 3);
    assert_eq!(result.point_count(), 3);
    assert_eq!(result.item_count(), 3);
}
