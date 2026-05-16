use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::constraint::grouped::ComplementedGroupedStateView;
use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

pub struct ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
{
    pub(super) source: Src,
    pub(super) extractor_b: EB,
    pub(super) filter: F,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) b_source: ChangeSource,
    pub(super) source_state: Option<Src::State>,
    pub(super) groups: HashMap<K, GroupState<Acc>>,
    pub(super) row_outputs: HashMap<ProjectedRowCoordinate, Out>,
    pub(super) row_keys: HashMap<ProjectedRowCoordinate, K>,
    pub(super) row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    pub(super) rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    pub(super) b_by_key: HashMap<K, Vec<usize>>,
    pub(super) b_index_to_key: HashMap<usize, K>,
    pub(super) b_entities: HashMap<usize, B>,
    changed_keys: Vec<K>,
    update_count: usize,
    changed_key_count: usize,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> B,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
    )>,
}

pub struct ProjectedComplementedGroupedEvaluationState<'a, K, B, V, R, Acc, D>
where
    Acc: Accumulator<V, R>,
{
    groups: HashMap<K, Acc>,
    complements: Vec<(K, B)>,
    default_fn: &'a D,
    _phantom: PhantomData<(fn() -> V, fn() -> R)>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
    ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&B) -> R + Send + Sync,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Src,
        extractor_b: EB,
        filter: F,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
    ) -> Self {
        let b_source = extractor_b.change_source();
        Self {
            source,
            extractor_b,
            filter,
            key_a,
            key_b,
            collector,
            default_fn,
            b_source,
            source_state: None,
            groups: HashMap::new(),
            row_outputs: HashMap::new(),
            row_keys: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            b_by_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            b_entities: HashMap::new(),
            changed_keys: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub fn evaluation_state(
        &self,
        solution: &S,
    ) -> ProjectedComplementedGroupedEvaluationState<'_, K, B, V, R, Acc, D> {
        let entities_b = self.extractor_b.extract(solution);
        let state = self.source.build_state(solution);
        let mut groups = HashMap::<K, Acc>::new();
        self.source.collect_all(solution, &state, |_, output| {
            if !self.filter.test(solution, &output) {
                return;
            }
            let Some(key) = (self.key_a)(&output) else {
                return;
            };
            let value = self.collector.extract(&output);
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        });

        let complements = entities_b
            .iter()
            .map(|entity| ((self.key_b)(entity), entity.clone()))
            .collect();
        ProjectedComplementedGroupedEvaluationState {
            groups,
            complements,
            default_fn: &self.default_fn,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let entities_b = self.extractor_b.extract(solution);
        for (idx, b) in entities_b.iter().enumerate() {
            self.insert_b(entities_b, idx);
            self.b_entities.insert(idx, b.clone());
        }

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
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, "projected complemented grouped");
        let entities_b = self.extractor_b.extract(solution);

        if !owners.is_empty() {
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
        }
        if b_changed {
            self.insert_b(entities_b, entity_index);
        }
        if !owners.is_empty() || b_changed {
            self.update_count += 1;
            self.changed_key_count += self.changed_keys.len();
        }
    }

    pub fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_keys.clear();
        let owners = self.localized_owners(descriptor_index, entity_index);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, "projected complemented grouped");

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
        if b_changed {
            self.retract_b(entity_index);
        }
        if !owners.is_empty() || b_changed {
            self.update_count += 1;
            self.changed_key_count += self.changed_keys.len();
        }
    }

    pub fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.row_outputs.clear();
        self.row_keys.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
        self.b_by_key.clear();
        self.b_index_to_key.clear();
        self.b_entities.clear();
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

    fn remove_index_from_key_bucket(
        indexes_by_key: &mut HashMap<K, Vec<usize>>,
        key: &K,
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

    fn index_b(&mut self, key: K, b_idx: usize) {
        if let Some(old_key) = self.b_index_to_key.insert(b_idx, key.clone()) {
            Self::remove_index_from_key_bucket(&mut self.b_by_key, &old_key, b_idx);
            self.mark_changed(old_key);
        }
        self.b_by_key.entry(key.clone()).or_default().push(b_idx);
        self.mark_changed(key);
    }

    fn insert_value(&mut self, key: K, value: V) -> CollectorRetraction<Acc, V, R> {
        let retraction = match self.groups.entry(key.clone()) {
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
        self.mark_changed(key);
        retraction
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

    pub(super) fn ensure_source_state(&mut self, solution: &S) {
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
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

    pub(super) fn insert_row(
        &mut self,
        solution: &S,
        coordinate: ProjectedRowCoordinate,
        output: Out,
    ) {
        if self.row_outputs.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return;
        }
        let Some(key) = (self.key_a)(&output) else {
            return;
        };
        let value = self.collector.extract(&output);
        let retraction = self.insert_value(key.clone(), value);
        self.row_outputs.insert(coordinate, output);
        self.row_keys.insert(coordinate, key);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
    }

    pub(super) fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) {
        let Some(_output) = self.row_outputs.remove(&coordinate) else {
            return;
        };
        self.unindex_coordinate(coordinate);
        let Some(key) = self.row_keys.remove(&coordinate) else {
            return;
        };
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return;
        };
        self.retract_value(key, retraction);
    }

    pub(super) fn localized_owners(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Vec<ProjectedRowOwner> {
        let mut owners = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, "projected complemented grouped")
            {
                owners.push(ProjectedRowOwner {
                    source_slot: slot,
                    entity_index,
                });
            }
        }
        owners
    }

    pub(super) fn coordinates_for_owners(
        &self,
        owners: &[ProjectedRowOwner],
    ) -> Vec<ProjectedRowCoordinate> {
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

    pub(super) fn insert_b(&mut self, entities_b: &[B], b_idx: usize) {
        if b_idx >= entities_b.len() {
            return;
        }
        let entity = entities_b[b_idx].clone();
        let key = (self.key_b)(&entity);
        self.b_entities.insert(b_idx, entity);
        self.index_b(key, b_idx);
    }

    pub(super) fn retract_b(&mut self, b_idx: usize) {
        let Some(key) = self.b_index_to_key.remove(&b_idx) else {
            return;
        };
        self.b_entities.remove(&b_idx);
        Self::remove_index_from_key_bucket(&mut self.b_by_key, &key, b_idx);
        self.mark_changed(key);
    }
}

