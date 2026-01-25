//! Selectors for entities, values, and moves.
//!
//! Selectors enumerate the elements that the solver considers when
//! exploring the solution space.

pub mod decorator;
pub mod entity;
pub mod k_opt;
pub mod mimic;
pub mod nearby;
pub mod pillar;
mod selection_order;
pub mod selector_impl;
pub mod typed_move_selector;
pub mod typed_value;

pub use entity::{
    AllEntitiesSelector, EntityReference, EntitySelector, FromSolutionEntitySelector,
};
pub use k_opt::{DefaultDistanceMeter, KOptConfig, ListPositionDistanceMeter};
pub use mimic::{MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector};
pub use nearby::{NearbyDistanceMeter, NearbyEntitySelector, NearbySelectionConfig};
pub use pillar::{DefaultPillarSelector, Pillar, PillarSelector, SubPillarConfig};
pub use selection_order::SelectionOrder;
pub use selector_impl::{BasicVariableFnPtrs, ListVariableFnPtrs, MoveSelectorImpl};
pub use typed_move_selector::MoveSelector;
pub use typed_value::{
    FromSolutionTypedValueSelector, RangeValueSelector, StaticTypedValueSelector,
    TypedValueSelector,
};
