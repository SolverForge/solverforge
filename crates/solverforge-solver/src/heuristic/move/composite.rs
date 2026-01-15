//! CompositeMove - applies two moves in sequence by arena indices.
//!
//! This move stores indices into two arenas. The moves themselves
//! live in their respective arenas - CompositeMove just references them.
//!
//! # Zero-Erasure Design
//!
//! No cloning, no boxing - just typed arena indices.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::{Move, MoveArena};

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
    _phantom: PhantomData<(S, M1, M2)>,
}

impl<S, M1, M2> CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    /// Creates a new composite move from two arena indices.
    pub fn new(index_1: usize, index_2: usize) -> Self {
        Self {
            index_1,
            index_2,
            _phantom: PhantomData,
        }
    }

    /// Returns the first move's arena index.
    pub fn index_1(&self) -> usize {
        self.index_1
    }

    /// Returns the second move's arena index.
    pub fn index_2(&self) -> usize {
        self.index_2
    }

    /// Checks if this composite move is doable given both arenas.
    pub fn is_doable_with_arenas<D: ScoreDirector<S>>(
        &self,
        arena_1: &MoveArena<M1>,
        arena_2: &MoveArena<M2>,
        score_director: &D,
    ) -> bool {
        let m1 = arena_1.get(self.index_1);
        let m2 = arena_2.get(self.index_2);

        match (m1, m2) {
            (Some(m1), Some(m2)) => m1.is_doable(score_director) || m2.is_doable(score_director),
            _ => false,
        }
    }

    /// Executes both moves using the arenas.
    pub fn do_move_with_arenas<D: ScoreDirector<S>>(
        &self,
        arena_1: &MoveArena<M1>,
        arena_2: &MoveArena<M2>,
        score_director: &mut D,
    ) {
        if let Some(m1) = arena_1.get(self.index_1) {
            m1.do_move(score_director);
        }
        if let Some(m2) = arena_2.get(self.index_2) {
            m2.do_move(score_director);
        }
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
