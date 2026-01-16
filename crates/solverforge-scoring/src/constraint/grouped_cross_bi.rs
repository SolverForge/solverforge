//! Zero-erasure grouped constraint for cross-bi-arity group-by operations.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, CrossBiCollector};

/// Zero-erasure constraint that groups cross-entity pairs by key and scores based on collector results.
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::grouped_cross_bi::GroupedCrossBiConstraint;
/// use solverforge_scoring::stream::collector::cross_bi_count;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Shift { employee_id: Option<usize>, day: u32 }
///
/// #[derive(Clone, Hash, PartialEq, Eq)]
/// struct Employee { id: usize, department: u32 }
///
/// #[derive(Clone)]
/// struct Schedule { shifts: Vec<Shift>, employees: Vec<Employee> }
///
/// // Count shift-employee pairs per department
/// let constraint = GroupedCrossBiConstraint::new(
///     ConstraintRef::new("", "Shifts per department"),
///     ImpactType::Penalty,
///     |s: &Schedule| &s.shifts,
///     |s: &Schedule| &s.employees,
///     |shift: &Shift| shift.employee_id,
///     |emp: &Employee| Some(emp.id),
///     |_shift: &Shift, emp: &Employee| emp.department,
///     |_s: &Schedule, _a: &Shift, _b: &Employee| true,
///     cross_bi_count::<Shift, Employee>(),
///     |count: &usize| SimpleScore::of(*count as i64),
///     false,
/// );
///
/// let schedule = Schedule {
///     shifts: vec![
///         Shift { employee_id: Some(0), day: 1 },
///         Shift { employee_id: Some(1), day: 2 },
///     ],
///     employees: vec![
///         Employee { id: 0, department: 10 },
///         Employee { id: 1, department: 10 },
///     ],
/// };
///
/// // 2 pairs, both in department 10 -> -2 penalty
/// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-2));
/// ```
pub struct GroupedCrossBiConstraint<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc>
where
    C: CrossBiCollector<A, B>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    key_fn: KF,
    filter: Flt,
    collector: C,
    weight_fn: W,
    is_hard: bool,
    groups: HashMap<GK, C::Accumulator>,
    pair_groups: HashMap<(usize, usize), GK>,
    pair_values: HashMap<(usize, usize), C::Value>,
    a_to_pairs: HashMap<usize, HashSet<(usize, usize)>>,
    b_to_pairs: HashMap<usize, HashSet<(usize, usize)>>,
    _phantom: PhantomData<(S, A, B, JK, Sc)>,
}

