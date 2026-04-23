/* Cartesian product move selector.

Combines moves from two selectors and yields owned sequential composites for
each legal pair.

# Zero-Erasure Design

Moves are stored in typed arenas. The cartesian product iterator
yields indices into both arenas. The caller creates CompositeMove
references on-the-fly for each evaluation - no cloning.
*/

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::heuristic::r#move::{
    Move, MoveArena, MoveTabuSignature, SequentialCompositeMove, SequentialPreviewDirector,
};
use crate::heuristic::selector::MoveSelector;

/// Holds two arenas of moves and provides iteration over all pairs.
///
/// This is NOT a MoveSelector - it's a specialized structure for
/// cartesian product iteration that preserves zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - First move type
/// * `M2` - Second move type
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

    /// Resets both arenas for the next step.
    pub fn reset(&mut self) {
        self.arena_1.reset();
        self.arena_2.reset();
    }

    /// Populates arena 1 from a move selector.
    pub fn populate_first<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M1>,
    {
        self.arena_1.extend(selector.open_cursor(score_director));
    }

    /// Populates arena 2 from a move selector.
    pub fn populate_second<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M2>,
    {
        self.arena_2.extend(selector.open_cursor(score_director));
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

    /// Returns an iterator over all (i, j) index pairs.
    pub fn iter_indices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let len_1 = self.arena_1.len();
        let len_2 = self.arena_2.len();
        (0..len_1).flat_map(move |i| (0..len_2).map(move |j| (i, j)))
    }

    /// Returns an iterator over all (i, j) pairs with references to both moves.
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

fn append_unique_entities(target: &mut smallvec::SmallVec<[usize; 8]>, entities: &[usize]) {
    for &entity in entities {
        if !target.contains(&entity) {
            target.push(entity);
        }
    }
}

fn append_unique_tokens<A>(target: &mut smallvec::SmallVec<A>, tokens: &[A::Item])
where
    A: smallvec::Array,
    A::Item: Copy + PartialEq,
{
    for &token in tokens {
        if !target.contains(&token) {
            target.push(token);
        }
    }
}

fn combine_tabu_signatures(
    first: &MoveTabuSignature,
    second: &MoveTabuSignature,
) -> MoveTabuSignature {
    let mut entity_tokens = first.entity_tokens.clone();
    append_unique_tokens(&mut entity_tokens, &second.entity_tokens);

    let mut destination_value_tokens = first.destination_value_tokens.clone();
    append_unique_tokens(
        &mut destination_value_tokens,
        &second.destination_value_tokens,
    );

    let mut move_id = smallvec::smallvec![crate::heuristic::r#move::metadata::hash_str(
        "cartesian_product"
    )];
    move_id.extend(first.move_id.iter().copied());
    move_id.extend(second.move_id.iter().copied());

    let mut undo_move_id = smallvec::smallvec![crate::heuristic::r#move::metadata::hash_str(
        "cartesian_product"
    )];
    undo_move_id.extend(second.undo_move_id.iter().copied());
    undo_move_id.extend(first.undo_move_id.iter().copied());

    let scope = if first.scope == second.scope {
        first.scope
    } else {
        crate::heuristic::r#move::metadata::MoveTabuScope::new(
            first.scope.descriptor_index,
            "cartesian_product",
        )
    };

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens(destination_value_tokens)
}

/// Cartesian product selector that evaluates the right child after the left
/// child on a preview solution and yields cached sequential composites.
pub struct CartesianProductSelector<S, M, Left, Right> {
    left: Left,
    right: Right,
    wrap: fn(SequentialCompositeMove<S, M>) -> M,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M, Left, Right> CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + Clone + 'static,
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

    fn build_moves<D: Director<S>>(&self, score_director: &D) -> Vec<M> {
        let wrap = self.wrap;
        let mut composites = Vec::new();

        for first_move in self.left.open_cursor(score_director) {
            if !first_move.is_doable(score_director) {
                continue;
            }

            let first_signature = first_move.tabu_signature(score_director);
            let first_descriptor_index = first_move.descriptor_index();
            let first_variable_name = first_move.variable_name().to_string();
            let first_entity_indices = first_move.entity_indices().to_vec();
            let mut preview = SequentialPreviewDirector::from_director(score_director);
            first_move.do_move(&mut preview);

            for second_move in self.right.open_cursor(&preview) {
                if !second_move.is_doable(&preview) {
                    continue;
                }
                let second_signature = second_move.tabu_signature(&preview);
                let second_variable_name = second_move.variable_name().to_string();
                let mut entity_indices = smallvec::SmallVec::<[usize; 8]>::new();
                append_unique_entities(&mut entity_indices, &first_entity_indices);
                append_unique_entities(&mut entity_indices, second_move.entity_indices());

                composites.push(wrap(SequentialCompositeMove::<S, M>::new(
                    first_move.clone(),
                    second_move,
                    first_descriptor_index,
                    entity_indices,
                    if first_variable_name == second_variable_name {
                        first_variable_name.clone()
                    } else {
                        "cartesian_product".to_string()
                    },
                    combine_tabu_signatures(&first_signature, &second_signature),
                )));
            }
        }

        composites
    }
}

impl<S, M, Left, Right> Debug for CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + Clone + 'static,
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
    M: Move<S> + Clone + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a {
        self.build_moves(score_director).into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        // Preview can prune or expand rows when the right selector depends on
        // post-left state; size must remain pure and never open child cursors.
        self.left
            .size(score_director)
            .saturating_mul(self.right.size(score_director))
    }

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        arena.extend(self.build_moves(score_director));
    }
}

#[cfg(test)]
#[path = "cartesian_product_tests.rs"]
mod tests;
