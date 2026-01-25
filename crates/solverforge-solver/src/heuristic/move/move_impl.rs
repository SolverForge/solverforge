//! Unified move enum for all move types.
//!
//! `MoveImpl` aggregates all concrete move types into a single enum,
//! enabling type-safe move handling without type erasure.
//!
//! # Zero-Erasure Design
//!
//! Each variant delegates to its underlying move type which uses typed
//! function pointers. No `Arc<dyn>`, no `Box<dyn Any>`, no downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::{ScoreDirector, ShadowVariableSupport};

use super::traits::Move;
use super::{
    ChangeMove, KOptMove, ListAssignMove, ListChangeMove, ListReverseMove, ListRuinMove,
    ListSwapMove, PillarChangeMove, PillarSwapMove, RuinMove, SubListChangeMove, SubListSwapMove,
    SwapMove,
};

/// Unified move enum containing all move type variants.
///
/// This enum enables runtime selection of move types while maintaining
/// full type safety and zero-erasure performance.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable/element value type
///
/// # Variants
///
/// **Basic Variable Moves:**
/// - `Change` - Assigns a value to an entity's variable
/// - `Swap` - Exchanges values between two entities
/// - `PillarChange` - Changes all entities in a pillar
/// - `PillarSwap` - Swaps between two pillars
/// - `Ruin` - Unassigns multiple entities (LNS)
///
/// **List Variable Moves:**
/// - `ListAssign` - Assigns element to list (construction)
/// - `ListChange` - Relocates element within/between lists
/// - `ListSwap` - Swaps two elements in lists
/// - `ListReverse` - Reverses a segment (2-opt)
/// - `SubListChange` - Relocates a contiguous sublist
/// - `SubListSwap` - Swaps two sublists
/// - `KOpt` - K-opt tour optimization
/// - `ListRuin` - Removes elements from lists (LNS)
pub enum MoveImpl<S, V> {
    // Basic variable moves
    /// Assigns a value to an entity's planning variable.
    Change(ChangeMove<S, V>),
    /// Swaps values between two entities.
    Swap(SwapMove<S, V>),
    /// Changes all entities in a pillar to a new value.
    PillarChange(PillarChangeMove<S, V>),
    /// Swaps values between two pillars.
    PillarSwap(PillarSwapMove<S, V>),
    /// Unassigns multiple entities for Large Neighborhood Search.
    Ruin(RuinMove<S, V>),

    // List variable moves
    /// Assigns an unassigned element to a list (construction phase).
    ListAssign(ListAssignMove<S, V>),
    /// Relocates an element within or between lists.
    ListChange(ListChangeMove<S, V>),
    /// Swaps two elements in lists.
    ListSwap(ListSwapMove<S, V>),
    /// Reverses a segment within a list (2-opt).
    ListReverse(ListReverseMove<S, V>),
    /// Relocates a contiguous sublist.
    SubListChange(SubListChangeMove<S, V>),
    /// Swaps two contiguous sublists.
    SubListSwap(SubListSwapMove<S, V>),
    /// K-opt tour optimization.
    KOpt(KOptMove<S, V>),
    /// Removes elements from lists for Large Neighborhood Search.
    ListRuin(ListRuinMove<S, V>),
}

impl<S, V: Clone> Clone for MoveImpl<S, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(*m),
            Self::PillarChange(m) => Self::PillarChange(m.clone()),
            Self::PillarSwap(m) => Self::PillarSwap(m.clone()),
            Self::Ruin(m) => Self::Ruin(m.clone()),
            Self::ListAssign(m) => Self::ListAssign(m.clone()),
            Self::ListChange(m) => Self::ListChange(*m),
            Self::ListSwap(m) => Self::ListSwap(*m),
            Self::ListReverse(m) => Self::ListReverse(*m),
            Self::SubListChange(m) => Self::SubListChange(*m),
            Self::SubListSwap(m) => Self::SubListSwap(*m),
            Self::KOpt(m) => Self::KOpt(m.clone()),
            Self::ListRuin(m) => Self::ListRuin(m.clone()),
        }
    }
}

