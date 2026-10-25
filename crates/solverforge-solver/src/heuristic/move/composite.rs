/* CompositeMove - applies two moves in sequence by arena indices.

This move stores indices into two arenas. The moves themselves
live in their respective arenas - CompositeMove just references them.

# Zero-Erasure Design

No cloning, no boxing - just concrete arena indices.
*/

use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{Move, MoveAffectedEntity, MoveArena, MoveTabuSignature};

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
    ) -> (M1::Undo, M2::Undo) {
        let m1 = arena_1
            .get(self.index_1)
            .expect("composite move first arena index must remain valid");
        let m2 = arena_2
            .get(self.index_2)
            .expect("composite move second arena index must remain valid");

        let first = m1.do_move(score_director);
        let second = m2.do_move(score_director);
        (first, second)
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

mod preview;
pub(crate) use preview::SequentialPreviewDirector;

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
    require_hard_improvement: bool,
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
            require_hard_improvement: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
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

pub struct SequentialCompositeMoveRef<'a, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    first: &'a M,
    second: &'a M,
    descriptor_index: usize,
    entity_indices: &'a [usize],
    variable_name: &'a str,
    tabu_signature: &'a MoveTabuSignature,
    require_hard_improvement: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Debug for SequentialCompositeMoveRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SequentialCompositeMoveRef")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("entity_indices", &self.entity_indices)
            .finish()
    }
}

impl<'a, S, M> SequentialCompositeMoveRef<'a, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(
        first: &'a M,
        second: &'a M,
        descriptor_index: usize,
        entity_indices: &'a [usize],
        variable_name: &'a str,
        tabu_signature: &'a MoveTabuSignature,
        require_hard_improvement: bool,
    ) -> Self {
        Self {
            first,
            second,
            descriptor_index,
            entity_indices,
            variable_name,
            tabu_signature,
            require_hard_improvement,
            _phantom: PhantomData,
        }
    }

    pub fn first(&self) -> &'a M {
        self.first
    }

    pub fn second(&self) -> &'a M {
        self.second
    }
}

impl<S, M> Move<S> for SequentialCompositeMoveRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Undo = (M::Undo, M::Undo);

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if !self.first.is_doable(score_director) {
            return false;
        }

        let mut preview = SequentialPreviewDirector::from_director(score_director);
        let _ = self.first.do_move(&mut preview);
        self.second.is_doable(&preview)
    }

    /// Live-director working preview: left applied and undone on the caller's
    /// director, right checked against the previewed state, score state
    /// restored before returning. No solution clone.
    fn is_doable_with_working_preview<D: Director<S>>(&self, score_director: &mut D) -> bool {
        if !self.first.is_doable_with_working_preview(score_director) {
            return false;
        }
        let score_state = score_director.snapshot_score_state();
        let undo = self.first.do_move(score_director);
        let doable = self.second.is_doable_with_working_preview(score_director);
        self.first.undo_move(score_director, undo);
        score_director.restore_score_state(score_state);
        doable
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let first = self.first.do_move(score_director);
        let second = self.second.do_move(score_director);
        (first, second)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        self.second.undo_move(score_director, undo.1);
        self.first.undo_move(score_director, undo.0);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard_improvement
            || self.first.requires_hard_improvement()
            || self.second.requires_hard_improvement()
    }

    fn requires_score_improvement(&self) -> bool {
        self.first.requires_score_improvement() || self.second.requires_score_improvement()
    }

    fn tabu_signature<D: Director<S>>(&self, _score_director: &D) -> Cow<'_, MoveTabuSignature> {
        Cow::Borrowed(self.tabu_signature)
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        Some(CandidateTraceIdentity::composite(
            "sequential_composite",
            [
                self.first.candidate_trace_identity()?,
                self.second.candidate_trace_identity()?,
            ],
        ))
    }

    fn for_each_affected_entity<V: FnMut(MoveAffectedEntity<'_>)>(&self, visitor: &mut V) {
        self.first.for_each_affected_entity(visitor);
        self.second.for_each_affected_entity(visitor);
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
    type Undo = (M::Undo, M::Undo);

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let first = self.first_move();
        if !first.is_doable(score_director) {
            return false;
        }

        let mut preview = SequentialPreviewDirector::from_director(score_director);
        let _ = first.do_move(&mut preview);
        self.second_move().is_doable(&preview)
    }

    /// Live-director working preview; see `SequentialCompositeMoveRef`'s
    /// override for the transaction shape.
    fn is_doable_with_working_preview<D: Director<S>>(&self, score_director: &mut D) -> bool {
        let first = self.first_move();
        if !first.is_doable_with_working_preview(score_director) {
            return false;
        }
        let score_state = score_director.snapshot_score_state();
        let undo = first.do_move(score_director);
        let doable = self
            .second_move()
            .is_doable_with_working_preview(score_director);
        first.undo_move(score_director, undo);
        score_director.restore_score_state(score_state);
        doable
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let first = self.first_move().do_move(score_director);
        let second = self.second_move().do_move(score_director);
        (first, second)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        self.second_move().undo_move(score_director, undo.1);
        self.first_move().undo_move(score_director, undo.0);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn for_each_affected_entity<V: FnMut(MoveAffectedEntity<'_>)>(&self, visitor: &mut V) {
        self.first_move().for_each_affected_entity(visitor);
        self.second_move().for_each_affected_entity(visitor);
    }

    fn variable_name(&self) -> &str {
        &self.variable_name
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard_improvement
            || self.first_move().requires_hard_improvement()
            || self.second_move().requires_hard_improvement()
    }

    fn requires_score_improvement(&self) -> bool {
        self.first_move().requires_score_improvement()
            || self.second_move().requires_score_improvement()
    }

    fn tabu_signature<D: Director<S>>(&self, _score_director: &D) -> Cow<'_, MoveTabuSignature> {
        Cow::Borrowed(&self.tabu_signature)
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        Some(CandidateTraceIdentity::composite(
            "sequential_composite",
            [
                self.first_move().candidate_trace_identity()?,
                self.second_move().candidate_trace_identity()?,
            ],
        ))
    }
}
