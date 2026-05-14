/* Zero-erasure grouped constraint for group-by operations.

Provides incremental scoring for constraints that group entities and
apply collectors to compute aggregate scores.
All type information is preserved at compile time - no Arc, no dyn.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;

struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

/* Zero-erasure constraint that groups entities by key and scores based on collector results.

This enables incremental scoring for group-by operations:
- Tracks which entities belong to which group
- Maintains collector state per group
- Computes score deltas when entities are added/removed

All type parameters are concrete - no trait objects, no Arc allocations.

# Type Parameters

- `S` - Solution type
- `A` - Entity type
- `K` - Group key type
- `E` - Extractor function for entities
- `Fi` - Filter type (applied before grouping)
- `KF` - Key function
- `C` - Collector type
- `W` - Weight function
- `Sc` - Score type

# Example

```
use solverforge_scoring::constraint::grouped::GroupedUniConstraint;
use solverforge_scoring::stream::collector::count;
use solverforge_scoring::stream::filter::TrueFilter;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_core::score::SoftScore;

#[derive(Clone, Hash, PartialEq, Eq)]
struct Shift { employee_id: usize }

#[derive(Clone)]
struct Solution { shifts: Vec<Shift> }

// Penalize based on squared workload per employee
let constraint = GroupedUniConstraint::new(
ConstraintRef::new("", "Balanced workload"),
ImpactType::Penalty,
|s: &Solution| &s.shifts,
TrueFilter,
|shift: &Shift| shift.employee_id,
    count(),
|_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64),
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

// Employee 1: 3 shifts -> 9 penalty
// Employee 2: 1 shift -> 1 penalty
// Total: -10
assert_eq!(constraint.evaluate(&solution), SoftScore::of(-10));
```
*/
pub struct GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    filter: Fi,
    key_fn: KF,
    collector: C,
    weight_fn: W,
    is_hard: bool,
    change_source: crate::stream::collection_extract::ChangeSource,
    // Group key -> accumulator plus count (scores computed on-the-fly, no cloning)
    groups: HashMap<K, GroupState<Acc>>,
    // Entity index -> group key (for tracking which group an entity belongs to)
    entity_groups: HashMap<usize, K>,
    // Entity index -> accumulator retraction token
    entity_retractions: HashMap<usize, CollectorRetraction<Acc, V, R>>,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
    GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Creates a new zero-erasure grouped constraint.

    # Arguments

    * `constraint_ref` - Identifier for this constraint
    * `impact_type` - Whether to penalize or reward
    * `extractor` - Function to get entity slice from solution
    * `filter` - Filter applied to entities before grouping
    * `key_fn` - Function to extract group key from entity
    * `collector` - Collector to aggregate entities per group
    * `weight_fn` - Function to compute score from collector result
    * `is_hard` - Whether this is a hard constraint
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        filter: Fi,
        key_fn: KF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let change_source = extractor.change_source();
        Self {
            constraint_ref,
            impact_type,
            extractor,
            filter,
            key_fn,
            collector,
            weight_fn,
            is_hard,
            change_source,
            groups: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_retractions: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    // Computes the score contribution for a group's result.
    fn compute_score(&self, key: &K, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = self.extractor.extract(solution);

        // Group entities by key, applying filter
        let mut groups: HashMap<K, Acc> = HashMap::new();

        for entity in entities {
            if !self.filter.test(solution, entity) {
                continue;
            }
            let key = (self.key_fn)(entity);
            let value = self.collector.extract(entity);
            let acc = groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator());
            acc.accumulate(value);
        }

        // Sum scores for all groups
        let mut total = Sc::zero();
        for (key, acc) in &groups {
            total = total + acc.with_result(|result| self.compute_score(key, result));
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = self.extractor.extract(solution);

        // Count unique groups (filtered)
        let mut groups: HashMap<K, ()> = HashMap::new();
        for entity in entities {
            if !self.filter.test(solution, entity) {
                continue;
            }
            let key = (self.key_fn)(entity);
            groups.insert(key, ());
        }

        groups.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities = self.extractor.extract(solution);
        let mut total = Sc::zero();

        for (idx, entity) in entities.iter().enumerate() {
            if !self.filter.test(solution, entity) {
                continue;
            }
            total = total + self.insert_entity(entities, idx, entity);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        if !self
            .change_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name)
        {
            return Sc::zero();
        }
        let entities = self.extractor.extract(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[entity_index];
        if !self.filter.test(solution, entity) {
            return Sc::zero();
        }
        self.insert_entity(entities, entity_index, entity)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        if !self
            .change_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name)
        {
            return Sc::zero();
        }
        let entities = self.extractor.extract(solution);
        self.retract_entity(entities, entity_index)
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.entity_groups.clear();
        self.entity_retractions.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
    GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn insert_entity(&mut self, _entities: &[A], entity_index: usize, entity: &A) -> Sc {
        let key = (self.key_fn)(entity);
        let entity_key = key.clone();
        let value = self.collector.extract(entity);
        let impact = self.impact_type;

        let weight_fn = &self.weight_fn;
        let (old, new_score) = match self.groups.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let old_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let old = match impact {
                    ImpactType::Penalty => -old_base,
                    ImpactType::Reward => old_base,
                };
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let new_score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                self.entity_retractions.insert(entity_index, retraction);
                (old, new_score)
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let mut entry = entry.insert_entry(GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let new_score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                self.entity_retractions.insert(entity_index, retraction);
                (Sc::zero(), new_score)
            }
        };

        // Track entity -> group mapping and accumulator token for correct retraction.
        self.entity_groups.insert(entity_index, entity_key);

        // Return delta (both scores computed fresh, no cloning)
        new_score - old
    }

    fn retract_entity(&mut self, _entities: &[A], entity_index: usize) -> Sc {
        // Find which group this entity belonged to
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return Sc::zero();
        };

        // Use cached retraction token (entity may have been mutated since insert)
        let Some(retraction) = self.entity_retractions.remove(&entity_index) else {
            return Sc::zero();
        };
        let impact = self.impact_type;

        let weight_fn = &self.weight_fn;
        let std::collections::hash_map::Entry::Occupied(mut entry) = self.groups.entry(key) else {
            return Sc::zero();
        };

        let old_base = entry
            .get()
            .accumulator
            .with_result(|result| weight_fn(entry.key(), result));
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        let is_empty = group.count == 0;
        let new_score = if is_empty {
            entry.remove();
            Sc::zero()
        } else {
            let new_base = entry
                .get()
                .accumulator
                .with_result(|result| weight_fn(entry.key(), result));
            match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            }
        };

        // Return delta (both scores computed fresh, no cloning)
        new_score - old
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> std::fmt::Debug
    for GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
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
