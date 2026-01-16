//! Zero-erasure grouped constraint for tri-arity group-by operations.
//!
//! Provides incremental scoring for constraints that group entity triples and
//! apply collectors to compute aggregate scores.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, TriCollector};

/// Zero-erasure constraint that groups entity triples by key and scores based on collector results.
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped_tri::GroupedTriConstraint;
/// use solverforge_scoring::stream::collector::tri_count;
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
/// let constraint = GroupedTriConstraint::new(
///     ConstraintRef::new("", "Team clustering"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.tasks,
///     |t: &Task| t.team,
///     |_a: &Task, _b: &Task, c: &Task| c.priority,
///     |_s: &Solution, _a: &Task, _b: &Task, _c: &Task| true,
///     tri_count::<Task>(),
///     |count: &usize| SimpleScore::of(*count as i64),
///     false,
/// );
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///     ],
/// };
///
/// // 3 tasks form 1 triple -> 1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct GroupedTriConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: TriCollector<A>,
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
    groups: HashMap<GK, C::Accumulator>,
    triple_groups: HashMap<(usize, usize, usize), GK>,
    triple_values: HashMap<(usize, usize, usize), C::Value>,
    entity_to_triples: HashMap<usize, HashSet<(usize, usize, usize)>>,
    _phantom: PhantomData<(S, A, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
    GroupedTriConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A) -> bool + Send + Sync,
    C: TriCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Creates a new zero-erasure grouped tri-constraint.
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
            triple_groups: HashMap::new(),
            triple_values: HashMap::new(),
            entity_to_triples: HashMap::new(),
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

    fn build_join_index(&self, entities: &[A]) -> HashMap<JK, Vec<usize>> {
        let mut index: HashMap<JK, Vec<usize>> = HashMap::new();
        for (idx, entity) in entities.iter().enumerate() {
            let key = (self.join_key_extractor)(entity);
            index.entry(key).or_default().push(idx);
        }
        index
    }

    fn insert_triple(
        &mut self,
        solution: &S,
        entities: &[A],
        i: usize,
        j: usize,
        k: usize,
    ) -> Sc {
        let a = &entities[i];
        let b = &entities[j];
        let c = &entities[k];

        if !(self.filter)(solution, a, b, c) {
            return Sc::zero();
        }

        let gk = (self.key_fn)(a, b, c);
        let value = self.collector.extract(a, b, c);
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

        self.triple_groups.insert((i, j, k), gk);
        self.triple_values.insert((i, j, k), value);
        self.entity_to_triples.entry(i).or_default().insert((i, j, k));
        self.entity_to_triples.entry(j).or_default().insert((i, j, k));
        self.entity_to_triples.entry(k).or_default().insert((i, j, k));

        new_score - old
    }

    fn retract_triple(&mut self, i: usize, j: usize, k: usize) -> Sc {
        let Some(gk) = self.triple_groups.remove(&(i, j, k)) else {
            return Sc::zero();
        };

        let Some(value) = self.triple_values.remove(&(i, j, k)) else {
            return Sc::zero();
        };

        if let Some(triples) = self.entity_to_triples.get_mut(&i) {
            triples.remove(&(i, j, k));
        }
        if let Some(triples) = self.entity_to_triples.get_mut(&j) {
            triples.remove(&(i, j, k));
        }
        if let Some(triples) = self.entity_to_triples.get_mut(&k) {
            triples.remove(&(i, j, k));
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
    for GroupedTriConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A) -> bool + Send + Sync,
    C: TriCollector<A> + Send + Sync + 'static,
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
            let n = indices.len();
            for pos_i in 0..n {
                for pos_j in (pos_i + 1)..n {
                    for pos_k in (pos_j + 1)..n {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        let a = &entities[i];
                        let b = &entities[j];
                        let c = &entities[k];

                        if !(self.filter)(solution, a, b, c) {
                            continue;
                        }

                        let gk = (self.key_fn)(a, b, c);
                        let value = self.collector.extract(a, b, c);
                        let acc = groups
                            .entry(gk)
                            .or_insert_with(|| self.collector.create_accumulator());
                        acc.accumulate(&value);
                    }
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
            let n = indices.len();
            for pos_i in 0..n {
                for pos_j in (pos_i + 1)..n {
                    for pos_k in (pos_j + 1)..n {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        let a = &entities[i];
                        let b = &entities[j];
                        let c = &entities[k];

                        if !(self.filter)(solution, a, b, c) {
                            continue;
                        }

                        let gk = (self.key_fn)(a, b, c);
                        groups.insert(gk);
                    }
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
            let n = indices.len();
            for pos_i in 0..n {
                for pos_j in (pos_i + 1)..n {
                    for pos_k in (pos_j + 1)..n {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        total = total + self.insert_triple(solution, entities, i, j, k);
                    }
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

        // Collect indices with same join key
        let same_key_indices: Vec<usize> = entities
            .iter()
            .enumerate()
            .filter(|(idx, e)| *idx != entity_index && (self.join_key_extractor)(e) == jk)
            .map(|(idx, _)| idx)
            .collect();

        // Form new triples with the inserted entity
        for (pos_i, &i) in same_key_indices.iter().enumerate() {
            for &j in same_key_indices.iter().skip(pos_i + 1) {
                // Create triple with entity_index as third
                let mut indices = [i, j, entity_index];
                indices.sort();
                let (a, b, c) = (indices[0], indices[1], indices[2]);
                total = total + self.insert_triple(solution, entities, a, b, c);
            }
        }

        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let triples_to_remove: Vec<(usize, usize, usize)> = self
            .entity_to_triples
            .get(&entity_index)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        let mut total = Sc::zero();
        for (i, j, k) in triples_to_remove {
            total = total + self.retract_triple(i, j, k);
        }

        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.triple_groups.clear();
        self.triple_values.clear();
        self.entity_to_triples.clear();
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
    for GroupedTriConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: TriCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedTriConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::tri_count;
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
    fn test_grouped_tri_evaluate() {
        let constraint = GroupedTriConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, c: &Task| c.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task| true,
            tri_count::<Task>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            tasks: vec![
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
            ],
        };

        // 4 tasks on same team = 4 triples (C(4,3)=4)
        // All grouped by priority 1 -> 4 penalty
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-4));
    }

    #[test]
    fn test_grouped_tri_incremental() {
        let mut constraint = GroupedTriConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, c: &Task| c.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task| true,
            tri_count::<Task>(),
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
        assert_eq!(total, SimpleScore::of(-1)); // 1 triple

        // Retract entity 0, removes the only triple
        let delta = constraint.on_retract(&solution, 0);
        assert_eq!(delta, SimpleScore::of(1));
    }
}
