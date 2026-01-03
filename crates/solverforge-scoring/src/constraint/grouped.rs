//! Zero-erasure grouped constraint for group-by operations.
//!
//! Provides incremental scoring for constraints that group entities and
//! apply collectors to compute aggregate scores.
//! All type information is preserved at compile time - no Arc, no dyn.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, UniCollector};

/// Zero-erasure constraint that groups entities by key and scores based on collector results.
///
/// This enables incremental scoring for group-by operations:
/// - Tracks which entities belong to which group
/// - Maintains collector state per group
/// - Computes score deltas when entities are added/removed
///
/// All type parameters are concrete - no trait objects, no Arc allocations.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `K` - Group key type
/// - `E` - Extractor function for entities
/// - `KF` - Key function
/// - `C` - Collector type
/// - `W` - Weight function
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped::GroupedUniConstraint;
/// use solverforge_scoring::stream::collector::count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Shift { employee_id: usize }
///
/// #[derive(Clone)]
/// struct Solution { shifts: Vec<Shift> }
///
/// // Penalize based on squared workload per employee
/// let constraint = GroupedUniConstraint::new(
///     ConstraintRef::new("", "Balanced workload"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.shifts,
///     |shift: &Shift| shift.employee_id,
///     count::<Shift>(),
///     |count: &usize| SimpleScore::of((*count * *count) as i64),
///     false,
/// );
///
/// let solution = Solution {
///     shifts: vec![
///         Shift { employee_id: 1 },
///         Shift { employee_id: 1 },
///         Shift { employee_id: 1 },
///         Shift { employee_id: 2 },
///     ],
/// };
///
/// // Employee 1: 3 shifts -> 9 penalty
/// // Employee 2: 1 shift -> 1 penalty
/// // Total: -10
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
/// ```
pub struct GroupedUniConstraint<S, A, K, E, KF, C, W, Sc>
where
    C: UniCollector<A>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    key_fn: KF,
    collector: C,
    weight_fn: W,
    is_hard: bool,
    /// Group key -> accumulator (scores computed on-the-fly, no cloning)
    groups: HashMap<K, C::Accumulator>,
    /// Entity index -> group key (for tracking which group an entity belongs to)
    entity_groups: HashMap<usize, K>,
    /// Entity index -> extracted value (for correct retraction after entity mutation)
    entity_values: HashMap<usize, C::Value>,
    _phantom: PhantomData<(S, A, Sc)>,
}

impl<S, A, K, E, KF, C, W, Sc> GroupedUniConstraint<S, A, K, E, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new zero-erasure grouped constraint.
    ///
    /// # Arguments
    ///
    /// * `constraint_ref` - Identifier for this constraint
    /// * `impact_type` - Whether to penalize or reward
    /// * `extractor` - Function to get entity slice from solution
    /// * `key_fn` - Function to extract group key from entity
    /// * `collector` - Collector to aggregate entities per group
    /// * `weight_fn` - Function to compute score from collector result
    /// * `is_hard` - Whether this is a hard constraint
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        key_fn: KF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor,
            key_fn,
            collector,
            weight_fn,
            is_hard,
            groups: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_values: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Computes the score contribution for a group's result.
    fn compute_score(&self, result: &C::Result) -> Sc {
        let base = (self.weight_fn)(result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }
}

