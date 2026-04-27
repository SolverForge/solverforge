use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, UniCollector};
use crate::stream::filter::UniFilter;
use crate::stream::ProjectedSource;

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

    fn insert_outputs(
        &mut self,
        solution: &S,
        slot: usize,
        entity_index: usize,
        outputs: Vec<Out>,
    ) -> Sc {
        let mut total = Sc::zero();
        let mut contributions = Vec::new();
        for output in outputs {
            if !self.filter.test(solution, &output) {
                continue;
            }
            let contribution = self.compute_score(&output);
            total = total + contribution;
            contributions.push(contribution);
        }
        self.entity_contributions
            .insert((slot, entity_index), contributions);
        total
    }

    fn insert_entity_outputs(&mut self, solution: &S, slot: usize, entity_index: usize) -> Sc {
        let mut outputs = Vec::new();
        self.source
            .collect_entity(solution, slot, entity_index, |output| outputs.push(output));
        self.insert_outputs(solution, slot, entity_index, outputs)
    }

    fn retract_entity_outputs(&mut self, slot: usize, entity_index: usize) -> Sc {
        self.entity_contributions
            .remove(&(slot, entity_index))
            .unwrap_or_default()
            .into_iter()
            .fold(Sc::zero(), |total, contribution| total - contribution)
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
        self.source.collect_all(solution, |_, _, output| {
            if self.filter.test(solution, &output) {
                total = total + self.compute_score(&output);
            }
        });
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let mut count = 0;
        self.source.collect_all(solution, |_, _, output| {
            if self.filter.test(solution, &output) {
                count += 1;
            }
        });
        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let mut rows = HashMap::<(usize, usize), Vec<Out>>::new();
        self.source.collect_all(solution, |slot, idx, output| {
            rows.entry((slot, idx)).or_default().push(output);
        });
        rows.into_iter()
            .fold(Sc::zero(), |total, ((slot, idx), outputs)| {
                total + self.insert_outputs(solution, slot, idx, outputs)
            })
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in 0..self.source.source_count() {
            if self.source.change_source(slot).reacts_to(descriptor_index) {
                total = total + self.insert_entity_outputs(solution, slot, entity_index);
            }
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in 0..self.source.source_count() {
            if self.source.change_source(slot).reacts_to(descriptor_index) {
                total = total + self.retract_entity_outputs(slot, entity_index);
            }
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

    fn insert_output(&mut self, key: K, value: C::Value) -> Sc {
        let impact = self.impact_type;
        let is_new = !self.groups.contains_key(&key);
        let acc = self
            .groups
            .entry(key.clone())
            .or_insert_with(|| self.collector.create_accumulator());

        let old = if is_new {
            Sc::zero()
        } else {
            let old_base = (self.weight_fn)(&acc.finish());
            match impact {
                ImpactType::Penalty => -old_base,
                ImpactType::Reward => old_base,
            }
        };

        acc.accumulate(&value);
        let new_base = (self.weight_fn)(&acc.finish());
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };
        *self.group_counts.entry(key).or_insert(0) += 1;
        new_score - old
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
        let mut projected = Vec::new();
        self.source
            .collect_entity(solution, slot, entity_index, |output| {
                projected.push(output)
            });
        self.insert_projected_outputs(solution, slot, entity_index, projected)
    }

    fn insert_projected_outputs(
        &mut self,
        solution: &S,
        slot: usize,
        entity_index: usize,
        outputs: Vec<Out>,
    ) -> Sc {
        let mut total = Sc::zero();
        let mut cached = Vec::new();
        for output in outputs {
            if !self.filter.test(solution, &output) {
                continue;
            }
            let key = (self.key_fn)(&output);
            let value = self.collector.extract(&output);
            total = total + self.insert_output(key.clone(), value.clone());
            cached.push((key, value));
        }
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
        self.source.collect_all(solution, |_, _, output| {
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
        self.source.collect_all(solution, |_, _, output| {
            if self.filter.test(solution, &output) {
                keys.insert((self.key_fn)(&output), ());
            }
        });
        keys.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let mut rows = HashMap::<(usize, usize), Vec<Out>>::new();
        self.source.collect_all(solution, |slot, idx, output| {
            rows.entry((slot, idx)).or_default().push(output);
        });
        rows.into_iter()
            .fold(Sc::zero(), |total, ((slot, idx), outputs)| {
                total + self.insert_projected_outputs(solution, slot, idx, outputs)
            })
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in 0..self.source.source_count() {
            if self.source.change_source(slot).reacts_to(descriptor_index) {
                total = total + self.insert_entity_outputs(solution, slot, entity_index);
            }
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for slot in 0..self.source.source_count() {
            if self.source.change_source(slot).reacts_to(descriptor_index) {
                total = total + self.retract_entity_outputs(slot, entity_index);
            }
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
