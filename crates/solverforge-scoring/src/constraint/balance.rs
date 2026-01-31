//! Zero-erasure balance constraint for load distribution scoring.
//!
//! Provides a constraint that penalizes uneven distribution across groups
//! using standard deviation. Unlike grouped constraints which score per-group,
//! the balance constraint computes a GLOBAL statistic across all groups.
//!
//! All type information is preserved at compile time - no Arc, no dyn.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::filter::UniFilter;

/// Zero-erasure balance constraint that penalizes uneven load distribution.
///
/// This constraint:
/// 1. Groups entities by key (e.g., employee_id)
/// 2. Counts how many entities belong to each group
/// 3. Computes population standard deviation across all group counts
/// 4. Multiplies the base score by std_dev to produce the final score
///
/// The key difference from `GroupedUniConstraint` is that balance computes
/// a GLOBAL statistic, not per-group scores.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `K` - Group key type
/// - `E` - Extractor function for entities
/// - `F` - Filter type
/// - `KF` - Key function
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::balance::BalanceConstraint;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_scoring::stream::filter::TrueFilter;
/// use solverforge_core::{ConstraintRef, ImpactType, HardSoftDecimalScore};
///
/// #[derive(Clone)]
/// struct Shift { employee_id: Option<usize> }
///
/// #[derive(Clone)]
/// struct Solution { shifts: Vec<Shift> }
///
/// // Base score of 1000 soft per unit of std_dev
/// let constraint = BalanceConstraint::new(
///     ConstraintRef::new("", "Balance workload"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.shifts,
///     TrueFilter,
///     |shift: &Shift| shift.employee_id,
///     HardSoftDecimalScore::of_soft(1),  // 1 soft per unit std_dev (scaled internally)
///     false,
/// );
///
/// let solution = Solution {
///     shifts: vec![
///         Shift { employee_id: Some(0) },
///         Shift { employee_id: Some(0) },
///         Shift { employee_id: Some(0) },
///         Shift { employee_id: Some(1) },
///         Shift { employee_id: None },  // Unassigned, filtered out
///     ],
/// };
///
/// // Employee 0: 3 shifts, Employee 1: 1 shift
/// // Mean = 2, Variance = ((3-2)² + (1-2)²) / 2 = 1
/// // StdDev = 1.0, Score = -1 soft (base_score * std_dev, negated for penalty)
/// let score = constraint.evaluate(&solution);
/// assert_eq!(score, HardSoftDecimalScore::of_soft(-1));
/// ```
pub struct BalanceConstraint<S, A, K, E, F, KF, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    filter: F,
    key_fn: KF,
    /// Base score representing 1 unit of standard deviation
    base_score: Sc,
    is_hard: bool,
    /// Group key → count of entities in that group
    counts: HashMap<K, i64>,
    /// Entity index → group key (for tracking assignments)
    entity_keys: HashMap<usize, K>,
    /// Cached statistics for incremental updates
    /// Number of groups (employees with at least one shift)
    group_count: i64,
    /// Sum of all counts (total assignments)
    total_count: i64,
    /// Sum of squared counts (for variance calculation)
    sum_squared: i64,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A, K, E, F, KF, Sc> BalanceConstraint<S, A, K, E, F, KF, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: UniFilter<S, A>,
    KF: Fn(&A) -> Option<K> + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new zero-erasure balance constraint.
    ///
    /// # Arguments
    ///
    /// * `constraint_ref` - Identifier for this constraint
    /// * `impact_type` - Whether to penalize or reward
    /// * `extractor` - Function to get entity slice from solution
    /// * `filter` - Filter to select which entities to consider
    /// * `key_fn` - Function to extract group key (returns None to skip entity)
    /// * `base_score` - Score per unit of standard deviation
    /// * `is_hard` - Whether this is a hard constraint
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        filter: F,
        key_fn: KF,
        base_score: Sc,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor,
            filter,
            key_fn,
            base_score,
            is_hard,
            counts: HashMap::new(),
            entity_keys: HashMap::new(),
            group_count: 0,
            total_count: 0,
            sum_squared: 0,
            _phantom: PhantomData,
        }
    }

    /// Computes standard deviation from cached statistics.
    fn compute_std_dev(&self) -> f64 {
        if self.group_count == 0 {
            return 0.0;
        }
        let n = self.group_count as f64;
        let mean = self.total_count as f64 / n;
        let variance = (self.sum_squared as f64 / n) - (mean * mean);
        if variance <= 0.0 {
            return 0.0;
        }
        variance.sqrt()
    }

    /// Computes the score from standard deviation.
    fn compute_score(&self) -> Sc {
        let std_dev = self.compute_std_dev();
        let base = self.base_score.multiply(std_dev);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Computes std_dev from raw counts (for stateless evaluate).
    fn compute_std_dev_from_counts(counts: &HashMap<K, i64>) -> f64 {
        if counts.is_empty() {
            return 0.0;
        }
        let n = counts.len() as f64;
        let total: i64 = counts.values().sum();
        let sum_sq: i64 = counts.values().map(|&c| c * c).sum();
        let mean = total as f64 / n;
        let variance = (sum_sq as f64 / n) - (mean * mean);
        if variance > 0.0 {
            variance.sqrt()
        } else {
            0.0
        }
    }
}

