use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;

struct GroupState<K, Acc> {
    key: K,
    accumulator: Acc,
    count: usize,
}

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub trait GroupedStateView<K, R> {
    fn for_each_group_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(&K, &R);

    fn for_each_group_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>);

    fn for_each_changed_group_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>);

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
    groups: Vec<GroupState<K, Acc>>,
    groups_by_hash: HashMap<u64, Vec<usize>>,
    entity_groups: HashMap<usize, usize>,
    entity_retractions: HashMap<usize, CollectorRetraction<Acc, V, R>>,
    changed_groups: Vec<usize>,
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
    A: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
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
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_retractions: HashMap::new(),
            changed_groups: Vec::new(),
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
        self.changed_groups.clear();
    }

    pub fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        constraint_name: &str,
    ) {
        self.changed_groups.clear();
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
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn on_retract(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
        constraint_name: &str,
    ) {
        self.changed_groups.clear();
        if !self
            .change_source
            .assert_localizes(descriptor_index, constraint_name)
        {
            return;
        }

        let entities = self.extractor.extract(solution);
        self.retract_entity(entities, entity_index);
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn reset(&mut self) {
        self.groups.clear();
        self.groups_by_hash.clear();
        self.entity_groups.clear();
        self.entity_retractions.clear();
        self.changed_groups.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    fn mark_changed(&mut self, group_id: usize) {
        if !self.changed_groups.contains(&group_id) {
            self.changed_groups.push(group_id);
        }
    }

    fn insert_entity(&mut self, entity_index: usize, entity: &A) {
        let key = (self.key_fn)(entity);
        let group_id = self.group_id_for_key(key);
        let group = &mut self.groups[group_id];
        if group.count == 0 {
            group.accumulator.reset();
        }
        let value = self.collector.extract(entity);
        let retraction = group.accumulator.accumulate(value);
        group.count += 1;
        self.entity_groups.insert(entity_index, group_id);
        self.entity_retractions.insert(entity_index, retraction);
        self.mark_changed(group_id);
    }

    fn retract_entity(&mut self, _entities: &[A], entity_index: usize) {
        let Some(group_id) = self.entity_groups.remove(&entity_index) else {
            return;
        };
        let Some(retraction) = self.entity_retractions.remove(&entity_index) else {
            return;
        };

        let group = &mut self.groups[group_id];
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        self.mark_changed(group_id);
    }

    fn group_id_for_key(&mut self, key: K) -> usize {
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

impl<S, A, K, E, Fi, KF, C, V, R, Acc> GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
    K: Eq + Hash,
{
    fn find_group(&self, hash: u64, key: &K) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
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

    fn for_each_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        for (group_id, (key, accumulator)) in self.groups.iter().enumerate() {
            accumulator.with_result(|result| visit(group_id, Some((key, result))));
        }
    }

    fn for_each_changed_group_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        self.for_each_group_slot_result(visit);
    }

    fn group_count(&self) -> usize {
        self.groups.len()
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
        for group in &self.groups {
            if group.count == 0 {
                continue;
            }
            group
                .accumulator
                .with_result(|result| visit(&group.key, result));
        }
    }

    fn for_each_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        for (group_id, group) in self.groups.iter().enumerate() {
            if group.count == 0 {
                visit(group_id, None);
                continue;
            }
            group
                .accumulator
                .with_result(|result| visit(group_id, Some((&group.key, result))));
        }
    }

    fn for_each_changed_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        for &group_id in &self.changed_groups {
            let Some(group) = self.groups.get(group_id) else {
                continue;
            };
            if group.count == 0 {
                visit(group_id, None);
                continue;
            }
            group
                .accumulator
                .with_result(|result| visit(group_id, Some((&group.key, result))));
        }
    }

    fn group_count(&self) -> usize {
        self.groups.iter().filter(|group| group.count > 0).count()
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
        let Some(group_id) = self.find_group(key_hash(key), key) else {
            return missing();
        };
        let group = &self.groups[group_id];
        if group.count == 0 {
            return missing();
        }
        group.accumulator.with_result(present)
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc> std::fmt::Debug
    for GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedNodeState")
            .field(
                "groups",
                &self.groups.iter().filter(|group| group.count > 0).count(),
            )
            .field("entity_groups", &self.entity_groups.len())
            .field("update_count", &self.update_count)
            .field("changed_key_count", &self.changed_key_count)
            .finish()
    }
}

fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
