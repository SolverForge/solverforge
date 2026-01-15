//! Domain model traits for defining planning problems
//!
//! These traits define the structure of a planning problem:
//! - `PlanningSolution`: The container for the complete problem and solution
//! - `PlanningEntity`: Things that can be planned/optimized
//! - `ProblemFact`: Immutable input data
//! - `PlanningId`: Unique identification for entities

mod descriptor;
mod entity_ref;
pub mod listener;
pub mod supply;
mod traits;
mod value_range;
mod variable;

pub use descriptor::{
    EntityDescriptor, ProblemFactDescriptor, SolutionDescriptor, VariableDescriptor,
};
pub use entity_ref::{EntityExtractor, EntityRef, TypedEntityExtractor};
pub use listener::{
    ListVariableListener, ListVariableNotification, VariableListener, VariableNotification,
};
pub use supply::{AnchorSupply, ElementPosition, InverseSupply, ListStateSupply};
pub use traits::{ListVariableSolution, PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
pub use value_range::{
    ComputedValueRangeProvider, FieldValueRangeProvider, IntegerRange, StaticValueRange,
    ValueRangeProvider,
};
pub use variable::{ChainedVariableInfo, ShadowVariableKind, ValueRangeType, VariableType};
