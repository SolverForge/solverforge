use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub struct ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    source: Src,
    filter: F,
    key_fn: KF,
    collector: C,
    weight_fn: W,
    is_hard: bool,
    source_state: Option<Src::State>,
    groups: HashMap<K, GroupState<Acc>>,
    row_outputs: HashMap<ProjectedRowCoordinate, Out>,
    row_retractions: HashMap<ProjectedRowCoordinate, CollectorRetraction<Acc, V, R>>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
    ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        source: Src,
        filter: F,
        key_fn: KF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            source,
            filter,
            key_fn,
            collector,
            weight_fn,
            is_hard,
            source_state: None,
            groups: HashMap::new(),
            row_outputs: HashMap::new(),
            row_retractions: HashMap::new(),
            rows_by_owner: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_score(&self, key: &K, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn ensure_source_state(&mut self, solution: &S) {
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

    fn insert_value(&mut self, key: K, value: V) -> (Sc, CollectorRetraction<Acc, V, R>) {
        let impact = self.impact_type;
        let weight_fn = &self.weight_fn;
        match self.groups.entry(key) {
            Entry::Occupied(mut entry) => {
                let old_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let old = match impact {
                    ImpactType::Penalty => -old_base,
                    ImpactType::Reward => old_base,
                };
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let new_score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                (new_score - old, retraction)
            }
            Entry::Vacant(entry) => {
                let mut entry = entry.insert_entry(GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                (score, retraction)
            }
        }
    }

    fn retract_value(&mut self, key: K, retraction: CollectorRetraction<Acc, V, R>) -> Sc {
        let impact = self.impact_type;
        let weight_fn = &self.weight_fn;
        let Entry::Occupied(mut entry) = self.groups.entry(key) else {
            return Sc::zero();
        };
        let old_base = entry
            .get()
            .accumulator
            .with_result(|result| weight_fn(entry.key(), result));
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };
        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        let new_score = if group.count == 0 {
            entry.remove();
            Sc::zero()
        } else {
            let new_base = entry
                .get()
                .accumulator
                .with_result(|result| weight_fn(entry.key(), result));
            match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            }
        };

        new_score - old
    }

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) -> Sc {
        if self.row_outputs.contains_key(&coordinate) || !self.filter.test(solution, &output) {
            return Sc::zero();
        }
        let key = (self.key_fn)(&output);
        let value = self.collector.extract(&output);
        let (delta, retraction) = self.insert_value(key, value);
        self.row_outputs.insert(coordinate, output);
        self.row_retractions.insert(coordinate, retraction);
        self.index_coordinate(coordinate);
        delta
    }

    fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) -> Sc {
        let Some(output) = self.row_outputs.remove(&coordinate) else {
            return Sc::zero();
        };
        self.unindex_coordinate(coordinate);
        let key = (self.key_fn)(&output);
        let Some(retraction) = self.row_retractions.remove(&coordinate) else {
            return Sc::zero();
        };
        self.retract_value(key, retraction)
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

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let state = self.source.build_state(solution);
        let mut groups: HashMap<K, Acc> = HashMap::new();
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
        groups.iter().fold(Sc::zero(), |total, (key, acc)| {
            total + acc.with_result(|result| self.compute_score(key, result))
        })
    }

    fn match_count(&self, solution: &S) -> usize {
        let state = self.source.build_state(solution);
        let mut keys = HashMap::<K, ()>::new();
        self.source.collect_all(solution, &state, |_, output| {
            if self.filter.test(solution, &output) {
                keys.insert((self.key_fn)(&output), ());
            }
        });
        keys.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let state = self.source.build_state(solution);
        let mut total = Sc::zero();
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                rows.push((coordinate, output));
            });
        self.source_state = Some(state);
        for (coordinate, output) in rows {
            total = total + self.insert_row(solution, coordinate, output);
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
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
        let mut total = Sc::zero();
        for (coordinate, output) in rows {
            total = total + self.insert_row(solution, coordinate, output);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
        let mut total = Sc::zero();
        for coordinate in self.coordinates_for_owners(&owners) {
            total = total + self.retract_row(coordinate);
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
        total
    }

    fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.row_outputs.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }
}
