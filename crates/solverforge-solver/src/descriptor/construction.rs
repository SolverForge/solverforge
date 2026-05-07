mod build;
mod placer;

#[cfg(test)]
pub(crate) use build::build_descriptor_construction;
pub(crate) use build::build_descriptor_construction_from_bindings;
pub use placer::{DescriptorConstruction, DescriptorEntityPlacer};
