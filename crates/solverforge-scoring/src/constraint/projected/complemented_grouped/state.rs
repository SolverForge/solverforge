use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

pub struct ProjectedComplementedGroupedConstraint<
    S,
    Out,
    B,
    K,
    Src,
    EB,
    F,
    KA,
    KB,
    C,
    V,
    R,
    Acc,
    D,
    W,
    Sc,
> where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) source: Src,
    pub(super) extractor_b: EB,
    pub(super) filter: F,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) b_source: ChangeSource,
    pub(super) source_state: Option<Src::State>,
    pub(super) groups: HashMap<K, GroupState<Acc>>,
    pub(super) row_outputs: HashMap<ProjectedRowCoordinate, Out>,
    pub(super) row_keys: HashMap<ProjectedRowCoordinate, K>,
    pub(super) row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    pub(super) rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    pub(super) b_by_key: HashMap<K, Vec<usize>>,
    pub(super) b_index_to_key: HashMap<usize, K>,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> B,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
    ProjectedComplementedGroupedConstraint<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
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
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        source: Src,
        extractor_b: EB,
        filter: F,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let b_source = extractor_b.change_source();
        Self {
            constraint_ref,
            impact_type,
            source,
            extractor_b,
            filter,
            key_a,
            key_b,
            collector,
            default_fn,
            weight_fn,
            is_hard,
            b_source,
            source_state: None,
            groups: HashMap::new(),
            row_outputs: HashMap::new(),
            row_keys: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            b_by_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    pub(super) fn compute_score(&self, key: &K, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }
    fn b_score_for_index(&self, entities_b: &[B], key: &K, b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        if let Some(group) = self.groups.get(key) {
            return group
                .accumulator
                .with_result(|result| self.compute_score(key, result));
        }
        let default_result = (self.default_fn)(&entities_b[b_idx]);
        self.compute_score(key, &default_result)
    }

    fn key_score(&self, entities_b: &[B], key: &K) -> Sc {
        let Some(indices) = self.b_by_key.get(key) else {
            return Sc::zero();
        };
        indices.iter().fold(Sc::zero(), |total, &b_idx| {
            total + self.b_score_for_index(entities_b, key, b_idx)
        })
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
        }
        self.b_by_key.entry(key).or_default().push(b_idx);
    }

    fn insert_value(
        &mut self,
        entities_b: &[B],
        key: K,
        value: V,
    ) -> (Sc, CollectorRetraction<Acc, V, R>) {
        let old = self.key_score(entities_b, &key);
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
        let new_score = self.key_score(entities_b, &key);
        (new_score - old, retraction)
    }
    fn retract_value(
        &mut self,
        entities_b: &[B],
        key: K,
        retraction: CollectorRetraction<Acc, V, R>,
    ) -> Sc {
        let old = self.key_score(entities_b, &key);
        let Entry::Occupied(mut entry) = self.groups.entry(key.clone()) else {
            return Sc::zero();
        };
        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        if group.count == 0 {
            entry.remove();
        }
        let new_score = self.key_score(entities_b, &key);
        new_score - old
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
        entities_b: &[B],
        coordinate: ProjectedRowCoordinate,
        output: Out,
    ) -> Sc {
        if self.row_outputs.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return Sc::zero();
        }
        let Some(key) = (self.key_a)(&output) else {
            return Sc::zero();
        };
        let value = self.collector.extract(&output);
        let (delta, retraction) = self.insert_value(entities_b, key.clone(), value);
        self.row_outputs.insert(coordinate, output);
        self.row_keys.insert(coordinate, key);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
        delta
    }

    pub(super) fn retract_row(
        &mut self,
        entities_b: &[B],
        coordinate: ProjectedRowCoordinate,
    ) -> Sc {
        let Some(_output) = self.row_outputs.remove(&coordinate) else {
            return Sc::zero();
        };
        self.unindex_coordinate(coordinate);
        let Some(key) = self.row_keys.remove(&coordinate) else {
            return Sc::zero();
        };
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return Sc::zero();
        };
        self.retract_value(entities_b, key, retraction)
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
                .assert_localizes(descriptor_index, &self.constraint_ref.name)
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

    pub(super) fn insert_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let key = (self.key_b)(&entities_b[b_idx]);
        self.index_b(key.clone(), b_idx);
        self.b_score_for_index(entities_b, &key, b_idx)
    }

    pub(super) fn retract_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        let Some(key) = self.b_index_to_key.remove(&b_idx) else {
            return Sc::zero();
        };
        let delta = -self.b_score_for_index(entities_b, &key, b_idx);
        Self::remove_index_from_key_bucket(&mut self.b_by_key, &key, b_idx);
        delta
    }
}
