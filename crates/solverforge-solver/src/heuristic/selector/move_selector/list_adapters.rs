use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListMoveImpl;

use super::super::entity::EntitySelector;
use super::super::k_opt::{KOptMoveSelector, ListPositionDistanceMeter, NearbyKOptMoveSelector};
use super::super::list_change::ListChangeMoveSelector;
use super::super::list_ruin::ListRuinMoveSelector;
use super::MoveSelector;

pub struct ListMoveListChangeSelector<S, V, ES> {
    inner: ListChangeMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveListChangeSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveListChangeSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveListChangeSelector<S, V, ES> {
    pub fn new(inner: ListChangeMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveListChangeSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(ListMoveImpl::ListChange)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

pub struct ListMoveKOptSelector<S, V, ES> {
    inner: KOptMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveKOptSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveKOptSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveKOptSelector<S, V, ES> {
    pub fn new(inner: KOptMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveKOptSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(ListMoveImpl::KOpt)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

pub struct ListMoveNearbyKOptSelector<S, V, D: ListPositionDistanceMeter<S>, ES> {
    inner: NearbyKOptMoveSelector<S, V, D, ES>,
}

impl<S, V: Debug, D: ListPositionDistanceMeter<S>, ES: Debug> Debug
    for ListMoveNearbyKOptSelector<S, V, D, ES>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveNearbyKOptSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, D: ListPositionDistanceMeter<S>, ES> ListMoveNearbyKOptSelector<S, V, D, ES> {
    pub fn new(inner: NearbyKOptMoveSelector<S, V, D, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, D, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveNearbyKOptSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S> + 'static,
    ES: EntitySelector<S>,
{
    fn open_cursor<'a, Dir: Director<S>>(
        &'a self,
        score_director: &Dir,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(ListMoveImpl::KOpt)
    }

    fn size<Dir: Director<S>>(&self, score_director: &Dir) -> usize {
        self.inner.size(score_director)
    }
}

pub struct ListMoveListRuinSelector<S, V> {
    inner: ListRuinMoveSelector<S, V>,
}

impl<S, V: Debug> Debug for ListMoveListRuinSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveListRuinSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V> ListMoveListRuinSelector<S, V> {
    pub fn new(inner: ListRuinMoveSelector<S, V>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveListRuinSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(ListMoveImpl::ListRuin)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
