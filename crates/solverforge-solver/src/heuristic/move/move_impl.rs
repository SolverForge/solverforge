//! Unified move enum for config-driven move selection.
//!
//! `MoveImpl` is a monomorphic enum containing ALL move types. This enables
//! config-driven solver pipelines while preserving full type information
//! throughout the entire solving process.
//!
//! # Zero-Erasure Design
//!
//! NO Box, NO dyn, NO Arc. Each variant wraps a concrete move type directly.
//! The compiler generates optimized code paths for each variant.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::change::ChangeMove;
use super::k_opt::KOptMove;
use super::list_change::ListChangeMove;
use super::list_reverse::ListReverseMove;
use super::list_ruin::ListRuinMove;
use super::list_swap::ListSwapMove;
use super::pillar_change::PillarChangeMove;
use super::pillar_swap::PillarSwapMove;
use super::ruin::RuinMove;
use super::sublist_change::SubListChangeMove;
use super::sublist_swap::SubListSwapMove;
use super::swap::SwapMove;
use super::traits::Move;

/// Monomorphic enum containing ALL move types.
///
/// This unified type enables config-driven move selection without type erasure.
/// The solver pipeline uses `MoveImpl<S, V>` as the concrete move type, and
/// the enum dispatches to the appropriate inner move at runtime.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `V` - The value type for variables (same across all move variants)
pub enum MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    /// Change move - assigns a new value to a single entity's variable.
    Change(ChangeMove<S, V>),

    /// Swap move - exchanges values between two entities.
    Swap(SwapMove<S, V>),

    /// Pillar change move - changes multiple entities with the same current value.
    PillarChange(PillarChangeMove<S, V>),

    /// Pillar swap move - swaps values between two groups of entities.
    PillarSwap(PillarSwapMove<S, V>),

    /// List change move - relocates a single element within/between list variables.
    ListChange(ListChangeMove<S, V>),

    /// List swap move - swaps two elements in list variables.
    ListSwap(ListSwapMove<S, V>),

    /// SubList change move - relocates a contiguous segment of a list.
    SubListChange(SubListChangeMove<S, V>),

    /// SubList swap move - swaps two contiguous segments.
    SubListSwap(SubListSwapMove<S, V>),

    /// List reverse move - reverses a segment (2-opt for TSP).
    ListReverse(ListReverseMove<S, V>),

    /// K-opt move - tour optimization via segment reconnection.
    KOpt(KOptMove<S, V>),

    /// Ruin move - unassigns multiple entities (for Large Neighborhood Search).
    Ruin(RuinMove<S, V>),

    /// List ruin move - removes elements from list variables (for LNS).
    ListRuin(ListRuinMove<S, V>),

    /// Composite move - applies two moves in sequence.
    /// Uses indices into arena rather than owned moves to avoid recursion.
    Composite {
        first_index: usize,
        second_index: usize,
        _phantom: PhantomData<(S, V)>,
    },
}

impl<S, V> Debug for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
            Self::PillarChange(m) => m.fmt(f),
            Self::PillarSwap(m) => m.fmt(f),
            Self::ListChange(m) => m.fmt(f),
            Self::ListSwap(m) => m.fmt(f),
            Self::SubListChange(m) => m.fmt(f),
            Self::SubListSwap(m) => m.fmt(f),
            Self::ListReverse(m) => m.fmt(f),
            Self::KOpt(m) => m.fmt(f),
            Self::Ruin(m) => m.fmt(f),
            Self::ListRuin(m) => m.fmt(f),
            Self::Composite {
                first_index,
                second_index,
                ..
            } => f
                .debug_struct("Composite")
                .field("first_index", first_index)
                .field("second_index", second_index)
                .finish(),
        }
    }
}

// Manual Send + Sync implementations to avoid phantom type bounds
unsafe impl<S, V> Send for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
}

unsafe impl<S, V> Sync for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
}

