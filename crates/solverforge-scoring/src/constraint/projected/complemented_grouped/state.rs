use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

use super::indexes::key_hash;

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<K, Acc> {
    pub(super) key: K,
    pub(super) accumulator: Acc,
    pub(super) count: usize,
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
    pub(super) groups: Vec<GroupState<K, Acc>>,
    pub(super) groups_by_hash: HashMap<u64, Vec<usize>>,
    row_groups: HashMap<ProjectedRowCoordinate, usize>,
    row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    pub(super) complement_groups: HashMap<usize, usize>,
    pub(super) complements_by_group: HashMap<usize, Vec<usize>>,
    pub(super) complement_defaults: HashMap<usize, R>,
    pub(super) changed_groups: Vec<usize>,
    pub(super) changed_complements: Vec<usize>,
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

pub struct ProjectedComplementedGroupedEvaluationState<K, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) groups: HashMap<K, Acc>,
    pub(super) complements: Vec<(K, R)>,
    pub(super) _phantom: PhantomData<fn() -> V>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
    ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
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
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            row_groups: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            complement_groups: HashMap::new(),
            complements_by_group: HashMap::new(),
            complement_defaults: HashMap::new(),
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
    ) -> ProjectedComplementedGroupedEvaluationState<K, V, R, Acc> {
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

        let mut complements = Vec::new();
        for entity in entities_b {
            if self.extractor_b.contains(solution, entity) {
                complements.push(((self.key_b)(entity), (self.default_fn)(entity)));
            }
        }
        ProjectedComplementedGroupedEvaluationState {
            groups,
            complements,
            _phantom: PhantomData,
        }
    }

    pub fn initialize(&mut self, solution: &S) {
        self.reset();
        let entities_b = self.extractor_b.extract(solution);
        for idx in 0..entities_b.len() {
            self.insert_b(solution, entities_b, idx);
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
        self.changed_groups.clear();
        self.changed_complements.clear();
    }

    pub fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_groups.clear();
        self.changed_complements.clear();
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
            self.insert_b(solution, entities_b, entity_index);
        }
        if !owners.is_empty() || b_changed {
            self.update_count += 1;
            self.changed_key_count += self.changed_groups.len();
        }
    }

    pub fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) {
        self.changed_groups.clear();
        self.changed_complements.clear();
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
            self.changed_key_count += self.changed_groups.len();
        }
    }

    pub fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.groups_by_hash.clear();
        self.row_groups.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
        self.complement_groups.clear();
        self.complements_by_group.clear();
        self.complement_defaults.clear();
        self.changed_groups.clear();
        self.changed_complements.clear();
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

    fn mark_complement_changed(&mut self, b_idx: usize) {
        if !self.changed_complements.contains(&b_idx) {
            self.changed_complements.push(b_idx);
        }
    }

    fn remove_complement_from_group(&mut self, group_id: usize, b_idx: usize) {
        let mut remove_bucket = false;
        if let Some(indices) = self.complements_by_group.get_mut(&group_id) {
            if let Some(pos) = indices.iter().position(|candidate| *candidate == b_idx) {
                indices.swap_remove(pos);
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            self.complements_by_group.remove(&group_id);
        }
    }

    fn index_b(&mut self, group_id: usize, b_idx: usize) {
        if let Some(old_group_id) = self.complement_groups.insert(b_idx, group_id) {
            self.remove_complement_from_group(old_group_id, b_idx);
            self.mark_changed(old_group_id);
        }
        self.complements_by_group
            .entry(group_id)
            .or_default()
            .push(b_idx);
        self.mark_complement_changed(b_idx);
        self.mark_changed(group_id);
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
        if self.row_groups.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return;
        }
        let Some(key) = (self.key_a)(&output) else {
            return;
        };
        let value = self.collector.extract(&output);
        let (group_id, retraction) = self.insert_value(key, value);
        self.row_groups.insert(coordinate, group_id);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
    }

    pub(super) fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) {
        let Some(group_id) = self.row_groups.remove(&coordinate) else {
            return;
        };
        self.unindex_coordinate(coordinate);
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return;
        };
        self.retract_value(group_id, retraction);
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

    pub(super) fn insert_b(&mut self, solution: &S, entities_b: &[B], b_idx: usize) {
        if b_idx >= entities_b.len() {
            return;
        }
        let entity = &entities_b[b_idx];
        if !self.extractor_b.contains(solution, entity) {
            return;
        }
        let key = (self.key_b)(entity);
        let default_result = (self.default_fn)(entity);
        let group_id = self.group_id_for_key(key);
        self.complement_defaults.insert(b_idx, default_result);
        self.index_b(group_id, b_idx);
    }

    pub(super) fn retract_b(&mut self, b_idx: usize) {
        let Some(group_id) = self.complement_groups.remove(&b_idx) else {
            return;
        };
        self.complement_defaults.remove(&b_idx);
        self.remove_complement_from_group(group_id, b_idx);
        self.mark_complement_changed(b_idx);
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
