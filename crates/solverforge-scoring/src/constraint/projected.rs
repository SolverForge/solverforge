use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, UniCollector};
use crate::stream::filter::{BiFilter, UniFilter};
use crate::stream::{ProjectedRowCoordinate, ProjectedSource};

pub struct ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    source: Src,
    filter: F,
    weight: W,
    is_hard: bool,
    entity_contributions: HashMap<(usize, usize), Vec<Sc>>,
    _phantom: PhantomData<(fn() -> S, fn() -> Out)>,
}

impl<S, Out, Src, F, W, Sc> ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        source: Src,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            source,
            filter,
            weight,
            is_hard,
            entity_contributions: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_score(&self, output: &Out) -> Sc {
        let base = (self.weight)(output);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn insert_entity_outputs(&mut self, solution: &S, slot: usize, entity_index: usize) -> Sc {
        let mut total = Sc::zero();
        let mut contributions = Vec::new();
        let source = &self.source;
        let filter = &self.filter;
        let weight = &self.weight;
        let impact = self.impact_type;
        source.collect_entity(solution, slot, entity_index, |_, output| {
            if !filter.test(solution, &output) {
                return;
            }
            let base = weight(&output);
            let contribution = match impact {
                ImpactType::Penalty => -base,
                ImpactType::Reward => base,
            };
            total = total + contribution;
            contributions.push(contribution);
        });
        self.entity_contributions
            .insert((slot, entity_index), contributions);
        total
    }

    fn retract_entity_outputs(&mut self, slot: usize, entity_index: usize) -> Sc {
        self.entity_contributions
            .remove(&(slot, entity_index))
            .unwrap_or_default()
            .into_iter()
            .fold(Sc::zero(), |total, contribution| total - contribution)
    }

    fn localized_slots(&self, descriptor_index: usize) -> Vec<usize> {
        let mut slots = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, &self.constraint_ref.name)
            {
                slots.push(slot);
            }
        }
        slots
    }
}

impl<S, Out, Src, F, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let mut total = Sc::zero();
        self.source.collect_all(solution, |_, output| {
            if self.filter.test(solution, &output) {
                total = total + self.compute_score(&output);
            }
        });
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let mut count = 0;
        self.source.collect_all(solution, |_, output| {
            if self.filter.test(solution, &output) {
                count += 1;
            }
        });
        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let mut total = Sc::zero();
        let source = &self.source;
        let filter = &self.filter;
        let weight = &self.weight;
        let impact = self.impact_type;
        let entity_contributions = &mut self.entity_contributions;
        source.collect_all(solution, |coordinate, output| {
            if !filter.test(solution, &output) {
                return;
            }
            let base = weight(&output);
            let contribution = match impact {
                ImpactType::Penalty => -base,
                ImpactType::Reward => base,
            };
            let mut contributions = entity_contributions
                .remove(&(coordinate.source_slot, coordinate.entity_index))
                .unwrap_or_default();
            total = total + contribution;
            contributions.push(contribution);
            entity_contributions.insert(
                (coordinate.source_slot, coordinate.entity_index),
                contributions,
            );
        });
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.insert_entity_outputs(solution, slot, entity_index);
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.retract_entity_outputs(slot, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.entity_contributions.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}

struct ProjectedJoinRow<Out, K> {
    key: K,
    output: Out,
    order: ProjectedRowCoordinate,
}

