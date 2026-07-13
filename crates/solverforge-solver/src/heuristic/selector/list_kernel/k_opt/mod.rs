//! Shared exhaustive and nearby K-opt selector kernels.

mod emission;
mod full;
mod nearby;
mod nearby_state;

pub(crate) use emission::{KOptEmitter, NativeKOptEmitter};
pub(crate) use full::KOptCursor;
pub(crate) use nearby::NearbyKOptCursor;
pub(crate) use nearby_state::{KOptDistanceProbe, NearbyCutState};
