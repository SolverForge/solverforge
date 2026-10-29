
impl<S, M> Move<S> for &M
where
    S: PlanningSolution,
    M: Move<S> + ?Sized,
{
    type Undo = M::Undo;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        (**self).is_doable(score_director)
    }

    fn is_doable_with_working_preview<D: Director<S>>(&self, score_director: &mut D) -> bool {
        (**self).is_doable_with_working_preview(score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        (**self).do_move(score_director)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        (**self).undo_move(score_director, undo);
    }

    fn do_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        journal_pool: &mut Vec<Self::Undo>,
    ) -> Self::Undo {
        (**self).do_move_with_scratch(score_director, journal_pool)
    }

    fn undo_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
        journal_pool: &mut Vec<Self::Undo>,
    ) {
        (**self).undo_move_with_scratch(score_director, undo, journal_pool)
    }

    fn descriptor_index(&self) -> usize {
        (**self).descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        (**self).entity_indices()
    }

    fn variable_name(&self) -> &str {
        (**self).variable_name()
    }

    fn telemetry_label(&self) -> &'static str {
        (**self).telemetry_label()
    }

    fn requires_hard_improvement(&self) -> bool {
        (**self).requires_hard_improvement()
    }

    fn requires_score_improvement(&self) -> bool {
        (**self).requires_score_improvement()
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Cow<'_, crate::heuristic::r#move::MoveTabuSignature> {
        (**self).tabu_signature(score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        (**self).candidate_trace_identity()
    }

    fn for_each_affected_entity<V: FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>)>(
        &self,
        visitor: &mut V,
    ) {
        (**self).for_each_affected_entity(visitor);
    }
}

pub enum MoveCandidateRef<'a, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    Borrowed(&'a M),
    Sequential(SequentialCompositeMoveRef<'a, S, M>),
}

pub enum MoveCandidateUndo<U> {
    Borrowed(U),
    Sequential { first: U, second: U },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CandidateId(usize);

impl CandidateId {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

impl<S, M> Debug for MoveCandidateRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Borrowed(_) => write!(f, "MoveCandidateRef::Borrowed(..)"),
            Self::Sequential(mov) => write!(f, "MoveCandidateRef::Sequential({mov:?})"),
        }
    }
}

impl<S, M> Move<S> for MoveCandidateRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Undo = MoveCandidateUndo<M::Undo>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Borrowed(mov) => mov.is_doable(score_director),
            Self::Sequential(mov) => mov.is_doable(score_director),
        }
    }

    fn is_doable_with_working_preview<D: Director<S>>(&self, score_director: &mut D) -> bool {
        match self {
            Self::Borrowed(mov) => mov.is_doable_with_working_preview(score_director),
            Self::Sequential(mov) => mov.is_doable_with_working_preview(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::Borrowed(mov) => MoveCandidateUndo::Borrowed(mov.do_move(score_director)),
            Self::Sequential(mov) => {
                let first = mov.first().do_move(score_director);
                let second = mov.second().do_move(score_director);
                MoveCandidateUndo::Sequential { first, second }
            }
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (self, undo) {
            (Self::Borrowed(mov), MoveCandidateUndo::Borrowed(undo)) => {
                mov.undo_move(score_director, undo);
            }
            (Self::Sequential(mov), MoveCandidateUndo::Sequential { first, second }) => {
                mov.second().undo_move(score_director, second);
                mov.first().undo_move(score_director, first);
            }
            _ => panic!("move candidate undo shape must match candidate shape"),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Borrowed(mov) => mov.descriptor_index(),
            Self::Sequential(mov) => mov.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Borrowed(mov) => mov.entity_indices(),
            Self::Sequential(mov) => mov.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Borrowed(mov) => mov.variable_name(),
            Self::Sequential(mov) => mov.variable_name(),
        }
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        match self {
            Self::Borrowed(mov) => mov.candidate_trace_identity(),
            Self::Sequential(mov) => mov.candidate_trace_identity(),
        }
    }

    fn telemetry_label(&self) -> &'static str {
        match self {
            Self::Borrowed(mov) => mov.telemetry_label(),
            Self::Sequential(mov) => mov.telemetry_label(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Borrowed(mov) => mov.requires_hard_improvement(),
            Self::Sequential(mov) => mov.requires_hard_improvement(),
        }
    }

    fn requires_score_improvement(&self) -> bool {
        match self {
            Self::Borrowed(mov) => mov.requires_score_improvement(),
            Self::Sequential(mov) => mov.requires_score_improvement(),
        }
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Cow<'_, crate::heuristic::r#move::MoveTabuSignature> {
        match self {
            Self::Borrowed(mov) => mov.tabu_signature(score_director),
            Self::Sequential(mov) => mov.tabu_signature(score_director),
        }
    }

    fn for_each_affected_entity<V: FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>)>(
        &self,
        visitor: &mut V,
    ) {
        match self {
            Self::Borrowed(mov) => mov.for_each_affected_entity(visitor),
            Self::Sequential(mov) => mov.for_each_affected_entity(visitor),
        }
    }
}

