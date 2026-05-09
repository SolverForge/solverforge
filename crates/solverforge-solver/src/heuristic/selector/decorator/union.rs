/* Union move selector combinator.

Combines moves from two selectors into a single stream.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Combines moves from two selectors into a single stream.
///
/// Yields all moves from the first selector, then all moves from the second.
/// Both selectors must produce the same move type.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::UnionMoveSelector;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
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
/// let low_values = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0,  0, "priority", vec![1, 2, 3],
/// );
/// let high_values = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0,  0, "priority", vec![100, 200],
/// );
/// // Union yields moves with values: 1, 2, 3, 100, 200
/// let combined: UnionMoveSelector<Solution, _, _, _> =
///     UnionMoveSelector::new(low_values, high_values);
/// assert!(!combined.is_never_ending());
/// ```
pub struct UnionMoveSelector<S, M, A, B> {
    first: A,
    second: B,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, A, B> UnionMoveSelector<S, M, A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, A: Debug, B: Debug> Debug for UnionMoveSelector<S, M, A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnionMoveSelector")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<S, M, A, B> MoveSelector<S, M> for UnionMoveSelector<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveSelector<S, M>,
    B: MoveSelector<S, M>,
{
    type Cursor<'a>
        = UnionMoveCursor<S, M, A::Cursor<'a>, B::Cursor<'a>>
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
        UnionMoveCursor::new(
            self.first.open_cursor_with_context(score_director, context),
            self.second
                .open_cursor_with_context(score_director, context),
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.first.size(score_director) + self.second.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.first.is_never_ending() || self.second.is_never_ending()
    }
}

enum ActiveSource {
    First,
    Second,
    Done,
}

pub struct UnionMoveCursor<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveCursor<S, M>,
    B: MoveCursor<S, M>,
{
    first: A,
    second: B,
    discovered: Vec<(u8, CandidateId)>,
    active: ActiveSource,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, A, B> UnionMoveCursor<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveCursor<S, M>,
    B: MoveCursor<S, M>,
{
    fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            discovered: Vec::new(),
            active: ActiveSource::First,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, A, B> MoveCursor<S, M> for UnionMoveCursor<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveCursor<S, M>,
    B: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self.active {
            ActiveSource::First => {
                let Some(child_index) = self.first.next_candidate() else {
                    self.active = ActiveSource::Second;
                    return self.next_candidate();
                };
                let global_id = CandidateId::new(self.discovered.len());
                self.discovered.push((0, child_index));
                self.first
                    .candidate(child_index)
                    .expect("union first candidate must remain valid");
                Some(global_id)
            }
            ActiveSource::Second => {
                let Some(child_index) = self.second.next_candidate() else {
                    self.active = ActiveSource::Done;
                    return None;
                };
                let global_id = CandidateId::new(self.discovered.len());
                self.discovered.push((1, child_index));
                self.second
                    .candidate(child_index)
                    .expect("union second candidate must remain valid");
                Some(global_id)
            }
            ActiveSource::Done => None,
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (source, child_index) = *self.discovered.get(index.index())?;
        match source {
            0 => self.first.candidate(child_index),
            1 => self.second.candidate(child_index),
            _ => None,
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (source, child_index) = self.discovered[index.index()];
        match source {
            0 => self.first.take_candidate(child_index),
            1 => self.second.take_candidate(child_index),
            _ => unreachable!("union cursor source id must remain valid"),
        }
    }
}

impl<S, M, A, B> Iterator for UnionMoveCursor<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveCursor<S, M>,
    B: MoveCursor<S, M>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

#[cfg(test)]
mod tests;
