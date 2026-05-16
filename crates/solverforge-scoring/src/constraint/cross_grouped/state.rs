use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::constraint::grouped::GroupedStateView;
use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};

pub(super) type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<Acc> {
    pub(super) accumulator: Acc,
    pub(super) count: usize,
}

pub(super) struct MatchRow<GK, Retraction> {
    pub(super) pair: (usize, usize),
    pub(super) group_key: GK,
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
    pub(super) match_rows: Vec<MatchRow<GK, CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) a_by_key: HashMap<JK, Vec<usize>>,
    pub(super) b_by_key: HashMap<JK, Vec<usize>>,
    pub(super) a_index_to_key: HashMap<usize, JK>,
    pub(super) b_index_to_key: HashMap<usize, JK>,
    pub(super) groups: HashMap<GK, GroupState<Acc>>,
    pub(super) changed_keys: Vec<GK>,
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
    groups: HashMap<GK, Acc>,
    _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
    CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
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
            a_by_key: HashMap::new(),
            b_by_key: HashMap::new(),
            a_index_to_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            groups: HashMap::new(),
            changed_keys: Vec::new(),
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
            let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();
            for b_idx in b_indices {
                self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
            }
        }
        self.changed_keys.clear();
    }

    pub fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        node_name: &str,
    ) {
        self.changed_keys.clear();
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
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn on_retract(&mut self, entity_index: usize, descriptor_index: usize, node_name: &str) {
        self.changed_keys.clear();
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
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn reset(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.a_by_key.clear();
        self.b_by_key.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
        self.groups.clear();
        self.changed_keys.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    pub fn take_changed_keys(&mut self) -> Vec<GK> {
        std::mem::take(&mut self.changed_keys)
    }

    pub(super) fn mark_changed(&mut self, key: GK) {
        if !self.changed_keys.contains(&key) {
            self.changed_keys.push(key);
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
        self.a_by_key.clear();
        self.b_by_key.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            self.a_index_to_key.insert(a_idx, key.clone());
            self.a_by_key.entry(key).or_default().push(a_idx);
        }
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            self.b_index_to_key.insert(b_idx, key.clone());
            self.b_by_key.entry(key).or_default().push(b_idx);
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

    pub(super) fn insert_value(&mut self, key: GK, value: V) -> CollectorRetraction<Acc, V, R> {
        match self.groups.entry(key) {
            Entry::Occupied(mut entry) => {
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
            Entry::Vacant(entry) => {
                let group = entry.insert(GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
        }
    }

    pub(super) fn retract_value(&mut self, key: GK, retraction: CollectorRetraction<Acc, V, R>) {
        let Entry::Occupied(mut entry) = self.groups.entry(key.clone()) else {
            return;
        };
        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        if group.count == 0 {
            entry.remove();
        }
        self.mark_changed(key);
    }
}

impl<GK, V, R, Acc> GroupedStateView<GK, R> for CrossGroupedEvaluationState<GK, V, R, Acc>
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for (key, accumulator) in &self.groups {
            accumulator.with_result(|result| visit(key, result));
        }
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &GK,
        present: Present,
        missing: Missing,
    ) -> T
    where
        Present: FnOnce(&R) -> T,
        Missing: FnOnce() -> T,
    {
        match self.groups.get(key) {
            Some(accumulator) => accumulator.with_result(present),
            None => missing(),
        }
    }

    fn group_count(&self) -> usize {
        self.groups.len()
    }
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc> GroupedStateView<GK, R>
    for CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for (key, group) in &self.groups {
            group.accumulator.with_result(|result| visit(key, result));
        }
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &GK,
        present: Present,
        missing: Missing,
    ) -> T
    where
        Present: FnOnce(&R) -> T,
        Missing: FnOnce() -> T,
    {
        match self.groups.get(key) {
            Some(group) => group.accumulator.with_result(present),
            None => missing(),
        }
    }

    fn group_count(&self) -> usize {
        self.groups.len()
    }
}
