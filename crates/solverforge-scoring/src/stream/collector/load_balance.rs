//! LoadBalance collector for computing unfairness (sqrt of variance).
//!
//! Matches Timefold's LoadBalance implementation for fair workload distribution.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// Result of load balancing - tracks loads per item and computes unfairness.
///
/// Unfairness is the square root of sum of squared deviations from mean.
/// Lower values indicate fairer distribution. Zero means perfectly balanced.
#[derive(Debug, Clone)]
pub struct LoadBalance<K> {
    loads: HashMap<K, i64>,
    /// Unfairness as integer (use with of_soft() for scoring)
    unfairness: i64,
}

impl<K> LoadBalance<K> {
    /// Returns map of balanced items to their total load.
    pub fn loads(&self) -> &HashMap<K, i64> {
        &self.loads
    }

    /// Returns unfairness as i64 for use with `of_soft()`.
    /// This is the raw value - `of_soft()` handles decimal scaling.
    #[inline]
    pub fn unfairness(&self) -> i64 {
        self.unfairness
    }
}

/// Creates a load balance collector.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{load_balance, UniCollector, Accumulator};
///
/// struct Shift { employee_id: usize }
///
/// let collector = load_balance(
///     |s: &Shift| s.employee_id,
///     |_s: &Shift| 1i64,  // Each shift counts as 1
/// );
///
/// let mut acc = collector.create_accumulator();
/// acc.accumulate(&collector.extract(&Shift { employee_id: 0 }));
/// acc.accumulate(&collector.extract(&Shift { employee_id: 0 }));
/// acc.accumulate(&collector.extract(&Shift { employee_id: 1 }));
///
/// let result = acc.finish();
/// // Employee 0 has 2, Employee 1 has 1 → unfairness = sqrt(0.5) ≈ 1
/// assert_eq!(result.unfairness(), 1);
/// ```
pub fn load_balance<A, K, F, M>(key_fn: F, metric_fn: M) -> LoadBalanceCollector<A, K, F, M>
where
    K: Clone + Eq + Hash + Send + Sync,
    F: Fn(&A) -> K + Send + Sync,
    M: Fn(&A) -> i64 + Send + Sync,
{
    LoadBalanceCollector {
        key_fn,
        metric_fn,
        _phantom: PhantomData,
    }
}

/// Collector for computing load balance unfairness.
pub struct LoadBalanceCollector<A, K, F, M> {
    key_fn: F,
    metric_fn: M,
    _phantom: PhantomData<fn(&A) -> K>,
}

impl<A, K, F, M> UniCollector<A> for LoadBalanceCollector<A, K, F, M>
where
    A: Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    F: Fn(&A) -> K + Send + Sync,
    M: Fn(&A) -> i64 + Send + Sync,
{
    type Value = (K, i64);
    type Result = LoadBalance<K>;
    type Accumulator = LoadBalanceAccumulator<K>;

    #[inline]
    fn extract(&self, entity: &A) -> Self::Value {
        ((self.key_fn)(entity), (self.metric_fn)(entity))
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        LoadBalanceAccumulator::new()
    }
}

/// Accumulator for load balance with incremental variance computation.
///
/// Uses Timefold's algorithm for O(1) incremental updates.
pub struct LoadBalanceAccumulator<K> {
    /// Count of items per balanced key (for duplicate tracking)
    item_counts: HashMap<K, usize>,
    /// Cumulative load per balanced key
    loads: HashMap<K, i64>,
    /// Sum of all loads
    sum: i64,
    /// Integral part of squared deviation
    squared_deviation_integral: i64,
    /// Fractional numerator for incremental variance
    squared_deviation_fraction_numerator: i64,
}

impl<K: Clone + Eq + Hash> LoadBalanceAccumulator<K> {
    fn new() -> Self {
        Self {
            item_counts: HashMap::new(),
            loads: HashMap::new(),
            sum: 0,
            squared_deviation_integral: 0,
            squared_deviation_fraction_numerator: 0,
        }
    }

    fn add_to_metric(&mut self, key: &K, diff: i64) {
        let old_value = *self.loads.get(key).unwrap_or(&0);
        let new_value = old_value + diff;

        if old_value != new_value {
            self.loads.insert(key.clone(), new_value);
            self.update_squared_deviation(old_value, new_value);
            self.sum += diff;
        }
    }

    fn reset_metric(&mut self, key: &K) {
        if let Some(old_value) = self.loads.remove(key) {
            if old_value != 0 {
                self.update_squared_deviation(old_value, 0);
                self.sum -= old_value;
            }
        }
    }

