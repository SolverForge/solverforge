//! Dynamic constraint system using expression trees with true incremental scoring.

#[cfg(test)]
mod tests;

// Module organization: types, closures, factories, and stream operations
mod closures_bi;
mod closures_cross;
mod closures_extract;
mod closures_flattened;
mod closures_penta;
mod closures_quad;
mod closures_tri;
mod factory_cross;
mod factory_self;
mod factory_self_higher;
mod factory_uni;
mod stream_ops;
mod stream_parser;
mod types;

// Re-export type aliases
pub use types::{
    DynALookup, DynBiFilter, DynBiWeight, DynCKeyFn, DynCrossExtractorA, DynCrossExtractorB,
    DynCrossFilter, DynCrossKeyA, DynCrossKeyB, DynCrossWeight, DynExtractor, DynFlatten,
    DynFlattenedFilter, DynFlattenedWeight, DynKeyExtractor, DynPentaFilter, DynPentaWeight,
    DynQuadFilter, DynQuadWeight, DynTriFilter, DynTriWeight, DynUniFilter, DynUniWeight,
};

// Re-export closure builders
pub use closures_bi::{make_bi_filter, make_bi_weight};
pub use closures_cross::{
    make_cross_extractor_a, make_cross_extractor_b, make_cross_filter, make_cross_key_a,
    make_cross_key_b, make_cross_weight,
};
pub use closures_extract::{make_extractor, make_key_extractor};
pub use closures_flattened::{
    make_a_lookup, make_c_key_fn, make_flatten, make_flattened_filter, make_flattened_weight,
};
pub use closures_penta::{make_penta_filter, make_penta_weight};
pub use closures_quad::{make_quad_filter, make_quad_weight};
pub use closures_tri::{make_tri_filter, make_tri_weight};

// Re-export factory functions
pub use factory_cross::{build_cross_bi_constraint, build_flattened_bi_constraint};
pub use factory_self::{build_bi_self_constraint, build_tri_self_constraint};
pub use factory_self_higher::{build_penta_self_constraint, build_quad_self_constraint};
pub use factory_uni::build_uni_constraint;

// Re-export stream operations
pub use stream_ops::{build_from_stream_ops, StreamOp};
