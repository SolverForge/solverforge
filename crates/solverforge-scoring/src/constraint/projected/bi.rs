use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::filter::{BiFilter, UniFilter};
use crate::stream::{ProjectedRowCoordinate, ProjectedRowOwner, ProjectedSource};

struct ProjectedJoinRow<Out> {
    output: Out,
    coordinate: ProjectedRowCoordinate,
}

pub struct ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Src: ProjectedSource<S, Out>,
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
    source_state: Option<Src::State>,
    rows: Vec<Option<ProjectedJoinRow<Out>>>,
    free_row_ids: Vec<usize>,
    rows_by_owner: HashMap<ProjectedRowOwner, Vec<usize>>,
    row_ids_by_coordinate: HashMap<ProjectedRowCoordinate, usize>,
    rows_by_key: HashMap<K, Vec<usize>>,
    _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
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
            source_state: None,
            rows: Vec::new(),
            free_row_ids: Vec::new(),
            rows_by_owner: HashMap::new(),
            row_ids_by_coordinate: HashMap::new(),
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
        first: &ProjectedJoinRow<Out>,
        second: &ProjectedJoinRow<Out>,
    ) -> Sc {
        let (left, right) = if first.coordinate <= second.coordinate {
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

    fn score_candidate_row(
        &self,
        solution: &S,
        candidate_output: &Out,
        candidate_coordinate: ProjectedRowCoordinate,
        other: &ProjectedJoinRow<Out>,
    ) -> Sc {
        let (left, right) = if candidate_coordinate <= other.coordinate {
            (candidate_output, &other.output)
        } else {
            (&other.output, candidate_output)
        };
        if !self.pair_filter.test(solution, left, right, 0, 1) {
            return Sc::zero();
        }
        self.compute_score(left, right)
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

    fn ensure_source_state(&mut self, solution: &S) {
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
    }

    fn index_row_owners(&mut self, coordinate: ProjectedRowCoordinate, row_id: usize) {
        coordinate.for_each_owner(|owner| {
            self.rows_by_owner.entry(owner).or_default().push(row_id);
        });
    }

    fn unindex_row_owners(&mut self, coordinate: ProjectedRowCoordinate, row_id: usize) {
        coordinate.for_each_owner(|owner| {
            let mut remove_bucket = false;
            if let Some(ids) = self.rows_by_owner.get_mut(&owner) {
                ids.retain(|candidate| *candidate != row_id);
                remove_bucket = ids.is_empty();
            }
            if remove_bucket {
                self.rows_by_owner.remove(&owner);
            }
        });
    }

    fn insert_row(&mut self, solution: &S, coordinate: ProjectedRowCoordinate, output: Out) -> Sc {
        if self.row_ids_by_coordinate.contains_key(&coordinate) {
            return Sc::zero();
        }
        let key = (self.key_fn)(&output);
        let mut total = Sc::zero();
        if let Some(existing) = self.rows_by_key.get(&key) {
            for &other_id in existing {
                if let Some(other) = self.rows.get(other_id).and_then(Option::as_ref) {
                    total = total + self.score_candidate_row(solution, &output, coordinate, other);
                }
            }
        }
        let row = Some(ProjectedJoinRow { output, coordinate });
        let row_id = if let Some(row_id) = self.free_row_ids.pop() {
            debug_assert!(self.rows[row_id].is_none());
            self.rows[row_id] = row;
            row_id
        } else {
            let row_id = self.rows.len();
            self.rows.push(row);
            row_id
        };
        self.row_ids_by_coordinate.insert(coordinate, row_id);
        self.index_row_owners(coordinate, row_id);
        self.rows_by_key.entry(key).or_default().push(row_id);
        total
    }

    fn retract_row(&mut self, solution: &S, row_id: usize) -> Sc {
        let Some((key, coordinate)) = self
            .rows
            .get(row_id)
            .and_then(Option::as_ref)
            .map(|row| ((self.key_fn)(&row.output), row.coordinate))
        else {
            return Sc::zero();
        };
        let mut total = Sc::zero();
        if let Some(candidates) = self.rows_by_key.get(&key) {
            for &other_id in candidates {
                if other_id == row_id {
                    continue;
                }
                total = total - self.score_pair(solution, row_id, other_id);
            }
        }

        if let Some(ids) = self.rows_by_key.get_mut(&key) {
            ids.retain(|&id| id != row_id);
            if ids.is_empty() {
                self.rows_by_key.remove(&key);
            }
        }
        self.row_ids_by_coordinate.remove(&coordinate);
        self.unindex_row_owners(coordinate, row_id);
        self.rows[row_id] = None;
        self.free_row_ids.push(row_id);
        total
    }

    fn evaluate_rows(&self, solution: &S) -> Vec<ProjectedJoinRow<Out>> {
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    rows.push(ProjectedJoinRow { output, coordinate });
                }
            });
        rows
    }

    fn score_evaluation_pair(
        &self,
        solution: &S,
        first: &ProjectedJoinRow<Out>,
        second: &ProjectedJoinRow<Out>,
    ) -> Sc {
        if (self.key_fn)(&first.output) == (self.key_fn)(&second.output) {
            self.score_ordered_rows(solution, first, second)
        } else {
            Sc::zero()
        }
    }

    fn evaluation_pair_matches(
        &self,
        solution: &S,
        first: &ProjectedJoinRow<Out>,
        second: &ProjectedJoinRow<Out>,
    ) -> bool {
        if (self.key_fn)(&first.output) != (self.key_fn)(&second.output) {
            return false;
        }
        let (left, right) = if first.coordinate <= second.coordinate {
            (first, second)
        } else {
            (second, first)
        };
        self.pair_filter
            .test(solution, &left.output, &right.output, 0, 1)
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

    fn row_ids_for_owners(&self, owners: &[ProjectedRowOwner]) -> Vec<usize> {
        let mut seen = HashSet::new();
        let mut row_ids = Vec::new();
        for owner in owners {
            let Some(ids) = self.rows_by_owner.get(owner) else {
                continue;
            };
            for &row_id in ids {
                if seen.insert(row_id) {
                    row_ids.push(row_id);
                }
            }
        }
        row_ids
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
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
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
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    rows.push((coordinate, output));
                }
            });
        self.source_state = Some(state);

        rows.into_iter()
            .fold(Sc::zero(), |total, (coordinate, output)| {
                total + self.insert_row(solution, coordinate, output)
            })
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
                |coordinate, output| {
                    if self.filter.test(solution, &output) {
                        rows.push((coordinate, output));
                    }
                },
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
        for row_id in self.row_ids_for_owners(&owners) {
            total = total + self.retract_row(solution, row_id);
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
        self.rows.clear();
        self.free_row_ids.clear();
        self.rows_by_owner.clear();
        self.row_ids_by_coordinate.clear();
        self.rows_by_key.clear();
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
