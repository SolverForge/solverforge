mod composite;
mod leaf;
mod move_union;

use crate::heuristic::selector::decorator::VecUnionSelector;

pub use composite::{
    CartesianChildCursor, CartesianChildSelector, Neighborhood, NeighborhoodCursor,
};
pub use leaf::{NeighborhoodLeaf, NeighborhoodLeafCursor};
pub use move_union::NeighborhoodMove;

pub(super) type LeafSelector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, NeighborhoodLeaf<S, V, DM, IDM>>;
