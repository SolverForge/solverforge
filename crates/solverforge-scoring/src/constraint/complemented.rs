//! Zero-erasure complemented group constraint.
//!
//! Evaluates grouped results plus complement entities with default values.
//! Provides true incremental scoring by tracking per-key accumulators.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, UniCollector};

/// Zero-erasure constraint for complemented grouped results.
///
/// Groups A entities by key, then iterates over B entities (complement source),
/// using grouped values where they exist and default values otherwise.
///
/// The key function for A returns `Option<K>`, allowing entities to be skipped
/// when they don't have a valid key (e.g., unassigned shifts).
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type being grouped (e.g., Shift)
/// - `B` - Complement entity type (e.g., Employee)
/// - `K` - Group key type
/// - `EA` - Extractor for A entities
/// - `EB` - Extractor for B entities
/// - `KA` - Key function for A (returns `Option<K>` to allow skipping)
/// - `KB` - Key function for B
/// - `C` - Collector type
/// - `D` - Default value function
/// - `W` - Weight function
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::complemented::ComplementedGroupConstraint;
/// use solverforge_scoring::stream::collector::count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Employee { id: usize }
///
/// #[derive(Clone)]
/// struct Shift { employee_id: Option<usize> }
///
/// #[derive(Clone)]
/// struct Schedule {
///     employees: Vec<Employee>,
///     shifts: Vec<Shift>,
/// }
///
/// let constraint = ComplementedGroupConstraint::new(
///     ConstraintRef::new("", "Shift count"),
///     ImpactType::Penalty,
///     |s: &Schedule| s.shifts.as_slice(),
///     |s: &Schedule| s.employees.as_slice(),
///     |shift: &Shift| shift.employee_id,  // Returns Option<usize>
///     |emp: &Employee| emp.id,
///     count(),
///     |_emp: &Employee| 0usize,
///     |count: &usize| SimpleScore::of(*count as i64),
///     false,
/// );
///
/// let schedule = Schedule {
///     employees: vec![Employee { id: 0 }, Employee { id: 1 }],
///     shifts: vec![
///         Shift { employee_id: Some(0) },
///         Shift { employee_id: Some(0) },
///         Shift { employee_id: None },  // Skipped - no key
///     ],
/// };
///
/// // Employee 0: 2 shifts, Employee 1: 0 shifts → Total: -2
/// // Unassigned shift is skipped
/// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
/// ```
pub struct ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    C: UniCollector<A>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    collector: C,
    default_fn: D,
    weight_fn: W,
    is_hard: bool,
    /// Group key -> accumulator for incremental scoring
    groups: HashMap<K, C::Accumulator>,
    /// A entity index -> group key (for tracking which group each entity belongs to)
    entity_groups: HashMap<usize, K>,
    /// A entity index -> extracted value (for correct retraction after entity mutation)
    entity_values: HashMap<usize, C::Value>,
    /// B key -> B entity index (for looking up B entities by key)
    b_by_key: HashMap<K, usize>,
    _phantom: PhantomData<(S, A, B, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Clone + Eq + Hash,
    EA: Fn(&S) -> &[A],
    EB: Fn(&S) -> &[B],
    KA: Fn(&A) -> Option<K>,
    KB: Fn(&B) -> K,
    C: UniCollector<A>,
    C::Result: Clone,
    D: Fn(&B) -> C::Result,
    W: Fn(&C::Result) -> Sc,
    Sc: Score,
{
    /// Creates a new complemented group constraint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            collector,
            default_fn,
            weight_fn,
            is_hard,
            groups: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_values: HashMap::new(),
            b_by_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(&self, result: &C::Result) -> Sc {
        let base = (self.weight_fn)(result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Build grouped results from A entities.
    fn build_groups(&self, entities_a: &[A]) -> HashMap<K, C::Result> {
        let mut accumulators: HashMap<K, C::Accumulator> = HashMap::new();

        for a in entities_a {
            // Skip entities with no key (e.g., unassigned shifts)
            let Some(key) = (self.key_a)(a) else {
                continue;
            };
            let value = self.collector.extract(a);
            accumulators
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(&value);
        }

        accumulators
            .into_iter()
            .map(|(k, acc)| (k, acc.finish()))
            .collect()
    }
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc> IncrementalConstraint<S, Sc>
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    C::Value: Send + Sync,
    D: Fn(&B) -> C::Result + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        let groups = self.build_groups(entities_a);

        let mut total = Sc::zero();
        for b in entities_b {
            let key = (self.key_b)(b);
            let result = groups
                .get(&key)
                .cloned()
                .unwrap_or_else(|| (self.default_fn)(b));
            total = total + self.compute_score(&result);
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_b = (self.extractor_b)(solution);
        entities_b.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        // Build B key -> index mapping
        for (idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            self.b_by_key.insert(key, idx);
        }

        // Initialize all B entities with default scores
        let mut total = Sc::zero();
        for b in entities_b {
            let default_result = (self.default_fn)(b);
            total = total + self.compute_score(&default_result);
        }

        // Now insert all A entities incrementally
        for (idx, a) in entities_a.iter().enumerate() {
            total = total + self.insert_entity(entities_b, idx, a);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        if entity_index >= entities_a.len() {
            return Sc::zero();
        }

        let entity = &entities_a[entity_index];
        self.insert_entity(entities_b, entity_index, entity)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        self.retract_entity(entities_a, entities_b, entity_index)
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.entity_groups.clear();
        self.entity_values.clear();
        self.b_by_key.clear();
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

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    C::Value: Send + Sync,
    D: Fn(&B) -> C::Result + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score,
{
    /// Insert an A entity and return the score delta.
    fn insert_entity(&mut self, entities_b: &[B], entity_index: usize, entity: &A) -> Sc {
        // Skip entities with no key (e.g., unassigned shifts)
        let Some(key) = (self.key_a)(entity) else {
            return Sc::zero();
        };
        let value = self.collector.extract(entity);
        let impact = self.impact_type;

        // Check if there's a B entity for this key
        let b_idx = self.b_by_key.get(&key).copied();
        let Some(b_idx) = b_idx else {
            // No B entity for this key - A entity doesn't affect score
            // Still track it for retraction
            let acc = self
                .groups
                .entry(key.clone())
                .or_insert_with(|| self.collector.create_accumulator());
            acc.accumulate(&value);
            self.entity_groups.insert(entity_index, key);
            self.entity_values.insert(entity_index, value);
            return Sc::zero();
        };

        let b = &entities_b[b_idx];

        // Compute old score for this B entity
        let old_result = self
            .groups
            .get(&key)
            .map(|acc| acc.finish())
            .unwrap_or_else(|| (self.default_fn)(b));
        let old_base = (self.weight_fn)(&old_result);
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Get or create accumulator and add value
        let acc = self
            .groups
            .entry(key.clone())
            .or_insert_with(|| self.collector.create_accumulator());
        acc.accumulate(&value);

        // Compute new score
        let new_result = acc.finish();
        let new_base = (self.weight_fn)(&new_result);
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Track entity -> key mapping and cache value for correct retraction
        self.entity_groups.insert(entity_index, key);
        self.entity_values.insert(entity_index, value);

        // Return delta
        new_score - old
    }

    /// Retract an A entity and return the score delta.
    fn retract_entity(&mut self, _entities_a: &[A], entities_b: &[B], entity_index: usize) -> Sc {
        // Find which group this entity belonged to
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return Sc::zero();
        };

        // Use cached value (entity may have been mutated since insert)
        let Some(value) = self.entity_values.remove(&entity_index) else {
            return Sc::zero();
        };
        let impact = self.impact_type;

        // Check if there's a B entity for this key
        let b_idx = self.b_by_key.get(&key).copied();
        let Some(b_idx) = b_idx else {
            // No B entity for this key - just update accumulator, no score delta
            if let Some(acc) = self.groups.get_mut(&key) {
                acc.retract(&value);
            }
            return Sc::zero();
        };

        let b = &entities_b[b_idx];

        // Get accumulator
        let Some(acc) = self.groups.get_mut(&key) else {
            return Sc::zero();
        };

        // Compute old score
        let old_result = acc.finish();
        let old_base = (self.weight_fn)(&old_result);
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Retract value
        acc.retract(&value);

        // Compute new score
        let new_result = acc.finish();
        let new_base = (self.weight_fn)(&new_result);
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Return delta
        new_score - old
    }
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc> std::fmt::Debug
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    C: UniCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplementedGroupConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::count;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct Employee {
        id: usize,
    }

    #[derive(Clone)]
    struct Shift {
        employee_id: Option<usize>,
    }

    #[derive(Clone)]
    struct Schedule {
        employees: Vec<Employee>,
        shifts: Vec<Shift>,
    }

    #[test]
    fn test_complemented_evaluate() {
        let constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Shift count"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }],
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
            ],
        };

        // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
        // Total: -2
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
    }

    #[test]
    fn test_complemented_skips_none_keys() {
        let constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Shift count"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }],
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift {
                    employee_id: Some(0),
                },
                Shift { employee_id: None }, // Unassigned - should be skipped
                Shift { employee_id: None }, // Unassigned - should be skipped
            ],
        };

        // Only 2 assigned shifts count, both to employee 0
        // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
        // Total: -2 (unassigned shifts don't count)
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
    }

    #[test]
    fn test_complemented_incremental() {
        let mut constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Shift count"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
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
            ],
        };

        // Initialize
        let total = constraint.initialize(&schedule);
        // Employee 0: 2 shifts -> -2
        // Employee 1: 1 shift -> -1
        // Employee 2: 0 shifts -> 0
        // Total: -3
        assert_eq!(total, SimpleScore::of(-3));

        // Retract shift at index 0 (employee 0)
        let delta = constraint.on_retract(&schedule, 0, 0);
        // Employee 0 now has 1 shift -> score goes from -2 to -1, delta = +1
        assert_eq!(delta, SimpleScore::of(1));

        // Insert shift at index 0 (employee 0)
        let delta = constraint.on_insert(&schedule, 0, 0);
        // Employee 0 now has 2 shifts -> score goes from -1 to -2, delta = -1
        assert_eq!(delta, SimpleScore::of(-1));
    }

    #[test]
    fn test_complemented_incremental_with_none_keys() {
        let mut constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Shift count"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }],
            shifts: vec![
                Shift {
                    employee_id: Some(0),
                },
                Shift { employee_id: None }, // Unassigned
                Shift {
                    employee_id: Some(0),
                },
            ],
        };

        // Initialize - only assigned shifts count
        let total = constraint.initialize(&schedule);
        // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
        // Total: -2
        assert_eq!(total, SimpleScore::of(-2));

        // Retract unassigned shift at index 1 - should be no-op
        let delta = constraint.on_retract(&schedule, 1, 0);
        assert_eq!(delta, SimpleScore::of(0));

        // Insert unassigned shift at index 1 - should be no-op
        let delta = constraint.on_insert(&schedule, 1, 0);
        assert_eq!(delta, SimpleScore::of(0));
    }

    #[test]
    fn test_complemented_with_default() {
        let constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Workload balance"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of((*count as i64).pow(2)),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
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
            ],
        };

        // Employee 0: 3 shifts -> 9
        // Employee 1: 0 shifts -> 0
        // Employee 2: 0 shifts -> 0
        // Total penalty: -9
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-9));
    }

    #[test]
    fn test_complemented_incremental_matches_evaluate() {
        let mut constraint = ComplementedGroupConstraint::new(
            ConstraintRef::new("", "Shift count"),
            ImpactType::Penalty,
            |s: &Schedule| s.shifts.as_slice(),
            |s: &Schedule| s.employees.as_slice(),
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| emp.id,
            count::<Shift>(),
            |_emp: &Employee| 0usize,
            |count: &usize| SimpleScore::of((*count as i64).pow(2)),
            false,
        );

        let schedule = Schedule {
            employees: vec![Employee { id: 0 }, Employee { id: 1 }],
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
            ],
        };

        // Verify initialize matches evaluate
        let init_total = constraint.initialize(&schedule);
        let eval_total = constraint.evaluate(&schedule);
        assert_eq!(init_total, eval_total);

        // Employee 0: 2 shifts -> 4, Employee 1: 1 shift -> 1
        // Total: -5
        assert_eq!(init_total, SimpleScore::of(-5));

        // Simulate retract + insert cycle and verify total remains consistent
        let mut running_total = init_total;

        // Retract shift 2 (employee 1)
        running_total = running_total + constraint.on_retract(&schedule, 2, 0);
        // Now: Employee 0: 2->4, Employee 1: 0->0, Total: -4
        assert_eq!(running_total, SimpleScore::of(-4));

        // Insert shift 2 back (employee 1)
        running_total = running_total + constraint.on_insert(&schedule, 2, 0);
        // Back to: Employee 0: 2->4, Employee 1: 1->1, Total: -5
        assert_eq!(running_total, SimpleScore::of(-5));
    }
}
