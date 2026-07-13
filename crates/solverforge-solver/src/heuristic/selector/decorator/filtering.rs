/* Filtering move selector decorator.

Filters moves from an inner selector based on a predicate function.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

pub struct FilteringMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    inner: C,
    predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool,
    discovered: Vec<CandidateId>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> FilteringMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(inner: C, predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool) -> Self {
        Self {
            inner,
            predicate,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> MoveCursor<S, M> for FilteringMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let child_id = self.inner.next_candidate()?;
            let candidate = self
                .inner
                .candidate(child_id)
                .expect("filtering candidate must remain valid");
            if (self.predicate)(candidate) {
                let outer_id = CandidateId::new(self.discovered.len());
                self.discovered.push(child_id);
                return Some(outer_id);
            }
            assert!(self.inner.release_candidate(child_id));
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(*self.discovered.get(id.index())?)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(self.discovered[id.index()])
    }

    // Keep the potentially large child cursor state machine out of every
    // iterator consumer. This boundary avoids instruction-layout duplication
    // while still returning each accepted move by value without storage.
    #[inline(never)]
    fn next_owned_candidate(&mut self) -> Option<M> {
        self.inner.next_owned_candidate_matching(self.predicate)
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        self.inner
            .apply_owned_candidate(self.discovered[id.index()], score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        let Some(&child_id) = self.discovered.get(id.index()) else {
            return false;
        };
        self.inner.release_candidate(child_id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(*self.discovered.get(id.index())?)
    }
}

/// Filters moves from an inner selector using a predicate function.
///
/// Only moves for which the predicate returns `true` are yielded.
/// Uses a function pointer for zero-erasure filtering.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::FilteringMoveSelector;
/// use solverforge_solver::heuristic::selector::move_selector::MoveCandidateRef;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_priority(s: &Solution, i: usize, _variable_index: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.priority) }
/// fn set_priority(s: &mut Solution, i: usize, _variable_index: usize, v: Option<i32>) { if let Some(t) = s.tasks.get_mut(i) { t.priority = v; } }
///
/// // Filter to only high-priority moves (value > 50)
/// fn high_priority_filter(
///     m: MoveCandidateRef<'_, Solution, ChangeMove<Solution, i32>>,
/// ) -> bool {
///     matches!(m, MoveCandidateRef::Borrowed(mov) if mov.to_value().is_some_and(|v| *v > 50))
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0,  0, "priority", vec![10, 60, 80],
/// );
/// let filtered: FilteringMoveSelector<Solution, _, _> =
///     FilteringMoveSelector::new(inner, high_priority_filter);
/// assert!(!filtered.is_never_ending());
/// ```
pub struct FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    inner: Inner,
    predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(inner: Inner, predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool) -> Self {
        Self {
            inner,
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteringMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    type Cursor<'a>
        = FilteringMoveCursor<S, M, Inner::Cursor<'a>>
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
        FilteringMoveCursor::new(
            self.inner.open_cursor_with_context(score_director, context),
            self.predicate,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }

    fn validate_cursor<D: Director<S>>(&self, score_director: &D) {
        self.inner.validate_cursor(score_director);
    }
}

#[cfg(test)]
mod tests;
