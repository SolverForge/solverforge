/* Zero-erasure constraint API.

This module provides:
- ConstraintSet trait for tuple-based constraint evaluation
- IncrementalConstraint trait for incremental scoring
- Analysis types for score explanation
- Runtime weight override configuration
*/

pub mod analysis;
pub mod constraint_set;
pub mod node_sharing;
pub mod weight_overrides;

#[cfg(test)]
mod tests;

pub use analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};
pub use constraint_set::{
    ConstraintMetadata, ConstraintSet, ConstraintSetChain, ConstraintSetSource,
    IncrementalConstraint, IncrementalConstraintSealed, OrderedConstraintSetChain,
};
pub use node_sharing::{SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation};
pub use weight_overrides::{ConstraintWeightOverrides, WeightProvider};
