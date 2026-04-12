/* O(1) flattened bi-constraint stream.

Provides O(1) lookup for flattened items by pre-indexing C items by key.
*/

mod base;
mod builder;
mod weighting;

pub use base::FlattenedBiConstraintStream;
pub use builder::FlattenedBiConstraintBuilder;