impl<K, B, V, R, Acc, D> ComplementedGroupedStateView<K, R>
    for ProjectedComplementedGroupedEvaluationState<'_, K, B, V, R, Acc, D>
where
    K: Eq + Hash,
    Acc: Accumulator<V, R>,
    D: Fn(&B) -> R,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for (key, entity) in &self.complements {
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(key, result));
            } else {
                let default_result = (self.default_fn)(entity);
                visit(key, &default_result);
            }
        }
    }

    fn for_each_key_result<Visit>(&self, key: &K, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        for (entity_key, entity) in &self.complements {
            if entity_key != key {
                continue;
            }
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(result));
            } else {
                let default_result = (self.default_fn)(entity);
                visit(&default_result);
            }
        }
    }

    fn complement_count(&self) -> usize {
        self.complements.len()
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D> ComplementedGroupedStateView<K, R>
    for ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    Src: ProjectedSource<S, Out>,
    K: Eq + Hash,
    B: Clone,
    D: Fn(&B) -> R,
    Acc: Accumulator<V, R>,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for key in self.b_by_key.keys() {
            self.for_each_key_result(key, |result| visit(key, result));
        }
    }

    fn for_each_key_result<Visit>(&self, key: &K, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        let Some(indices) = self.b_by_key.get(key) else {
            return;
        };
        for &b_idx in indices {
            if let Some(group) = self.groups.get(key) {
                group.accumulator.with_result(|result| visit(result));
            } else if let Some(entity) = self.b_entities.get(&b_idx) {
                let default_result = (self.default_fn)(entity);
                visit(&default_result);
            }
        }
    }

    fn complement_count(&self) -> usize {
        self.b_index_to_key.len()
    }
}