impl<S, A, K, E, F, KF, Sc> IncrementalConstraint<S, Sc>
    for BalanceConstraint<S, A, K, E, F, KF, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    F: UniFilter<S, A>,
    KF: Fn(&A) -> Option<K> + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = (self.extractor)(solution);

        // Build counts from scratch
        let mut counts: HashMap<K, i64> = HashMap::new();
        for entity in entities {
            if !self.filter.test(solution, entity) {
                continue;
            }
            if let Some(key) = (self.key_fn)(entity) {
                *counts.entry(key).or_insert(0) += 1;
            }
        }

        if counts.is_empty() {
            return Sc::zero();
        }

        let std_dev = Self::compute_std_dev_from_counts(&counts);
        let base = self.base_score.multiply(std_dev);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);

        // Count groups that deviate from mean
        let mut counts: HashMap<K, i64> = HashMap::new();
        for entity in entities {
            if !self.filter.test(solution, entity) {
                continue;
            }
            if let Some(key) = (self.key_fn)(entity) {
                *counts.entry(key).or_insert(0) += 1;
            }
        }

        if counts.is_empty() {
            return 0;
        }

        let total: i64 = counts.values().sum();
        let mean = total as f64 / counts.len() as f64;

        // Count groups that deviate significantly from mean
        counts
            .values()
            .filter(|&&c| (c as f64 - mean).abs() > 0.5)
            .count()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities = (self.extractor)(solution);

        for (idx, entity) in entities.iter().enumerate() {
            if !self.filter.test(solution, entity) {
                continue;
            }
            if let Some(key) = (self.key_fn)(entity) {
                let old_count = *self.counts.get(&key).unwrap_or(&0);
                let new_count = old_count + 1;
                self.counts.insert(key.clone(), new_count);
                self.entity_keys.insert(idx, key);

                if old_count == 0 {
                    self.group_count += 1;
                }
                self.total_count += 1;
                self.sum_squared += new_count * new_count - old_count * old_count;
            }
        }

        self.compute_score()
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[entity_index];
        if !self.filter.test(solution, entity) {
            return Sc::zero();
        }

        let Some(key) = (self.key_fn)(entity) else {
            return Sc::zero();
        };

        let old_score = self.compute_score();

        let old_count = *self.counts.get(&key).unwrap_or(&0);
        let new_count = old_count + 1;
        self.counts.insert(key.clone(), new_count);
        self.entity_keys.insert(entity_index, key);

        if old_count == 0 {
            self.group_count += 1;
        }
        self.total_count += 1;
        self.sum_squared += new_count * new_count - old_count * old_count;

        let new_score = self.compute_score();
        new_score - old_score
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        // Check if this entity was tracked
        let Some(key) = self.entity_keys.remove(&entity_index) else {
            return Sc::zero();
        };

        let old_score = self.compute_score();

        let old_count = *self.counts.get(&key).unwrap_or(&0);
        if old_count == 0 {
            return Sc::zero();
        }

        let new_count = old_count - 1;
        if new_count == 0 {
            self.counts.remove(&key);
            self.group_count -= 1;
        } else {
            self.counts.insert(key, new_count);
        }
        self.total_count -= 1;
        self.sum_squared += new_count * new_count - old_count * old_count;

        let new_score = self.compute_score();
        new_score - old_score
    }

    fn reset(&mut self) {
        self.counts.clear();
        self.entity_keys.clear();
        self.group_count = 0;
        self.total_count = 0;
        self.sum_squared = 0;
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}

