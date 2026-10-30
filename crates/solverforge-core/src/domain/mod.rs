//! Domain model traits for defining planning problems
//!
//! These traits define the structure of a planning problem:
//! - `PlanningSolution`: The container for the complete problem and solution
//! - `PlanningEntity`: Things that can be planned/optimized
//! - `ProblemFact`: Immutable input data
//! - `PlanningId`: Unique identification for entities

mod descriptor;
mod dynamic;
mod entity_ref;
mod traits;
mod variable;

#[cfg(test)]
mod tests;

pub use descriptor::{
    EntityClassId, EntityDescriptor, ProblemFactClassId, ProblemFactDescriptor, SolutionDescriptor,
    UsizeCandidateValues, UsizeConstructionEntityOrderKey, UsizeConstructionValueOrderKey,
    UsizeEntityValueProvider, UsizeGetter, UsizeNearbyEntityDistanceMeter,
    UsizeNearbyValueDistanceMeter, UsizeSearchEntityOrderKey, UsizeSearchValueOrderKey,
    UsizeSetter, VariableDescriptor, VariableId,
};
pub use dynamic::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, DynamicModelBackend,
    DynamicScalarAccess, DynamicScalarAssignmentMetadata,
    DynamicScalarAssignmentMetadataCapabilities, DynamicScalarVariableSlot,
};
pub use entity_ref::{EntityCollectionExtractor, EntityExtractor, EntityRef};
pub use traits::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
pub use variable::{ShadowVariableKind, ValueRangeType, VariableType};
