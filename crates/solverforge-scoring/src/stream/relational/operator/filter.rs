use super::Operator;
use crate::stream::relational::{HandleMap, RowHandle};

/// Membership filter retaining input identities, not copied row payloads.
pub struct FilterNode<O, F> {
    input: O,
    predicate: F,
    accepted: HandleMap<()>,
}
impl<O, F> FilterNode<O, F> {
    pub fn new(input: O, predicate: F) -> Self {
        Self {
            input,
            predicate,
            accepted: HandleMap::new(),
        }
    }
}
impl<S: 'static, O, F> Operator<S> for FilterNode<O, F>
where
    O: Operator<S>,
    F: for<'a> Fn(&S, &O::View<'a>) -> bool + 'static,
{
    type View<'a> = O::View<'a>;
    #[inline]
    fn visit_all<'a>(&'a self, solution: &'a S, visitor: &mut impl FnMut(Self::View<'a>)) {
        self.input.visit_all(solution, &mut |row| {
            if (self.predicate)(solution, &row) {
                visitor(row);
            }
        });
    }
    fn clear(&mut self) {
        self.input.clear();
        self.accepted.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.input.initialize(solution);
        self.accepted.clear();
        for h in self.input.handles() {
            let row = self
                .input
                .resolve(solution, h)
                .expect("live filtered input");
            if (self.predicate)(solution, &row) {
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
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        self.input
            .retract(solution, descriptor, index)
            .into_iter()
            .filter(|h| self.accepted.remove(*h).is_some())
            .collect()
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let inserted = self.input.insert(solution, descriptor, index);
        let mut accepted = Vec::new();
        for h in inserted {
            let row = self
                .input
                .resolve(solution, h)
                .expect("inserted filtered input");
            if (self.predicate)(solution, &row) {
                self.accepted.insert(h, ());
                accepted.push(h);
            }
        }
        accepted
    }
}
