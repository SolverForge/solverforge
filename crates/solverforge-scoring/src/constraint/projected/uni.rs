use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::filter::UniFilter;
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

pub struct ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    Src: ProjectedSource<S, Out>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    source: Src,
    filter: F,
    weight: W,
    is_hard: bool,
    source_state: Option<Src::State>,
    row_contributions: HashMap<ProjectedRowCoordinate, Sc>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<ProjectedRowCoordinate>>,
    _phantom: PhantomData<(fn() -> S, fn() -> Out)>,
}

impl<S, Out, Src, F, W, Sc> ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
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
            source_state: None,
            row_contributions: HashMap::new(),
            rows_by_owner: HashMap::new(),
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

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) -> Sc {
        if self.row_contributions.contains_key(&coordinate) || !self.filter.test(solution, &output)
        {
            return Sc::zero();
        }
        let contribution = self.compute_score(&output);
        self.row_contributions.insert(coordinate, contribution);
        self.index_coordinate(coordinate);
        contribution
    }

    fn retract_row(&mut self, coordinate: ProjectedRowCoordinate) -> Sc {
        let Some(contribution) = self.row_contributions.remove(&coordinate) else {
            return Sc::zero();
        };
        self.unindex_coordinate(coordinate);
        -contribution
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

impl<S, Out, Src, F, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let state = self.source.build_state(solution);
        let mut total = Sc::zero();
        self.source.collect_all(solution, &state, |_, output| {
            if self.filter.test(solution, &output) {
                total = total + self.compute_score(&output);
            }
        });
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let state = self.source.build_state(solution);
        let mut count = 0;
        self.source.collect_all(solution, &state, |_, output| {
            if self.filter.test(solution, &output) {
                count += 1;
            }
        });
        count
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
        self.row_contributions.clear();
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
