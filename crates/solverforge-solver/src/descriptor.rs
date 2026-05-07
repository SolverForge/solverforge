mod bindings;
mod construction;
mod move_types;
mod selectors;

pub(crate) use bindings::{
    collect_bindings, find_resolved_binding, scalar_work_remaining_with_frontier,
    ResolvedVariableBinding,
};
pub use bindings::{descriptor_has_bindings, scalar_target_matches, scalar_work_remaining};
#[cfg(test)]
pub(crate) use construction::build_descriptor_construction;
pub(crate) use construction::build_descriptor_construction_from_bindings;
pub use construction::{DescriptorConstruction, DescriptorEntityPlacer};
pub use move_types::{
    DescriptorChangeMove, DescriptorMoveUnion, DescriptorPillarChangeMove,
    DescriptorPillarSwapMove, DescriptorRuinRecreateMove, DescriptorSwapMove,
};
pub use selectors::{
    build_descriptor_move_selector, DescriptorChangeMoveSelector, DescriptorFlatSelector,
    DescriptorLeafSelector, DescriptorSelector, DescriptorSelectorNode, DescriptorSwapMoveSelector,
};

#[cfg(test)]
mod tests;
