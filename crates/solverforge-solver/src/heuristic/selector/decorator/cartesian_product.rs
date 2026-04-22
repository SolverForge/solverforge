/* Cartesian product move selector.

Combines moves from two selectors by storing them in separate arenas
and yielding cached sequential composite moves for each pair.

# Zero-Erasure Design

Moves are stored in typed arenas. The cartesian product iterator
yields indices into both arenas. The caller creates CompositeMove
references on-the-fly for each evaluation - no cloning.
*/

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, DirectorScoreState};
use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::heuristic::r#move::{Move, MoveArena, MoveTabuSignature, SequentialCompositeMove};
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

struct CartesianProductState<M> {
    first_arena: MoveArena<M>,
    second_arena: MoveArena<M>,
}

impl<M> CartesianProductState<M> {
    fn new() -> Self {
        Self {
            first_arena: MoveArena::new(),
            second_arena: MoveArena::new(),
        }
    }

    fn reset(&mut self) {
        self.first_arena.reset();
        self.second_arena.reset();
    }
}

struct PreviewDirector<'a, S: PlanningSolution> {
    working_solution: S,
    descriptor: &'a solverforge_core::domain::SolutionDescriptor,
    entity_counts: Vec<Option<usize>>,
    total_entity_count: Option<usize>,
}

impl<'a, S: PlanningSolution> PreviewDirector<'a, S> {
    fn new(
        working_solution: S,
        descriptor: &'a solverforge_core::domain::SolutionDescriptor,
        entity_counts: Vec<Option<usize>>,
        total_entity_count: Option<usize>,
    ) -> Self {
        Self {
            working_solution,
            descriptor,
            entity_counts,
            total_entity_count,
        }
    }
}

impl<S: PlanningSolution> Director<S> for PreviewDirector<'_, S> {
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        panic!("preview directors are only for selector generation")
    }

    fn solution_descriptor(&self) -> &solverforge_core::domain::SolutionDescriptor {
        self.descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.entity_counts.get(descriptor_index).copied().flatten()
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.total_entity_count
    }

    fn is_incremental(&self) -> bool {
        false
    }

    fn snapshot_score_state(&self) -> DirectorScoreState<S::Score> {
        DirectorScoreState {
            solution_score: self.working_solution.score(),
            committed_score: self.working_solution.score(),
            initialized: self.working_solution.score().is_some(),
        }
    }

    fn restore_score_state(&mut self, state: DirectorScoreState<S::Score>) {
        self.working_solution.set_score(state.solution_score);
    }

    fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>) {}
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
    state: RefCell<CartesianProductState<M>>,
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
            state: RefCell::new(CartesianProductState::new()),
            _phantom: PhantomData,
        }
    }

    fn build_moves<D: Director<S>>(&self, score_director: &D) -> Vec<M> {
        let mut state = self.state.borrow_mut();
        state.reset();
        state
            .first_arena
            .extend(self.left.open_cursor(score_director));
        let descriptor = score_director.solution_descriptor();
        let entity_counts: Vec<_> = (0..descriptor.entity_descriptor_count())
            .map(|descriptor_index| score_director.entity_count(descriptor_index))
            .collect();
        let total_entity_count = score_director.total_entity_count();

        let first_arena_addr = (&state.first_arena as *const MoveArena<M>) as usize;
        let second_arena_addr = (&state.second_arena as *const MoveArena<M>) as usize;
        let wrap = self.wrap;
        let mut composites = Vec::new();

        for first_index in 0..state.first_arena.len() {
            let Some((
                first_doable,
                first_signature,
                first_descriptor_index,
                first_variable_name,
                first_entity_indices,
                preview,
            )) = ({
                let first_arena = first_arena_addr as *const MoveArena<M>;
                // SAFETY: the selector owns `state.first_arena` for the full step; this
                // immutable read is disjoint from the later mutation of `state.second_arena`.
                unsafe { first_arena.as_ref() }.and_then(|arena| {
                    arena.get(first_index).map(|first_move| {
                        let first_doable = first_move.is_doable(score_director);
                        let mut preview = PreviewDirector::new(
                            score_director.clone_working_solution(),
                            descriptor,
                            entity_counts.clone(),
                            total_entity_count,
                        );
                        if first_doable {
                            first_move.do_move(&mut preview);
                        }
                        (
                            first_doable,
                            first_move.tabu_signature(score_director),
                            first_move.descriptor_index(),
                            first_move.variable_name(),
                            first_move.entity_indices().to_vec(),
                            preview,
                        )
                    })
                })
            })
            else {
                continue;
            };

            let second_start = state.second_arena.len();
            state.second_arena.extend(self.right.open_cursor(&preview));
            let second_end = state.second_arena.len();

            for second_index in second_start..second_end {
                let Some(second_move) = state.second_arena.get(second_index) else {
                    continue;
                };
                let second_doable = second_move.is_doable(&preview);
                let second_signature = second_move.tabu_signature(&preview);
                let mut entity_indices = smallvec::SmallVec::<[usize; 8]>::new();
                append_unique_entities(&mut entity_indices, &first_entity_indices);
                append_unique_entities(&mut entity_indices, second_move.entity_indices());

                composites.push(wrap(SequentialCompositeMove::<S, M>::new(
                    first_index,
                    second_index,
                    first_arena_addr,
                    second_arena_addr,
                    first_doable,
                    second_doable,
                    first_descriptor_index,
                    entity_indices,
                    if first_variable_name == second_move.variable_name() {
                        first_variable_name
                    } else {
                        "cartesian_product"
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
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a {
        self.build_moves(score_director).into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.build_moves(score_director).len()
    }

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        arena.extend(self.build_moves(score_director));
    }
}

#[cfg(test)]
#[path = "cartesian_product_tests.rs"]
mod tests;