pub struct ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    source: Src,
    filter: F,
    key_fn: KF,
    pair_filter: PF,
    weight: W,
    is_hard: bool,
    rows: Vec<Option<ProjectedJoinRow<Out, K>>>,
    free_row_ids: Vec<usize>,
    rows_by_entity: HashMap<(usize, usize), Vec<usize>>,
    rows_by_key: HashMap<K, Vec<usize>>,
    _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        source: Src,
        filter: F,
        key_fn: KF,
        pair_filter: PF,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            source,
            filter,
            key_fn,
            pair_filter,
            weight,
            is_hard,
            rows: Vec::new(),
            free_row_ids: Vec::new(),
            rows_by_entity: HashMap::new(),
            rows_by_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_score(&self, left: &Out, right: &Out) -> Sc {
        let base = (self.weight)(left, right);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn score_ordered_rows(
        &self,
        solution: &S,
        first: &ProjectedJoinRow<Out, K>,
        second: &ProjectedJoinRow<Out, K>,
    ) -> Sc {
        let (left, right) = if first.order <= second.order {
            (first, second)
        } else {
            (second, first)
        };
        if !self
            .pair_filter
            .test(solution, &left.output, &right.output, 0, 1)
        {
            return Sc::zero();
        }
        self.compute_score(&left.output, &right.output)
    }

    fn score_pair(&self, solution: &S, first_id: usize, second_id: usize) -> Sc {
        let Some(first) = self.rows.get(first_id).and_then(Option::as_ref) else {
            return Sc::zero();
        };
        let Some(second) = self.rows.get(second_id).and_then(Option::as_ref) else {
            return Sc::zero();
        };
        self.score_ordered_rows(solution, first, second)
    }

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) -> Sc {
        let key = (self.key_fn)(&output);
        let existing = self.rows_by_key.get(&key).cloned().unwrap_or_default();
        let row = Some(ProjectedJoinRow {
            key: key.clone(),
            output,
            order: coordinate,
        });
        let row_id = if let Some(row_id) = self.free_row_ids.pop() {
            debug_assert!(self.rows[row_id].is_none());
            self.rows[row_id] = row;
            row_id
        } else {
            let row_id = self.rows.len();
            self.rows.push(row);
            row_id
        };
        self.rows_by_entity
            .entry((coordinate.source_slot, coordinate.entity_index))
            .or_default()
            .push(row_id);

        let mut total = Sc::zero();
        for other_id in existing {
            total = total + self.score_pair(solution, row_id, other_id);
        }
        self.rows_by_key.entry(key).or_default().push(row_id);
        total
    }

    fn retract_row(&mut self, solution: &S, row_id: usize) -> Sc {
        let Some(row) = self.rows.get(row_id).and_then(Option::as_ref) else {
            return Sc::zero();
        };
        let key = row.key.clone();
        let candidates = self.rows_by_key.get(&key).cloned().unwrap_or_default();
        let mut total = Sc::zero();
        for other_id in candidates {
            if other_id == row_id {
                continue;
            }
            total = total - self.score_pair(solution, row_id, other_id);
        }

        if let Some(ids) = self.rows_by_key.get_mut(&key) {
            ids.retain(|&id| id != row_id);
            if ids.is_empty() {
                self.rows_by_key.remove(&key);
            }
        }
        self.rows[row_id] = None;
        self.free_row_ids.push(row_id);
        total
    }

    fn insert_entity_outputs(&mut self, solution: &S, slot: usize, entity_index: usize) -> Sc {
        let mut outputs = Vec::new();
        self.source
            .collect_entity(solution, slot, entity_index, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    outputs.push((coordinate, output));
                }
            });

        outputs
            .into_iter()
            .fold(Sc::zero(), |total, (coordinate, output)| {
                total + self.insert_row(solution, coordinate, output)
            })
    }

    fn retract_entity_outputs(&mut self, solution: &S, slot: usize, entity_index: usize) -> Sc {
        let Some(row_ids) = self.rows_by_entity.remove(&(slot, entity_index)) else {
            return Sc::zero();
        };
        row_ids.into_iter().fold(Sc::zero(), |total, row_id| {
            total + self.retract_row(solution, row_id)
        })
    }

    fn evaluate_rows(&self, solution: &S) -> Vec<ProjectedJoinRow<Out, K>> {
        let mut rows = Vec::new();
        self.source.collect_all(solution, |coordinate, output| {
            if self.filter.test(solution, &output) {
                rows.push(ProjectedJoinRow {
                    key: (self.key_fn)(&output),
                    output,
                    order: coordinate,
                });
            }
        });
        rows
    }

    fn score_evaluation_pair(
        &self,
        solution: &S,
        first: &ProjectedJoinRow<Out, K>,
        second: &ProjectedJoinRow<Out, K>,
    ) -> Sc {
        if first.key == second.key {
            self.score_ordered_rows(solution, first, second)
        } else {
            Sc::zero()
        }
    }

    fn evaluation_pair_matches(
        &self,
        solution: &S,
        first: &ProjectedJoinRow<Out, K>,
        second: &ProjectedJoinRow<Out, K>,
    ) -> bool {
        if first.key != second.key {
            return false;
        }
        let (left, right) = if first.order <= second.order {
            (first, second)
        } else {
            (second, first)
        };
        self.pair_filter
            .test(solution, &left.output, &right.output, 0, 1)
    }

    fn localized_slots(&self, descriptor_index: usize) -> Vec<usize> {
        let mut slots = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, &self.constraint_ref.name)
            {
                slots.push(slot);
            }
        }
        slots
    }

    #[cfg(test)]
    pub(crate) fn debug_row_storage_len(&self) -> usize {
        self.rows.len()
    }

    #[cfg(test)]
    pub(crate) fn debug_free_row_count(&self) -> usize {
        self.free_row_ids.len()
    }
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let rows = self.evaluate_rows(solution);

        let mut total = Sc::zero();
        for left_index in 0..rows.len() {
            for right_index in (left_index + 1)..rows.len() {
                total = total
                    + self.score_evaluation_pair(solution, &rows[left_index], &rows[right_index]);
            }
        }
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let rows = self.evaluate_rows(solution);

        let mut count = 0;
        for left_index in 0..rows.len() {
            for right_index in (left_index + 1)..rows.len() {
                if self.evaluation_pair_matches(solution, &rows[left_index], &rows[right_index]) {
                    count += 1;
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let mut rows = Vec::new();
        self.source.collect_all(solution, |coordinate, output| {
            if self.filter.test(solution, &output) {
                rows.push((coordinate, output));
            }
        });

        rows.into_iter()
            .fold(Sc::zero(), |total, (coordinate, output)| {
                total + self.insert_row(solution, coordinate, output)
            })
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.insert_entity_outputs(solution, slot, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.retract_entity_outputs(solution, slot, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.rows.clear();
        self.free_row_ids.clear();
        self.rows_by_entity.clear();
        self.rows_by_key.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}

pub struct ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, W, Sc>
where
    C: UniCollector<Out>,
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
    groups: HashMap<K, C::Accumulator>,
    group_counts: HashMap<K, usize>,
    entity_values: HashMap<(usize, usize), Vec<(K, C::Value)>>,
    _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, C, W, Sc> ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
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
            groups: HashMap::new(),
            group_counts: HashMap::new(),
            entity_values: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_score(&self, result: &C::Result) -> Sc {
        let base = (self.weight_fn)(result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn retract_output(&mut self, key: &K, value: &C::Value) -> Sc {
        let Some(acc) = self.groups.get_mut(key) else {
            return Sc::zero();
        };
        let impact = self.impact_type;
        let old_base = (self.weight_fn)(&acc.finish());
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        let is_empty = {
            let count = self.group_counts.entry(key.clone()).or_insert(0);
            *count = count.saturating_sub(1);
            *count == 0
        };
        if is_empty {
            self.group_counts.remove(key);
        }

        acc.retract(value);
        let new_score = if is_empty {
            self.groups.remove(key);
            Sc::zero()
        } else {
            let new_base = (self.weight_fn)(&acc.finish());
            match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            }
        };

        new_score - old
    }

    fn insert_entity_outputs(&mut self, solution: &S, slot: usize, entity_index: usize) -> Sc {
        let mut total = Sc::zero();
        let mut cached = Vec::new();
        let source = &self.source;
        let filter = &self.filter;
        let key_fn = &self.key_fn;
        let collector = &self.collector;
        let weight_fn = &self.weight_fn;
        let impact = self.impact_type;
        let groups = &mut self.groups;
        let group_counts = &mut self.group_counts;
        source.collect_entity(solution, slot, entity_index, |_, output| {
            if !filter.test(solution, &output) {
                return;
            }
            let key = key_fn(&output);
            let value = collector.extract(&output);
            let is_new = !groups.contains_key(&key);
            let acc = groups
                .entry(key.clone())
                .or_insert_with(|| collector.create_accumulator());
            let old = if is_new {
                Sc::zero()
            } else {
                let old_base = weight_fn(&acc.finish());
                match impact {
                    ImpactType::Penalty => -old_base,
                    ImpactType::Reward => old_base,
                }
            };
            acc.accumulate(&value);
            let new_base = weight_fn(&acc.finish());
            let new_score = match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            };
            *group_counts.entry(key.clone()).or_insert(0) += 1;
            cached.push((key, value));
            total = total + (new_score - old);
        });
        self.entity_values.insert((slot, entity_index), cached);
        total
    }

    fn retract_entity_outputs(&mut self, slot: usize, entity_index: usize) -> Sc {
        let Some(cached) = self.entity_values.remove(&(slot, entity_index)) else {
            return Sc::zero();
        };
        let mut total = Sc::zero();
        for (key, value) in cached {
            total = total + self.retract_output(&key, &value);
        }
        total
    }

    fn localized_slots(&self, descriptor_index: usize) -> Vec<usize> {
        let mut slots = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, &self.constraint_ref.name)
            {
                slots.push(slot);
            }
        }
        slots
    }
}

impl<S, Out, K, Src, F, KF, C, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Send + Sync,
    C::Value: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let mut groups: HashMap<K, C::Accumulator> = HashMap::new();
        self.source.collect_all(solution, |_, output| {
            if !self.filter.test(solution, &output) {
                return;
            }
            let key = (self.key_fn)(&output);
            let value = self.collector.extract(&output);
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(&value);
        });
        groups.values().fold(Sc::zero(), |total, acc| {
            total + self.compute_score(&acc.finish())
        })
    }

    fn match_count(&self, solution: &S) -> usize {
        let mut keys = HashMap::<K, ()>::new();
        self.source.collect_all(solution, |_, output| {
            if self.filter.test(solution, &output) {
                keys.insert((self.key_fn)(&output), ());
            }
        });
        keys.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let mut total = Sc::zero();
        let source = &self.source;
        let filter = &self.filter;
        let key_fn = &self.key_fn;
        let collector = &self.collector;
        let weight_fn = &self.weight_fn;
        let impact = self.impact_type;
        let groups = &mut self.groups;
        let group_counts = &mut self.group_counts;
        let entity_values = &mut self.entity_values;
        source.collect_all(solution, |coordinate, output| {
            if !filter.test(solution, &output) {
                return;
            }
            let key = key_fn(&output);
            let value = collector.extract(&output);
            let is_new = !groups.contains_key(&key);
            let acc = groups
                .entry(key.clone())
                .or_insert_with(|| collector.create_accumulator());
            let old = if is_new {
                Sc::zero()
            } else {
                let old_base = weight_fn(&acc.finish());
                match impact {
                    ImpactType::Penalty => -old_base,
                    ImpactType::Reward => old_base,
                }
            };
            acc.accumulate(&value);
            let new_base = weight_fn(&acc.finish());
            let new_score = match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            };
            *group_counts.entry(key.clone()).or_insert(0) += 1;
            entity_values
                .entry((coordinate.source_slot, coordinate.entity_index))
                .or_default()
                .push((key, value));
            total = total + (new_score - old);
        });
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.insert_entity_outputs(solution, slot, entity_index);
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in self.localized_slots(descriptor_index) {
            total = total + self.retract_entity_outputs(slot, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.group_counts.clear();
        self.entity_values.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}
