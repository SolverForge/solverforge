//! Selectors for entities, values, and moves.
//!
//! Selectors enumerate the elements that the solver considers when
//! exploring the solution space.

pub mod decorator;
pub mod entity;
pub mod k_opt;
pub mod list_change;
pub mod list_reverse;
pub mod list_ruin;
pub mod list_swap;
pub mod mimic;
pub mod nearby;
pub mod nearby_list_change;
pub mod nearby_list_swap;
pub mod pillar;
pub mod ruin;
mod selection_order;
pub mod sublist_change;
pub mod sublist_swap;
pub mod typed_move_selector;
pub mod typed_value;

#[cfg(test)]
mod tests;

pub use entity::{
    AllEntitiesSelector, EntityReference, EntitySelector, FromSolutionEntitySelector,
};
pub use k_opt::{
    DefaultDistanceMeter, KOptConfig, KOptMoveSelector, ListPositionDistanceMeter,
    NearbyKOptMoveSelector,
};
pub use list_change::ListChangeMoveSelector;
pub use list_reverse::{ListMoveListReverseSelector, ListReverseMoveSelector};
pub use list_ruin::ListRuinMoveSelector;
pub use list_swap::{ListMoveListSwapSelector, ListSwapMoveSelector};
pub use mimic::{MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector};
pub use nearby::{NearbyDistanceMeter, NearbyEntitySelector, NearbySelectionConfig};
pub use nearby_list_change::{
    CrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter, ListMoveNearbyListChangeSelector,
    NearbyListChangeMoveSelector,
};
pub use nearby_list_swap::{ListMoveNearbyListSwapSelector, NearbyListSwapMoveSelector};
pub use pillar::{DefaultPillarSelector, Pillar, PillarSelector, SubPillarConfig};
pub use ruin::RuinMoveSelector;
pub use selection_order::SelectionOrder;
pub use sublist_change::{ListMoveSubListChangeSelector, SubListChangeMoveSelector};
pub use sublist_swap::{ListMoveSubListSwapSelector, SubListSwapMoveSelector};
pub use typed_move_selector::{
    ChangeMoveSelector, EitherChangeMoveSelector, EitherSwapMoveSelector, ListMoveKOptSelector,
    ListMoveListChangeSelector, ListMoveListRuinSelector, MoveSelector, SwapMoveSelector,
};
pub use typed_value::{
    FromSolutionTypedValueSelector, StaticTypedValueSelector, TypedValueSelector,
};
