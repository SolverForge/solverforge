use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::constraint::grouped::GroupedStateView;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

struct GroupState<Acc> {
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
    groups: HashMap<K, GroupState<Acc>>,
    row_outputs: HashMap<ProjectedRowCoordinate, Out>,
    row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    changed_keys: Vec<K>,
    update_count: usize,
    changed_key_count: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V, fn() -> R, fn() -> Acc)>,
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
    K: Clone + Eq + Hash + Send + Sync + 'static,
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
            groups: HashMap::new(),
            row_outputs: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            changed_keys: Vec::new(),
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
        self.changed_keys.clear();
    }

    pub fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_keys.clear();
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
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_keys.clear();
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
        self.changed_key_count += self.changed_keys.len();
    }

    pub fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.row_outputs.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
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

    fn ensure_source_state(&mut self, solution: &S) {
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
    }

    fn mark_changed(&mut self, key: K) {
        if !self.changed_keys.contains(&key) {
            self.changed_keys.push(key);
        }
    }

    fn insert_value(&mut self, key: K, value: V) -> CollectorRetraction<Acc, V, R> {
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

    fn retract_value(&mut self, key: K, retraction: CollectorRetraction<Acc, V, R>) {
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

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) {
        if self.row_outputs.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return;
        }
        let key = (self.key_fn)(&output);
        let value = self.collector.extract(&output);
        let retraction = self.insert_value(key.clone(), value);
        self.row_outputs.insert(coordinate, output);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
        self.mark_changed(key);
    }

    fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) {
        let Some(output) = self.row_outputs.remove(&coordinate) else {
            return;
        };
        self.unindex_coordinate(coordinate);
        let key = (self.key_fn)(&output);
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return;
        };
        self.retract_value(key, retraction);
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
