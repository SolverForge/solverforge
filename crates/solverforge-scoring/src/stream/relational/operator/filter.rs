use super::Operator;

/* A row predicate over a borrowed operator view. Named as a trait so a filter
node's predicate type need not be a bare closure in an operator's type. */
pub trait FilterPredicate<S, V>: Send + Sync {
    fn accepts(&self, solution: &S, row: &V) -> bool;
}

impl<S, V, F> FilterPredicate<S, V> for F
where
    F: Fn(&S, &V) -> bool + Send + Sync,
{
    #[inline]
    fn accepts(&self, solution: &S, row: &V) -> bool {
        self(solution, row)
    }
}
use crate::stream::relational::{HandleMap, RowHandle};

/// Membership filter retaining input identities, not copied row payloads.
pub struct FilterNode<O, F> {
    input: O,
    predicate: F,
    accepted: HandleMap<()>,
    /// Child notification scratch, reused across events.
    child_removed: Vec<RowHandle>,
    child_inserted: Vec<RowHandle>,
}
impl<O, F> FilterNode<O, F> {
    pub fn new(input: O, predicate: F) -> Self {
        Self {
            input,
            predicate,
            accepted: HandleMap::new(),
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
        }
    }
}
impl<O, F> FilterNode<O, F> {
    #[inline(always)]
    fn apply_changes<S: 'static>(
        &mut self,
        solution: &S,
        input_removed: &[RowHandle],
        input_inserted: &[RowHandle],
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) where
        O: Operator<S>,
        F: for<'a> FilterPredicate<S, O::View<'a>> + 'static,
    {
        for h in input_removed.iter().copied() {
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        for h in input_inserted.iter().copied() {
            let row = self
                .input
                .resolve(solution, h)
                .expect("inserted filtered input");
            if self.predicate.accepts(solution, &row) {
                self.accepted.insert(h, ());
                inserted.push(h);
            }
        }
    }
}
impl<S: 'static, O, F> Operator<S> for FilterNode<O, F>
where
    O: Operator<S>,
    F: for<'a> FilterPredicate<S, O::View<'a>> + 'static,
{
    type View<'a> = O::View<'a>;
    type Evaluation = O::Evaluation;
    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.input.prepare_evaluation(solution)
    }
    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.input
            .visit_evaluation(solution, evaluation, &mut |row| {
                if self.predicate.accepts(solution, &row) {
                    visitor(row);
                }
            });
    }
    fn clear(&mut self) {
        self.input.clear();
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
    }
    #[inline(always)]
    fn initialize(&mut self, solution: &S) {
        self.input.initialize(solution);
        self.accepted.clear();
        for h in self.input.handles() {
            let row = self
                .input
                .resolve(solution, h)
                .expect("live filtered input");
            if self.predicate.accepts(solution, &row) {
                self.accepted.insert(h, ());
            }
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.input
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(handle)?;
        self.input.resolve(solution, handle)
    }
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(handle).is_some() {
            self.input.visit_provenance(handle, visitor);
        }
    }
    #[inline(always)]
    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut in_removed = std::mem::take(&mut self.child_removed);
        let mut in_inserted = std::mem::take(&mut self.child_inserted);
        in_removed.clear();
        in_inserted.clear();
        self.input.retract(
            solution,
            descriptor,
            index,
            &mut in_removed,
            &mut in_inserted,
        );
        self.apply_changes(solution, &in_removed, &in_inserted, removed, inserted);
        self.child_removed = in_removed;
        self.child_inserted = in_inserted;
    }
    #[inline(always)]
    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut in_removed = std::mem::take(&mut self.child_removed);
        let mut in_inserted = std::mem::take(&mut self.child_inserted);
        in_removed.clear();
        in_inserted.clear();
        self.input.insert(
            solution,
            descriptor,
            index,
            &mut in_removed,
            &mut in_inserted,
        );
        self.apply_changes(solution, &in_removed, &in_inserted, removed, inserted);
        self.child_removed = in_removed;
        self.child_inserted = in_inserted;
    }
}
