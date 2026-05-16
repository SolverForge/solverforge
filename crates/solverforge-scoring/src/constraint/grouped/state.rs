use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;

struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub trait GroupedStateView<K, R> {
    fn for_each_group_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(&K, &R);

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &K,
        present: Present,
        missing: Missing,
    ) -> T
    where
        Present: FnOnce(&R) -> T,
        Missing: FnOnce() -> T;

    fn group_count(&self) -> usize;
}

pub struct GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    extractor: E,
    filter: Fi,
    key_fn: KF,
    collector: C,
    change_source: ChangeSource,
    groups: HashMap<K, GroupState<Acc>>,
    entity_groups: HashMap<usize, K>,
    entity_retractions: HashMap<usize, CollectorRetraction<Acc, V, R>>,
    changed_keys: Vec<K>,
    update_count: usize,
    changed_key_count: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> V, fn() -> R, fn() -> Acc)>,
}

pub struct GroupedEvaluationState<K, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    groups: HashMap<K, Acc>,
    _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc> GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
{
    pub fn new(extractor: E, filter: Fi, key_fn: KF, collector: C) -> Self {
        let change_source = extractor.change_source();
        Self {
            extractor,
            filter,
            key_fn,
            collector,
            change_source,
            groups: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_retractions: HashMap::new(),
            changed_keys: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub fn evaluation_state(&self, solution: &S) -> GroupedEvaluationState<K, V, R, Acc> {
        let entities = self.extractor.extract(solution);
        let mut groups: HashMap<K, Acc> = HashMap::new();

        for entity in entities {
            if !self.filter.test(solution, entity) {
                continue;
            }
            let key = (self.key_fn)(entity);
            let value = self.collector.extract(entity);
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        }

        GroupedEvaluationState {
            groups,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let entities = self.extractor.extract(solution);
        for (idx, entity) in entities.iter().enumerate() {
            if self.filter.test(solution, entity) {
                self.insert_entity(idx, entity);
            }
        }
        self.changed_keys.clear();
    }

    pub fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        constraint_name: &str,
    ) {
        self.changed_keys.clear();
        if !self
            .change_source
            .assert_localizes(descriptor_index, constraint_name)
        {
            return;
        }

        let entities = self.extractor.extract(solution);
        let Some(entity) = entities.get(entity_index) else {
            return;
        };
        if self.filter.test(solution, entity) {
            self.insert_entity(entity_index, entity);
            self.update_count += 1;
        }
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn on_retract(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        constraint_name: &str,
    ) {
        self.changed_keys.clear();
        if !self
            .change_source
            .assert_localizes(descriptor_index, constraint_name)
        {
            return;
        }

        let entities = self.extractor.extract(solution);
        self.retract_entity(entities, entity_index);
        self.update_count += 1;
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn reset(&mut self) {
        self.groups.clear();
        self.entity_groups.clear();
        self.entity_retractions.clear();
        self.changed_keys.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    pub fn take_changed_keys(&mut self) -> Vec<K> {
        std::mem::take(&mut self.changed_keys)
    }

    fn mark_changed(&mut self, key: K) {
        if !self.changed_keys.contains(&key) {
            self.changed_keys.push(key);
        }
    }

    fn insert_entity(&mut self, entity_index: usize, entity: &A) {
        let key = (self.key_fn)(entity);
        let entity_key = key.clone();
        let value = self.collector.extract(entity);
        let retraction = match self.groups.entry(key) {
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
        };
        self.entity_groups.insert(entity_index, entity_key.clone());
        self.entity_retractions.insert(entity_index, retraction);
        self.mark_changed(entity_key);
    }

    fn retract_entity(&mut self, _entities: &[A], entity_index: usize) {
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return;
        };
        let Some(retraction) = self.entity_retractions.remove(&entity_index) else {
            return;
        };

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

impl<K, V, R, Acc> GroupedStateView<K, R> for GroupedEvaluationState<K, V, R, Acc>
where
    K: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for (key, accumulator) in &self.groups {
            accumulator.with_result(|result| visit(key, result));
        }
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &K,
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

impl<S, A, K, E, Fi, KF, C, V, R, Acc> GroupedStateView<K, R>
    for GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
    K: Eq + Hash,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for (key, group) in &self.groups {
            group.accumulator.with_result(|result| visit(key, result));
        }
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &K,
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

impl<S, A, K, E, Fi, KF, C, V, R, Acc> std::fmt::Debug
    for GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedNodeState")
            .field("groups", &self.groups.len())
            .field("entity_groups", &self.entity_groups.len())
            .field("update_count", &self.update_count)
            .field("changed_key_count", &self.changed_key_count)
            .finish()
    }
}
