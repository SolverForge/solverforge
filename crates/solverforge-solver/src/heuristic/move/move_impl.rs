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
//! Moves store only indices - no value type parameter needed.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

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
/// The solver pipeline uses `MoveImpl<S>` as the concrete move type, and
/// the enum dispatches to the appropriate inner move at runtime.
///
/// # Type Parameters
///
/// * `S` - The planning solution type (must implement VariableOperations)
pub enum MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    /// Change move - assigns a new value to a single entity's variable.
    Change(ChangeMove<S>),

    /// Swap move - exchanges values between two entities.
    Swap(SwapMove<S>),

    /// Pillar change move - changes multiple entities with the same current value.
    PillarChange(PillarChangeMove<S>),

    /// Pillar swap move - swaps values between two groups of entities.
    PillarSwap(PillarSwapMove<S>),

    /// List change move - relocates a single element within/between list variables.
    ListChange(ListChangeMove<S>),

    /// List swap move - swaps two elements in list variables.
    ListSwap(ListSwapMove<S>),

    /// SubList change move - relocates a contiguous segment of a list.
    SubListChange(SubListChangeMove<S>),

    /// SubList swap move - swaps two contiguous segments.
    SubListSwap(SubListSwapMove<S>),

    /// List reverse move - reverses a segment (2-opt for TSP).
    ListReverse(ListReverseMove<S>),

    /// K-opt move - tour optimization via segment reconnection.
    KOpt(KOptMove<S>),

    /// Ruin move - unassigns multiple entities (for Large Neighborhood Search).
    Ruin(RuinMove<S>),

    /// List ruin move - removes elements from list variables (for LNS).
    ListRuin(ListRuinMove<S>),

    /// Composite move - applies two moves in sequence.
    /// Uses indices into arena rather than owned moves to avoid recursion.
    Composite {
        first_index: usize,
        second_index: usize,
        _phantom: PhantomData<fn() -> S>,
    },
}

impl<S> Debug for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
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

impl<S> Clone for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(*m),
            Self::Swap(m) => Self::Swap(*m),
            Self::PillarChange(m) => Self::PillarChange(m.clone()),
            Self::PillarSwap(m) => Self::PillarSwap(m.clone()),
            Self::ListChange(m) => Self::ListChange(*m),
            Self::ListSwap(m) => Self::ListSwap(*m),
            Self::SubListChange(m) => Self::SubListChange(*m),
            Self::SubListSwap(m) => Self::SubListSwap(*m),
            Self::ListReverse(m) => Self::ListReverse(*m),
            Self::KOpt(m) => Self::KOpt(*m),
            Self::Ruin(m) => Self::Ruin(m.clone()),
            Self::ListRuin(m) => Self::ListRuin(m.clone()),
            Self::Composite {
                first_index,
                second_index,
                _phantom,
            } => Self::Composite {
                first_index: *first_index,
                second_index: *second_index,
                _phantom: PhantomData,
            },
        }
    }
}

// Manual Send + Sync implementations
unsafe impl<S> Send for MoveImpl<S> where S: PlanningSolution + VariableOperations {}

unsafe impl<S> Sync for MoveImpl<S> where S: PlanningSolution + VariableOperations {}

impl<S> Move<S> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
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
impl<S> From<ChangeMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: ChangeMove<S>) -> Self {
        Self::Change(m)
    }
}

impl<S> From<SwapMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: SwapMove<S>) -> Self {
        Self::Swap(m)
    }
}

impl<S> From<PillarChangeMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: PillarChangeMove<S>) -> Self {
        Self::PillarChange(m)
    }
}

impl<S> From<PillarSwapMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: PillarSwapMove<S>) -> Self {
        Self::PillarSwap(m)
    }
}

impl<S> From<ListChangeMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: ListChangeMove<S>) -> Self {
        Self::ListChange(m)
    }
}

impl<S> From<ListSwapMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: ListSwapMove<S>) -> Self {
        Self::ListSwap(m)
    }
}

impl<S> From<SubListChangeMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: SubListChangeMove<S>) -> Self {
        Self::SubListChange(m)
    }
}

impl<S> From<SubListSwapMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: SubListSwapMove<S>) -> Self {
        Self::SubListSwap(m)
    }
}

impl<S> From<ListReverseMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: ListReverseMove<S>) -> Self {
        Self::ListReverse(m)
    }
}

impl<S> From<KOptMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: KOptMove<S>) -> Self {
        Self::KOpt(m)
    }
}

impl<S> From<RuinMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: RuinMove<S>) -> Self {
        Self::Ruin(m)
    }
}

impl<S> From<ListRuinMove<S>> for MoveImpl<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn from(m: ListRuinMove<S>) -> Self {
        Self::ListRuin(m)
    }
}
