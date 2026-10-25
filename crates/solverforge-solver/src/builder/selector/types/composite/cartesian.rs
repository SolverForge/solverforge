//! Indexed owned-pair Cartesian cursor.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::compose_sequential_tabu_signature;
use crate::heuristic::r#move::{
    Move, MoveTabuSignature, SequentialCompositeMove, SequentialCompositeMoveRef,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveStreamContext,
};

use super::state::SelectorCompositionCursor;
use super::{CompositionNodeId, SequentialMoveCarrier, StatefulComposedFlat};

struct CartesianRow<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    left: M,
    right_moves: CandidateStore<S, M>,
    live_pairs: usize,
}

struct ActiveCartesianRow {
    row_index: usize,
    left_signature: MoveTabuSignature,
    right_open: bool,
}

struct CartesianPair {
    row_index: usize,
    right_id: CandidateId,
    entity_indices: SmallVec<[usize; 8]>,
    tabu_signature: MoveTabuSignature,
}

/// Cartesian payloads own selected child carriers; graph edges are stable IDs.
/// Right cursors open only after left preview application, using the same arena.
pub(super) struct SelectorCompositionCartesianCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
{
    require_hard_improvement: bool,
    left: CompositionNodeId,
    right: CompositionNodeId,
    rows: Vec<Option<CartesianRow<S, M>>>,
    active_row: Option<ActiveCartesianRow>,
    pairs: Vec<Option<CartesianPair>>,
    context: MoveStreamContext,
}

impl<S, M> SelectorCompositionCartesianCursor<S, M>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
{
    pub(super) fn new(
        require_hard_improvement: bool,
        left: CompositionNodeId,
        right: CompositionNodeId,
        context: MoveStreamContext,
    ) -> Self {
        Self {
            require_hard_improvement,
            left,
            right,
            rows: Vec::new(),
            active_row: None,
            pairs: Vec::new(),
            context,
        }
    }
    pub(super) fn left(&self) -> CompositionNodeId {
        self.left
    }
    pub(super) fn right(&self) -> CompositionNodeId {
        self.right
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

    fn finish_active_row<Flat, FlatState, Resources>(
        &mut self,
        arena: &mut SelectorCompositionCursor<'_, S, M, Flat, FlatState, Resources>,
    ) where
        Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
    {
        let active = self
            .active_row
            .take()
            .expect("active cartesian row must remain live");
        let row_index = active.row_index;
        arena.close_node(self.right);
        self.release_row_if_unused(row_index);
    }

    pub(super) fn take<Flat, FlatState, Resources>(
        &mut self,
        arena: &mut SelectorCompositionCursor<'_, S, M, Flat, FlatState, Resources>,
        index: CandidateId,
    ) -> M
    where
        Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
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

        if active.is_some() {
            arena.close_node(self.right);
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

    pub(super) fn next<Flat, FlatState, Resources, D>(
        &mut self,
        arena: &mut SelectorCompositionCursor<'_, S, M, Flat, FlatState, Resources>,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId>
    where
        Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
        D: Director<S>,
    {
        loop {
            if let Some(active) = self.active_row.as_mut() {
                let row_index = active.row_index;
                let row = self.rows[row_index]
                    .as_mut()
                    .expect("active cartesian row must remain live");
                let score_state = director.snapshot_score_state();
                let undo = row.left.do_move(director);
                if !active.right_open {
                    arena.open_node(self.right, resources, director, self.context);
                    active.right_open = true;
                }
                let inspected = loop {
                    let Some(id) = arena.next_at(self.right, director, resources) else {
                        break None;
                    };
                    let metadata = {
                        let right = arena
                            .candidate_at(self.right, id)
                            .expect("right candidate must remain valid");
                        if !right.is_doable_with_working_preview(director) {
                            None
                        } else {
                            Some((
                                combined_entity_indices(
                                    row.left.entity_indices(),
                                    right.entity_indices(),
                                ),
                                compose_sequential_tabu_signature(
                                    "cartesian_product",
                                    &active.left_signature,
                                    &right.tabu_signature(director),
                                ),
                            ))
                        }
                    };
                    if let Some(metadata) = metadata {
                        break Some((arena.take_at(self.right, id), metadata));
                    }
                    assert!(arena.release_at(self.right, id));
                };
                row.left.undo_move(director, undo);
                director.restore_score_state(score_state);
                if let Some((right, (entity_indices, tabu_signature))) = inspected {
                    let right_id = row.right_moves.push(right);
                    let pair_id = CandidateId::new(self.pairs.len());
                    self.pairs.push(Some(CartesianPair {
                        row_index,
                        right_id,
                        entity_indices,
                        tabu_signature,
                    }));
                    row.live_pairs += 1;
                    return Some(pair_id);
                }
                self.finish_active_row(arena);
            }
            let left_id = arena.next_at(self.left, director, resources)?;
            let (left_is_doable, left_signature) = {
                let left = arena
                    .candidate_at(self.left, left_id)
                    .expect("left cartesian candidate must remain valid");
                (
                    left.is_doable_with_working_preview(director),
                    left.tabu_signature(director).into_owned(),
                )
            };
            if !left_is_doable {
                assert!(arena.release_at(self.left, left_id));
                continue;
            }
            let left = arena.take_at(self.left, left_id);
            let row_index = self.rows.len();
            self.rows.push(Some(CartesianRow {
                left,
                right_moves: CandidateStore::new(),
                live_pairs: 0,
            }));
            self.active_row = Some(ActiveCartesianRow {
                row_index,
                left_signature,
                right_open: false,
            });
        }
    }

    pub(super) fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.build_candidate(index)
    }

    pub(super) fn release(&mut self, index: CandidateId) -> bool {
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
