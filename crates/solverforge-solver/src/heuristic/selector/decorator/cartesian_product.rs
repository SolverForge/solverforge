/* Cartesian product move selector.

Combines moves from two selectors and exposes borrowable sequential candidates.
The selected winner is materialized by ownership only after the search phase
chooses a stable candidate index.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::compose_sequential_tabu_signature;
use crate::heuristic::r#move::{
    Move, MoveArena, MoveTabuSignature, SequentialCompositeMove, SequentialCompositeMoveRef,
    SequentialPreviewDirector,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveSelectorIter,
    MoveStreamContext,
};

/// Holds two owned move arenas and provides indexed pair iteration.
pub struct CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    arena_1: MoveArena<M1>,
    arena_2: MoveArena<M2>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M1, M2> CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    pub fn new() -> Self {
        Self {
            arena_1: MoveArena::new(),
            arena_2: MoveArena::new(),
            _phantom: PhantomData,
        }
    }

    pub fn reset(&mut self) {
        self.arena_1.reset();
        self.arena_2.reset();
    }

    pub fn populate_first<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M1>,
    {
        self.arena_1.extend(selector.iter_moves(score_director));
    }

    pub fn populate_second<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M2>,
    {
        self.arena_2.extend(selector.iter_moves(score_director));
    }

    pub fn len(&self) -> usize {
        self.arena_1.len() * self.arena_2.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena_1.is_empty() || self.arena_2.is_empty()
    }

    pub fn get_first(&self, index: usize) -> Option<&M1> {
        self.arena_1.get(index)
    }

    pub fn get_second(&self, index: usize) -> Option<&M2> {
        self.arena_2.get(index)
    }

    pub fn iter_indices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let len_1 = self.arena_1.len();
        let len_2 = self.arena_2.len();
        (0..len_1).flat_map(move |i| (0..len_2).map(move |j| (i, j)))
    }

    pub fn iter_pairs(&self) -> impl Iterator<Item = (usize, usize, &M1, &M2)> + '_ {
        self.iter_indices().filter_map(|(i, j)| {
            let m1 = self.arena_1.get(i)?;
            let m2 = self.arena_2.get(j)?;
            Some((i, j, m1, m2))
        })
    }
}

impl<S, M1, M2> Default for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M1, M2> Debug for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductArena")
            .field("arena_1_len", &self.arena_1.len())
            .field("arena_2_len", &self.arena_2.len())
            .finish()
    }
}

struct CartesianRow<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    left: M,
    right_moves: CandidateStore<S, M>,
    live_pairs: usize,
}

struct ActiveCartesianRow<C, U> {
    row_index: usize,
    right_cursor: C,
    left_undo: U,
    left_signature: MoveTabuSignature,
}

struct CartesianPair {
    row_index: usize,
    right_id: CandidateId,
    entity_indices: SmallVec<[usize; 8]>,
    tabu_signature: MoveTabuSignature,
}

pub struct CartesianProductCursor<'a, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    require_hard_improvement: bool,
    left_cursor: LeftCursor,
    right_selector: &'a Right,
    rows: Vec<Option<CartesianRow<S, M>>>,
    active_row: Option<ActiveCartesianRow<Right::Cursor<'a>, M::Undo>>,
    pairs: Vec<Option<CartesianPair>>,
    preview: SequentialPreviewDirector<S>,
    context: MoveStreamContext,
    _phantom: PhantomData<fn() -> S>,
}

impl<'a, S, M, LeftCursor, Right> CartesianProductCursor<'a, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    fn new<D: Director<S>>(
        require_hard_improvement: bool,
        left_cursor: LeftCursor,
        right_selector: &'a Right,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self {
        Self {
            require_hard_improvement,
            left_cursor,
            right_selector,
            rows: Vec::new(),
            active_row: None,
            pairs: Vec::new(),
            preview: SequentialPreviewDirector::from_director(score_director),
            context,
            _phantom: PhantomData,
        }
    }

    fn build_candidate(&self, pair_id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let pair = self.pairs.get(pair_id.index())?.as_ref()?;
        let row = self.rows.get(pair.row_index)?.as_ref()?;
        let left = &row.left;
        let right = match row.right_moves.candidate(pair.right_id)? {
            MoveCandidateRef::Borrowed(mov) => mov,
            MoveCandidateRef::Sequential(_) => return None,
        };
        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name()
        } else {
            "cartesian_product"
        };
        Some(MoveCandidateRef::Sequential(
            SequentialCompositeMoveRef::new(
                left,
                right,
                left.descriptor_index(),
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
}

impl<S, M, LeftCursor, Right> MoveCursor<S, M>
    for CartesianProductCursor<'_, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.active_row.is_some() {
                let inspected = {
                    let active = self
                        .active_row
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let row_index = active.row_index;
                    let row = self.rows[row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let left = &row.left;
                    let left_signature = &active.left_signature;
                    active.right_cursor.next_owned_candidate_inspected(|right| {
                        let right = match right {
                            MoveCandidateRef::Borrowed(mov) => mov,
                            MoveCandidateRef::Sequential(_) => {
                                panic!("nested cartesian right candidates are not supported")
                            }
                        };
                        if !right.is_doable(&self.preview) {
                            return None;
                        }
                        Some((
                            combined_entity_indices(left.entity_indices(), right.entity_indices()),
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

                let active = self
                    .active_row
                    .take()
                    .expect("active cartesian row must remain live");
                let row_index = active.row_index;
                self.rows[row_index]
                    .as_ref()
                    .expect("active cartesian row must remain live")
                    .left
                    .undo_move(&mut self.preview, active.left_undo);
                self.release_row_if_unused(row_index);
            }

            let left_id = self.left_cursor.next_candidate()?;
            let left = self
                .left_cursor
                .candidate(left_id)
                .expect("left cartesian candidate must remain valid");
            let left = match left {
                MoveCandidateRef::Borrowed(mov) => mov,
                MoveCandidateRef::Sequential(_) => {
                    panic!("nested cartesian left candidates are not supported")
                }
            };
            if !left.is_doable(&self.preview) {
                assert!(self.left_cursor.release_candidate(left_id));
                continue;
            }
            let left_signature = left.tabu_signature(&self.preview);
            let left = self.left_cursor.take_candidate(left_id);
            let left_undo = left.do_move(&mut self.preview);
            let right_cursor = self
                .right_selector
                .open_cursor_with_context(&self.preview, self.context);
            let row_index = self.rows.len();
            self.rows.push(Some(CartesianRow {
                left,
                right_moves: CandidateStore::new(),
                live_pairs: 0,
            }));
            self.active_row = Some(ActiveCartesianRow {
                row_index,
                right_cursor,
                left_undo,
                left_signature,
            });
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.build_candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let _ = index;
        panic!(
            "cartesian product cursors expose borrowed sequential candidates; apply the selected candidate before dropping the cursor",
        );
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let pair = self
            .pairs
            .get_mut(index.index())
            .and_then(Option::take)
            .expect("selected cartesian pair must remain live");
        let mut row = self.rows[pair.row_index]
            .take()
            .expect("selected cartesian row must remain live");
        if self
            .active_row
            .as_ref()
            .is_some_and(|active| active.row_index == pair.row_index)
        {
            self.active_row = None;
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
        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name().to_string()
        } else {
            "cartesian_product".to_string()
        };
        let descriptor_index = left.descriptor_index();
        let selected = SequentialCompositeMove::new(
            left,
            right,
            descriptor_index,
            pair.entity_indices,
            variable_name,
            pair.tabu_signature,
        )
        .with_require_hard_improvement(self.require_hard_improvement);
        selected.do_move(score_director);
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

/// Cartesian product selector that evaluates the right child after the left
/// child on a preview solution and yields borrowable sequential candidates.
pub struct CartesianProductSelector<S, M, Left, Right> {
    left: Left,
    right: Right,
    require_hard_improvement: bool,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, Left, Right> CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    pub fn new(left: Left, right: Right) -> Self {
        Self {
            left,
            right,
            require_hard_improvement: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
    }
}

impl<S, M, Left, Right> Debug for CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M> + Debug,
    Right: MoveSelector<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductSelector")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

impl<S, M, Left, Right> MoveSelector<S, M> for CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    type Cursor<'a>
        = CartesianProductCursor<'a, S, M, Left::Cursor<'a>, Right>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.right.validate_cursor(score_director);
        CartesianProductCursor::new(
            self.require_hard_improvement,
            self.left.open_cursor_with_context(score_director, context),
            &self.right,
            score_director,
            context,
        )
    }

    fn validate_cursor<D: Director<S>>(&self, score_director: &D) {
        self.left.validate_cursor(score_director);
        self.right.validate_cursor(score_director);
    }

    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        _score_director: &D,
    ) -> MoveSelectorIter<S, M, Self::Cursor<'a>> {
        panic!(
            "cartesian selectors do not support owned iter_moves(); use open_cursor() and candidate() to evaluate or commit the selected borrowed sequential candidate",
        );
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.left
            .size(score_director)
            .saturating_mul(self.right.size(score_director))
    }

    fn append_moves<D: Director<S>>(&self, _score_director: &D, _arena: &mut MoveArena<M>) {
        panic!(
            "cartesian selectors do not support append_moves(); use open_cursor() and candidate() to evaluate or commit the selected borrowed sequential candidate",
        );
    }
}

#[cfg(test)]
mod tests;
