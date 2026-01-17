//! Unified move selector enum for config-driven selection.
//!
//! `MoveSelectorImpl` wraps ALL selector types and produces `MoveImpl`.
//! This enables config-driven solver pipelines with full type preservation.
//!
//! # Zero-Erasure Design
//!
//! NO Box<dyn>, NO Arc. Each variant wraps a concrete selector and
//! produces `MoveImpl` variants directly.

use std::fmt::Debug;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::move_impl::MoveImpl;
use crate::operations::VariableOperations;

use super::entity::FromSolutionEntitySelector;
use super::k_opt::{KOptConfig, KOptMoveSelector};
use super::list_change::ListChangeMoveSelector;
use super::list_ruin::ListRuinMoveSelector;
use super::list_swap::ListSwapMoveSelector;
use super::ruin::RuinMoveSelector;
use super::sublist_change::SubListChangeMoveSelector;
use super::sublist_swap::SubListSwapMoveSelector;
use super::typed_move_selector::MoveSelector;
use super::typed_move_selector::{ChangeMoveSelector, SwapMoveSelector};
use super::typed_value::StaticTypedValueSelector;

/// Unified move selector enum producing `MoveImpl`.
///
/// Each variant wraps a concrete selector. The `iter_moves` implementation
/// maps inner moves to `MoveImpl` variants.
pub enum MoveSelectorImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    /// Change moves - assigns values to entities.
    Change(ChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticTypedValueSelector<S, V>>),
    /// Swap moves - exchanges values between entities.
    Swap(SwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>),
    /// List change moves - relocate elements.
    ListChange(ListChangeMoveSelector<S, V, FromSolutionEntitySelector>),
    /// List swap moves - swap two elements.
    ListSwap(ListSwapMoveSelector<S, V, FromSolutionEntitySelector>),
    /// SubList change moves - relocate segments.
    SubListChange(SubListChangeMoveSelector<S, V, FromSolutionEntitySelector>),
    /// SubList swap moves - swap two segments.
    SubListSwap(SubListSwapMoveSelector<S, V, FromSolutionEntitySelector>),
    /// K-opt moves - tour optimization.
    KOpt(KOptMoveSelector<S, V, FromSolutionEntitySelector>),
    /// Ruin moves for basic variables.
    Ruin(RuinMoveSelector<S, V>),
    /// List ruin moves for list variables.
    ListRuin(ListRuinMoveSelector<S, V>),
    /// Union of multiple selectors.
    Union(Vec<MoveSelectorImpl<S, V>>),
}

impl<S, V> Debug for MoveSelectorImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(sel) => sel.fmt(f),
            Self::Swap(sel) => sel.fmt(f),
            Self::ListChange(sel) => sel.fmt(f),
            Self::ListSwap(sel) => sel.fmt(f),
            Self::SubListChange(sel) => sel.fmt(f),
            Self::SubListSwap(sel) => sel.fmt(f),
            Self::KOpt(sel) => sel.fmt(f),
            Self::Ruin(sel) => sel.fmt(f),
            Self::ListRuin(sel) => sel.fmt(f),
            Self::Union(sels) => f.debug_tuple("Union").field(&sels.len()).finish(),
        }
    }
}

unsafe impl<S, V> Send for MoveSelectorImpl<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
}

