//! Scoring operator for the flattened bi-constraint.
//!
//! The join tree publishes `Pair<Leaf<A>, FlattenView<Leaf<B>, C>>`: the left
//! entity plus the flattened child with its owning B row. The authored bi filter
//! runs inside this operator on `(A, C)` with the A and owning-B source indexes,
//! so evaluate, match_count, initialize, and mutations honor it exactly once and
//! scoring is the generic `OperatorTerminal`.

use std::marker::PhantomData;

use super::super::filter::BiFilter;
use super::super::relational::operator::{FlattenView, Operator, Pair};
use super::super::relational::{HandleMap, Leaf, RowHandle};

/// Borrowed (A, C) pair plus the A and owning-B source indexes.
pub(super) trait FlattenedBiEntities<'a, A, B, C> {
    fn entities(&self) -> (&'a A, &'a C);
    fn indexes(&self) -> (usize, usize);
}

impl<'a, A, B, C> FlattenedBiEntities<'a, A, B, C>
    for Pair<Leaf<'a, A>, FlattenView<'a, Leaf<'a, B>, C>>
{
    #[inline]
    fn entities(&self) -> (&'a A, &'a C) {
        (self.left.entity, self.right.value)
    }
    #[inline]
    fn indexes(&self) -> (usize, usize) {
        (self.left.index, self.right.input.index)
    }
}

pub struct FlattenedBiScored<S, A, B, C, O, F> {
    inner: O,
    filter: F,
    accepted: HandleMap<()>,
    /// Child notification scratch, reused across events.
    child_removed: Vec<RowHandle>,
    child_inserted: Vec<RowHandle>,
    marker: PhantomData<fn() -> (S, A, B, C)>,
}

impl<S, A, B, C, O, F> FlattenedBiScored<S, A, B, C, O, F> {
    pub(super) fn new(inner: O, filter: F) -> Self {
        Self {
            inner,
            filter,
            accepted: HandleMap::new(),
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn test<'a>(
        &self,
        solution: &S,
        row: &Pair<Leaf<'a, A>, FlattenView<'a, Leaf<'a, B>, C>>,
    ) -> bool
    where
        S: 'static,
        F: BiFilter<S, A, C>,
    {
        let (a, c) = row.entities();
        let (a_idx, b_idx) = row.indexes();
        self.filter.test(solution, a, c, a_idx, b_idx)
    }
}

impl<S, A, B, C, O, F> Operator<S> for FlattenedBiScored<S, A, B, C, O, F>
where
    S: Send + Sync + 'static,
    A: 'static,
    B: 'static,
    C: 'static,
    O: Operator<S>,
    for<'a> O: Operator<S, View<'a> = Pair<Leaf<'a, A>, FlattenView<'a, Leaf<'a, B>, C>>>,
    F: BiFilter<S, A, C> + 'static,
{
    type View<'a> = O::View<'a>;
    type Evaluation = O::Evaluation;

    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.inner.prepare_evaluation(solution)
    }

    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.inner
            .visit_evaluation(solution, evaluation, &mut |row| {
                if self.test(solution, &row) {
                    visitor(row);
                }
            });
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.inner.initialize(solution);
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
        for handle in self.inner.handles() {
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live flattened row");
            if self.test(solution, &row) {
                self.accepted.insert(handle, ());
            }
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.inner
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }

    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(handle)?;
        self.inner.resolve(solution, handle)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(handle).is_some() {
            self.inner.visit_provenance(handle, visitor);
        }
    }

    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        _inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        let inner_removed = std::mem::take(&mut self.child_removed);
        let inner_inserted = std::mem::take(&mut self.child_inserted);
        let mut inner_removed = inner_removed;
        let mut inner_inserted = inner_inserted;
        inner_removed.clear();
        inner_inserted.clear();
        self.inner.retract(
            solution,
            descriptor,
            index,
            &mut inner_removed,
            &mut inner_inserted,
        );
        for h in inner_removed.drain(..) {
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        self.child_removed = inner_removed;
        self.child_inserted = inner_inserted;
    }

    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        _removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let mut inner_removed = std::mem::take(&mut self.child_removed);
        let mut inner_inserted = std::mem::take(&mut self.child_inserted);
        inner_removed.clear();
        inner_inserted.clear();
        self.inner.insert(
            solution,
            descriptor,
            index,
            &mut inner_removed,
            &mut inner_inserted,
        );
        for handle in inner_inserted.drain(..) {
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live scored row");
            if self.test(solution, &row) {
                self.accepted.insert(handle, ());
                inserted.push(handle);
            }
        }
        self.child_removed = inner_removed;
        self.child_inserted = inner_inserted;
    }
}

impl<S, A, B, C, O, F> std::fmt::Debug for FlattenedBiScored<S, A, B, C, O, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlattenedBiScored").finish()
    }
}
