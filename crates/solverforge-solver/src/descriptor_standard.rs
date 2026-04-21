mod bindings;
mod construction;
mod frontier;
mod move_types;
mod selectors;

pub(crate) use bindings::standard_work_remaining_with_frontier;
pub use bindings::{descriptor_has_bindings, standard_target_matches, standard_work_remaining};
pub use construction::{
    build_descriptor_construction, DescriptorConstruction, DescriptorEntityPlacer,
};
pub(crate) use frontier::ConstructionFrontier;
pub use move_types::{DescriptorChangeMove, DescriptorEitherMove, DescriptorSwapMove};
pub use selectors::{
    build_descriptor_move_selector, DescriptorChangeMoveSelector, DescriptorLeafSelector,
    DescriptorSwapMoveSelector,
};

#[cfg(test)]
mod tests;
