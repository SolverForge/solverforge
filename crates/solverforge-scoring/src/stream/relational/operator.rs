// Concrete row producer protocol. Views never survive a solution/update borrow.
use super::RowHandle;

mod analysis;
mod collection;
mod complement;
mod complement_view;
pub use complement::{ComplementEvaluation, ComplementNode};
pub use complement_view::ComplementView;
mod existence;
mod filter;
mod flatten;
mod joiner_existence;
pub use existence::{ExistenceNode, GroupRightKeys, ParentChildKeys, RightKeySet, UnitKeys};
pub use flatten::{FlattenNode, FlattenSource, FlattenView, ParentFlatten};
pub use joiner_existence::JoinerExistenceNode;
mod group;
mod group_view;
pub use group::{GroupEvaluation, GroupNode};
pub use group_view::GroupView;
mod merge;
mod project;
mod self_join;
pub use merge::MergeNode;
pub use project::{ProjectEvaluation, ProjectNode, ProjectView};
mod join;
mod links;
pub use filter::{FilterNode, FilterPredicate};
pub use self_join::SelfJoinNode;

#[doc(hidden)]
pub use analysis::ExplainRow;
pub use collection::CollectionNode;
pub use join::JoinNode;

#[derive(Clone, Copy, Debug)]
pub struct Pair<L, R> {
    pub left: L,
    pub right: R,
}

/* The four input slices of one join-shaped event. Passed by value; no heap. */
pub(crate) struct PairDelta<'a> {
    pub left_removed: &'a [RowHandle],
    pub left_inserted: &'a [RowHandle],
    pub right_removed: &'a [RowHandle],
    pub right_inserted: &'a [RowHandle],
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
    /* Incremental notifications, emitted into caller-owned buffers.

    The caller passes empty lists and reuses them across events; nodes append
    their retracted and inserted row handles without allocating. Every node
    chains its own scratch pairs through its children, so a full event costs
    one copy per hop and no heap traffic. */
    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    );
    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    );
}