impl<S, M> MoveCandidateRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Applies the candidate, letting a borrowed move reuse a retired undo
    /// journal of its own type from `journal_pool`.
    ///
    /// The pool holds the moves' own undo values, so a borrowed candidate
    /// lends it to the move directly and the trial allocates nothing.
    /// Sequential legs own their do/undo pairs; pooled journals across
    /// differently shaped legs need per-leg keying the composite does not
    /// provide.
    pub(crate) fn do_move_recycling<D: Director<S>>(
        &self,
        score_director: &mut D,
        journal_pool: &mut Vec<M::Undo>,
    ) -> MoveCandidateUndo<M::Undo> {
        match self {
            Self::Borrowed(mov) => {
                MoveCandidateUndo::Borrowed(mov.do_move_with_scratch(score_director, journal_pool))
            }
            Self::Sequential(mov) => {
                let first = mov.first().do_move(score_director);
                let second = mov.second().do_move(score_director);
                MoveCandidateUndo::Sequential { first, second }
            }
        }
    }

    /// Reverts `do_move_recycling`, returning retired journals to the pool.
    pub(crate) fn undo_move_recycling<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: MoveCandidateUndo<M::Undo>,
        journal_pool: &mut Vec<M::Undo>,
    ) {
        match (self, undo) {
            (Self::Borrowed(mov), MoveCandidateUndo::Borrowed(undo)) => {
                mov.undo_move_with_scratch(score_director, undo, journal_pool);
            }
            (Self::Sequential(mov), MoveCandidateUndo::Sequential { first, second }) => {
                mov.second().undo_move(score_director, second);
                mov.first().undo_move(score_director, first);
            }
            _ => panic!("move candidate undo shape must match candidate shape"),
        }
    }
}

/// Incremental, cursor-owned candidate storage for a single neighborhood.
///
/// `next_candidate()` constructs or discovers at most the next candidate and
/// returns a stable ID. A consumer may stop at any time: dropping the cursor
/// must release all retained candidates and any unconsumed source state without
/// requiring the remaining neighborhood to be enumerated. Implementations must
/// not rely on full exhaustion for correctness, callback delivery, or cleanup.
///
/// Candidates remain borrowable until they are released or transferred. The
/// selected candidate is consumed exactly once through `take_candidate()` or
/// `apply_owned_candidate()`; unselected live candidates remain cursor-owned.
pub trait MoveCursor<S: PlanningSolution, M: Move<S>> {
    /// Pulls the next candidate against the caller's live working solution.
    ///
    /// The solution is lent for the duration of the pull only. Cursors own no
    /// solution state: everything derived at open is plain data (indexes,
    /// coordinates, seeds), so a commit between pulls invalidates nothing the
    /// cursor retains and the next pull always sees current state.
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId>;

    /// Pulls the next candidate while lending the live director.
    ///
    /// Composite cursors that must preview a child (apply, shadow, check,
    /// undo) do so on the caller's director instead of a cloned preview
    /// solution. The default simply forwards to [`MoveCursor::next_candidate`]
    /// with `director.working_solution()`; cursors that never mutate keep
    /// paying nothing.
    fn next_candidate_with_director<D: Director<S>>(&mut self, director: &mut D) -> Option<CandidateId> {
        self.next_candidate(director.working_solution())
    }

    /// Pulls the next candidate while allowing expensive cursor implementations
    /// to observe solve control between units of generation work. A `None`
    /// result may mean either exhaustion or requested interruption, so callers
    /// must recheck the same control signal before resolving an exhausted scan.
    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        solution: &S,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            None
        } else {
            self.next_candidate(solution)
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, id: CandidateId) -> M;

    fn next_owned_candidate(&mut self, solution: &S) -> Option<M> {
        let id = self.next_candidate(solution)?;
        Some(self.take_candidate(id))
    }

    fn next_owned_candidate_matching(
        &mut self,
        solution: &S,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool,
    ) -> Option<M> {
        loop {
            let id = self.next_candidate(solution)?;
            let candidate = self
                .candidate(id)
                .expect("newly generated cursor candidate must remain live");
            let accepted = predicate(candidate);
            if accepted {
                return Some(self.take_candidate(id));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn next_owned_candidate_inspected<T, F>(&mut self, solution: &S, mut inspect: F) -> Option<(M, T)>
    where
        F: for<'a> FnMut(MoveCandidateRef<'a, S, M>) -> Option<T>,
    {
        loop {
            let id = self.next_candidate(solution)?;
            let candidate = self
                .candidate(id)
                .expect("newly generated cursor candidate must remain live");
            if let Some(metadata) = inspect(candidate) {
                return Some((self.take_candidate(id), metadata));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        let mov = self.take_candidate(id);
        mov.do_move(score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        if self.candidate(id).is_none() {
            return false;
        }
        drop(self.take_candidate(id));
        true
    }

    fn selector_index(&self, _id: CandidateId) -> Option<usize> {
        None
    }
}

/// A cursor whose next pull lends the live working director plus one
/// explicitly owned execution resource.
///
/// Ordinary selectors use [`MoveCursor`] directly. Runtime provider cursors
/// need a solve-owned reason arena only at the instant a reachable child is
/// pulled, however; retaining that arena in a leaf or hiding it behind
/// interior mutability would break lazy callback delivery and resource
/// ownership. Recursive composition therefore uses this private companion
/// contract and lends the live solution and its resource only to the child
/// selected by the canonical union scheduler.
#[doc(hidden)]
pub trait ResourceMoveCursor<S: PlanningSolution, M: Move<S>, Resources> {
    fn next_candidate_with_resources<D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId>;

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, id: CandidateId) -> M;

    fn next_owned_candidate_inspected_with_resources<T, F, D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
        mut inspect: F,
    ) -> Option<(M, T)>
    where
        F: for<'a> FnMut(MoveCandidateRef<'a, S, M>) -> Option<T>,
    {
        loop {
            let id = self.next_candidate_with_resources(director, resources)?;
            let candidate = self
                .candidate(id)
                .expect("newly generated resource cursor candidate must remain live");
            if let Some(metadata) = inspect(candidate) {
                return Some((self.take_candidate(id), metadata));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D);

    fn release_candidate(&mut self, id: CandidateId) -> bool;

    fn selector_index(&self, id: CandidateId) -> Option<usize>;
}

