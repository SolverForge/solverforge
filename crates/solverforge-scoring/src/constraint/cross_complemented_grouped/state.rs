use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};

use super::indexes::{key_hash, matching_indexed_indices, remove_index_from_group_bucket};

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

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

pub struct CrossComplementedGroupedNodeState<
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
> where
    Acc: Accumulator<V, R>,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_t: ET,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) key_t: KT,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) t_source: ChangeSource,
    pub(super) matches: HashMap<(usize, usize), usize>,
    pub(super) match_rows: Vec<MatchRow<CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) a_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) b_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) a_index_to_key: HashMap<usize, JK>,
    pub(super) b_index_to_key: HashMap<usize, JK>,
    pub(super) t_by_group: HashMap<usize, Vec<usize>>,
    pub(super) t_index_to_group: HashMap<usize, usize>,
    pub(super) t_defaults: HashMap<usize, R>,
    pub(super) groups: Vec<GroupState<GK, Acc>>,
    pub(super) groups_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) changed_groups: Vec<usize>,
    pub(super) changed_complements: Vec<usize>,
    update_count: usize,
    changed_key_count: usize,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> T,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
    )>,
}

pub struct CrossComplementedGroupedEvaluationState<GK, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) groups: HashMap<GK, Acc>,
    pub(super) targets: Vec<(GK, R)>,
    pub(super) _phantom: PhantomData<fn() -> V>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    CrossComplementedGroupedNodeState<
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
    >
where
    S: Send + Sync + 'static,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    T: Send + Sync + 'static,
    JK: Eq + Hash + Send + Sync,
    GK: Eq + Hash + Send + Sync,
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
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        extractor_t: ET,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        key_t: KT,
        collector: C,
        default_fn: D,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        let t_source = extractor_t.change_source();
        Self {
            extractor_a,
            extractor_b,
            extractor_t,
            key_a,
            key_b,
            filter,
            group_key_fn,
            key_t,
            collector,
            default_fn,
            a_source,
            b_source,
            t_source,
            matches: HashMap::new(),
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            a_by_hash: HashMap::new(),
            b_by_hash: HashMap::new(),
            a_index_to_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            t_by_group: HashMap::new(),
            t_index_to_group: HashMap::new(),
            t_defaults: HashMap::new(),
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            changed_groups: Vec::new(),
            changed_complements: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub fn evaluation_state(
        &self,
        solution: &S,
    ) -> CrossComplementedGroupedEvaluationState<GK, V, R, Acc> {
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

        let mut targets = Vec::new();
        for target in entities_t {
            if self.extractor_t.contains(solution, target) {
                targets.push(((self.key_t)(target), (self.default_fn)(target)));
            }
        }

        CrossComplementedGroupedEvaluationState {
            groups,
            targets,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_t = self.extractor_t.extract(solution);
        self.build_join_indexes(solution, entities_a, entities_b);

        for t_idx in 0..entities_t.len() {
            self.insert_complement(solution, entities_t, t_idx);
        }
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
        self.changed_complements.clear();
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

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_t = self.extractor_t.extract(solution);
        if a_changed {
            self.insert_a(solution, entities_a, entities_b, entity_index);
        }
        if b_changed {
            self.insert_b(solution, entities_a, entities_b, entity_index);
        }
        if t_changed {
            self.insert_complement(solution, entities_t, entity_index);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn on_retract(&mut self, entity_index: usize, descriptor_index: usize, node_name: &str) {
        self.changed_groups.clear();
        self.changed_complements.clear();
        let a_changed = self.a_source.assert_localizes(descriptor_index, node_name);
        let b_changed = self.b_source.assert_localizes(descriptor_index, node_name);
        let t_changed = self.t_source.assert_localizes(descriptor_index, node_name);
        if !a_changed && !b_changed && !t_changed {
            return;
        }

        if a_changed {
            self.retract_a(entity_index);
        }
        if b_changed {
            self.retract_b(entity_index);
        }
        if t_changed {
            self.retract_complement(entity_index);
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

    pub(super) fn mark_changed(&mut self, group_id: usize) {
        if !self.changed_groups.contains(&group_id) {
            self.changed_groups.push(group_id);
        }
    }

    pub(super) fn mark_complement_changed(&mut self, t_idx: usize) {
        if !self.changed_complements.contains(&t_idx) {
            self.changed_complements.push(t_idx);
        }
    }

    pub(super) fn b_index_for(&self, solution: &S, entities_b: &[B]) -> HashMap<JK, Vec<usize>> {
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

    pub(super) fn build_join_indexes(&mut self, solution: &S, entities_a: &[A], entities_b: &[B]) {
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

    #[inline]
    pub(super) fn matching_b_indices_in<'a>(
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

    pub(super) fn index_complement(&mut self, group_id: usize, t_idx: usize) {
        if let Some(old_group_id) = self.t_index_to_group.insert(t_idx, group_id) {
            remove_index_from_group_bucket(&mut self.t_by_group, old_group_id, t_idx);
            self.mark_changed(old_group_id);
        }
        self.t_by_group.entry(group_id).or_default().push(t_idx);
        self.mark_complement_changed(t_idx);
        self.mark_changed(group_id);
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

    pub(super) fn group_id_for_key(&mut self, key: GK) -> usize {
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
