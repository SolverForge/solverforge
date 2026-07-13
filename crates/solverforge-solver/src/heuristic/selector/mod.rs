/* Selectors for entities, values, and moves.

Selectors enumerate the elements that the solver considers when
exploring the solution space.
*/

pub mod decorator;
pub mod dynamic_list_change;
pub mod dynamic_scalar_change;
pub mod dynamic_scalar_nearby_change;
pub mod dynamic_scalar_nearby_swap;
pub mod entity;
pub mod k_opt;
pub mod list_change;
pub(crate) mod list_kernel;
pub mod list_permute;
pub mod list_precedence;
pub mod list_reverse;
pub mod list_ruin;
pub(crate) mod list_support;
pub mod list_swap;
pub mod mimic;
pub mod move_selector;
pub mod nearby;
pub mod nearby_list_change;
pub(crate) mod nearby_list_support;
pub mod nearby_list_swap;
pub(crate) mod nearby_support;
pub mod pillar;
pub(crate) mod pillar_support;
pub(crate) mod precedence_route;
pub mod ruin;
pub(crate) mod scalar_neighborhood;
pub(crate) mod seed;
pub mod sublist_change;
mod sublist_support;
pub mod sublist_swap;
pub mod value_selector;

#[cfg(test)]
mod tests;

pub use dynamic_list_change::DynamicListChangeMoveSelector;
pub use dynamic_scalar_change::DynamicScalarChangeMoveSelector;
pub use dynamic_scalar_nearby_change::DynamicScalarNearbyChangeMoveSelector;
pub use dynamic_scalar_nearby_swap::DynamicScalarNearbySwapMoveSelector;
pub use entity::{
    AllEntitiesSelector, EntityReference, EntitySelector, FromSolutionEntitySelector,
};
pub use k_opt::{
    DefaultDistanceMeter, KOptConfig, KOptMoveSelector, ListPositionDistanceMeter,
    NearbyKOptMoveSelector,
};
pub use list_change::ListChangeMoveSelector;
pub use list_permute::ListPermuteMoveSelector;
pub use list_precedence::ListPrecedenceMoveSelector;
pub use list_reverse::ListReverseMoveSelector;
pub use list_ruin::ListRuinMoveSelector;
pub use list_swap::ListSwapMoveSelector;
pub use mimic::{MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector};
pub use move_selector::{
    ChangeMoveSelector, MoveSelector, MoveStreamContext, ScalarChangeMoveSelector,
    ScalarSwapMoveSelector, SwapMoveSelector,
};
pub use nearby::{NearbyDistanceMeter, NearbyEntitySelector, NearbySelectionConfig};
pub use nearby_list_change::{
    CrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter, NearbyListChangeMoveSelector,
};
pub use nearby_list_swap::NearbyListSwapMoveSelector;
pub use pillar::{DefaultPillarSelector, Pillar, PillarSelector, SubPillarConfig};
pub use ruin::{RuinMoveSelector, RuinVariableAccess};
pub use scalar_neighborhood::ScalarNeighborhoodBindingError;
pub use solverforge_config::SelectionOrder;
pub use sublist_change::SublistChangeMoveSelector;
pub use sublist_swap::SublistSwapMoveSelector;
pub use value_selector::{
    FromSolutionValueSelector, PerEntitySliceValueSelector, PerEntityValueSelector,
    RangeValueSelector, StaticValueSelector, ValueSelector,
};
