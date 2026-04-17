use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::EitherMove;

use super::super::entity::{EntitySelector, FromSolutionEntitySelector};
use super::super::value_selector::{StaticValueSelector, ValueSelector};
use super::{ChangeMoveSelector, MoveSelector, SwapMoveSelector};

pub struct EitherChangeMoveSelector<S, V, ES, VS> {
    inner: ChangeMoveSelector<S, V, ES, VS>,
}

impl<S, V: Debug, ES: Debug, VS: Debug> Debug for EitherChangeMoveSelector<S, V, ES, VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EitherChangeMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static>
    EitherChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticValueSelector<S, V>>
{
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            inner: ChangeMoveSelector::simple(
                getter,
                setter,
                descriptor_index,
                variable_name,
                values,
            ),
        }
    }
}

impl<S, V, ES, VS> MoveSelector<S, EitherMove<S, V>> for EitherChangeMoveSelector<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EitherMove<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(EitherMove::Change)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

pub struct EitherSwapMoveSelector<S, V, LES, RES> {
    inner: SwapMoveSelector<S, V, LES, RES>,
}

impl<S, V: Debug, LES: Debug, RES: Debug> Debug for EitherSwapMoveSelector<S, V, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EitherSwapMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V>
    EitherSwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            inner: SwapMoveSelector::simple(getter, setter, descriptor_index, variable_name),
        }
    }
}

impl<S, V, LES, RES> MoveSelector<S, EitherMove<S, V>> for EitherSwapMoveSelector<S, V, LES, RES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    LES: EntitySelector<S>,
    RES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EitherMove<S, V>> + 'a {
        self.inner.open_cursor(score_director).map(EitherMove::Swap)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
