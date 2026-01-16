//! Zero-erasure grouped constraint for penta-arity group-by operations.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, PentaCollector};

/// Zero-erasure constraint that groups entity quintuples by key and scores based on collector results.
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped_penta::GroupedPentaConstraint;
/// use solverforge_scoring::stream::collector::penta_count;
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
/// let constraint = GroupedPentaConstraint::new(
///     ConstraintRef::new("", "Penta clustering"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.tasks,
///     |t: &Task| t.team,
///     |_a: &Task, _b: &Task, _c: &Task, _d: &Task, e: &Task| e.priority,
///     |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task, _e: &Task| true,
///     penta_count::<Task>(),
///     |count: &usize| SimpleScore::of(*count as i64),
///     false,
/// );
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///         Task { team: 1, priority: 1 },
///     ],
/// };
///
/// // 5 tasks form 1 quintuple -> 1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct GroupedPentaConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: PentaCollector<A>,
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
    penta_groups: HashMap<(usize, usize, usize, usize, usize), GK>,
    penta_values: HashMap<(usize, usize, usize, usize, usize), C::Value>,
    entity_to_pentas: HashMap<usize, HashSet<(usize, usize, usize, usize, usize)>>,
    _phantom: PhantomData<(S, A, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
    GroupedPentaConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
    C: PentaCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
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
            penta_groups: HashMap::new(),
            penta_values: HashMap::new(),
            entity_to_pentas: HashMap::new(),
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

    #[allow(clippy::too_many_arguments)]
    fn insert_penta(
        &mut self,
        solution: &S,
        entities: &[A],
        i: usize,
        j: usize,
        k: usize,
        l: usize,
        m: usize,
    ) -> Sc {
        let (a, b, c, d, e) = (&entities[i], &entities[j], &entities[k], &entities[l], &entities[m]);

        if !(self.filter)(solution, a, b, c, d, e) {
            return Sc::zero();
        }

        let gk = (self.key_fn)(a, b, c, d, e);
        let value = self.collector.extract(a, b, c, d, e);
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

        let tuple = (i, j, k, l, m);
        self.penta_groups.insert(tuple, gk);
        self.penta_values.insert(tuple, value);
        for idx in [i, j, k, l, m] {
            self.entity_to_pentas.entry(idx).or_default().insert(tuple);
        }

        new_score - old
    }

    fn retract_penta(&mut self, i: usize, j: usize, k: usize, l: usize, m: usize) -> Sc {
        let tuple = (i, j, k, l, m);
        let Some(gk) = self.penta_groups.remove(&tuple) else {
            return Sc::zero();
        };

        let Some(value) = self.penta_values.remove(&tuple) else {
            return Sc::zero();
        };

        for idx in [i, j, k, l, m] {
            if let Some(pentas) = self.entity_to_pentas.get_mut(&idx) {
                pentas.remove(&tuple);
            }
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
    for GroupedPentaConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
    C: PentaCollector<A> + Send + Sync + 'static,
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
            for p_i in 0..n {
                for p_j in (p_i + 1)..n {
                    for p_k in (p_j + 1)..n {
                        for p_l in (p_k + 1)..n {
                            for p_m in (p_l + 1)..n {
                                let (i, j, k, l, m) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l], indices[p_m]);
                                let (a, b, c, d, e) = (&entities[i], &entities[j], &entities[k], &entities[l], &entities[m]);

                                if !(self.filter)(solution, a, b, c, d, e) {
                                    continue;
                                }

                                let gk = (self.key_fn)(a, b, c, d, e);
                                let value = self.collector.extract(a, b, c, d, e);
                                let acc = groups
                                    .entry(gk)
                                    .or_insert_with(|| self.collector.create_accumulator());
                                acc.accumulate(&value);
                            }
                        }
                    }
                }
            }
        }

        let mut total = Sc::zero();
        for acc in groups.values() {
            total = total + self.compute_score(&acc.finish());
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);
        let join_index = self.build_join_index(entities);

        let mut groups: HashSet<GK> = HashSet::new();

        for indices in join_index.values() {
            let n = indices.len();
            for p_i in 0..n {
                for p_j in (p_i + 1)..n {
                    for p_k in (p_j + 1)..n {
                        for p_l in (p_k + 1)..n {
                            for p_m in (p_l + 1)..n {
                                let (i, j, k, l, m) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l], indices[p_m]);
                                let (a, b, c, d, e) = (&entities[i], &entities[j], &entities[k], &entities[l], &entities[m]);

                                if !(self.filter)(solution, a, b, c, d, e) {
                                    continue;
                                }

                                let gk = (self.key_fn)(a, b, c, d, e);
                                groups.insert(gk);
                            }
                        }
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
            for p_i in 0..n {
                for p_j in (p_i + 1)..n {
                    for p_k in (p_j + 1)..n {
                        for p_l in (p_k + 1)..n {
                            for p_m in (p_l + 1)..n {
                                let (i, j, k, l, m) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l], indices[p_m]);
                                total = total + self.insert_penta(solution, entities, i, j, k, l, m);
                            }
                        }
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

        let same_key_indices: Vec<usize> = entities
            .iter()
            .enumerate()
            .filter(|(idx, e)| *idx != entity_index && (self.join_key_extractor)(e) == jk)
            .map(|(idx, _)| idx)
            .collect();

        let n = same_key_indices.len();
        for p_i in 0..n {
            for p_j in (p_i + 1)..n {
                for p_k in (p_j + 1)..n {
                    for p_l in (p_k + 1)..n {
                        let mut indices = [
                            same_key_indices[p_i],
                            same_key_indices[p_j],
                            same_key_indices[p_k],
                            same_key_indices[p_l],
                            entity_index,
                        ];
                        indices.sort();
                        let (a, b, c, d, e) = (indices[0], indices[1], indices[2], indices[3], indices[4]);
                        total = total + self.insert_penta(solution, entities, a, b, c, d, e);
                    }
                }
            }
        }

        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        if entity_index >= entities.len() {
            return Sc::zero();
        }

        let pentas_to_remove: Vec<(usize, usize, usize, usize, usize)> = self
            .entity_to_pentas
            .get(&entity_index)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        let mut total = Sc::zero();
        for (i, j, k, l, m) in pentas_to_remove {
            total = total + self.retract_penta(i, j, k, l, m);
        }

        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.penta_groups.clear();
        self.penta_values.clear();
        self.entity_to_pentas.clear();
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
    for GroupedPentaConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: PentaCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedPentaConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::penta_count;
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
    fn test_grouped_penta_evaluate() {
        let constraint = GroupedPentaConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, _c: &Task, _d: &Task, e: &Task| e.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task, _e: &Task| true,
            penta_count::<Task>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            tasks: vec![
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
            ],
        };

        // 6 tasks = C(6,5) = 6 quintuples
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-6));
    }

    #[test]
    fn test_grouped_penta_incremental() {
        let mut constraint = GroupedPentaConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, _c: &Task, _d: &Task, e: &Task| e.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task, _e: &Task| true,
            penta_count::<Task>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let solution = Solution {
            tasks: vec![
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
                Task { team: 1, priority: 1 },
            ],
        };

        let total = constraint.initialize(&solution);
        assert_eq!(total, SimpleScore::of(-1)); // 1 quintuple

        let delta = constraint.on_retract(&solution, 0);
        assert_eq!(delta, SimpleScore::of(1)); // quintuple removed
    }
}
