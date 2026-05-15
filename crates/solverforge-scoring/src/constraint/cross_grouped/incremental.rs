use std::collections::HashMap;
use std::hash::Hash;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::CrossGroupedConstraint;

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync,
    GK: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score,
{
    pub(super) fn add_match(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        let pair = (a_idx, b_idx);
        if self.matches.contains_key(&pair) {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        let b = &entities_b[b_idx];
        if !self.extractor_a.contains(solution, a) || !self.extractor_b.contains(solution, b) {
            return Sc::zero();
        }
        if !(self.filter)(solution, a, b, a_idx, b_idx) {
            return Sc::zero();
        }

        let group_key = (self.group_key_fn)(a, b);
        let value = self.collector.extract((a, b));
        let (delta, retraction) = self.insert_value(group_key.clone(), value);
        let row_idx = self.match_rows.len();
        let a_bucket = self.a_to_matches.entry(a_idx).or_default();
        let a_pos = a_bucket.len();
        a_bucket.push(row_idx);
        let b_bucket = self.b_to_matches.entry(b_idx).or_default();
        let b_pos = b_bucket.len();
        b_bucket.push(row_idx);
        self.match_rows.push(super::state::MatchRow {
            pair,
            group_key,
            retraction,
            a_pos,
            b_pos,
        });
        self.matches.insert(pair, row_idx);
        delta
    }

    pub(super) fn remove_match_at(&mut self, row_idx: usize) -> Sc {
        if row_idx >= self.match_rows.len() {
            return Sc::zero();
        }

        let pair = self.match_rows[row_idx].pair;
        let a_pos = self.match_rows[row_idx].a_pos;
        let b_pos = self.match_rows[row_idx].b_pos;
        self.matches.remove(&pair);
        self.remove_from_a_bucket(pair.0, row_idx, a_pos);
        self.remove_from_b_bucket(pair.1, row_idx, b_pos);

        let last_idx = self.match_rows.len() - 1;
        let row = self.match_rows.swap_remove(row_idx);
        if row_idx != last_idx {
            let moved = &self.match_rows[row_idx];
            self.matches.insert(moved.pair, row_idx);
            if let Some(a_matches) = self.a_to_matches.get_mut(&moved.pair.0) {
                a_matches[moved.a_pos] = row_idx;
            }
            if let Some(b_matches) = self.b_to_matches.get_mut(&moved.pair.1) {
                b_matches[moved.b_pos] = row_idx;
            }
        }

        self.retract_value(row.group_key, row.retraction)
    }

    pub(super) fn remove_from_a_bucket(&mut self, a_idx: usize, row_idx: usize, pos: usize) {
        let mut remove_bucket = false;
        if let Some(a_matches) = self.a_to_matches.get_mut(&a_idx) {
            debug_assert_eq!(a_matches[pos], row_idx);
            a_matches.swap_remove(pos);
            if pos < a_matches.len() {
                let moved_row_idx = a_matches[pos];
                self.match_rows[moved_row_idx].a_pos = pos;
            }
            remove_bucket = a_matches.is_empty();
        }
        if remove_bucket {
            self.a_to_matches.remove(&a_idx);
        }
    }

    pub(super) fn remove_from_b_bucket(&mut self, b_idx: usize, row_idx: usize, pos: usize) {
        let mut remove_bucket = false;
        if let Some(b_matches) = self.b_to_matches.get_mut(&b_idx) {
            debug_assert_eq!(b_matches[pos], row_idx);
            b_matches.swap_remove(pos);
            if pos < b_matches.len() {
                let moved_row_idx = b_matches[pos];
                self.match_rows[moved_row_idx].b_pos = pos;
            }
            remove_bucket = b_matches.is_empty();
        }
        if remove_bucket {
            self.b_to_matches.remove(&b_idx);
        }
    }

    fn remove_index_from_key_bucket(
        indexes_by_key: &mut HashMap<JK, Vec<usize>>,
        key: &JK,
        idx: usize,
    ) {
        let mut remove_bucket = false;
        if let Some(indices) = indexes_by_key.get_mut(key) {
            if let Some(pos) = indices.iter().position(|candidate| *candidate == idx) {
                indices.swap_remove(pos);
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            indexes_by_key.remove(key);
        }
    }

    pub(super) fn insert_a(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
    ) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        if !self.extractor_a.contains(solution, a) {
            return Sc::zero();
        }
        let key = (self.key_a)(a);
        self.a_index_to_key.insert(a_idx, key.clone());
        self.a_by_key.entry(key.clone()).or_default().push(a_idx);

        let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();
        let mut total = Sc::zero();
        for b_idx in b_indices {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }
        total
    }

    pub(super) fn retract_a(&mut self, a_idx: usize) -> Sc {
        if let Some(key) = self.a_index_to_key.remove(&a_idx) {
            Self::remove_index_from_key_bucket(&mut self.a_by_key, &key, a_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .a_to_matches
            .get(&a_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(row_idx);
        }
        total
    }

    pub(super) fn insert_b(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        b_idx: usize,
    ) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }

        let b = &entities_b[b_idx];
        if !self.extractor_b.contains(solution, b) {
            return Sc::zero();
        }
        let key = (self.key_b)(b);
        self.b_index_to_key.insert(b_idx, key.clone());
        self.b_by_key.entry(key.clone()).or_default().push(b_idx);

        let a_indices = self.a_by_key.get(&key).cloned().unwrap_or_default();
        let mut total = Sc::zero();
        for a_idx in a_indices {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }
        total
    }

    pub(super) fn retract_b(&mut self, b_idx: usize) -> Sc {
        if let Some(key) = self.b_index_to_key.remove(&b_idx) {
            Self::remove_index_from_key_bucket(&mut self.b_by_key, &key, b_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .b_to_matches
            .get(&b_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(row_idx);
        }
        total
    }
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync,
    GK: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
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

        groups.iter().fold(Sc::zero(), |total, (key, acc)| {
            total + acc.with_result(|result| self.compute_score(key, result))
        })
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let b_by_key = self.b_index_for(solution, entities_b);
        let mut groups = HashMap::<GK, ()>::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b, a_idx, b_idx) {
                    groups.insert((self.group_key_fn)(a, b), ());
                }
            }
        }

        groups.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        self.build_indexes(solution, entities_a, entities_b);

        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            if !self.extractor_a.contains(solution, &entities_a[a_idx]) {
                continue;
            }
            let key = (self.key_a)(&entities_a[a_idx]);
            let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();
            for b_idx in b_indices {
                total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
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
        let mut total = Sc::zero();
        if !a_changed && !b_changed {
            return total;
        }

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        if a_changed {
            total = total + self.insert_a(solution, entities_a, entities_b, entity_index);
        }
        if b_changed {
            total = total + self.insert_b(solution, entities_a, entities_b, entity_index);
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let mut total = Sc::zero();
        if !a_changed && !b_changed {
            return total;
        }

        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if b_changed {
            total = total + self.retract_b(entity_index);
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
