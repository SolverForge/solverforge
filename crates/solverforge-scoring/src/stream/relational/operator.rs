// Concrete row producer protocol. Views never survive a solution/update borrow.
use super::RowHandle;

mod analysis;
mod collection;
mod filter;
mod merge;
pub use merge::MergeNode;
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
    fn visit_all<'a>(&'a self, solution: &'a S, visitor: &mut impl FnMut(Self::View<'a>));
    fn clear(&mut self);
    fn initialize(&mut self, solution: &S);
    fn handles(&self) -> Vec<RowHandle>;
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>>;
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize));
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle>;
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle>;
}
