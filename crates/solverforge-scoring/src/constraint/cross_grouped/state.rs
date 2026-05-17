use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};

use super::indexes::{key_hash, matching_indexed_indices};

pub(super) type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<K, Acc> {
    pub(super) key: K,
    pub(super) accumulator: Acc,
    pub(super) count: usize,
}

pub(super) struct MatchRow<Retraction> {
    pub(super) pair: (usize, usize),
    pub(super) group_id: usize,
    pub(super) retraction: Retraction,
    pub(super) a_pos: usize,
    pub(super) b_pos: usize,
}

pub struct CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) collector: C,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) matches: HashMap<(usize, usize), usize>,
    pub(super) match_rows: Vec<MatchRow<CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) a_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) b_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) a_index_to_key: HashMap<usize, JK>,
    pub(super) b_index_to_key: HashMap<usize, JK>,
    pub(super) groups: Vec<GroupState<GK, Acc>>,
    pub(super) groups_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) changed_groups: Vec<usize>,
    pub(super) update_count: usize,
    pub(super) changed_key_count: usize,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
    )>,
}

pub struct CrossGroupedEvaluationState<GK, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) groups: HashMap<GK, Acc>,
    pub(super) _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
    CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    S: Send + Sync + 'static,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    JK: Eq + Hash + Send + Sync,
    GK: Eq + Hash + Send + Sync,
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
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            group_key_fn,
            collector,
            a_source,
            b_source,
            matches: HashMap::new(),
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            a_by_hash: HashMap::new(),
            b_by_hash: HashMap::new(),
            a_index_to_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            changed_groups: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub fn evaluation_state(&self, solution: &S) -> CrossGroupedEvaluationState<GK, V, R, Acc> {
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

        CrossGroupedEvaluationState {
            groups,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        self.build_indexes(solution, entities_a, entities_b);

        for a_idx in 0..entities_a.len() {
            if !self.extractor_a.contains(solution, &entities_a[a_idx]) {
                continue;
            }
            let key = (self.key_a)(&entities_a[a_idx]);
            let b_indices = self.matching_indexed_b_indices(&key);
            for b_idx in b_indices {
                self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
            }
        }
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

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        if a_changed {
            self.insert_a(solution, entities_a, entities_b, entity_index);
        }
        if b_changed {
            self.insert_b(solution, entities_a, entities_b, entity_index);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn on_retract(&mut self, entity_index: usize, descriptor_index: usize, node_name: &str) {
        self.changed_groups.clear();
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed {
            return;
        }

        if a_changed {
            self.retract_a(entity_index);
        }
        if b_changed {
            self.retract_b(entity_index);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn reset(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.a_by_hash.clear();
        self.b_by_hash.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
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

    fn b_index_for(&self, solution: &S, entities_b: &[B]) -> HashMap<JK, Vec<usize>> {
        let mut b_by_key = HashMap::<JK, Vec<usize>>::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            b_by_key.entry(key).or_default().push(b_idx);
        }
        b_by_key
    }

    fn build_indexes(&mut self, solution: &S, entities_a: &[A], entities_b: &[B]) {
        self.a_by_hash.clear();
        self.b_by_hash.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            let hash = key_hash(&key);
            self.a_by_hash.entry(hash).or_default().push(a_idx);
            self.a_index_to_key.insert(a_idx, key);
        }
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            let hash = key_hash(&key);
            self.b_by_hash.entry(hash).or_default().push(b_idx);
            self.b_index_to_key.insert(b_idx, key);
        }
    }

    fn matching_b_indices_in<'a>(
        &self,
        b_by_key: &'a HashMap<JK, Vec<usize>>,
        a: &A,
    ) -> &'a [usize] {
        let key = (self.key_a)(a);
        b_by_key.get(&key).map_or(&[], Vec::as_slice)
    }

    pub(super) fn matching_indexed_a_indices(&self, key: &JK) -> Vec<usize> {
        matching_indexed_indices(&self.a_by_hash, &self.a_index_to_key, key)
    }

    pub(super) fn matching_indexed_b_indices(&self, key: &JK) -> Vec<usize> {
        matching_indexed_indices(&self.b_by_hash, &self.b_index_to_key, key)
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
    CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
    GK: Eq + Hash,
{
    pub(super) fn find_group(&self, hash: u64, key: &GK) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
    }
}
