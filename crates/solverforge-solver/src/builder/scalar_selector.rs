use std::fmt::{self, Debug};
use std::ops::Range;

use solverforge_config::{MoveSelectorConfig, RecreateHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{
    ChangeMove, Move, MoveArena, PillarChangeMove, PillarSwapMove, RuinRecreateMove,
    ScalarMoveUnion, ScalarRecreateValueSource, SequentialCompositeMove,
};
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, MappedMoveCursor, VecUnionSelector,
};
use crate::heuristic::selector::{
    move_selector::{
        collect_cursor_indices, ArenaMoveCursor, CandidateId, MoveCandidateRef, MoveCursor,
    },
    nearby_support::truncate_nearby_candidates,
    pillar_support::{intersect_legal_values_for_pillar, pillars_are_swap_compatible, PillarGroup},
    seed::scoped_seed,
    ChangeMoveSelector, DefaultPillarSelector, FromSolutionEntitySelector, MoveSelector,
    PillarSelector, RuinMoveSelector, RuinVariableAccess, ValueSelector,
};

use super::context::{ScalarVariableContext, ValueSource};

pub type ScalarFlatSelector<S> =
    VecUnionSelector<S, ScalarMoveUnion<S, usize>, ScalarLeafSelector<S>>;
#[cfg_attr(not(test), allow(dead_code))]
pub type ScalarSelector<S> = VecUnionSelector<S, ScalarMoveUnion<S, usize>, ScalarSelectorNode<S>>;
#[cfg_attr(not(test), allow(dead_code))]
type ScalarCartesianSelector<S> = CartesianProductSelector<
    S,
    ScalarMoveUnion<S, usize>,
    ScalarFlatSelector<S>,
    ScalarFlatSelector<S>,
>;

include!("scalar_selector/values.rs");
include!("scalar_selector/leaf_selectors.rs");
include!("scalar_selector/dispatch.rs");
include!("scalar_selector/build.rs");