impl<S, A, K, E, KF, C, W, Sc> IncrementalConstraint<S, Sc>
    for GroupedUniConstraint<S, A, K, E, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = (self.extractor)(solution);

        // Group entities by key
        let mut groups: HashMap<K, C::Accumulator> = HashMap::new();

        for entity in entities {
            let key = (self.key_fn)(entity);
            let value = self.collector.extract(entity);
            let acc = groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator());
            acc.accumulate(&value);
        }

        // Sum scores for all groups
        let mut total = Sc::zero();
        for acc in groups.values() {
            let result = acc.finish();
            total = total + self.compute_score(&result);
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);

        // Count unique groups
        let mut groups: HashMap<K, ()> = HashMap::new();
        for entity in entities {
            let key = (self.key_fn)(entity);
            groups.insert(key, ());
        }

        groups.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities = (self.extractor)(solution);
        let mut total = Sc::zero();

        for (idx, entity) in entities.iter().enumerate() {
            total = total + self.insert_entity(entities, idx, entity);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[entity_index];
        self.insert_entity(entities, entity_index, entity)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        self.retract_entity(entities, entity_index)
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.entity_groups.clear();
        self.entity_values.clear();
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

impl<S, A, K, E, KF, C, W, Sc> GroupedUniConstraint<S, A, K, E, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn insert_entity(&mut self, _entities: &[A], entity_index: usize, entity: &A) -> Sc {
        let key = (self.key_fn)(entity);
        let value = self.collector.extract(entity);
        let impact = self.impact_type;

        // Get or create group accumulator
        let acc = self.groups
            .entry(key.clone())
            .or_insert_with(|| self.collector.create_accumulator());

        // Compute old score from current state (inlined to avoid borrow conflict)
        let old_base = (self.weight_fn)(&acc.finish());
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Accumulate and compute new score
        acc.accumulate(&value);
        let new_base = (self.weight_fn)(&acc.finish());
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Track entity -> group mapping and cache value for correct retraction
        self.entity_groups.insert(entity_index, key);
        self.entity_values.insert(entity_index, value);

        // Return delta (both scores computed fresh, no cloning)
        new_score - old
    }

    fn retract_entity(&mut self, _entities: &[A], entity_index: usize) -> Sc {
        // Find which group this entity belonged to
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return Sc::zero();
        };

        // Use cached value (entity may have been mutated since insert)
        let Some(value) = self.entity_values.remove(&entity_index) else {
            return Sc::zero();
        };
        let impact = self.impact_type;

        // Get the group accumulator
        let Some(acc) = self.groups.get_mut(&key) else {
            return Sc::zero();
        };

        // Compute old score from current state (inlined to avoid borrow conflict)
        let old_base = (self.weight_fn)(&acc.finish());
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Retract and compute new score
        acc.retract(&value);
        let new_base = (self.weight_fn)(&acc.finish());
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Return delta (both scores computed fresh, no cloning)
        new_score - old
    }
}

impl<S, A, K, E, KF, C, W, Sc> std::fmt::Debug for GroupedUniConstraint<S, A, K, E, KF, C, W, Sc>
where
    C: UniCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedUniConstraint")
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
    struct Shift {
        employee_id: usize,
    }

    #[derive(Clone)]
    struct Solution {
        shifts: Vec<Shift>,
    }

    #[test]
    fn test_grouped_constraint_evaluate() {
        let constraint = GroupedUniConstraint::new(
            ConstraintRef::new("", "Workload"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            |shift: &Shift| shift.employee_id,
            count::<Shift>(),
            |count: &usize| SimpleScore::of((*count * *count) as i64),
            false,
        );

        let solution = Solution {
            shifts: vec![
                Shift { employee_id: 1 },
                Shift { employee_id: 1 },
                Shift { employee_id: 1 },
                Shift { employee_id: 2 },
            ],
        };

        // Employee 1: 3 shifts -> 9
        // Employee 2: 1 shift -> 1
        // Total penalty: -10
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
    }

    #[test]
    fn test_grouped_constraint_incremental() {
        let mut constraint = GroupedUniConstraint::new(
            ConstraintRef::new("", "Workload"),
            ImpactType::Penalty,
            |s: &Solution| &s.shifts,
            |shift: &Shift| shift.employee_id,
            count::<Shift>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            shifts: vec![
                Shift { employee_id: 1 },
                Shift { employee_id: 1 },
                Shift { employee_id: 2 },
            ],
        };

        // Initialize
        let total = constraint.initialize(&solution);
        // Employee 1: 2 shifts -> -2
        // Employee 2: 1 shift -> -1
        // Total: -3
        assert_eq!(total, SimpleScore::of(-3));

        // Retract shift at index 0 (employee 1)
        let delta = constraint.on_retract(&solution, 0);
        // Employee 1 now has 1 shift -> score goes from -2 to -1, delta = +1
        assert_eq!(delta, SimpleScore::of(1));

        // Insert shift at index 0 (employee 1)
        let delta = constraint.on_insert(&solution, 0);
        // Employee 1 now has 2 shifts -> score goes from -1 to -2, delta = -1
        assert_eq!(delta, SimpleScore::of(-1));
    }

    #[test]
    fn test_grouped_constraint_reward() {
        let constraint = GroupedUniConstraint::new(
            ConstraintRef::new("", "Collaboration"),
            ImpactType::Reward,
            |s: &Solution| &s.shifts,
            |shift: &Shift| shift.employee_id,
            count::<Shift>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            shifts: vec![
                Shift { employee_id: 1 },
                Shift { employee_id: 1 },
            ],
        };

        // 2 shifts in one group -> reward of +2
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(2));
    }
}
