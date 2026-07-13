//! Recursive owned-pair Cartesian cursor.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::compose_sequential_tabu_signature;
use crate::heuristic::r#move::{
    Move, MoveTabuSignature, SequentialCompositeMove, SequentialCompositeMoveRef,
    SequentialPreviewDirector,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};

use super::state::{
    open_cursor_with_owned_stream_state, SelectorCompositionCursor, SelectorCompositionStreamState,
};
use super::{SelectorCompositionChild, SequentialMoveCarrier, StatefulComposedFlat};

struct CartesianRow<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    left: M,
    right_moves: CandidateStore<S, M>,
    live_pairs: usize,
}

struct ActiveCartesianRow<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    row_index: usize,
    right_cursor: Box<SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>>,
    left_undo: M::Undo,
    left_signature: MoveTabuSignature,
}

struct CartesianPair {
    row_index: usize,
    right_id: CandidateId,
    entity_indices: SmallVec<[usize; 8]>,
    tabu_signature: MoveTabuSignature,
}

/// The one recursive Cartesian cursor.
///
/// It materializes a selected nested pair through `SequentialMoveCarrier`, so
/// a nested Cartesian candidate becomes an ordinary owned `M` for its parent.
/// The parent then previews that selected left and opens its right only then.
/// The solve-owned resource is lent only to the currently reached left/right
/// pull, preserving lazy provider delivery through nested products.
pub(crate) struct SelectorCompositionCartesianCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    require_hard_improvement: bool,
    left_cursor: Box<SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>>,
    right_selector: &'a SelectorCompositionChild<S, M, Flat, FlatState>,
    right_stream_state: Option<SelectorCompositionStreamState<FlatState>>,
    rows: Vec<Option<CartesianRow<S, M>>>,
    active_row: Option<ActiveCartesianRow<'a, S, M, Flat, FlatState, Resources>>,
    pairs: Vec<Option<CartesianPair>>,
    preview: SequentialPreviewDirector<S>,
    context: MoveStreamContext,
}

impl<'a, S, M, Flat, FlatState, Resources>
    SelectorCompositionCartesianCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    pub(super) fn new<D: Director<S>>(
        require_hard_improvement: bool,
        left_cursor: SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>,
        right_selector: &'a SelectorCompositionChild<S, M, Flat, FlatState>,
        right_stream_state: SelectorCompositionStreamState<FlatState>,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self
    where
        M: SequentialMoveCarrier<S>,
    {
        Self {
            require_hard_improvement,
            left_cursor: Box::new(left_cursor),
            right_selector,
            right_stream_state: Some(right_stream_state),
            rows: Vec::new(),
            active_row: None,
            pairs: Vec::new(),
            preview: SequentialPreviewDirector::from_director(score_director),
            context,
        }
    }

    fn build_candidate(&self, pair_id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let pair = self.pairs.get(pair_id.index())?.as_ref()?;
        let row = self.rows.get(pair.row_index)?.as_ref()?;
        let right = match row.right_moves.candidate(pair.right_id)? {
            MoveCandidateRef::Borrowed(mov) => mov,
            MoveCandidateRef::Sequential(_) => {
                unreachable!("candidate stores retain owned move carriers")
            }
        };
        let variable_name = if row.left.variable_name() == right.variable_name() {
            row.left.variable_name()
        } else {
            "cartesian_product"
        };
        Some(MoveCandidateRef::Sequential(
            SequentialCompositeMoveRef::new(
                &row.left,
                right,
                row.left.descriptor_index(),
                pair.entity_indices.as_slice(),
                variable_name,
                &pair.tabu_signature,
                self.require_hard_improvement,
            ),
        ))
    }

    fn release_row_if_unused(&mut self, row_index: usize) {
        let should_release = self
            .rows
            .get(row_index)
            .and_then(Option::as_ref)
            .is_some_and(|row| {
                self.active_row
                    .as_ref()
                    .is_none_or(|active| active.row_index != row_index)
                    && row.live_pairs == 0
            });
        if should_release {
            let _row = self.rows[row_index]
                .take()
                .expect("unused cartesian row must remain live");
        }
    }

    fn finish_active_row(&mut self)
    where
        M: SequentialMoveCarrier<S>,
    {
        let active = self
            .active_row
            .take()
            .expect("active cartesian row must remain live");
        let row_index = active.row_index;
        let right_stream_state = (*active.right_cursor).into_stream_state();
        assert!(
            self.right_stream_state
                .replace(right_stream_state)
                .is_none(),
            "a Cartesian right stream state must be owned by exactly one live cursor"
        );
        self.rows[row_index]
            .as_ref()
            .expect("active cartesian row must remain live")
            .left
            .undo_move(&mut self.preview, active.left_undo);
        self.release_row_if_unused(row_index);
    }

    fn take_selected_pair(&mut self, index: CandidateId) -> M
    where
        M: SequentialMoveCarrier<S>,
    {
        let pair = self
            .pairs
            .get_mut(index.index())
            .and_then(Option::take)
            .expect("selected cartesian pair must remain live");
        let active = self
            .active_row
            .as_ref()
            .is_some_and(|active| active.row_index == pair.row_index)
            .then(|| {
                self.active_row
                    .take()
                    .expect("selected active cartesian row must remain live")
            });
        let mut row = self.rows[pair.row_index]
            .take()
            .expect("selected cartesian row must remain live");

        if let Some(active) = active {
            let right_stream_state = (*active.right_cursor).into_stream_state();
            assert!(
                self.right_stream_state
                    .replace(right_stream_state)
                    .is_none(),
                "a Cartesian right stream state must be owned by exactly one live cursor"
            );
            row.left.undo_move(&mut self.preview, active.left_undo);
        }

        for sibling in &mut self.pairs {
            if sibling
                .as_ref()
                .is_some_and(|sibling| sibling.row_index == pair.row_index)
            {
                let sibling = sibling
                    .take()
                    .expect("selected cartesian sibling must remain live");
                assert!(row.right_moves.release_candidate(sibling.right_id));
            }
        }

        let left = row.left;
        let right = row.right_moves.take_candidate(pair.right_id);
        let descriptor_index = left.descriptor_index();
        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name().to_string()
        } else {
            "cartesian_product".to_string()
        };
        let selected = SequentialCompositeMove::new(
            left,
            right,
            descriptor_index,
            pair.entity_indices,
            variable_name,
            pair.tabu_signature,
        )
        .with_require_hard_improvement(self.require_hard_improvement);
        M::from_sequential(selected)
    }

    pub(super) fn into_stream_state(mut self) -> SelectorCompositionStreamState<FlatState>
    where
        M: SequentialMoveCarrier<S>,
    {
        let right_stream_state = match self.active_row.take() {
            Some(active) => (*active.right_cursor).into_stream_state(),
            None => self
                .right_stream_state
                .take()
                .expect("Cartesian right stream state must remain owned"),
        };
        SelectorCompositionStreamState::Cartesian {
            left: Box::new((*self.left_cursor).into_stream_state()),
            right: Box::new(right_stream_state),
        }
    }
}

