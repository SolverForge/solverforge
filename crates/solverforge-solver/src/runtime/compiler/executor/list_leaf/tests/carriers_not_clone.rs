//! Compile-time guard: the production move carriers and the frozen access they
//! borrow must not implement `Clone`, so a per-candidate copy cannot be
//! reintroduced by calling `.clone()` on them.

use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter as Meter;
use crate::heuristic::selector::scalar_neighborhood::spec::RuntimeScalarRecipe;
use crate::heuristic::selector::scalar_neighborhood::RuntimeScalarMove;
use crate::runtime::compiler::executor::list_leaf::move_access::RuntimeListMoveAccess;
use crate::runtime::compiler::executor::local_search::RuntimeNeighborhoodMove;

use super::support::ListPlan;

/// Resolves to one impl for non-`Clone` types and becomes ambiguous (a
/// compile error) for `Clone` types.
trait AmbiguousIfClone<Marker> {
    fn check() {}
}

impl<T> AmbiguousIfClone<()> for T {}

struct IsClone;

impl<T: Clone> AmbiguousIfClone<IsClone> for T {}

#[test]
fn move_carriers_and_borrowed_access_are_not_clone() {
    <RuntimeListMoveAccess<ListPlan, usize> as AmbiguousIfClone<_>>::check();
    <RuntimeScalarRecipe<'static, ListPlan> as AmbiguousIfClone<_>>::check();
    <RuntimeScalarMove<'static, ListPlan> as AmbiguousIfClone<_>>::check();
    <RuntimeNeighborhoodMove<'static, ListPlan, usize, Meter, Meter> as AmbiguousIfClone<_>>::check(
    );
}
