// Joiner functions for constraint stream joins.

mod comparison;
mod equal;
mod filtering;
mod match_condition;
mod overlapping;
#[doc(hidden)]
pub mod plan;
mod row_key;

pub use comparison::{
    greater_than, greater_than_on, greater_than_or_equal, greater_than_or_equal_on, less_than,
    less_than_on, less_than_or_equal, less_than_or_equal_on, GreaterThanJoiner,
    GreaterThanOrEqualJoiner, LessThanJoiner, LessThanOrEqualJoiner,
};
pub use equal::{equal, equal_bi, equal_on, Directed, EqualJoiner, Symmetric};
pub use filtering::{filtering, filtering_on, FilteringJoiner};
pub use match_condition::{AndJoiner, FnJoiner, Joiner};
pub use overlapping::{overlap_on, overlapping, OverlappingJoiner};
pub use plan::{plan_strategy, PlannedCondition, Strategy};
pub use row_key::pair_row;
