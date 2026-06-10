use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::filter::{BiFilter, UniFilter};
use crate::stream::projected::{RowCoordinate, RowOwner, Source};

pub(super) struct ProjectedDirectedJoinRow<Out> {
    pub(super) output: Out,
    pub(super) coordinate: RowCoordinate,
}

pub struct DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) source: Src,
    pub(super) filter: F,
    pub(super) left_key_fn: KL,
    pub(super) right_key_fn: KR,
    pub(super) pair_filter: PF,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) source_state: Option<Src::State>,
    pub(super) rows: Vec<Option<ProjectedDirectedJoinRow<Out>>>,
    pub(super) free_row_ids: Vec<usize>,
    pub(super) rows_by_owner: HashMap<RowOwner, Vec<usize>>,
    pub(super) row_ids_by_coordinate: HashMap<RowCoordinate, usize>,
    pub(super) rows_by_left_key: HashMap<K, Vec<usize>>,
    pub(super) rows_by_right_key: HashMap<K, Vec<usize>>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KL, KR, PF, W, Sc> DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KL: Fn(&Out) -> K + Send + Sync,
    KR: Fn(&Out) -> K + Send + Sync,
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
        left_key_fn: KL,
        right_key_fn: KR,
        pair_filter: PF,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            source,
            filter,
            left_key_fn,
            right_key_fn,
            pair_filter,
            weight,
            is_hard,
            source_state: None,
            rows: Vec::new(),
            free_row_ids: Vec::new(),
            rows_by_owner: HashMap::new(),
            row_ids_by_coordinate: HashMap::new(),
            rows_by_left_key: HashMap::new(),
            rows_by_right_key: HashMap::new(),
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

    fn score_outputs(
        &self,
        solution: &S,
        left: &Out,
        right: &Out,
        left_idx: usize,
        right_idx: usize,
    ) -> Sc {
        if !self
            .pair_filter
            .test(solution, left, right, left_idx, right_idx)
        {
            return Sc::zero();
        }
        self.compute_score(left, right)
    }

    fn filter_index(coordinate: RowCoordinate) -> usize {
        coordinate.primary_owner.entity_index
    }

    fn score_retained_rows(
        &self,
        solution: &S,
        left: &ProjectedDirectedJoinRow<Out>,
        right: &ProjectedDirectedJoinRow<Out>,
    ) -> Sc {
        self.score_outputs(
            solution,
            &left.output,
            &right.output,
            Self::filter_index(left.coordinate),
            Self::filter_index(right.coordinate),
        )
    }

    fn score_candidate_left(
        &self,
        solution: &S,
        candidate_output: &Out,
        candidate_coordinate: RowCoordinate,
        right: &ProjectedDirectedJoinRow<Out>,
    ) -> Sc {
        self.score_outputs(
            solution,
            candidate_output,
            &right.output,
            Self::filter_index(candidate_coordinate),
            Self::filter_index(right.coordinate),
        )
    }

    fn score_candidate_right(
        &self,
        solution: &S,
        left: &ProjectedDirectedJoinRow<Out>,
        candidate_output: &Out,
        candidate_coordinate: RowCoordinate,
    ) -> Sc {
        self.score_outputs(
            solution,
            &left.output,
            candidate_output,
            Self::filter_index(left.coordinate),
            Self::filter_index(candidate_coordinate),
        )
    }

    fn score_pair(&self, solution: &S, left_id: usize, right_id: usize) -> Sc {
        let Some(left) = self.rows.get(left_id).and_then(Option::as_ref) else {
            return Sc::zero();
        };
        let Some(right) = self.rows.get(right_id).and_then(Option::as_ref) else {
            return Sc::zero();
        };
        self.score_retained_rows(solution, left, right)
    }

    pub(super) fn ensure_source_state(&mut self, solution: &S) {
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
    }

    fn index_row_owners(&mut self, coordinate: RowCoordinate, row_id: usize) {
        coordinate.for_each_owner(|owner| {
            self.rows_by_owner.entry(owner).or_default().push(row_id);
        });
    }

    fn unindex_row_owners(&mut self, coordinate: RowCoordinate, row_id: usize) {
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

    fn remove_row_from_key(index: &mut HashMap<K, Vec<usize>>, key: &K, row_id: usize) {
        let mut remove_bucket = false;
        if let Some(ids) = index.get_mut(key) {
            ids.retain(|&id| id != row_id);
            remove_bucket = ids.is_empty();
        }
        if remove_bucket {
            index.remove(key);
        }
    }

    pub(super) fn insert_row(
        &mut self,
        solution: &S,
        coordinate: RowCoordinate,
        output: Out,
    ) -> Sc {
        if self.row_ids_by_coordinate.contains_key(&coordinate) {
            return Sc::zero();
        }
        let left_key = (self.left_key_fn)(&output);
        let right_key = (self.right_key_fn)(&output);
        let mut total = Sc::zero();

        if let Some(existing) = self.rows_by_right_key.get(&left_key) {
            for &other_id in existing {
                if let Some(other) = self.rows.get(other_id).and_then(Option::as_ref) {
                    total = total + self.score_candidate_left(solution, &output, coordinate, other);
                }
            }
        }
        if let Some(existing) = self.rows_by_left_key.get(&right_key) {
            for &other_id in existing {
                if let Some(other) = self.rows.get(other_id).and_then(Option::as_ref) {
                    total =
                        total + self.score_candidate_right(solution, other, &output, coordinate);
                }
            }
        }

        let row = Some(ProjectedDirectedJoinRow { output, coordinate });
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
        self.rows_by_left_key
            .entry(left_key)
            .or_default()
            .push(row_id);
        self.rows_by_right_key
            .entry(right_key)
            .or_default()
            .push(row_id);
        total
    }

    pub(super) fn retract_row(&mut self, solution: &S, row_id: usize) -> Sc {
        let Some((left_key, right_key, coordinate)) =
            self.rows.get(row_id).and_then(Option::as_ref).map(|row| {
                (
                    (self.left_key_fn)(&row.output),
                    (self.right_key_fn)(&row.output),
                    row.coordinate,
                )
            })
        else {
            return Sc::zero();
        };
        let mut total = Sc::zero();
        if let Some(candidates) = self.rows_by_right_key.get(&left_key) {
            for &other_id in candidates {
                if other_id != row_id {
                    total = total - self.score_pair(solution, row_id, other_id);
                }
            }
        }
        if let Some(candidates) = self.rows_by_left_key.get(&right_key) {
            for &other_id in candidates {
                if other_id != row_id {
                    total = total - self.score_pair(solution, other_id, row_id);
                }
            }
        }

        Self::remove_row_from_key(&mut self.rows_by_left_key, &left_key, row_id);
        Self::remove_row_from_key(&mut self.rows_by_right_key, &right_key, row_id);
        self.row_ids_by_coordinate.remove(&coordinate);
        self.unindex_row_owners(coordinate, row_id);
        self.rows[row_id] = None;
        self.free_row_ids.push(row_id);
        total
    }

    pub(super) fn evaluate_rows(&self, solution: &S) -> Vec<ProjectedDirectedJoinRow<Out>> {
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    rows.push(ProjectedDirectedJoinRow { output, coordinate });
                }
            });
        rows
    }

    pub(super) fn right_index_for(
        &self,
        rows: &[ProjectedDirectedJoinRow<Out>],
    ) -> HashMap<K, Vec<usize>> {
        let mut right_index = HashMap::new();
        for (index, row) in rows.iter().enumerate() {
            right_index
                .entry((self.right_key_fn)(&row.output))
                .or_insert_with(Vec::new)
                .push(index);
        }
        right_index
    }

    pub(super) fn score_evaluation_pair(
        &self,
        solution: &S,
        left: &ProjectedDirectedJoinRow<Out>,
        right: &ProjectedDirectedJoinRow<Out>,
    ) -> Sc {
        self.score_retained_rows(solution, left, right)
    }

    pub(super) fn evaluation_pair_matches(
        &self,
        solution: &S,
        left: &ProjectedDirectedJoinRow<Out>,
        right: &ProjectedDirectedJoinRow<Out>,
    ) -> bool {
        self.pair_filter.test(
            solution,
            &left.output,
            &right.output,
            Self::filter_index(left.coordinate),
            Self::filter_index(right.coordinate),
        )
    }

    pub(super) fn localized_owners(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Vec<RowOwner> {
        let mut owners = Vec::new();
        for slot in 0..self.source.source_count() {
            if self
                .source
                .change_source(slot)
                .assert_localizes(descriptor_index, &self.constraint_ref.name)
            {
                owners.push(RowOwner {
                    source_slot: slot,
                    entity_index,
                });
            }
        }
        owners
    }

    pub(super) fn row_ids_for_owners(&self, owners: &[RowOwner]) -> Vec<usize> {
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
}
