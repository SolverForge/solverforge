mod bindings;
mod move_types;
mod selectors;

pub use bindings::descriptor_has_bindings;
pub(crate) use bindings::{collect_bindings, ResolvedVariableBinding};
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
