/* Zero-erasure grouped constraint stream for group-by constraint patterns.

A `GroupedConstraintStream` operates on groups of entities and supports
filtering, weighting, and constraint finalization.
All type information is preserved at compile time - no Arc, no dyn.
*/

mod base;
mod weighting;

pub use base::GroupedConstraintStream;
pub use weighting::GroupedConstraintBuilder;