impl<S, M, Flat, FlatState, Resources> ResourceMoveCursor<S, M, Resources>
    for SelectorCompositionCartesianCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId> {
        loop {
            if self.active_row.is_some() {
                let inspected = {
                    let active = self
                        .active_row
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let row = self.rows[active.row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let left = &row.left;
                    let left_signature = &active.left_signature;
                    active
                        .right_cursor
                        .next_owned_candidate_inspected_with_resources(resources, |right| {
                            if !right.is_doable(&self.preview) {
                                return None;
                            }
                            Some((
                                combined_entity_indices(
                                    left.entity_indices(),
                                    right.entity_indices(),
                                ),
                                compose_sequential_tabu_signature(
                                    "cartesian_product",
                                    left_signature,
                                    &right.tabu_signature(&self.preview),
                                ),
                            ))
                        })
                };
                if let Some((right, (entity_indices, tabu_signature))) = inspected {
                    let row_index = self
                        .active_row
                        .as_ref()
                        .expect("active cartesian row must remain live")
                        .row_index;
                    let right_id = self.rows[row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live")
                        .right_moves
                        .push(right);
                    let pair_id = CandidateId::new(self.pairs.len());
                    self.pairs.push(Some(CartesianPair {
                        row_index,
                        right_id,
                        entity_indices,
                        tabu_signature,
                    }));
                    self.rows[row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live")
                        .live_pairs += 1;
                    return Some(pair_id);
                }
                self.finish_active_row();
            }

            let left_id = self.left_cursor.next_candidate_with_resources(resources)?;
            let (left_is_doable, left_signature) = {
                let left = self
                    .left_cursor
                    .candidate(left_id)
                    .expect("left cartesian candidate must remain valid");
                (
                    left.is_doable(&self.preview),
                    left.tabu_signature(&self.preview),
                )
            };
            if !left_is_doable {
                assert!(self.left_cursor.release_candidate(left_id));
                continue;
            }
            let left = self.left_cursor.take_candidate(left_id);
            let left_undo = left.do_move(&mut self.preview);
            let right_stream_state = self
                .right_stream_state
                .take()
                .expect("Cartesian right stream state must be available before opening a row");
            let right_cursor = open_cursor_with_owned_stream_state(
                self.right_selector,
                right_stream_state,
                resources,
                &self.preview,
                self.context,
            );
            let row_index = self.rows.len();
            self.rows.push(Some(CartesianRow {
                left,
                right_moves: CandidateStore::new(),
                live_pairs: 0,
            }));
            self.active_row = Some(ActiveCartesianRow {
                row_index,
                right_cursor: Box::new(right_cursor),
                left_undo,
                left_signature,
            });
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.build_candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        self.take_selected_pair(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.take_selected_pair(index).do_move(score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(pair) = self.pairs.get_mut(index.index()).and_then(Option::take) else {
            return false;
        };
        let row = self.rows[pair.row_index]
            .as_mut()
            .expect("released cartesian row must remain live");
        assert!(row.right_moves.release_candidate(pair.right_id));
        row.live_pairs = row
            .live_pairs
            .checked_sub(1)
            .expect("cartesian row live-pair count must remain positive");
        self.release_row_if_unused(pair.row_index);
        true
    }

    fn selector_index(&self, _index: CandidateId) -> Option<usize> {
        None
    }
}

fn combined_entity_indices(left: &[usize], right: &[usize]) -> SmallVec<[usize; 8]> {
    let mut entity_indices = SmallVec::<[usize; 8]>::new();
    for &entity_index in left.iter().chain(right.iter()) {
        if !entity_indices.contains(&entity_index) {
            entity_indices.push(entity_index);
        }
    }
    entity_indices
}