impl<S, V> Move<S> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
            Self::PillarChange(m) => m.is_doable(score_director),
            Self::PillarSwap(m) => m.is_doable(score_director),
            Self::ListChange(m) => m.is_doable(score_director),
            Self::ListSwap(m) => m.is_doable(score_director),
            Self::SubListChange(m) => m.is_doable(score_director),
            Self::SubListSwap(m) => m.is_doable(score_director),
            Self::ListReverse(m) => m.is_doable(score_director),
            Self::KOpt(m) => m.is_doable(score_director),
            Self::Ruin(m) => m.is_doable(score_director),
            Self::ListRuin(m) => m.is_doable(score_director),
            Self::Composite { .. } => {
                // Composite moves with arena indices - always doable if both children are
                // The actual check happens when executing via the arena
                true
            }
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
            Self::PillarChange(m) => m.do_move(score_director),
            Self::PillarSwap(m) => m.do_move(score_director),
            Self::ListChange(m) => m.do_move(score_director),
            Self::ListSwap(m) => m.do_move(score_director),
            Self::SubListChange(m) => m.do_move(score_director),
            Self::SubListSwap(m) => m.do_move(score_director),
            Self::ListReverse(m) => m.do_move(score_director),
            Self::KOpt(m) => m.do_move(score_director),
            Self::Ruin(m) => m.do_move(score_director),
            Self::ListRuin(m) => m.do_move(score_director),
            Self::Composite { .. } => {
                // Composite execution requires arena access - handled at phase level
                panic!("Composite moves must be executed via arena.execute_composite()")
            }
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
            Self::PillarChange(m) => m.variable_name(),
            Self::PillarSwap(m) => m.variable_name(),
            Self::ListChange(m) => m.variable_name(),
            Self::ListSwap(m) => m.variable_name(),
            Self::SubListChange(m) => m.variable_name(),
            Self::SubListSwap(m) => m.variable_name(),
            Self::ListReverse(m) => m.variable_name(),
            Self::KOpt(m) => m.variable_name(),
            Self::Ruin(m) => m.variable_name(),
            Self::ListRuin(m) => m.variable_name(),
            Self::Composite { .. } => "composite",
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
            Self::PillarChange(m) => m.descriptor_index(),
            Self::PillarSwap(m) => m.descriptor_index(),
            Self::ListChange(m) => m.descriptor_index(),
            Self::ListSwap(m) => m.descriptor_index(),
            Self::SubListChange(m) => m.descriptor_index(),
            Self::SubListSwap(m) => m.descriptor_index(),
            Self::ListReverse(m) => m.descriptor_index(),
            Self::KOpt(m) => m.descriptor_index(),
            Self::Ruin(m) => m.descriptor_index(),
            Self::ListRuin(m) => m.descriptor_index(),
            Self::Composite { .. } => 0,
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
            Self::PillarChange(m) => m.entity_indices(),
            Self::PillarSwap(m) => m.entity_indices(),
            Self::ListChange(m) => m.entity_indices(),
            Self::ListSwap(m) => m.entity_indices(),
            Self::SubListChange(m) => m.entity_indices(),
            Self::SubListSwap(m) => m.entity_indices(),
            Self::ListReverse(m) => m.entity_indices(),
            Self::KOpt(m) => m.entity_indices(),
            Self::Ruin(m) => m.entity_indices(),
            Self::ListRuin(m) => m.entity_indices(),
            Self::Composite { .. } => &[],
        }
    }
}

// Conversion traits for easy wrapping
impl<S, V> From<ChangeMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: ChangeMove<S, V>) -> Self {
        Self::Change(m)
    }
}

impl<S, V> From<SwapMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: SwapMove<S, V>) -> Self {
        Self::Swap(m)
    }
}

impl<S, V> From<PillarChangeMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: PillarChangeMove<S, V>) -> Self {
        Self::PillarChange(m)
    }
}

impl<S, V> From<PillarSwapMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: PillarSwapMove<S, V>) -> Self {
        Self::PillarSwap(m)
    }
}

impl<S, V> From<ListChangeMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: ListChangeMove<S, V>) -> Self {
        Self::ListChange(m)
    }
}

impl<S, V> From<ListSwapMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: ListSwapMove<S, V>) -> Self {
        Self::ListSwap(m)
    }
}

impl<S, V> From<SubListChangeMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: SubListChangeMove<S, V>) -> Self {
        Self::SubListChange(m)
    }
}

impl<S, V> From<SubListSwapMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: SubListSwapMove<S, V>) -> Self {
        Self::SubListSwap(m)
    }
}

impl<S, V> From<ListReverseMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: ListReverseMove<S, V>) -> Self {
        Self::ListReverse(m)
    }
}

impl<S, V> From<KOptMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: KOptMove<S, V>) -> Self {
        Self::KOpt(m)
    }
}

impl<S, V> From<RuinMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: RuinMove<S, V>) -> Self {
        Self::Ruin(m)
    }
}

impl<S, V> From<ListRuinMove<S, V>> for MoveImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn from(m: ListRuinMove<S, V>) -> Self {
        Self::ListRuin(m)
    }
}