impl<S, A, K, E, F, KF, Sc> std::fmt::Debug for BalanceConstraint<S, A, K, E, F, KF, Sc>
where
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalanceConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.counts.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::filter::TrueFilter;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone)]
    struct Shift {
        employee_id: Option<usize>,
    }

    #[derive(Clone)]
    struct Solution {
        shifts: Vec<Shift>,
    }

    #[test]
    fn test_balance_evaluate_equal_distribution() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000), // 1000 per unit std_dev
            false,
        );

        // Equal distribution: 2 shifts each
        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(1),
                },
                Shift {
                    employee_id: Some(1),
                },
            ],
        };

        // Mean = 2, all counts = 2, variance = 0, std_dev = 0
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_balance_evaluate_unequal_distribution() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000), // 1000 per unit std_dev
            false,
        );

        // Unequal: employee 0 has 3, employee 1 has 1
        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(1),
                },
            ],
        };

        // Mean = 2, variance = ((3-2)² + (1-2)²) / 2 = 1, std_dev = 1.0
        // base_score * std_dev = 1000 * 1.0 = 1000, negated = -1000
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1000));
    }

    #[test]
    fn test_balance_filters_unassigned() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000),
            false,
        );

        // Employee 0: 2, Employee 1: 2, plus unassigned (ignored)
        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(1),
                },
                Shift {
                    employee_id: Some(1),
                },
                Shift { employee_id: None },
            ],
        };

        // Balanced, std_dev = 0
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_balance_incremental() {
        let mut constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000),
            false,
        );

        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(1),
                },
                Shift {
                    employee_id: Some(1),
                },
            ],
        };

        // Initialize with balanced state (std_dev = 0)
        let initial = constraint.initialize(&solution);
        assert_eq!(initial, SimpleScore::of(0));

        // Retract one shift from employee 0
        let delta = constraint.on_retract(&solution, 0, 0);
        // Now: employee 0 has 1, employee 1 has 2
        // Mean = 1.5, variance = (0.25 + 0.25) / 2 = 0.25, std_dev = 0.5
        // Score = -1000 * 0.5 = -500
        assert_eq!(delta, SimpleScore::of(-500));

        // Insert it back
        let delta = constraint.on_insert(&solution, 0, 0);
        // Back to balanced: delta = +500
        assert_eq!(delta, SimpleScore::of(500));
    }

    #[test]
    fn test_balance_empty_solution() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000),
            false,
        );

        let solution = Solution { shifts: vec![] };
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_balance_single_employee() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000),
            false,
        );

        // Single employee with 5 shifts - no variance possible
        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
            ],
        };

        // With only one group, variance = 0
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_balance_reward() {
        let constraint = BalanceConstraint::new(
            ConstraintRef::new("", "Balance reward"),
            ImpactType::Reward,
            |s: &Solution| &s.shifts,
            TrueFilter,
            |shift: &Shift| shift.employee_id,
            SimpleScore::of(1000),
            false,
        );

        let solution = Solution {
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(1),
                },
            ],
        };

        // std_dev = 1.0, reward = +1000
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(1000));
    }
}
