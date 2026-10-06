use std::hash::Hash;

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::relational::operator::Operator;

use super::indexes::remove_index_from_group_bucket;
use super::state::{ComplementedGroupedNodeState, MatchRow};

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    ComplementedGroupedNodeState<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
where
    S: Send + Sync + 'static,
    A: 'static,
    B: 'static,
    T: Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Send + Sync,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    ET: CollectionExtract<S, Item = T> + 'static,
    KA: Fn(&A) -> JK + Send + Sync + 'static,
    KB: Fn(&B) -> JK + Send + Sync + 'static,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&T) -> R + Send + Sync,
{
    /* Builds the aggregate-only evaluation of every matching (A, B) pair and
    every target domain, using the common join for candidate enumeration. */
    pub fn evaluation_state(
        &self,
        solution: &S,
    ) -> super::state::ComplementedGroupedEvaluationState<GK, V, R, Acc> {
        let mut groups = std::collections::HashMap::<GK, Acc>::new();
        self.join.visit_all(solution, &mut |row| {
            let a = row.left.entity;
            let b = row.right.entity;
            let (a_idx, b_idx) = (row.left.index, row.right.index);
            if !(self.filter)(solution, a, b, a_idx, b_idx) {
                return;
            }
            let key = (self.group_key_fn)(a, b);
            let value = self.collector.extract((a, b));
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        });

        let mut targets = Vec::new();
        for target in self.extractor_t.extract(solution) {
            if self.extractor_t.contains(solution, target) {
                targets.push(((self.key_t)(target), (self.default_fn)(target)));
            }
        }

        super::state::ComplementedGroupedEvaluationState {
            groups,
            targets,
            _phantom: std::marker::PhantomData,
        }
    }

    /* Initializes retained state: join indexes plus the accumulated groups and
    the complement membership. */
    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        self.join.initialize(solution);
        let handles = self.join.handles();
        for handle in handles {
            let Some(row) = self.join.resolve(solution, handle) else {
                continue;
            };
            let (a_idx, b_idx) = (row.left.index, row.right.index);
            if (self.filter)(solution, row.left.entity, row.right.entity, a_idx, b_idx) {
                self.retain(solution, a_idx, b_idx);
            }
        }
        let entities_t = self.extractor_t.extract(solution);
        for t_idx in 0..entities_t.len() {
            self.insert_complement(solution, entities_t, t_idx);
        }
        self.changed_groups.clear();
        self.changed_complements.clear();
    }

    /* Borrows the (A, B) entities for a retained pair from the join's own leaf
    sources and accumulates it under its group key. */
    pub(super) fn retain(&mut self, solution: &S, a_idx: usize, b_idx: usize) {
        let (group_key, value) = {
            let a = &self.join.left().extractor().extract(solution)[a_idx];
            let b = &self.join.right().extractor().extract(solution)[b_idx];
            ((self.group_key_fn)(a, b), self.collector.extract((a, b)))
        };
        let (group_id, retraction) = self.insert_value(group_key, value);
        let row_idx = self.match_rows.len();
        self.a_to_matches.entry(a_idx).or_default().push(row_idx);
        self.b_to_matches.entry(b_idx).or_default().push(row_idx);
        self.match_rows.push(MatchRow {
            pair: (a_idx, b_idx),
            group_id,
            retraction,
        });
    }

    pub fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        node_name: &str,
    ) {
        self.changed_groups.clear();
        self.changed_complements.clear();
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        let t_changed = self.t_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed && !t_changed {
            return;
        }
        if a_changed {
            self.retract_by_a(entity_index);
        }
        if b_changed {
            self.retract_by_b(entity_index);
        }

        if a_changed || b_changed {
            let changes = self.join.insert(solution, descriptor_index, entity_index);
            for handle in changes.inserted {
                if let Some(row) = self.join.resolve(solution, handle) {
                    let (a_idx, b_idx) = (row.left.index, row.right.index);
                    if (self.filter)(solution, row.left.entity, row.right.entity, a_idx, b_idx) {
                        self.retain(solution, a_idx, b_idx);
                    }
                }
            }
        }
        if t_changed {
            let entities_t = self.extractor_t.extract(solution);
            self.insert_complement(solution, entities_t, entity_index);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn on_retract(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        node_name: &str,
    ) {
        self.changed_groups.clear();
        self.changed_complements.clear();
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        let t_changed = self.t_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed && !t_changed {
            return;
        }
        if a_changed {
            self.retract_by_a(entity_index);
        }
        if b_changed {
            self.retract_by_b(entity_index);
        }
        if t_changed {
            self.retract_complement(entity_index);
        }
        if a_changed || b_changed {
            self.join.retract(solution, descriptor_index, entity_index);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn reset(&mut self) {
        self.join.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.t_by_group.clear();
        self.t_index_to_group.clear();
        self.t_defaults.clear();
        self.groups.clear();
        self.groups_by_hash.clear();
        self.changed_groups.clear();
        self.changed_complements.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    pub(super) fn retract_by_a(&mut self, a_idx: usize) {
        while let Some(row_idx) = self
            .a_to_matches
            .get(&a_idx)
            .and_then(|bucket| bucket.last())
            .copied()
        {
            self.remove_match(row_idx);
        }
    }

    pub(super) fn retract_by_b(&mut self, b_idx: usize) {
        while let Some(row_idx) = self
            .b_to_matches
            .get(&b_idx)
            .and_then(|bucket| bucket.last())
            .copied()
        {
            self.remove_match(row_idx);
        }
    }

    /* Removes one retained match by exact token and repairs the swap-removed
    positions in both contributor buckets. */
    pub(super) fn remove_match(&mut self, row_idx: usize) {
        if row_idx >= self.match_rows.len() {
            return;
        }
        let last_idx = self.match_rows.len() - 1;
        let removed = self.match_rows.swap_remove(row_idx);
        Self::remove_from_bucket(&mut self.a_to_matches, removed.pair.0, row_idx);
        Self::remove_from_bucket(&mut self.b_to_matches, removed.pair.1, row_idx);
        if row_idx != last_idx {
            let moved_pair = self.match_rows[row_idx].pair;
            Self::replace_position(&mut self.a_to_matches, moved_pair.0, last_idx, row_idx);
            Self::replace_position(&mut self.b_to_matches, moved_pair.1, last_idx, row_idx);
        }
        self.retract_value(removed.group_id, removed.retraction);
    }

    fn remove_from_bucket(
        buckets: &mut std::collections::HashMap<usize, Vec<usize>>,
        key: usize,
        row_idx: usize,
    ) {
        let mut empty = false;
        if let Some(bucket) = buckets.get_mut(&key) {
            if let Some(position) = bucket.iter().position(|held| *held == row_idx) {
                bucket.swap_remove(position);
            }
            empty = bucket.is_empty();
        }
        if empty {
            buckets.remove(&key);
        }
    }

    fn replace_position(
        buckets: &mut std::collections::HashMap<usize, Vec<usize>>,
        key: usize,
        from: usize,
        to: usize,
    ) {
        if let Some(bucket) = buckets.get_mut(&key) {
            for held in bucket.iter_mut() {
                if *held == from {
                    *held = to;
                }
            }
        }
    }

    /* Complement layer: a target belongs to exactly one group key domain. */
    pub(super) fn index_complement(&mut self, group_id: usize, t_idx: usize) {
        if let Some(old_group_id) = self.t_index_to_group.insert(t_idx, group_id) {
            remove_index_from_group_bucket(&mut self.t_by_group, old_group_id, t_idx);
            self.mark_changed(old_group_id);
        }
        self.t_by_group.entry(group_id).or_default().push(t_idx);
        self.mark_complement_changed(t_idx);
        self.mark_changed(group_id);
    }

    pub(super) fn insert_complement(&mut self, solution: &S, entities_t: &[T], t_idx: usize) {
        if t_idx >= entities_t.len() {
            return;
        }
        let complement = &entities_t[t_idx];
        if !self.extractor_t.contains(solution, complement) {
            return;
        }
        let key = (self.key_t)(complement);
        let default_result = (self.default_fn)(complement);
        let group_id = self.group_id_for_key(key);
        self.t_defaults.insert(t_idx, default_result);
        self.index_complement(group_id, t_idx);
    }

    pub(super) fn retract_complement(&mut self, t_idx: usize) {
        let Some(group_id) = self.t_index_to_group.remove(&t_idx) else {
            return;
        };
        self.t_defaults.remove(&t_idx);
        remove_index_from_group_bucket(&mut self.t_by_group, group_id, t_idx);
        self.mark_complement_changed(t_idx);
        self.mark_changed(group_id);
    }
}
