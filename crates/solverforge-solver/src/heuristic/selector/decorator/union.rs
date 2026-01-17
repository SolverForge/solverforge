//! Union move selector combinator.
//!
//! Combines moves from two selectors into a single stream.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Combines moves from two selectors into a single stream.
///
/// Yields all moves from the first selector, then all moves from the second.
/// Both selectors must produce the same move type.
pub struct UnionMoveSelector<S, M, A, B> {
    first: A,
    second: B,
    _phantom: PhantomData<(S, M)>,
}

impl<S, M, A, B> UnionMoveSelector<S, M, A, B> {
    /// Creates a new union selector combining two selectors.
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
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        Box::new(
            self.first
                .iter_moves(score_director)
                .chain(self.second.iter_moves(score_director)),
        )
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.first.size(score_director) + self.second.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.first.is_never_ending() || self.second.is_never_ending()
    }
}