impl<S, V> MoveSelectorImpl<S, V>
where
    S: PlanningSolution + VariableOperations<Element = V>,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    /// Creates a change selector with the given values.
    pub fn change(values: Vec<V>) -> Self {
        Self::Change(ChangeMoveSelector::simple(
            |s: &S, entity_idx| {
                if entity_idx < s.list_len(0) {
                    Some(s.get(0, entity_idx))
                } else {
                    None
                }
            },
            |s: &mut S, entity_idx, val| {
                if let Some(v) = val {
                    if entity_idx < s.list_len(0) {
                        s.remove(0, entity_idx);
                        s.insert(0, entity_idx, v);
                    }
                }
            },
            S::descriptor_index(),
            S::variable_name(),
            values,
        ))
    }

    /// Creates a swap selector.
    pub fn swap() -> Self {
        Self::Swap(SwapMoveSelector::new(
            FromSolutionEntitySelector::new(S::descriptor_index()),
            FromSolutionEntitySelector::new(S::descriptor_index()),
            |s: &S, entity_idx| {
                if entity_idx < s.list_len(0) {
                    Some(s.get(0, entity_idx))
                } else {
                    None
                }
            },
            |s: &mut S, entity_idx, val| {
                if let Some(v) = val {
                    if entity_idx < s.list_len(0) {
                        s.remove(0, entity_idx);
                        s.insert(0, entity_idx, v);
                    }
                }
            },
            S::descriptor_index(),
            S::variable_name(),
        ))
    }

    /// Creates a list change selector.
    pub fn list_change() -> Self {
        let entity_selector = FromSolutionEntitySelector::new(S::descriptor_index());
        Self::ListChange(ListChangeMoveSelector::new(
            entity_selector,
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &mut S, entity_idx, pos| Some(s.remove(entity_idx, pos)),
            |s: &mut S, entity_idx, pos, elem| s.insert(entity_idx, pos, elem),
            S::variable_name(),
            S::descriptor_index(),
        ))
    }

    /// Creates a list swap selector.
    pub fn list_swap() -> Self {
        let entity_selector = FromSolutionEntitySelector::new(S::descriptor_index());
        Self::ListSwap(ListSwapMoveSelector::new(
            entity_selector,
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &S, entity_idx, pos| {
                if pos < s.list_len(entity_idx) {
                    Some(s.get(entity_idx, pos))
                } else {
                    None
                }
            },
            |s: &mut S, entity_idx, pos, elem| {
                if pos < s.list_len(entity_idx) {
                    s.remove(entity_idx, pos);
                    s.insert(entity_idx, pos, elem);
                }
            },
            S::variable_name(),
            S::descriptor_index(),
        ))
    }

    /// Creates a sublist change selector.
    pub fn sublist_change(min_len: usize, max_len: usize) -> Self {
        let entity_selector = FromSolutionEntitySelector::new(S::descriptor_index());
        Self::SubListChange(SubListChangeMoveSelector::new(
            entity_selector,
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &mut S, entity_idx, start, end| s.remove_sublist(entity_idx, start, end),
            |s: &mut S, entity_idx, pos, elems| s.insert_sublist(entity_idx, pos, elems),
            S::variable_name(),
            S::descriptor_index(),
            min_len,
            max_len,
        ))
    }

    /// Creates a sublist swap selector.
    pub fn sublist_swap(min_len: usize, max_len: usize) -> Self {
        let entity_selector = FromSolutionEntitySelector::new(S::descriptor_index());
        Self::SubListSwap(SubListSwapMoveSelector::new(
            entity_selector,
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &mut S, entity_idx, start, end| s.remove_sublist(entity_idx, start, end),
            |s: &mut S, entity_idx, pos, elems| s.insert_sublist(entity_idx, pos, elems),
            S::variable_name(),
            S::descriptor_index(),
            min_len,
            max_len,
        ))
    }

    /// Creates a k-opt selector.
    pub fn k_opt(k: usize) -> Self {
        let entity_selector = FromSolutionEntitySelector::new(S::descriptor_index());
        let config = KOptConfig::new(k);
        Self::KOpt(KOptMoveSelector::new(
            entity_selector,
            config,
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &mut S, entity_idx, start, end| s.remove_sublist(entity_idx, start, end),
            |s: &mut S, entity_idx, pos, elems| s.insert_sublist(entity_idx, pos, elems),
            S::variable_name(),
            S::descriptor_index(),
        ))
    }

    /// Creates a 2-opt selector.
    pub fn two_opt() -> Self {
        Self::k_opt(2)
    }

    /// Creates a 3-opt selector.
    pub fn three_opt() -> Self {
        Self::k_opt(3)
    }

    /// Creates a ruin selector for basic variables.
    pub fn ruin(min_ruin: usize, max_ruin: usize) -> Self {
        Self::Ruin(RuinMoveSelector::new(
            min_ruin,
            max_ruin,
            |s: &S| s.entity_count(),
            |s: &S, entity_idx| {
                if entity_idx < s.list_len(0) {
                    Some(s.get(0, entity_idx))
                } else {
                    None
                }
            },
            |s: &mut S, entity_idx, _val| {
                if entity_idx < s.list_len(0) {
                    s.remove(0, entity_idx);
                }
            },
            S::variable_name(),
            S::descriptor_index(),
        ))
    }

    /// Creates a list ruin selector.
    pub fn list_ruin(min_ruin: usize, max_ruin: usize) -> Self {
        Self::ListRuin(ListRuinMoveSelector::new(
            min_ruin,
            max_ruin,
            |s: &S| s.entity_count(),
            |s: &S, entity_idx| s.list_len(entity_idx),
            |s: &mut S, entity_idx, pos| s.remove(entity_idx, pos),
            |s: &mut S, entity_idx, pos, elem| s.insert(entity_idx, pos, elem),
            S::variable_name(),
            S::descriptor_index(),
        ))
    }

    /// Creates a union of selectors.
    pub fn union(selectors: Vec<Self>) -> Self {
        Self::Union(selectors)
    }

    /// Creates from config.
    pub fn from_config(config: Option<&MoveSelectorConfig>) -> Self {
        match config {
            Some(MoveSelectorConfig::ChangeMoveSelector(_)) => {
                // Change selector requires values - default to swap for basic variables
                Self::swap()
            }
            Some(MoveSelectorConfig::SwapMoveSelector(_)) => Self::swap(),
            Some(MoveSelectorConfig::ListChangeMoveSelector(_)) => Self::list_change(),
            Some(MoveSelectorConfig::ListSwapMoveSelector(_)) => Self::list_swap(),
            Some(MoveSelectorConfig::SubListChangeMoveSelector(cfg)) => {
                let min_len = cfg.min_sublist_length.unwrap_or(1);
                let max_len = cfg.max_sublist_length.unwrap_or(usize::MAX);
                Self::sublist_change(min_len, max_len)
            }
            Some(MoveSelectorConfig::SubListSwapMoveSelector(cfg)) => {
                let min_len = cfg.min_sublist_length.unwrap_or(1);
                let max_len = cfg.max_sublist_length.unwrap_or(usize::MAX);
                Self::sublist_swap(min_len, max_len)
            }
            Some(MoveSelectorConfig::KOptMoveSelector(cfg)) => {
                let k = cfg.k_value.unwrap_or(3);
                Self::k_opt(k)
            }
            Some(MoveSelectorConfig::RuinMoveSelector(_)) => Self::ruin(2, 5),
            Some(MoveSelectorConfig::ListRuinMoveSelector(_)) => Self::list_ruin(2, 5),
            Some(MoveSelectorConfig::UnionMoveSelector(cfg)) => {
                let selectors: Vec<Self> = cfg
                    .selectors
                    .iter()
                    .map(|c| Self::from_config(Some(c)))
                    .collect();
                Self::union(selectors)
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cfg)) => {
                // CartesianProduct treated as Union for now
                let selectors: Vec<Self> = cfg
                    .selectors
                    .iter()
                    .map(|c| Self::from_config(Some(c)))
                    .collect();
                Self::union(selectors)
            }
            None => Self::list_change(),
        }
    }
}

