use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_config::RecreateHeuristicType;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};

use crate::builder::context::ConstructionValueOrderKey;
use crate::heuristic::r#move::metadata::{
    append_canonical_usize_slice_pair, encode_option_usize, encode_usize, hash_str,
    ordered_coordinate_pair, scoped_move_identity, MoveTabuScope, ScopedEntityTabuToken,
    TABU_OP_PILLAR_SWAP, TABU_OP_SWAP,
};
use crate::heuristic::r#move::{Move, MoveTabuSignature, SequentialCompositeMove};

use super::bindings::VariableBinding;

include!("move_types/change.rs");
include!("move_types/swap.rs");
include!("move_types/pillar_change.rs");
include!("move_types/pillar_swap.rs");
include!("move_types/ruin_recreate.rs");
include!("move_types/union.rs");
