/* Typed condition planning: concrete indexes and exact residual matching.

Equality conjunctions normalize into one heterogeneous composite hash key,
including equality components separated by predicates or range conditions.
Mixed plans index equality and retain the authored residual predicates.
Ordered and interval plans execute their concrete candidate indexes; arbitrary
predicates explicitly scan. No closure inspection or dynamic condition list.
*/

use super::match_condition::AndJoiner;
use super::{EqualJoiner, FilteringJoiner};
use super::{GreaterThanJoiner, GreaterThanOrEqualJoiner, LessThanJoiner, LessThanOrEqualJoiner};
use super::{Joiner, OverlappingJoiner};
mod composite;
mod conjunction;
pub use composite::{
    ComposePlans, CompositeEquality, EqualityKeys, EqualityKind, EqualityPlan, ResidualKind,
};
mod executable;
mod hash_scan;
mod mixed_equality;
pub use mixed_equality::EqualityWithResidual;
mod ordered_interval;
pub use executable::{CompileCondition, ExecutablePlan, IndexedPlan};

/* Deterministic execution strategy compiled from a condition's structure. */
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /* Typed hash indexes on both inputs plus exact equality residual. */
    EquiHash,
    /* Typed ordered bounds plus exact strict/inclusive residual. */
    OrderedScan,
    /* Interval candidate narrowing plus exact half-open predicate. */
    IntervalNarrow,
    /* Explicit opposite-input scan with the exact predicate. */
    Scan,
}

impl Strategy {
    /* Prefer the most selective usable structural index; retain all residuals.
    Equality > ordered > interval > explicit scan. */
    #[inline]
    pub const fn combine(self, other: Strategy) -> Strategy {
        match (self, other) {
            (Strategy::EquiHash, _) | (_, Strategy::EquiHash) => Strategy::EquiHash,
            (Strategy::OrderedScan, _) | (_, Strategy::OrderedScan) => Strategy::OrderedScan,
            (Strategy::IntervalNarrow, _) | (_, Strategy::IntervalNarrow) => {
                Strategy::IntervalNarrow
            }
            (Strategy::Scan, Strategy::Scan) => Strategy::Scan,
        }
    }
}

/* A condition with a statically planned execution strategy.

`check` is the exact residual semantic check over one borrowed left row
and one right entity — the same predicate `matches` evaluates, exposed
so operators apply residuals without re-dispatching through `Joiner`.
*/
pub trait PlannedCondition<Row, C>: Send + Sync {
    /* The deterministic strategy for this condition's structure. */
    const STRATEGY: Strategy;

    /* Exact residual check: must agree with `Joiner::matches`. */
    fn check(&self, row: &Row, c: &C) -> bool;
}

/* Reads the planned strategy for one condition value. */
#[inline]
pub fn plan_strategy<Row, C, P>(condition: &P) -> Strategy
where
    P: PlannedCondition<Row, C>,
{
    let _ = condition;
    P::STRATEGY
}

impl<Row, C, Fa, Fb, T, Mode> PlannedCondition<Row, C> for EqualJoiner<Fa, Fb, T, Mode>
where
    T: PartialEq,
    Fa: Fn(&Row) -> T + Send + Sync,
    Fb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::EquiHash;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, Fa, Fb, T> PlannedCondition<Row, C> for LessThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&Row) -> T + Send + Sync,
    Fb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::OrderedScan;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, Fa, Fb, T> PlannedCondition<Row, C> for LessThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&Row) -> T + Send + Sync,
    Fb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::OrderedScan;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, Fa, Fb, T> PlannedCondition<Row, C> for GreaterThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&Row) -> T + Send + Sync,
    Fb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::OrderedScan;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, Fa, Fb, T> PlannedCondition<Row, C> for GreaterThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&Row) -> T + Send + Sync,
    Fb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::OrderedScan;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, Fsa, Fea, Fsb, Feb, T> PlannedCondition<Row, C>
    for OverlappingJoiner<Fsa, Fea, Fsb, Feb, T>
where
    T: Ord,
    Fsa: Fn(&Row) -> T + Send + Sync,
    Fea: Fn(&Row) -> T + Send + Sync,
    Fsb: Fn(&C) -> T + Send + Sync,
    Feb: Fn(&C) -> T + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::IntervalNarrow;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, F> PlannedCondition<Row, C> for FilteringJoiner<F>
where
    F: Fn(&Row, &C) -> bool + Send + Sync,
{
    const STRATEGY: Strategy = Strategy::Scan;

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}

impl<Row, C, J1, J2> PlannedCondition<Row, C> for AndJoiner<J1, J2>
where
    J1: PlannedCondition<Row, C> + Joiner<Row, C>,
    J2: PlannedCondition<Row, C> + Joiner<Row, C>,
{
    const STRATEGY: Strategy = J1::STRATEGY.combine(J2::STRATEGY);

    #[inline]
    fn check(&self, row: &Row, c: &C) -> bool {
        self.matches(row, c)
    }
}
