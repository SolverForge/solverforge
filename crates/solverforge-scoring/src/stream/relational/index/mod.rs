/* Concrete index strategies for relational operators.

No runtime type-erased index registry: each strategy is a concrete
generic type chosen by the condition plan at compile time. Equality uses
`HashIndex`; ordered and interval strategies arrive with comparison and
overlap conditions. Predicate scans need no index and stay explicit.
*/

mod hash;
mod interval;
mod ordered;

pub(crate) use hash::HashIndex;
pub(crate) use interval::IntervalIndex;
pub(crate) use ordered::OrderedIndex;