    /// Incremental variance update formula from Timefold.
    fn update_squared_deviation(&mut self, old_value: i64, new_value: i64) {
        // Term 1: x_new² - x_old²
        let term1 = new_value * new_value - old_value * old_value;

        // Term 2: 2 * (sum_others) * (sum_old - sum_new)
        let sum_others = 2 * (self.sum - old_value);
        let new_sum = self.sum - old_value + new_value;
        let sum_diff = self.sum - new_sum;

        // Term 3: sum_new² - sum_old²
        let term3 = new_sum * new_sum - self.sum * self.sum;

        // Term 4: 2 * (old*sum_old - new*sum_new)
        let term4 = 2 * (old_value * self.sum - new_value * new_sum);

        let fraction_delta = sum_others * sum_diff + term3 + term4;

        self.squared_deviation_integral += term1;
        self.squared_deviation_fraction_numerator += fraction_delta;
    }

    /// Returns unfairness as i64 (matching Timefold's formula, no pre-scaling).
    /// Use with `of_soft()` which handles the decimal scaling.
    fn compute_unfairness(&self) -> i64 {
        let n = self.item_counts.len();
        match n {
            0 => 0,
            1 => {
                // For n=1, Timefold uses: sqrt(fraction + integral)
                let tmp = self.squared_deviation_fraction_numerator as f64
                    + self.squared_deviation_integral as f64;
                tmp.sqrt().round() as i64
            }
            _ => {
                // For n>1: sqrt(fraction/n + integral)
                let tmp = (self.squared_deviation_fraction_numerator as f64 / n as f64)
                    + self.squared_deviation_integral as f64;
                tmp.sqrt().round() as i64
            }
        }
    }
}

impl<K: Clone + Eq + Hash + Send + Sync> Accumulator<(K, i64), LoadBalance<K>>
    for LoadBalanceAccumulator<K>
{
    #[inline]
    fn accumulate(&mut self, value: &(K, i64)) {
        let (key, metric) = value;
        if *metric == 0 {
            return; // Skip zero-metric entries (e.g., unassigned shifts)
        }
        let count = self.item_counts.entry(key.clone()).or_insert(0);
        *count += 1;
        self.add_to_metric(key, *metric);
    }

    #[inline]
    fn retract(&mut self, value: &(K, i64)) {
        let (key, metric) = value;
        if *metric == 0 {
            return; // Skip zero-metric entries
        }
        if let Some(count) = self.item_counts.get_mut(key) {
            if *count > 0 {
                *count -= 1;
                if *count == 0 {
                    self.item_counts.remove(key);
                    self.reset_metric(key);
                } else {
                    self.add_to_metric(key, -*metric);
                }
            }
        }
    }

    fn finish(&self) -> LoadBalance<K> {
        LoadBalance {
            loads: self.loads.clone(),
            unfairness: self.compute_unfairness(),
        }
    }

    fn reset(&mut self) {
        self.item_counts.clear();
        self.loads.clear();
        self.sum = 0;
        self.squared_deviation_integral = 0;
        self.squared_deviation_fraction_numerator = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Squared deviations: (2-1.5)² + (1-1.5)² = 0.25 + 0.25 = 0.5
        // unfairness = sqrt(0.5) ≈ 0.707 → rounds to 1
        assert_eq!(result.unfairness(), 1);
    }

    #[test]
    fn test_retract() {
        let collector = load_balance(|x: &i32| *x, |_| 1i64);
        let mut acc = collector.create_accumulator();

        acc.accumulate(&collector.extract(&0));
        acc.accumulate(&collector.extract(&0));
        acc.accumulate(&collector.extract(&1));

        // Remove one from item 0 → now balanced
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

    /// Matches Timefold's InnerUniConstraintCollectorsTest.loadBalance() pattern.
    /// Note: We return i64 (rounded) instead of BigDecimal, so 0.707 → 1.
    #[test]
    fn test_timefold_parity() {
        struct LoadBalanced {
            value: &'static str,
            metric: i64,
        }

        let collector = load_balance(
            |lb: &LoadBalanced| lb.value,
            |lb: &LoadBalanced| lb.metric,
        );
        let mut acc = collector.create_accumulator();

        // Default state
        assert_eq!(acc.finish().unfairness(), 0);

        // Add A with metric 2
        let a = LoadBalanced { value: "A", metric: 2 };
        acc.accumulate(&collector.extract(&a));
        assert_eq!(acc.finish().unfairness(), 0); // Single item

        // Add B with metric 1 → A=2, B=1
        let b = LoadBalanced { value: "B", metric: 1 };
        acc.accumulate(&collector.extract(&b));
        // sqrt((2-1.5)² + (1-1.5)²) = sqrt(0.5) ≈ 0.707 → rounds to 1
        assert_eq!(acc.finish().unfairness(), 1);

        // Add another B → A=2, B=2 → perfectly balanced
        acc.accumulate(&collector.extract(&b));
        assert_eq!(acc.finish().unfairness(), 0);

        // Retract B → A=2, B=1 again
        acc.retract(&collector.extract(&b));
        assert_eq!(acc.finish().unfairness(), 1);

        // Retract B completely → only A left
        acc.retract(&collector.extract(&b));
        assert_eq!(acc.finish().unfairness(), 0);

        // Retract A → empty
        acc.retract(&collector.extract(&a));
        assert_eq!(acc.finish().unfairness(), 0);
    }
}
