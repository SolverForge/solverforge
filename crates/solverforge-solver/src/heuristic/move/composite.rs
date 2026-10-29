/* SequentialCompositeMove - applies two owned moves in sequence.

The composite owns both child moves in one two-slot heap allocation, the
same single allocation the composite always made. The indirection is
required: the runtime's move union contains this composite of itself, so the
children cannot be stored inline.
*/

use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{Move, MoveAffectedEntity, MoveTabuSignature};

mod preview;
pub(crate) use preview::SequentialPreviewDirector;

/// A cached sequential composite that owns both child moves.
///
/// This keeps cartesian selector output valid even after the selector is
/// reused or dropped.
pub(crate) struct SequentialCompositeMove<S, M> {
    moves: Box<[M; 2]>,
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
    pub(crate) fn new(
        first: M,
        second: M,
        descriptor_index: usize,
        entity_indices: SmallVec<[usize; 8]>,
        variable_name: impl Into<String>,
        tabu_signature: MoveTabuSignature,
    ) -> Self {
        Self {
            moves: Box::new([first, second]),
            descriptor_index,
            entity_indices,
            variable_name: variable_name.into(),
            tabu_signature,
            require_hard_improvement: false,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
    }

    fn first_move(&self) -> &M {
        &self.moves[0]
    }

    fn second_move(&self) -> &M {
        &self.moves[1]
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
