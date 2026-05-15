use std::collections::HashMap;
use std::hash::Hash;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::CrossComplementedGroupedConstraint;

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    IncrementalConstraint<S, Sc>
    for CrossComplementedGroupedConstraint<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync,
    GK: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    ET: CollectionExtract<S, Item = T> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&T) -> R + Send + Sync,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_t = self.extractor_t.extract(solution);
        let b_by_key = self.b_index_for(solution, entities_b);
        let mut groups = HashMap::<GK, Acc>::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if !(self.filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                let key = (self.group_key_fn)(a, b);
                let value = self.collector.extract((a, b));
                groups
                    .entry(key)
                    .or_insert_with(|| self.collector.create_accumulator())
                    .accumulate(value);
            }
        }

        entities_t.iter().fold(Sc::zero(), |total, complement| {
            if !self.extractor_t.contains(solution, complement) {
                return total;
            }
            let key = (self.key_t)(complement);
            total
                + match groups.get(&key) {
                    Some(acc) => acc.with_result(|result| self.compute_score(&key, result)),
                    None => {
                        let default_result = (self.default_fn)(complement);
                        self.compute_score(&key, &default_result)
                    }
                }
        })
    }

    fn match_count(&self, solution: &S) -> usize {
        self.extractor_t
            .extract(solution)
            .iter()
            .filter(|target| self.extractor_t.contains(solution, target))
            .count()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_t = self.extractor_t.extract(solution);
        self.build_join_indexes(solution, entities_a, entities_b);

        let mut total = Sc::zero();
        for t_idx in 0..entities_t.len() {
            total = total + self.insert_complement(solution, entities_t, t_idx);
        }
        for a_idx in 0..entities_a.len() {
            if !self.extractor_a.contains(solution, &entities_a[a_idx]) {
                continue;
            }
            let key = (self.key_a)(&entities_a[a_idx]);
            let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();
            for b_idx in b_indices {
                total = total
                    + self.add_match(solution, entities_a, entities_b, entities_t, a_idx, b_idx);
            }
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let t_changed = self
            .t_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        if !a_changed && !b_changed && !t_changed {
            return Sc::zero();
        }

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_t = self.extractor_t.extract(solution);
        let mut total = Sc::zero();
        if a_changed {
            total =
                total + self.insert_a(solution, entities_a, entities_b, entities_t, entity_index);
        }
        if b_changed {
            total =
                total + self.insert_b(solution, entities_a, entities_b, entities_t, entity_index);
        }
        if t_changed {
            total = total + self.insert_complement(solution, entities_t, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let t_changed = self
            .t_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        if !a_changed && !b_changed && !t_changed {
            return Sc::zero();
        }

        let entities_t = self.extractor_t.extract(solution);
        let mut total = Sc::zero();
        if a_changed {
            total = total + self.retract_a(entities_t, entity_index);
        }
        if b_changed {
            total = total + self.retract_b(entities_t, entity_index);
        }
        if t_changed {
            total = total + self.retract_complement(entities_t, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.clear_state();
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }
}
