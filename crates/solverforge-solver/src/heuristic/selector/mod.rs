//! Selectors for entities, values, and moves.
//!
//! Selectors enumerate the elements that the solver considers when
//! exploring the solution space.

pub mod decorator;
pub mod entity;
pub mod k_opt;
pub mod list_change;
pub mod list_ruin;
pub mod mimic;
pub mod nearby;
pub mod pillar;
pub mod ruin;
mod selection_order;
pub mod typed_move_selector;
pub mod typed_value;

pub use entity::{
    AllEntitiesSelector, EntityReference, EntitySelector, FromSolutionEntitySelector,
};
pub use k_opt::{KOptConfig, KOptMoveSelector, ListPositionDistanceMeter, NearbyKOptMoveSelector};
pub use list_change::ListChangeMoveSelector;
pub use list_ruin::ListRuinMoveSelector;
pub use mimic::{MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector};
pub use nearby::{NearbyDistanceMeter, NearbyEntitySelector, NearbySelectionConfig};
pub use pillar::{DefaultPillarSelector, Pillar, PillarSelector, SubPillarConfig};
pub use ruin::RuinMoveSelector;
pub use selection_order::SelectionOrder;
pub use typed_move_selector::{ChangeMoveSelector, MoveSelector, SwapMoveSelector};
pub use typed_value::{
    FromSolutionTypedValueSelector, StaticTypedValueSelector, TypedValueSelector,
};