impl<S, V> Default for MoveSelectorImpl<S, V>
where
    S: PlanningSolution + VariableOperations<Element = V>,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn default() -> Self {
        Self::list_change()
    }
}

impl<S, V> MoveSelector<S, MoveImpl<S, V>> for MoveSelectorImpl<S, V>
where
    S: PlanningSolution + VariableOperations<Element = V>,
    V: Copy + PartialEq + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = MoveImpl<S, V>> + 'a> {
        match self {
            Self::Change(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::Swap(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::ListChange(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::ListSwap(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::SubListChange(sel) => {
                Box::new(sel.iter_moves(score_director).map(MoveImpl::from))
            }
            Self::SubListSwap(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::KOpt(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::Ruin(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::ListRuin(sel) => Box::new(sel.iter_moves(score_director).map(MoveImpl::from)),
            Self::Union(sels) => {
                let iters: Vec<_> = sels
                    .iter()
                    .map(|s| s.iter_moves(score_director))
                    .collect();
                Box::new(iters.into_iter().flatten())
            }
        }
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(sel) => sel.size(score_director),
            Self::Swap(sel) => sel.size(score_director),
            Self::ListChange(sel) => sel.size(score_director),
            Self::ListSwap(sel) => sel.size(score_director),
            Self::SubListChange(sel) => sel.size(score_director),
            Self::SubListSwap(sel) => sel.size(score_director),
            Self::KOpt(sel) => sel.size(score_director),
            Self::Ruin(sel) => sel.size(score_director),
            Self::ListRuin(sel) => sel.size(score_director),
            Self::Union(sels) => sels.iter().map(|s| s.size(score_director)).sum(),
        }
    }
}
