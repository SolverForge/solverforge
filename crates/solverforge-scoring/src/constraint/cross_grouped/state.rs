use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::relational::operator::{CollectionNode, JoinNode, Operator};
use crate::stream::relational::view_plan::{EntityKey, ViewEqualPlan};
use crate::stream::relational::{DenseRowStore, RowHandle};

use super::indexes::key_hash;

pub(super) type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<K, Acc> {
    pub(super) key: K,
    pub(super) accumulator: Acc,
    pub(super) count: usize,
}

/* One retained match: its (A index, B index) domain coordinates, the group it
feeds, and the accumulator token that retracts exactly this contributor. */
pub(super) struct MatchRow<Retraction> {
    pub(super) pair: (usize, usize),
    pub(super) group_id: usize,
    pub(super) retraction: Retraction,
}

/* Shared grouped-over-join state.

Match and filter retention belongs to the common operator tree: a `JoinNode`
over the two leaf collections, keyed by a typed view-equality plan, with the
authored residual predicate applied to each candidate pair. This state owns
only the grouped accumulator layer — which group each joined row contributes
to, the exact retraction token per contributor, and the reverse contributor->
row links used to retract a changed entity's rows.
*/
pub struct GroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    JK: Eq + std::hash::Hash + Clone,
{
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) collector: C,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    /* Common join operator owns typed key indexing and candidate enumeration. */
    pub(super) join: JoinNode<
        S,
        CollectionNode<S, EA>,
        CollectionNode<S, EB>,
        ViewEqualPlan<JK, EntityKey<KA>, EntityKey<KB>>,
    >,
    pub(super) rows: DenseRowStore<RowHandle>,
    pub(super) match_rows: Vec<MatchRow<CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) groups: Vec<GroupState<GK, Acc>>,
    pub(super) groups_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) changed_groups: Vec<usize>,
    pub(super) update_count: usize,
    pub(super) changed_key_count: usize,
    _phantom: PhantomData<(fn() -> A, fn() -> B, fn() -> V, fn() -> R)>,
}

pub struct GroupedEvaluationState<GK, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) groups: HashMap<GK, Acc>,
    pub(super) _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
    GroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    S: Send + Sync + 'static,
    A: 'static,
    B: 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Send + Sync,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    KA: Fn(&A) -> JK + Send + Sync + 'static,
    KB: Fn(&B) -> JK + Send + Sync + 'static,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        collector: C,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        let join = JoinNode::new(
            CollectionNode::new(extractor_a, 1),
            CollectionNode::new(extractor_b, 0),
            ViewEqualPlan::new(EntityKey::new(key_a), EntityKey::new(key_b)),
        );
        Self {
            filter,
            group_key_fn,
            collector,
            a_source,
            b_source,
            join,
            rows: DenseRowStore::new(),
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            changed_groups: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    /* Enumerates every matching pair through the common join operator and
    accumulates each into its group. Used for both full evaluation and the
    retained-state rebuild after a localized change. */
    /* Retains every accepted joined row after a full (re)initialization.
    Matching pairs are collected as owned (A index, B index) coordinates, then
    materialized, so no join borrow is held across the mutable accumulation. */
    fn rebuild_grouped(&mut self, solution: &S) {
        let mut accepted: Vec<(usize, usize)> = Vec::new();
        self.join.visit_all(solution, &mut |row| {
            let (a_idx, b_idx) = (row.left.index, row.right.index);
            if (self.filter)(solution, row.left.entity, row.right.entity, a_idx, b_idx) {
                accepted.push((a_idx, b_idx));
            }
        });
        for (a_idx, b_idx) in accepted {
            self.retain(solution, a_idx, b_idx);
        }
    }

    /* Borrows the (A, B) entities for a retained pair from the join's sources,
    then accumulates. Keeps the borrow confined to the argument evaluation. */
    fn retain(&mut self, solution: &S, a_idx: usize, b_idx: usize) {
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

    pub fn evaluation_state(&self, solution: &S) -> GroupedEvaluationState<GK, V, R, Acc> {
        let mut groups = HashMap::<GK, Acc>::new();
        self.join.visit_all(solution, &mut |row| {
            let a = row.left.entity;
            let b = row.right.entity;
            if !(self.filter)(solution, a, b, row.left.index, row.right.index) {
                return;
            }
            let key = (self.group_key_fn)(a, b);
            let value = self.collector.extract((a, b));
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        });
        GroupedEvaluationState {
            groups,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        self.join.initialize(solution);
        self.rebuild_grouped(solution);
        self.changed_groups.clear();
    }

    pub fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        node_name: &str,
    ) {
        self.changed_groups.clear();
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed {
            return;
        }
        if a_changed {
            self.retract_by_a(entity_index);
        }
        if b_changed {
            self.retract_by_b(entity_index);
        }
        let changes = self.join.insert(solution, descriptor_index, entity_index);
        for handle in changes.inserted {
            if let Some(row) = self.join.resolve(solution, handle) {
                let (a_idx, b_idx) = (row.left.index, row.right.index);
                if (self.filter)(solution, row.left.entity, row.right.entity, a_idx, b_idx) {
                    self.retain(solution, a_idx, b_idx);
                }
            }
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
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed {
            return;
        }
        if a_changed {
            self.retract_by_a(entity_index);
        }
        if b_changed {
            self.retract_by_b(entity_index);
        }
        self.join.retract(solution, descriptor_index, entity_index);
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn reset(&mut self) {
        self.join.clear();
        self.rows.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.groups.clear();
        self.groups_by_hash.clear();
        self.changed_groups.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    pub(super) fn mark_changed(&mut self, group_id: usize) {
        if !self.changed_groups.contains(&group_id) {
            self.changed_groups.push(group_id);
        }
    }

    /* Retracts every retained row contributed by A index `a_idx`. */
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

    /* Retracts every retained row contributed by B index `b_idx`. */
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

    fn remove_from_bucket(buckets: &mut HashMap<usize, Vec<usize>>, key: usize, row_idx: usize) {
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
        buckets: &mut HashMap<usize, Vec<usize>>,
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

    pub(super) fn insert_value(
        &mut self,
        key: GK,
        value: V,
    ) -> (usize, CollectorRetraction<Acc, V, R>) {
        let group_id = self.group_id_for_key(key);
        let group = &mut self.groups[group_id];
        if group.count == 0 {
            group.accumulator.reset();
        }
        let retraction = group.accumulator.accumulate(value);
        group.count += 1;
        self.mark_changed(group_id);
        (group_id, retraction)
    }

    pub(super) fn retract_value(
        &mut self,
        group_id: usize,
        retraction: CollectorRetraction<Acc, V, R>,
    ) {
        let Some(group) = self.groups.get_mut(group_id) else {
            return;
        };
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        self.mark_changed(group_id);
    }

    fn group_id_for_key(&mut self, key: GK) -> usize {
        let hash = key_hash(&key);
        if let Some(group_id) = self.find_group(hash, &key) {
            return group_id;
        }
        let group_id = self.groups.len();
        self.groups.push(GroupState {
            key,
            accumulator: self.collector.create_accumulator(),
            count: 0,
        });
        self.groups_by_hash.entry(hash).or_default().push(group_id);
        group_id
    }
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
    GroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
{
    pub(super) fn find_group(&self, hash: u64, key: &GK) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
    }
}
