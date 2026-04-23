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
    collect_cursor_indices, MoveCandidateRef, MoveCursor, MoveSelector, MoveSelectorIter,
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

struct CartesianRow<M> {
    right_moves: Vec<Option<M>>,
}

struct CartesianPair {
    left_index: usize,
    right_index: usize,
    entity_indices: SmallVec<[usize; 8]>,
    tabu_signature: MoveTabuSignature,
}

pub struct CartesianProductCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    wrap: fn(SequentialCompositeMove<S, M>) -> M,
    left_moves: Vec<Option<M>>,
    rows: Vec<CartesianRow<M>>,
    pairs: Vec<CartesianPair>,
    next_pair: usize,
    selected: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> CartesianProductCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn new<LeftCursor, Right, D>(
        wrap: fn(SequentialCompositeMove<S, M>) -> M,
        mut left_cursor: LeftCursor,
        right_selector: &Right,
        score_director: &D,
    ) -> Self
    where
        LeftCursor: MoveCursor<S, M>,
        Right: MoveSelector<S, M>,
        D: Director<S>,
    {
        let mut left_moves = Vec::new();
        let mut rows = Vec::new();
        let mut pairs = Vec::new();

        for left_index in collect_cursor_indices::<S, M, _>(&mut left_cursor) {
            let left_signature = left_cursor
                .candidate(left_index)
                .expect("left cartesian candidate must remain valid")
                .tabu_signature(score_director);
            let left_move = left_cursor.take_candidate(left_index);
            let row_index = left_moves.len();

            let mut row = CartesianRow {
                right_moves: Vec::new(),
            };

            if left_move.is_doable(score_director) {
                let mut preview = SequentialPreviewDirector::from_director(score_director);
                left_move.do_move(&mut preview);
                let mut right_cursor = right_selector.open_cursor(&preview);
                for right_index in collect_cursor_indices::<S, M, _>(&mut right_cursor) {
                    let right_signature = right_cursor
                        .candidate(right_index)
                        .expect("right cartesian candidate must remain valid")
                        .tabu_signature(&preview);
                    let right_move = right_cursor.take_candidate(right_index);
                    if !right_move.is_doable(&preview) {
                        continue;
                    }
                    let right_local_index = row.right_moves.len();
                    let entity_indices = combined_entity_indices(
                        left_move.entity_indices(),
                        right_move.entity_indices(),
                    );
                    let tabu_signature = compose_sequential_tabu_signature(
                        "cartesian_product",
                        &left_signature,
                        &right_signature,
                    );
                    row.right_moves.push(Some(right_move));
                    pairs.push(CartesianPair {
                        left_index: row_index,
                        right_index: right_local_index,
                        entity_indices,
                        tabu_signature,
                    });
                }
            }

            left_moves.push(Some(left_move));
            rows.push(row);
        }

        Self {
            wrap,
            left_moves,
            rows,
            pairs,
            next_pair: 0,
            selected: false,
            _phantom: PhantomData,
        }
    }

    fn build_candidate(&self, pair_index: usize) -> Option<MoveCandidateRef<'_, S, M>> {
        let pair = self.pairs.get(pair_index)?;
        let left = self.left_moves.get(pair.left_index)?.as_ref()?;
        let row = self.rows.get(pair.left_index)?;
        let right = row.right_moves.get(pair.right_index)?.as_ref()?;
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
            ),
        ))
    }
}

impl<S, M> MoveCursor<S, M> for CartesianProductCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn next_candidate(&mut self) -> Option<(usize, MoveCandidateRef<'_, S, M>)> {
        let index = self.next_pair;
        self.next_pair = index + 1;
        self.build_candidate(index)
            .map(|candidate| (index, candidate))
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, M>> {
        self.build_candidate(index)
    }

    fn take_candidate(&mut self, index: usize) -> M {
        assert!(
            !self.selected,
            "cartesian product cursors only support materializing one selected winner",
        );
        let pair = &self.pairs[index];
        let left = self.left_moves[pair.left_index]
            .take()
            .expect("selected left cartesian move must remain valid");
        let row = &mut self.rows[pair.left_index];
        let right = row.right_moves[pair.right_index]
            .take()
            .expect("selected right cartesian move must remain valid");
        self.selected = true;

        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name().to_string()
        } else {
            "cartesian_product".to_string()
        };
        (self.wrap)(SequentialCompositeMove::new(
            left,
            right,
            pair.tabu_signature.scope.descriptor_index,
            pair.entity_indices.clone(),
            variable_name,
            pair.tabu_signature.clone(),
        ))
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
    wrap: fn(SequentialCompositeMove<S, M>) -> M,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M, Left, Right> CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    pub fn new(left: Left, right: Right, wrap: fn(SequentialCompositeMove<S, M>) -> M) -> Self {
        Self {
            left,
            right,
            wrap,
            _phantom: PhantomData,
        }
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
        = CartesianProductCursor<S, M>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        CartesianProductCursor::new(
            self.wrap,
            self.left.open_cursor(score_director),
            &self.right,
            score_director,
        )
    }

    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        _score_director: &D,
    ) -> MoveSelectorIter<S, M, Self::Cursor<'a>> {
        panic!(
            "cartesian selectors do not support owned iter_moves(); use open_cursor() and take_candidate() to materialize only the selected winner",
        );
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.left
            .size(score_director)
            .saturating_mul(self.right.size(score_director))
    }

    fn append_moves<D: Director<S>>(&self, _score_director: &D, _arena: &mut MoveArena<M>) {
        panic!(
            "cartesian selectors do not support append_moves(); use open_cursor() and take_candidate() to materialize only the selected winner",
        );
    }
}

#[cfg(test)]
mod tests;