impl<S, V: Debug> Debug for MoveImpl<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
            Self::PillarChange(m) => m.fmt(f),
            Self::PillarSwap(m) => m.fmt(f),
            Self::Ruin(m) => m.fmt(f),
            Self::ListAssign(m) => m.fmt(f),
            Self::ListChange(m) => m.fmt(f),
            Self::ListSwap(m) => m.fmt(f),
            Self::ListReverse(m) => m.fmt(f),
            Self::SubListChange(m) => m.fmt(f),
            Self::SubListSwap(m) => m.fmt(f),
            Self::KOpt(m) => m.fmt(f),
            Self::ListRuin(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for MoveImpl<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        C: ConstraintSet<S, S::Score>,
    {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
            Self::PillarChange(m) => m.is_doable(score_director),
            Self::PillarSwap(m) => m.is_doable(score_director),
            Self::Ruin(m) => m.is_doable(score_director),
            Self::ListAssign(m) => m.is_doable(score_director),
            Self::ListChange(m) => m.is_doable(score_director),
            Self::ListSwap(m) => m.is_doable(score_director),
            Self::ListReverse(m) => m.is_doable(score_director),
            Self::SubListChange(m) => m.is_doable(score_director),
            Self::SubListSwap(m) => m.is_doable(score_director),
            Self::KOpt(m) => m.is_doable(score_director),
            Self::ListRuin(m) => m.is_doable(score_director),
        }
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: ConstraintSet<S, S::Score>,
    {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
            Self::PillarChange(m) => m.do_move(score_director),
            Self::PillarSwap(m) => m.do_move(score_director),
            Self::Ruin(m) => m.do_move(score_director),
            Self::ListAssign(m) => m.do_move(score_director),
            Self::ListChange(m) => m.do_move(score_director),
            Self::ListSwap(m) => m.do_move(score_director),
            Self::ListReverse(m) => m.do_move(score_director),
            Self::SubListChange(m) => m.do_move(score_director),
            Self::SubListSwap(m) => m.do_move(score_director),
            Self::KOpt(m) => m.do_move(score_director),
            Self::ListRuin(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
            Self::PillarChange(m) => m.descriptor_index(),
            Self::PillarSwap(m) => m.descriptor_index(),
            Self::Ruin(m) => m.descriptor_index(),
            Self::ListAssign(m) => m.descriptor_index(),
            Self::ListChange(m) => m.descriptor_index(),
            Self::ListSwap(m) => m.descriptor_index(),
            Self::ListReverse(m) => m.descriptor_index(),
            Self::SubListChange(m) => m.descriptor_index(),
            Self::SubListSwap(m) => m.descriptor_index(),
            Self::KOpt(m) => m.descriptor_index(),
            Self::ListRuin(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
            Self::PillarChange(m) => m.entity_indices(),
            Self::PillarSwap(m) => m.entity_indices(),
            Self::Ruin(m) => m.entity_indices(),
            Self::ListAssign(m) => m.entity_indices(),
            Self::ListChange(m) => m.entity_indices(),
            Self::ListSwap(m) => m.entity_indices(),
            Self::ListReverse(m) => m.entity_indices(),
            Self::SubListChange(m) => m.entity_indices(),
            Self::SubListSwap(m) => m.entity_indices(),
            Self::KOpt(m) => m.entity_indices(),
            Self::ListRuin(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
            Self::PillarChange(m) => m.variable_name(),
            Self::PillarSwap(m) => m.variable_name(),
            Self::Ruin(m) => m.variable_name(),
            Self::ListAssign(m) => m.variable_name(),
            Self::ListChange(m) => m.variable_name(),
            Self::ListSwap(m) => m.variable_name(),
            Self::ListReverse(m) => m.variable_name(),
            Self::SubListChange(m) => m.variable_name(),
            Self::SubListSwap(m) => m.variable_name(),
            Self::KOpt(m) => m.variable_name(),
            Self::ListRuin(m) => m.variable_name(),
        }
    }

    fn strength(&self) -> i64 {
        match self {
            Self::Change(m) => m.strength(),
            Self::Swap(m) => m.strength(),
            Self::PillarChange(m) => m.strength(),
            Self::PillarSwap(m) => m.strength(),
            Self::Ruin(m) => m.strength(),
            Self::ListAssign(m) => m.strength(),
            Self::ListChange(m) => m.strength(),
            Self::ListSwap(m) => m.strength(),
            Self::ListReverse(m) => m.strength(),
            Self::SubListChange(m) => m.strength(),
            Self::SubListSwap(m) => m.strength(),
            Self::KOpt(m) => m.strength(),
            Self::ListRuin(m) => m.strength(),
        }
    }
}

impl<S, V> MoveImpl<S, V> {
    /// Returns the variant name as a string.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Change(_) => "Change",
            Self::Swap(_) => "Swap",
            Self::PillarChange(_) => "PillarChange",
            Self::PillarSwap(_) => "PillarSwap",
            Self::Ruin(_) => "Ruin",
            Self::ListAssign(_) => "ListAssign",
            Self::ListChange(_) => "ListChange",
            Self::ListSwap(_) => "ListSwap",
            Self::ListReverse(_) => "ListReverse",
            Self::SubListChange(_) => "SubListChange",
            Self::SubListSwap(_) => "SubListSwap",
            Self::KOpt(_) => "KOpt",
            Self::ListRuin(_) => "ListRuin",
        }
    }
}
