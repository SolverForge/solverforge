/* ScalarMoveUnion - a monomorphized union of the canonical scalar move family.

This allows local search to combine scalar change, swap, pillar, ruin-recreate,
and cartesian-composite moves without trait-object dispatch.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::{
    ChangeMove, CompoundScalarMove, ConflictRepairMove, Move, MoveTabuSignature, PillarChangeMove,
    PillarSwapMove, RuinRecreateMove, SequentialCompositeMove, SwapMove,
};

/// A monomorphized union of the canonical scalar move family.
///
/// Implements `Move<S>` by delegating to the inner variant.
/// `Copy` when `V: Copy`, avoiding heap allocation in the move selector hot path.
#[allow(clippy::large_enum_variant)]
pub enum ScalarMoveUnion<S, V> {
    Change(ChangeMove<S, V>),
    Swap(SwapMove<S, V>),
    PillarChange(PillarChangeMove<S, V>),
    PillarSwap(PillarSwapMove<S, V>),
    RuinRecreate(RuinRecreateMove<S>),
    Composite(SequentialCompositeMove<S, ScalarMoveUnion<S, V>>),
    CompoundScalar(CompoundScalarMove<S>),
    ConflictRepair(ConflictRepairMove<S>),
}

impl<S, V> Clone for ScalarMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(*m),
            Self::PillarChange(m) => Self::PillarChange(m.clone()),
            Self::PillarSwap(m) => Self::PillarSwap(m.clone()),
            Self::RuinRecreate(m) => Self::RuinRecreate(m.clone()),
            Self::Composite(m) => Self::Composite(m.clone()),
            Self::CompoundScalar(m) => Self::CompoundScalar(m.clone()),
            Self::ConflictRepair(m) => Self::ConflictRepair(m.clone()),
        }
    }
}

impl<S, V> Debug for ScalarMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
            Self::PillarChange(m) => m.fmt(f),
            Self::PillarSwap(m) => m.fmt(f),
            Self::RuinRecreate(m) => m.fmt(f),
            Self::Composite(m) => m.fmt(f),
            Self::CompoundScalar(m) => m.fmt(f),
            Self::ConflictRepair(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for ScalarMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
            Self::PillarChange(m) => m.is_doable(score_director),
            Self::PillarSwap(m) => m.is_doable(score_director),
            Self::RuinRecreate(m) => m.is_doable(score_director),
            Self::Composite(m) => m.is_doable(score_director),
            Self::CompoundScalar(m) => m.is_doable(score_director),
            Self::ConflictRepair(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
            Self::PillarChange(m) => m.do_move(score_director),
            Self::PillarSwap(m) => m.do_move(score_director),
            Self::RuinRecreate(m) => m.do_move(score_director),
            Self::Composite(m) => m.do_move(score_director),
            Self::CompoundScalar(m) => m.do_move(score_director),
            Self::ConflictRepair(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
            Self::PillarChange(m) => m.descriptor_index(),
            Self::PillarSwap(m) => m.descriptor_index(),
            Self::RuinRecreate(m) => m.descriptor_index(),
            Self::Composite(m) => m.descriptor_index(),
            Self::CompoundScalar(m) => m.descriptor_index(),
            Self::ConflictRepair(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
            Self::PillarChange(m) => m.entity_indices(),
            Self::PillarSwap(m) => m.entity_indices(),
            Self::RuinRecreate(m) => m.entity_indices(),
            Self::Composite(m) => m.entity_indices(),
            Self::CompoundScalar(m) => m.entity_indices(),
            Self::ConflictRepair(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
            Self::PillarChange(m) => m.variable_name(),
            Self::PillarSwap(m) => m.variable_name(),
            Self::RuinRecreate(m) => m.variable_name(),
            Self::Composite(m) => m.variable_name(),
            Self::CompoundScalar(m) => m.variable_name(),
            Self::ConflictRepair(m) => m.variable_name(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Change(m) => m.requires_hard_improvement(),
            Self::Swap(m) => m.requires_hard_improvement(),
            Self::PillarChange(m) => m.requires_hard_improvement(),
            Self::PillarSwap(m) => m.requires_hard_improvement(),
            Self::RuinRecreate(m) => m.requires_hard_improvement(),
            Self::Composite(m) => m.requires_hard_improvement(),
            Self::CompoundScalar(m) => m.requires_hard_improvement(),
            Self::ConflictRepair(m) => m.requires_hard_improvement(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match self {
            Self::Change(m) => m.tabu_signature(score_director),
            Self::Swap(m) => m.tabu_signature(score_director),
            Self::PillarChange(m) => m.tabu_signature(score_director),
            Self::PillarSwap(m) => m.tabu_signature(score_director),
            Self::RuinRecreate(m) => m.tabu_signature(score_director),
            Self::Composite(m) => m.tabu_signature(score_director),
            Self::CompoundScalar(m) => m.tabu_signature(score_director),
            Self::ConflictRepair(m) => m.tabu_signature(score_director),
        }
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(super::MoveAffectedEntity<'_>)) {
        match self {
            Self::Change(m) => m.for_each_affected_entity(visitor),
            Self::Swap(m) => m.for_each_affected_entity(visitor),
            Self::PillarChange(m) => m.for_each_affected_entity(visitor),
            Self::PillarSwap(m) => m.for_each_affected_entity(visitor),
            Self::RuinRecreate(m) => m.for_each_affected_entity(visitor),
            Self::Composite(m) => m.for_each_affected_entity(visitor),
            Self::CompoundScalar(m) => m.for_each_affected_entity(visitor),
            Self::ConflictRepair(m) => m.for_each_affected_entity(visitor),
        }
    }
}