impl<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc>
    GroupedCrossBiConstraint<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    KF: Fn(&A, &B) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &B) -> bool + Send + Sync,
    C: CrossBiCollector<A, B> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        key_fn: KF,
        filter: Flt,
        collector: C,
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
            key_fn,
            filter,
            collector,
            weight_fn,
            is_hard,
            groups: HashMap::new(),
            pair_groups: HashMap::new(),
            pair_values: HashMap::new(),
            a_to_pairs: HashMap::new(),
            b_to_pairs: HashMap::new(),
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

    fn build_b_index(&self, entities_b: &[B]) -> HashMap<JK, Vec<usize>> {
        let mut index: HashMap<JK, Vec<usize>> = HashMap::new();
        for (idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            index.entry(key).or_default().push(idx);
        }
        index
    }

    fn insert_pair(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        let a = &entities_a[a_idx];
        let b = &entities_b[b_idx];

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

        self.pair_groups.insert((a_idx, b_idx), gk);
        self.pair_values.insert((a_idx, b_idx), value);
        self.a_to_pairs.entry(a_idx).or_default().insert((a_idx, b_idx));
        self.b_to_pairs.entry(b_idx).or_default().insert((a_idx, b_idx));

        new_score - old
    }

    fn retract_pair(&mut self, a_idx: usize, b_idx: usize) -> Sc {
        let Some(gk) = self.pair_groups.remove(&(a_idx, b_idx)) else {
            return Sc::zero();
        };

        let Some(value) = self.pair_values.remove(&(a_idx, b_idx)) else {
            return Sc::zero();
        };

        if let Some(pairs) = self.a_to_pairs.get_mut(&a_idx) {
            pairs.remove(&(a_idx, b_idx));
        }
        if let Some(pairs) = self.b_to_pairs.get_mut(&b_idx) {
            pairs.remove(&(a_idx, b_idx));
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

impl<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc> IncrementalConstraint<S, Sc>
    for GroupedCrossBiConstraint<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    GK: Clone + Eq + Hash + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    KF: Fn(&A, &B) -> GK + Send + Sync,
    Flt: Fn(&S, &A, &B) -> bool + Send + Sync,
    C: CrossBiCollector<A, B> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let b_index = self.build_b_index(entities_b);

        let mut groups: HashMap<GK, C::Accumulator> = HashMap::new();

        for a in entities_a.iter() {
            let key = (self.key_a)(a);
            if let Some(b_indices) = b_index.get(&key) {
                for &b_idx in b_indices {
                    let b = &entities_b[b_idx];

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
            total = total + self.compute_score(&acc.finish());
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let b_index = self.build_b_index(entities_b);

        let mut groups: HashSet<GK> = HashSet::new();

        for a in entities_a.iter() {
            let key = (self.key_a)(a);
            if let Some(b_indices) = b_index.get(&key) {
                for &b_idx in b_indices {
                    let b = &entities_b[b_idx];

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

        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let b_index = self.build_b_index(entities_b);
        let mut total = Sc::zero();

        for (a_idx, a) in entities_a.iter().enumerate() {
            let key = (self.key_a)(a);
            if let Some(b_indices) = b_index.get(&key) {
                for &b_idx in b_indices {
                    total = total + self.insert_pair(solution, entities_a, entities_b, a_idx, b_idx);
                }
            }
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        if entity_index < entities_a.len() {
            // Entity from A collection
            let a = &entities_a[entity_index];
            let key = (self.key_a)(a);
            let b_index = self.build_b_index(entities_b);
            let mut total = Sc::zero();

            if let Some(b_indices) = b_index.get(&key) {
                for &b_idx in b_indices {
                    total = total + self.insert_pair(solution, entities_a, entities_b, entity_index, b_idx);
                }
            }

            total
        } else {
            // Entity from B collection (offset by A length)
            let b_idx = entity_index - entities_a.len();
            if b_idx >= entities_b.len() {
                return Sc::zero();
            }

            let b = &entities_b[b_idx];
            let key = (self.key_b)(b);
            let mut total = Sc::zero();

            for (a_idx, a) in entities_a.iter().enumerate() {
                if (self.key_a)(a) == key {
                    total = total + self.insert_pair(solution, entities_a, entities_b, a_idx, b_idx);
                }
            }

            total
        }
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);

        if entity_index < entities_a.len() {
            // Entity from A collection
            let pairs_to_remove: Vec<(usize, usize)> = self
                .a_to_pairs
                .get(&entity_index)
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default();

            let mut total = Sc::zero();
            for (a_idx, b_idx) in pairs_to_remove {
                total = total + self.retract_pair(a_idx, b_idx);
            }

            total
        } else {
            // Entity from B collection (offset by A length)
            let b_idx = entity_index - entities_a.len();

            let pairs_to_remove: Vec<(usize, usize)> = self
                .b_to_pairs
                .get(&b_idx)
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default();

            let mut total = Sc::zero();
            for (a_idx, b_idx) in pairs_to_remove {
                total = total + self.retract_pair(a_idx, b_idx);
            }

            total
        }
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.pair_groups.clear();
        self.pair_values.clear();
        self.a_to_pairs.clear();
        self.b_to_pairs.clear();
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

impl<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc> std::fmt::Debug
    for GroupedCrossBiConstraint<S, A, B, GK, JK, EA, EB, KA, KB, KF, Flt, C, W, Sc>
where
    C: CrossBiCollector<A, B>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedCrossBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::collector::cross_bi_count;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct Shift {
        employee_id: Option<usize>,
        day: u32,
    }

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct Employee {
        id: usize,
        department: u32,
    }

    #[derive(Clone)]
    struct Schedule {
        shifts: Vec<Shift>,
        employees: Vec<Employee>,
    }

    #[test]
    fn test_grouped_cross_bi_evaluate() {
        let constraint = GroupedCrossBiConstraint::new(
            ConstraintRef::new("", "Shifts per department"),
            ImpactType::Penalty,
            |s: &Schedule| &s.shifts,
            |s: &Schedule| &s.employees,
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| Some(emp.id),
            |_shift: &Shift, emp: &Employee| emp.department,
            |_s: &Schedule, _a: &Shift, _b: &Employee| true,
            cross_bi_count::<Shift, Employee>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            shifts: vec![
                Shift { employee_id: Some(0), day: 1 },
                Shift { employee_id: Some(1), day: 2 },
                Shift { employee_id: Some(0), day: 3 },
            ],
            employees: vec![
                Employee { id: 0, department: 10 },
                Employee { id: 1, department: 20 },
            ],
        };

        // 2 pairs with dept 10, 1 pair with dept 20 -> -3 penalty
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-3));
    }

    #[test]
    fn test_grouped_cross_bi_incremental() {
        let mut constraint = GroupedCrossBiConstraint::new(
            ConstraintRef::new("", "Shifts per department"),
            ImpactType::Penalty,
            |s: &Schedule| &s.shifts,
            |s: &Schedule| &s.employees,
            |shift: &Shift| shift.employee_id,
            |emp: &Employee| Some(emp.id),
            |_shift: &Shift, emp: &Employee| emp.department,
            |_s: &Schedule, _a: &Shift, _b: &Employee| true,
            cross_bi_count::<Shift, Employee>(),
            |count: &usize| SimpleScore::of(*count as i64),
            false,
        );

        let schedule = Schedule {
            shifts: vec![
                Shift { employee_id: Some(0), day: 1 },
                Shift { employee_id: Some(0), day: 2 },
            ],
            employees: vec![
                Employee { id: 0, department: 10 },
            ],
        };

        let total = constraint.initialize(&schedule);
        assert_eq!(total, SimpleScore::of(-2));

        // Retract first shift
        let delta = constraint.on_retract(&schedule, 0);
        assert_eq!(delta, SimpleScore::of(1));
    }
}
