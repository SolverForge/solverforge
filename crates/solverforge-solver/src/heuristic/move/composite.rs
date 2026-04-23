/* CompositeMove - applies two moves in sequence by arena indices.

This move stores indices into two arenas. The moves themselves
live in their respective arenas - CompositeMove just references them.

# Zero-Erasure Design

No cloning, no boxing - just typed arena indices.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_scoring::{Director, DirectorScoreState};

use super::{Move, MoveArena, MoveTabuSignature};

/// A move that applies two moves in sequence via arena indices.
///
/// The moves live in separate arenas. CompositeMove stores the indices
/// and arena references needed to execute both moves.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - The first move type
/// * `M2` - The second move type
pub struct CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    index_1: usize,
    index_2: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> M1, fn() -> M2)>,
}

impl<S, M1, M2> CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    pub fn new(index_1: usize, index_2: usize) -> Self {
        Self {
            index_1,
            index_2,
            _phantom: PhantomData,
        }
    }

    pub fn index_1(&self) -> usize {
        self.index_1
    }

    pub fn index_2(&self) -> usize {
        self.index_2
    }

    pub fn is_doable_with_arenas<D: Director<S>>(
        &self,
        arena_1: &MoveArena<M1>,
        arena_2: &MoveArena<M2>,
        score_director: &D,
    ) -> bool {
        let m1 = arena_1.get(self.index_1);
        let m2 = arena_2.get(self.index_2);

        match (m1, m2) {
            (Some(m1), Some(m2)) => m1.is_doable(score_director) && m2.is_doable(score_director),
            _ => false,
        }
    }

    /// Executes both moves using the arenas.
    pub fn do_move_with_arenas<D: Director<S>>(
        &self,
        arena_1: &MoveArena<M1>,
        arena_2: &MoveArena<M2>,
        score_director: &mut D,
    ) {
        let m1 = arena_1
            .get(self.index_1)
            .expect("composite move first arena index must remain valid");
        let m2 = arena_2
            .get(self.index_2)
            .expect("composite move second arena index must remain valid");

        m1.do_move(score_director);
        m2.do_move(score_director);
    }
}

impl<S, M1, M2> Clone for CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M1, M2> Copy for CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
}

impl<S, M1, M2> Debug for CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeMove")
            .field("index_1", &self.index_1)
            .field("index_2", &self.index_2)
            .finish()
    }
}

pub(crate) struct SequentialPreviewDirector<'a, S: PlanningSolution> {
    working_solution: S,
    descriptor: &'a SolutionDescriptor,
    entity_counts: Vec<Option<usize>>,
    total_entity_count: Option<usize>,
}

impl<'a, S: PlanningSolution> SequentialPreviewDirector<'a, S> {
    pub(crate) fn from_director<D: Director<S>>(score_director: &'a D) -> Self {
        let descriptor = score_director.solution_descriptor();
        let entity_counts = (0..descriptor.entity_descriptor_count())
            .map(|descriptor_index| score_director.entity_count(descriptor_index))
            .collect();

        Self {
            working_solution: score_director.clone_working_solution(),
            descriptor,
            entity_counts,
            total_entity_count: score_director.total_entity_count(),
        }
    }
}

impl<S: PlanningSolution> Director<S> for SequentialPreviewDirector<'_, S> {
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        panic!("preview directors are only for selector generation")
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {
        self.working_solution.set_score(None);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.working_solution
            .update_entity_shadows(descriptor_index, entity_index);
        self.working_solution.set_score(None);
    }

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

/// A cached sequential composite that owns both child moves.
///
/// This keeps cartesian selector output valid even after the selector is
/// reused or dropped.
pub struct SequentialCompositeMove<S, M> {
    moves: MoveArena<M>,
    descriptor_index: usize,
    entity_indices: SmallVec<[usize; 8]>,
    variable_name: String,
    tabu_signature: MoveTabuSignature,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> SequentialCompositeMove<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(
        first: M,
        second: M,
        descriptor_index: usize,
        entity_indices: SmallVec<[usize; 8]>,
        variable_name: impl Into<String>,
        tabu_signature: MoveTabuSignature,
    ) -> Self {
        let mut moves = MoveArena::with_capacity(2);
        moves.push(first);
        moves.push(second);

        Self {
            moves,
            descriptor_index,
            entity_indices,
            variable_name: variable_name.into(),
            tabu_signature,
            _phantom: PhantomData,
        }
    }

    fn first_move(&self) -> &M {
        self.moves
            .get(0)
            .expect("sequential composite first move must remain valid")
    }

    fn second_move(&self) -> &M {
        self.moves
            .get(1)
            .expect("sequential composite second move must remain valid")
    }
}

impl<S, M> Clone for SequentialCompositeMove<S, M>
where
    S: PlanningSolution,
    M: Move<S> + Clone,
{
    fn clone(&self) -> Self {
        Self::new(
            self.first_move().clone(),
            self.second_move().clone(),
            self.descriptor_index,
            self.entity_indices.clone(),
            self.variable_name.clone(),
            self.tabu_signature.clone(),
        )
    }
}

impl<S, M> Debug for SequentialCompositeMove<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SequentialCompositeMove")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("entity_indices", &self.entity_indices)
            .finish()
    }
}

impl<S, M> Move<S> for SequentialCompositeMove<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let first = self.first_move();
        if !first.is_doable(score_director) {
            return false;
        }

        let mut preview = SequentialPreviewDirector::from_director(score_director);
        first.do_move(&mut preview);
        self.second_move().is_doable(&preview)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        self.first_move().do_move(score_director);
        self.second_move().do_move(score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        &self.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, _score_director: &D) -> MoveTabuSignature {
        self.tabu_signature.clone()
    }
}
