use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ScalarMoveUnion;

use super::super::entity::{EntitySelector, FromSolutionEntitySelector};
use super::super::value_selector::{StaticValueSelector, ValueSelector};
use super::{ArenaMoveCursor, ChangeMoveSelector, MoveSelector, SwapMoveSelector};

pub struct ScalarChangeMoveSelector<S, V, ES, VS> {
    inner: ChangeMoveSelector<S, V, ES, VS>,
}

impl<S, V: Debug, ES: Debug, VS: Debug> Debug for ScalarChangeMoveSelector<S, V, ES, VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarChangeMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static>
    ScalarChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticValueSelector<S, V>>
{
    pub fn simple(
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            inner: ChangeMoveSelector::simple(
                getter,
                setter,
                descriptor_index,
                variable_index,
                variable_name,
                values,
            ),
        }
    }
}

impl<S, V, ES, VS> MoveSelector<S, ScalarMoveUnion<S, V>> for ScalarChangeMoveSelector<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(
            self.inner
                .iter_moves(score_director)
                .map(ScalarMoveUnion::Change),
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

pub struct ScalarSwapMoveSelector<S, V, LES, RES> {
    inner: SwapMoveSelector<S, V, LES, RES>,
}

impl<S, V: Debug, LES: Debug, RES: Debug> Debug for ScalarSwapMoveSelector<S, V, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarSwapMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V>
    ScalarSwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    pub fn simple(
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            inner: SwapMoveSelector::simple(
                getter,
                setter,
                descriptor_index,
                variable_index,
                variable_name,
            ),
        }
    }
}

impl<S, V, LES, RES> MoveSelector<S, ScalarMoveUnion<S, V>>
    for ScalarSwapMoveSelector<S, V, LES, RES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    LES: EntitySelector<S>,
    RES: EntitySelector<S>,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ScalarMoveUnion<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(
            self.inner
                .iter_moves(score_director)
                .map(ScalarMoveUnion::Swap),
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
