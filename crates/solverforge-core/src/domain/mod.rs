//! Domain model traits for defining planning problems
//!
//! These traits define the structure of a planning problem:
//! - `PlanningSolution`: The container for the complete problem and solution
//! - `PlanningEntity`: Things that can be planned/optimized
//! - `ProblemFact`: Immutable input data
//! - `PlanningId`: Unique identification for entities

mod traits;
mod descriptor;
mod variable;
mod value_range;
mod entity_ref;
pub mod supply;
pub mod listener;

pub use traits::{PlanningSolution, PlanningEntity, ProblemFact, PlanningId};
pub use descriptor::{
    SolutionDescriptor, EntityDescriptor, VariableDescriptor,
    ProblemFactDescriptor,
};
pub use variable::{VariableType, ShadowVariableKind, ValueRangeType, ChainedVariableInfo};
pub use entity_ref::{EntityRef, EntityExtractor, TypedEntityExtractor};
pub use supply::{
    Supply, SupplyDemand, SupplyManager, DemandKey,
    SingletonInverseVariableSupply, ExternalizedSingletonInverseVariableSupply,
    AnchorVariableSupply, ExternalizedAnchorVariableSupply,
    ListVariableStateSupply, ListVariableStateDemand, ElementPosition,
    IndexVariableSupply, InverseVariableSupply,
};
pub use listener::{
    VariableListener, ListVariableListener,
    VariableNotification, ListVariableNotification,
};
pub use value_range::{
    ValueRangeProvider,
    FieldValueRangeProvider, ComputedValueRangeProvider,
    StaticValueRange, IntegerRange,
};
