//! Zero-erasure grouped constraint for quad-arity group-by operations.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, QuadCollector};

/// Zero-erasure constraint that groups entity quadruples by key and scores based on collector results.
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped_quad::GroupedQuadConstraint;
/// use solverforge_scoring::stream::collector::quad_count;
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
/// let constraint = GroupedQuadConstraint::new(
///     ConstraintRef::new("", "Quad clustering"),
///     ImpactType::Penalty,
///     |s: &Solution| &s.tasks,
///     |t: &Task| t.team,
///     |_a: &Task, _b: &Task, _c: &Task, d: &Task| d.priority,
///     |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
///     quad_count::<Task>(),
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
///     ],
/// };
///
/// // 4 tasks form 1 quadruple -> 1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct GroupedQuadConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: QuadCollector<A>,
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
    quad_groups: HashMap<(usize, usize, usize, usize), GK>,
    quad_values: HashMap<(usize, usize, usize, usize), C::Value>,
    entity_to_quads: HashMap<usize, HashSet<(usize, usize, usize, usize)>>,
    _phantom: PhantomData<(S, A, JK, Sc)>,
}

impl<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
    GroupedQuadConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync,
    C: QuadCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
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
            quad_groups: HashMap::new(),
            quad_values: HashMap::new(),
            entity_to_quads: HashMap::new(),
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

    fn insert_quad(
        &mut self,
        solution: &S,
        entities: &[A],
        i: usize,
        j: usize,
        k: usize,
        l: usize,
    ) -> Sc {
        let a = &entities[i];
        let b = &entities[j];
        let c = &entities[k];
        let d = &entities[l];

        if !(self.filter)(solution, a, b, c, d) {
            return Sc::zero();
        }

        let gk = (self.key_fn)(a, b, c, d);
        let value = self.collector.extract(a, b, c, d);
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

        self.quad_groups.insert((i, j, k, l), gk);
        self.quad_values.insert((i, j, k, l), value);
        self.entity_to_quads.entry(i).or_default().insert((i, j, k, l));
        self.entity_to_quads.entry(j).or_default().insert((i, j, k, l));
        self.entity_to_quads.entry(k).or_default().insert((i, j, k, l));
        self.entity_to_quads.entry(l).or_default().insert((i, j, k, l));

        new_score - old
    }

    fn retract_quad(&mut self, i: usize, j: usize, k: usize, l: usize) -> Sc {
        let Some(gk) = self.quad_groups.remove(&(i, j, k, l)) else {
            return Sc::zero();
        };

        let Some(value) = self.quad_values.remove(&(i, j, k, l)) else {
            return Sc::zero();
        };

        for idx in [i, j, k, l] {
            if let Some(quads) = self.entity_to_quads.get_mut(&idx) {
                quads.remove(&(i, j, k, l));
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
    for GroupedQuadConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    JKE: Fn(&A) -> JK + Send + Sync,
    KF: Fn(&A, &A, &A, &A) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync,
    C: QuadCollector<A> + Send + Sync + 'static,
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
                            let (i, j, k, l) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l]);
                            let (a, b, c, d) = (&entities[i], &entities[j], &entities[k], &entities[l]);

                            if !(self.filter)(solution, a, b, c, d) {
                                continue;
                            }

                            let gk = (self.key_fn)(a, b, c, d);
                            let value = self.collector.extract(a, b, c, d);
                            let acc = groups
                                .entry(gk)
                                .or_insert_with(|| self.collector.create_accumulator());
                            acc.accumulate(&value);
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
                            let (i, j, k, l) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l]);
                            let (a, b, c, d) = (&entities[i], &entities[j], &entities[k], &entities[l]);

                            if !(self.filter)(solution, a, b, c, d) {
                                continue;
                            }

                            let gk = (self.key_fn)(a, b, c, d);
                            groups.insert(gk);
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
                            let (i, j, k, l) = (indices[p_i], indices[p_j], indices[p_k], indices[p_l]);
                            total = total + self.insert_quad(solution, entities, i, j, k, l);
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
                    let mut indices = [same_key_indices[p_i], same_key_indices[p_j], same_key_indices[p_k], entity_index];
                    indices.sort();
                    let (a, b, c, d) = (indices[0], indices[1], indices[2], indices[3]);
                    total = total + self.insert_quad(solution, entities, a, b, c, d);
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

        let quads_to_remove: Vec<(usize, usize, usize, usize)> = self
            .entity_to_quads
            .get(&entity_index)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        let mut total = Sc::zero();
        for (i, j, k, l) in quads_to_remove {
            total = total + self.retract_quad(i, j, k, l);
        }

        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.quad_groups.clear();
        self.quad_values.clear();
        self.entity_to_quads.clear();
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
    for GroupedQuadConstraint<S, A, GK, JK, E, JKE, KF, Flt, C, W, Sc>
where
    C: QuadCollector<A>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedQuadConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::quad_count;
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
    fn test_grouped_quad_evaluate() {
        let constraint = GroupedQuadConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, _c: &Task, d: &Task| d.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
            quad_count::<Task>(),
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

        // 5 tasks = C(5,4) = 5 quads
        assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-5));
    }

    #[test]
    fn test_grouped_quad_incremental() {
        let mut constraint = GroupedQuadConstraint::new(
            ConstraintRef::new("", "Priority clustering"),
            ImpactType::Penalty,
            |s: &Solution| &s.tasks,
            |t: &Task| t.team,
            |_a: &Task, _b: &Task, _c: &Task, d: &Task| d.priority,
            |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
            quad_count::<Task>(),
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

        let total = constraint.initialize(&solution);
        assert_eq!(total, SimpleScore::of(-1)); // 1 quad

        let delta = constraint.on_retract(&solution, 0);
        assert_eq!(delta, SimpleScore::of(1)); // quad removed
    }
}
