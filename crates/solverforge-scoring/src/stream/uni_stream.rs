/* Zero-erasure uni-constraint stream for single-entity constraint patterns.

A `UniConstraintStream` operates on a single entity type and supports
filtering, weighting, and constraint finalization. All type information
is preserved at compile time - no Arc, no dyn, fully monomorphized.
*/

mod base;
mod weighting;

pub use base::UniConstraintStream;
pub use weighting::UniConstraintBuilder;
