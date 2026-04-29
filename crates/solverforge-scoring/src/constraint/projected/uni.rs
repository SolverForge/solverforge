use std::collections::HashMap;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
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
