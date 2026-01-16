//! Zero-erasure grouped constraint for bi-arity group-by operations.
//!
//! Provides incremental scoring for constraints that group entity pairs and
//! apply collectors to compute aggregate scores.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, BiCollector};

/// Zero-erasure constraint that groups entity pairs by key and scores based on collector results.
///
/// This enables incremental scoring for bi-arity group-by operations:
/// - Tracks which pairs belong to which group
/// - Maintains collector state per group
/// - Computes score deltas when pairs are added/removed
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `GK` - Group key type (from key_fn)
/// - `JK` - Join key type (for self-join matching)
/// - `E` - Extractor function for entities
/// - `JKE` - Join key extractor
/// - `KF` - Group key function
/// - `Flt` - Filter predicate
/// - `C` - Collector type
/// - `W` - Weight function
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped_bi::GroupedBiConstraint;
/// use solverforge_scoring::stream::collector::bi_count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Task { team: u32, priority: u32 }
///
/// #[derive(Clone)]
/// struct Solution { tasks: Vec<Task> }
///
/// // Penalize squared count of same-team pairs per priority
/// let constraint = GroupedBiConstraint::new(
///     ConstraintRef::new("", "Team clustering by priority"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.tasks,
///     |t: &Task| t.team,
///     |_a: &Task, b: &Task| b.priority,
///     |_s: &Solution, _a: &Task, _b: &Task| true,
///     bi_count::<Task>(),
///     |count: &usize| SimpleScore::of((*count * *count) as i64),
///     false,
/// );
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 2, priority: 2 },
///     ],
/// };
///
/// // Team 1 has 3 tasks with priority 1, forming 3 pairs (0,1), (0,2), (1,2)
/// // Grouped by priority 1: 3 pairs -> 9 penalty
/// // Total: -9
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-9));
/// ```
pub struct GroupedBiConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: BiCollector<A>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    join_key_extractor: JKE,
    key_fn: KF,
    filter: Flt,
    collector: C,
    weight_fn: W,
    is_hard: bool,
    /// Group key -> accumulator
    groups: HashMap<GK, C::Accumulator>,
    /// Pair (i, j) -> group key
    pair_groups: HashMap<(usize, usize), GK>,
    /// Pair (i, j) -> cached value for retraction
    pair_values: HashMap<(usize, usize), C::Value>,
    /// Entity index -> pairs involving this entity
    entity_to_pairs: HashMap<usize, HashSet<(usize, usize)>>,
    _phantom: PhantomData<(S, A, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
    GroupedBiConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A) -> bool + Send + Sync,
    C: BiCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new zero-erasure grouped bi-constraint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        join_key_extractor: JKE,
        key_fn: KF,
        filter: Flt,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor,
            join_key_extractor,
            key_fn,
            filter,
            collector,
            weight_fn,
            is_hard,
            groups: HashMap::new(),
            pair_groups: HashMap::new(),
            pair_values: HashMap::new(),
            entity_to_pairs: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_score(&self, result: &C::Result) -> Sc {
        let base = (self.weight_fn)(result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Builds a join index: join_key -> entity indices
    fn build_join_index(&self, entities: &[A]) -> HashMap<JK, Vec<usize>> {
        let mut index: HashMap<JK, Vec<usize>> = HashMap::new();
        for (idx, entity) in entities.iter().enumerate() {
            let key = (self.join_key_extractor)(entity);
            index.entry(key).or_default().push(idx);
        }
        index
    }

    fn insert_pair(
        &mut self,
        solution: &S,
        entities: &[A],
        i: usize,
        j: usize,
    ) -> Sc {
        let a = &entities[i];
        let b = &entities[j];

        if !(self.filter)(solution, a, b) {
            return Sc::zero();
        }

        let gk = (self.key_fn)(a, b);
        let value = self.collector.extract(a, b);
        let impact = self.impact_type;

        let acc = self
            .groups
            .entry(gk.clone())
            .or_insert_with(|| self.collector.create_accumulator());

        let old_base = (self.weight_fn)(&acc.finish());
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        acc.accumulate(&value);

        let new_base = (self.weight_fn)(&acc.finish());
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        self.pair_groups.insert((i, j), gk);
        self.pair_values.insert((i, j), value);
        self.entity_to_pairs.entry(i).or_default().insert((i, j));
        self.entity_to_pairs.entry(j).or_default().insert((i, j));

        new_score - old
    }

    fn retract_pair(&mut self, i: usize, j: usize) -> Sc {
        let Some(gk) = self.pair_groups.remove(&(i, j)) else {
            return Sc::zero();
        };

        let Some(value) = self.pair_values.remove(&(i, j)) else {
            return Sc::zero();
        };

        if let Some(pairs) = self.entity_to_pairs.get_mut(&i) {
            pairs.remove(&(i, j));
        }
        if let Some(pairs) = self.entity_to_pairs.get_mut(&j) {
            pairs.remove(&(i, j));
        }

        let impact = self.impact_type;

        let Some(acc) = self.groups.get_mut(&gk) else {
            return Sc::zero();
        };

        let old_base = (self.weight_fn)(&acc.finish());
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        acc.retract(&value);

        let new_base = (self.weight_fn)(&acc.finish());
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        new_score - old
    }
}

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc> IncrementalConstraint<S, Sc>
    for GroupedBiConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A) -> bool + Send + Sync,
    C: BiCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = (self.extractor)(solution);
        let join_index = self.build_join_index(entities);

        let mut groups: HashMap<GK, C::Accumulator> = HashMap::new();

        for indices in join_index.values() {
            for (pos_i, &i) in indices.iter().enumerate() {
                for &j in indices.iter().skip(pos_i + 1) {
                    let a = &entities[i];
                    let b = &entities[j];

                    if !(self.filter)(solution, a, b) {
                        continue;
                    }

                    let gk = (self.key_fn)(a, b);
                    let value = self.collector.extract(a, b);
                    let acc = groups
                        .entry(gk)
                        .or_insert_with(|| self.collector.create_accumulator());
                    acc.accumulate(&value);
                }
            }
        }

        let mut total = Sc::zero();
        for acc in groups.values() {
            let result = acc.finish();
            total = total + self.compute_score(&result);
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);
        let join_index = self.build_join_index(entities);

        let mut groups: HashSet<GK> = HashSet::new();

        for indices in join_index.values() {
            for (pos_i, &i) in indices.iter().enumerate() {
                for &j in indices.iter().skip(pos_i + 1) {
                    let a = &entities[i];
                    let b = &entities[j];

                    if !(self.filter)(solution, a, b) {
                        continue;
                    }

                    let gk = (self.key_fn)(a, b);
                    groups.insert(gk);
                }
            }
        }

        groups.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities = (self.extractor)(solution);
        let join_index = self.build_join_index(entities);
        let mut total = Sc::zero();

        for indices in join_index.values() {
            for (pos_i, &i) in indices.iter().enumerate() {
                for &j in indices.iter().skip(pos_i + 1) {
                    total = total + self.insert_pair(solution, entities, i, j);
                }
            }
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[entity_index];
        let jk = (self.join_key_extractor)(entity);
        let mut total = Sc::zero();

        for (idx, other) in entities.iter().enumerate() {
            if idx == entity_index {
                continue;
            }

            let other_jk = (self.join_key_extractor)(other);
            if jk != other_jk {
                continue;
            }

            let (i, j) = if entity_index < idx {
                (entity_index, idx)
            } else {
                (idx, entity_index)
            };

            total = total + self.insert_pair(solution, entities, i, j);
        }

        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let pairs_to_remove: Vec<(usize, usize)> = self
            .entity_to_pairs
            .get(&entity_index)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        let mut total = Sc::zero();
        for (i, j) in pairs_to_remove {
            total = total + self.retract_pair(i, j);
        }

        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.pair_groups.clear();
        self.pair_values.clear();
        self.entity_to_pairs.clear();
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

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc> std::fmt::Debug
    for GroupedBiConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: BiCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::bi_count;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct Task {
        team: u32,
        priority: u32,
    }

    #[derive(Clone)]
    struct Solution {
        tasks: Vec<Task>,
    }

    #[test]
    fn test_grouped_bi_evaluate() {
        let constraint = GroupedBiConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, b: &Task| b.priority,
            |_s: &Solution, _a: &Task, _b: &Task| true,
            bi_count::<Task>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            tasks: vec![
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
            ],
        };

        // 3 tasks on same team = 3 pairs, all with priority 1
        // 3 pairs -> penalty of -3
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-3));
    }

    #[test]
    fn test_grouped_bi_incremental() {
        let mut constraint = GroupedBiConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, b: &Task| b.priority,
            |_s: &Solution, _a: &Task, _b: &Task| true,
            bi_count::<Task>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            tasks: vec![
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
            ],
        };

        let total = constraint.initialize(&solution);
        assert_eq!(total, SimpleScore::of(-3));

        // Retract entity 0, removes pairs (0,1) and (0,2)
        let delta = constraint.on_retract(&solution, 0);
        // Was -3, now -1 (only pair (1,2) remains)
        assert_eq!(delta, SimpleScore::of(2));
    }
}
