use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use smallvec::SmallVec;
use solverforge_config::{MoveSelectorConfig, RecreateHeuristicType};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor, ValueRangeType};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, VecUnionSelector,
};
use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, CandidateId, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::heuristic::selector::nearby_support::truncate_nearby_candidates;
use crate::heuristic::selector::pillar::SubPillarConfig;
use crate::heuristic::selector::pillar_support::{
    collect_pillar_groups, intersect_legal_values_for_pillar, pillars_are_swap_compatible,
};
use crate::heuristic::selector::seed::scoped_seed;

use super::bindings::{collect_bindings, find_binding, VariableBinding};
use super::move_types::{
    DescriptorChangeMove, DescriptorPillarChangeMove, DescriptorPillarSwapMove,
    DescriptorRuinRecreateMove, DescriptorScalarMoveUnion, DescriptorSwapMove,
};

pub type DescriptorFlatSelector<S> =
    VecUnionSelector<S, DescriptorScalarMoveUnion<S>, DescriptorLeafSelector<S>>;
type DescriptorCartesianSelector<S> = CartesianProductSelector<
    S,
    DescriptorScalarMoveUnion<S>,
    DescriptorFlatSelector<S>,
    DescriptorFlatSelector<S>,
>;
pub type DescriptorSelector<S> =
    VecUnionSelector<S, DescriptorScalarMoveUnion<S>, DescriptorSelectorNode<S>>;

const SWAP_LEGALITY_WORD_BITS: usize = usize::BITS as usize;

include!("selectors/swap_legality.rs");
include!("selectors/change_swap.rs");
include!("selectors/pillar_ruin.rs");
include!("selectors/dispatch.rs");
