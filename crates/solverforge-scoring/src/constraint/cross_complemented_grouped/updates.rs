use std::collections::HashMap;
use std::hash::Hash;

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;

use super::CrossComplementedGroupedConstraint;

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    CrossComplementedGroupedConstraint<
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
    pub(super) fn add_match(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_t: &[T],
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
        let (delta, retraction) = self.insert_value(entities_t, group_key.clone(), value);
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

    pub(super) fn remove_match_at(&mut self, entities_t: &[T], row_idx: usize) -> Sc {
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

        self.retract_value(entities_t, row.group_key, row.retraction)
    }

    pub(super) fn remove_from_a_bucket(&mut self, a_idx: usize, row_idx: usize, pos: usize) {
        let mut remove_bucket = false;
        if let Some(a_matches) = self.a_to_matches.get_mut(&a_idx) {
            if let Some(remove_pos) = a_matches
                .get(pos)
                .filter(|candidate| **candidate == row_idx)
                .map(|_| pos)
                .or_else(|| a_matches.iter().position(|candidate| *candidate == row_idx))
            {
                a_matches.swap_remove(remove_pos);
                if remove_pos < a_matches.len() {
                    let moved_row_idx = a_matches[remove_pos];
                    self.match_rows[moved_row_idx].a_pos = remove_pos;
                }
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
            if let Some(remove_pos) = b_matches
                .get(pos)
                .filter(|candidate| **candidate == row_idx)
                .map(|_| pos)
                .or_else(|| b_matches.iter().position(|candidate| *candidate == row_idx))
            {
                b_matches.swap_remove(remove_pos);
                if remove_pos < b_matches.len() {
                    let moved_row_idx = b_matches[remove_pos];
                    self.match_rows[moved_row_idx].b_pos = remove_pos;
                }
            }
            remove_bucket = b_matches.is_empty();
        }
        if remove_bucket {
            self.b_to_matches.remove(&b_idx);
        }
    }

    fn remove_index_from_join_key_bucket(
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
        entities_t: &[T],
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
            total =
                total + self.add_match(solution, entities_a, entities_b, entities_t, a_idx, b_idx);
        }
        total
    }

    pub(super) fn retract_a(&mut self, entities_t: &[T], a_idx: usize) -> Sc {
        if let Some(key) = self.a_index_to_key.remove(&a_idx) {
            Self::remove_index_from_join_key_bucket(&mut self.a_by_key, &key, a_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .a_to_matches
            .get(&a_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(entities_t, row_idx);
        }
        total
    }

    pub(super) fn insert_b(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_t: &[T],
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
            total =
                total + self.add_match(solution, entities_a, entities_b, entities_t, a_idx, b_idx);
        }
        total
    }

    pub(super) fn retract_b(&mut self, entities_t: &[T], b_idx: usize) -> Sc {
        if let Some(key) = self.b_index_to_key.remove(&b_idx) {
            Self::remove_index_from_join_key_bucket(&mut self.b_by_key, &key, b_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .b_to_matches
            .get(&b_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(entities_t, row_idx);
        }
        total
    }

    pub(super) fn insert_complement(&mut self, solution: &S, entities_t: &[T], t_idx: usize) -> Sc {
        if t_idx >= entities_t.len() {
            return Sc::zero();
        }
        let complement = &entities_t[t_idx];
        if !self.extractor_t.contains(solution, complement) {
            return Sc::zero();
        }
        let key = (self.key_t)(complement);
        self.index_complement(key.clone(), t_idx);
        self.complement_score_for_index(entities_t, &key, t_idx)
    }

    pub(super) fn retract_complement(&mut self, entities_t: &[T], t_idx: usize) -> Sc {
        let Some(key) = self.t_index_to_key.remove(&t_idx) else {
            return Sc::zero();
        };
        let delta = -self.complement_score_for_index(entities_t, &key, t_idx);
        let mut remove_bucket = false;
        if let Some(indices) = self.t_by_key.get_mut(&key) {
            if let Some(pos) = indices.iter().position(|candidate| *candidate == t_idx) {
                indices.swap_remove(pos);
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            self.t_by_key.remove(&key);
        }
        delta
    }
}
