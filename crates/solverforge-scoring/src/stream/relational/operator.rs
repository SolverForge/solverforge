// Concrete row producer protocol. Views never survive a solution/update borrow.
use super::RowHandle;

mod analysis;
mod changes;
pub use changes::RowChanges;
mod collection;
mod existence;
mod filter;
pub use existence::ExistenceNode;
mod group;
mod group_view;
pub use group::{GroupEvaluation, GroupNode};
pub use group_view::GroupView;
mod merge;
mod project;
pub use merge::MergeNode;
pub use project::{ProjectEvaluation, ProjectNode, ProjectView};
mod join;
pub use filter::FilterNode;

#[doc(hidden)]
pub use analysis::ExplainRow;
pub use collection::CollectionNode;
pub use join::JoinNode;

#[derive(Clone, Copy, Debug)]
pub struct Pair<L, R> {
    pub left: L,
    pub right: R,
}

pub trait Operator<S: 'static>: 'static {
    type View<'a>: Copy;
    /// Fresh owned derived values for one full evaluation, independent of retained state.
    type Evaluation;
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation;
    /// Views may borrow the evaluation owner, but never outlive it.
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    );
    /// Streaming root traversal. Only producers of owned values materialize payloads.
    #[inline]
    fn visit_all(&self, solution: &S, visitor: &mut impl for<'a> FnMut(Self::View<'a>)) {
        let evaluation = self.prepare_evaluation(solution);
        self.visit_evaluation(solution, &evaluation, visitor);
    }
    fn clear(&mut self);
    fn initialize(&mut self, solution: &S);
    fn handles(&self) -> Vec<RowHandle>;
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>>;
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize));
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges;
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges;
}
