use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use crate::constraint::grouped::GroupedStateView;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

struct GroupState<K, Acc> {
    key: K,
    accumulator: Acc,
    count: usize,
}

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub struct ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
{
    source: Src,
    filter: F,
    key_fn: KF,
    collector: C,
    source_state: Option<Src::State>,
    groups: Vec<GroupState<K, Acc>>,
    groups_by_hash: HashMap<u64, Vec<usize>>,
    row_groups: HashMap<ProjectedRowCoordinate, usize>,
    row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    changed_groups: Vec<usize>,
    update_count: usize,
    changed_key_count: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> V, fn() -> R, fn() -> Acc)>,
}

pub struct ProjectedGroupedEvaluationState<K, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    groups: HashMap<K, Acc>,
    _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc>
    ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
{
    pub fn new(source: Src, filter: F, key_fn: KF, collector: C) -> Self {
        Self {
            source,
            filter,
            key_fn,
            collector,
            source_state: None,
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            row_groups: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            changed_groups: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub fn evaluation_state(&self, solution: &S) -> ProjectedGroupedEvaluationState<K, V, R, Acc> {
        let state = self.source.build_state(solution);
        let mut groups = HashMap::<K, Acc>::new();
        self.source.collect_all(solution, &state, |_, output| {
            if !self.filter.test(solution, &output) {
                return;
            }
            let key = (self.key_fn)(&output);
            let value = self.collector.extract(&output);
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        });
        ProjectedGroupedEvaluationState {
            groups,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                rows.push((coordinate, output));
            });
        self.source_state = Some(state);
        for (coordinate, output) in rows {
            self.insert_row(solution, coordinate, output);
        }
        self.changed_groups.clear();
    }

    pub fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_groups.clear();
        let owners = self.localized_owners(descriptor_index, entity_index);
        if owners.is_empty() {
            return;
        }
        self.ensure_source_state(solution);
        {
            let state = self.source_state.as_mut().expect("projected source state");
            for owner in &owners {
                self.source.insert_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }

        let mut rows = Vec::new();
        let state = self.source_state.as_ref().expect("projected source state");
        for owner in &owners {
            self.source.collect_entity(
                solution,
                state,
                owner.source_slot,
                owner.entity_index,
                |coordinate, output| rows.push((coordinate, output)),
            );
        }
        for (coordinate, output) in rows {
            self.insert_row(solution, coordinate, output);
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_groups.clear();
        let owners = self.localized_owners(descriptor_index, entity_index);
        if owners.is_empty() {
            return;
        }
        for coordinate in self.coordinates_for_owners(&owners) {
            self.retract_row(coordinate);
        }
        if let Some(state) = self.source_state.as_mut() {
            for owner in &owners {
                self.source.retract_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }
        self.update_count += 1;
        self.changed_key_count += self.changed_groups.len();
    }

    pub fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.groups_by_hash.clear();
        self.row_groups.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
        self.changed_groups.clear();
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    fn ensure_source_state(&mut self, solution: &S) {
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
    }

    fn mark_changed(&mut self, group_id: usize) {
        if !self.changed_groups.contains(&group_id) {
            self.changed_groups.push(group_id);
        }
    }

    fn insert_value(&mut self, key: K, value: V) -> (usize, CollectorRetraction<Acc, V, R>) {
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

    fn retract_value(&mut self, group_id: usize, retraction: CollectorRetraction<Acc, V, R>) {
        let Some(group) = self.groups.get_mut(group_id) else {
            return;
        };
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        self.mark_changed(group_id);
    }

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) {
        if self.row_groups.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return;
        }
        let key = (self.key_fn)(&output);
        let value = self.collector.extract(&output);
        let (group_id, retraction) = self.insert_value(key, value);
        self.row_groups.insert(coordinate, group_id);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
    }

    fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) {
        let Some(group_id) = self.row_groups.remove(&coordinate) else {
            return;
        };
        self.unindex_coordinate(coordinate);
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return;
        };
        self.retract_value(group_id, retraction);
    }

    fn index_coordinate(&mut self, coordinate: ProjectedRowCoordinate) {
        coordinate.for_each_owner(|owner| {
            self.rows_by_owner
                .entry(owner)
                .or_default()
                .push(coordinate);
        });
    }

    fn unindex_coordinate(&mut self, coordinate: ProjectedRowCoordinate) {
        coordinate.for_each_owner(|owner| {
            let mut remove_bucket = false;
            if let Some(rows) = self.rows_by_owner.get_mut(&owner) {
                rows.retain(|candidate| *candidate != coordinate);
                remove_bucket = rows.is_empty();
            }
            if remove_bucket {
                self.rows_by_owner.remove(&owner);
            }
        });
    }

    fn localized_owners(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Vec<ProjectedRowOwner> {
        let mut owners = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, "sharedProjectedGroupedNode")
            {
                owners.push(ProjectedRowOwner {
                    source_slot: slot,
                    entity_index,
                });
            }
        }
        owners
    }

    fn coordinates_for_owners(&self, owners: &[ProjectedRowOwner]) -> Vec<ProjectedRowCoordinate> {
        let mut seen = HashSet::new();
        let mut coordinates = Vec::new();
        for owner in owners {
            let Some(rows) = self.rows_by_owner.get(owner) else {
                continue;
            };
            for &coordinate in rows {
                if seen.insert(coordinate) {
                    coordinates.push(coordinate);
                }
            }
        }
        coordinates
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

impl<S, Out, K, Src, F, KF, C, V, R, Acc>
    ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>
where
    Src: ProjectedSource<S, Out>,
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

impl<K, V, R, Acc> GroupedStateView<K, R> for ProjectedGroupedEvaluationState<K, V, R, Acc>
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

impl<S, Out, K, Src, F, KF, C, V, R, Acc> GroupedStateView<K, R>
    for ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>
where
    K: Eq + Hash,
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for group in &self.groups {
            if group.count > 0 {
                group
                    .accumulator
                    .with_result(|result| visit(&group.key, result));
            }
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
        let hash = key_hash(key);
        let Some(group_id) = self.find_group(hash, key) else {
            return missing();
        };
        let group = &self.groups[group_id];
        if group.count == 0 {
            return missing();
        }
        group.accumulator.with_result(present)
    }

    fn group_count(&self) -> usize {
        self.groups.iter().filter(|group| group.count > 0).count()
    }
}

fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
