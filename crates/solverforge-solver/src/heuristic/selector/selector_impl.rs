//! Unified move selector enum with direct move generation.
//!
//! `MoveSelectorImpl` aggregates all move generation strategies into a single enum.
//! Each variant holds function pointers directly, eliminating the need for separate
//! selector types per move type.
//!
//! # Zero-Erasure Design
//!
//! Function pointers are resolved at construction time from config strings.
//! Move generation happens inline via match dispatch - no virtual calls.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::{ScoreDirector, ShadowVariableSupport};

use super::typed_move_selector::MoveSelector;
use crate::heuristic::r#move::k_opt_reconnection::{enumerate_reconnections, KOptReconnection};
use crate::heuristic::r#move::{
    ChangeMove, CutPoint, KOptMove, ListChangeMove, ListReverseMove, ListRuinMove, ListSwapMove,
    MoveImpl, PillarChangeMove, PillarSwapMove, RuinMove, SubListChangeMove, SubListSwapMove,
    SwapMove,
};

/// Function pointer struct for basic variable operations.
///
/// Note: Manual Clone/Copy impls avoid requiring bounds on S/V.
/// All fields are function pointers (inherently Copy) or primitives.
pub struct BasicVariableFnPtrs<S, V> {
    /// Get entity count.
    pub entity_count: fn(&S) -> usize,
    /// Get value range.
    pub value_range: fn(&S) -> Vec<V>,
    /// Get current value for entity.
    pub getter: fn(&S, usize) -> Option<V>,
    /// Set value for entity.
    pub setter: fn(&mut S, usize, Option<V>),
    /// Variable name.
    pub variable_name: &'static str,
    /// Descriptor index.
    pub descriptor_index: usize,
}

impl<S, V> Clone for BasicVariableFnPtrs<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for BasicVariableFnPtrs<S, V> {}

impl<S, V: Debug> Debug for BasicVariableFnPtrs<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicVariableFnPtrs")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

/// Function pointer struct for list variable operations.
///
/// Note: Manual Clone/Copy impls avoid requiring bounds on S/V.
/// All fields are function pointers (inherently Copy) or primitives.
pub struct ListVariableFnPtrs<S, V> {
    /// Get entity count.
    pub entity_count: fn(&S) -> usize,
    /// Get element count (total elements to assign).
    pub element_count: fn(&S) -> usize,
    /// Get assigned elements.
    pub assigned_elements: fn(&S) -> Vec<V>,
    /// Get list length for entity.
    pub list_len: fn(&S, usize) -> usize,
    /// Get element at position (returns Option for bounds safety).
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    /// Set element at position.
    pub list_set: fn(&mut S, usize, usize, V),
    /// Remove element at position (returns Option for bounds safety).
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    /// Insert element at position.
    pub list_insert: fn(&mut S, usize, usize, V),
    /// Remove sublist [start, end), returns elements.
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert multiple elements at position.
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Reverse elements in range [start, end).
    pub list_reverse: fn(&mut S, usize, usize, usize),
    /// Get element index at position (for shadow updates).
    pub list_get_element_idx: fn(&S, usize, usize) -> usize,
    /// Assign element to entity (append).
    pub assign: fn(&mut S, usize, V),
    /// Variable name.
    pub variable_name: &'static str,
    /// Descriptor index.
    pub descriptor_index: usize,
}

impl<S, V> Clone for ListVariableFnPtrs<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ListVariableFnPtrs<S, V> {}

impl<S, V: Debug> Debug for ListVariableFnPtrs<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListVariableFnPtrs")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

/// Unified move selector enum containing all selector variants.
///
/// Each variant holds function pointers for direct move generation.
/// No separate selector types - move generation logic is inline.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable/element value type
pub enum MoveSelectorImpl<S, V> {
    // ========================================================================
    // Basic Variable Selectors
    // ========================================================================
    /// Generates ChangeMove: assigns value to entity.
    Change(BasicVariableFnPtrs<S, V>),

    /// Generates SwapMove: exchanges values between entities.
    Swap(BasicVariableFnPtrs<S, V>),

    /// Generates PillarChangeMove: changes all entities with same value.
    PillarChange(BasicVariableFnPtrs<S, V>),

    /// Generates PillarSwapMove: swaps between entity groups.
    PillarSwap(BasicVariableFnPtrs<S, V>),

    /// Generates RuinMove: unassigns multiple entities (LNS).
    Ruin {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        /// Number of entities to unassign per move.
        ruin_count: usize,
    },

    // ========================================================================
    // List Variable Selectors
    // ========================================================================
    /// Generates ListAssignMove: assigns element to list (construction).
    ListAssign(ListVariableFnPtrs<S, V>),

    /// Generates ListChangeMove: relocates element within/between lists.
    ListChange(ListVariableFnPtrs<S, V>),

    /// Generates ListSwapMove: swaps two elements.
    ListSwap(ListVariableFnPtrs<S, V>),

    /// Generates ListReverseMove: reverses segment (2-opt style).
    ListReverse {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum segment length (default: 2).
        min_segment_len: usize,
        /// Maximum segment length (None = entire list).
        max_segment_len: Option<usize>,
    },

    /// Generates SubListChangeMove: relocates contiguous sublist.
    SubListChange {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum sublist length.
        min_sublist_len: usize,
        /// Maximum sublist length (None = entire list).
        max_sublist_len: Option<usize>,
    },

    /// Generates SubListSwapMove: swaps two sublists.
    SubListSwap {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum sublist length.
        min_sublist_len: usize,
        /// Maximum sublist length (None = entire list).
        max_sublist_len: Option<usize>,
    },

    /// Generates KOptMove: k-opt tour optimization for any k (2-5).
    KOpt {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// k value (2-5).
        k: usize,
        /// Minimum segment length between cuts.
        min_segment_len: usize,
    },

    /// Generates ListRuinMove: removes elements from lists (LNS).
    ListRuin {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Number of elements to remove per move.
        ruin_count: usize,
    },

    // ========================================================================
    // Composite Selectors
    // ========================================================================
    /// Union of multiple selectors.
    Union(Vec<MoveSelectorImpl<S, V>>),
}

// ============================================================================
// Constructors
// ============================================================================

impl<S, V> MoveSelectorImpl<S, V> {
    /// Creates a Change selector from function pointers.
    pub fn change(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::Change(fn_ptrs)
    }

    /// Creates a Swap selector from function pointers.
    pub fn swap(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::Swap(fn_ptrs)
    }

    /// Creates a PillarChange selector from function pointers.
    pub fn pillar_change(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::PillarChange(fn_ptrs)
    }

    /// Creates a PillarSwap selector from function pointers.
    pub fn pillar_swap(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::PillarSwap(fn_ptrs)
    }

    /// Creates a Ruin selector from function pointers.
    pub fn ruin(fn_ptrs: BasicVariableFnPtrs<S, V>, ruin_count: usize) -> Self {
        Self::Ruin {
            fn_ptrs,
            ruin_count,
        }
    }

    /// Creates a ListAssign selector from function pointers.
    pub fn list_assign(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::ListAssign(fn_ptrs)
    }

    /// Creates a ListChange selector from function pointers.
    pub fn list_change(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::ListChange(fn_ptrs)
    }

    /// Creates a ListSwap selector from function pointers.
    pub fn list_swap(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::ListSwap(fn_ptrs)
    }

    /// Creates a ListReverse selector from function pointers.
    pub fn list_reverse(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_segment_len: usize,
        max_segment_len: Option<usize>,
    ) -> Self {
        Self::ListReverse {
            fn_ptrs,
            min_segment_len,
            max_segment_len,
        }
    }

    /// Creates a SubListChange selector from function pointers.
    pub fn sublist_change(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        Self::SubListChange {
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
        }
    }

    /// Creates a SubListSwap selector from function pointers.
    pub fn sublist_swap(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        Self::SubListSwap {
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
        }
    }

    /// Creates a KOpt selector from function pointers.
    pub fn k_opt(fn_ptrs: ListVariableFnPtrs<S, V>, k: usize, min_segment_len: usize) -> Self {
        Self::KOpt {
            fn_ptrs,
            k,
            min_segment_len,
        }
    }

    /// Creates a ListRuin selector from function pointers.
    pub fn list_ruin(fn_ptrs: ListVariableFnPtrs<S, V>, ruin_count: usize) -> Self {
        Self::ListRuin {
            fn_ptrs,
            ruin_count,
        }
    }

    /// Creates a Union selector from multiple selectors.
    pub fn union(selectors: Vec<MoveSelectorImpl<S, V>>) -> Self {
        Self::Union(selectors)
    }

    /// Creates a selector from config for basic variables.
    pub fn from_basic_config(
        config: &solverforge_config::MoveSelectorConfig,
        fn_ptrs: BasicVariableFnPtrs<S, V>,
    ) -> Option<Self> {
        use solverforge_config::MoveSelectorConfig;
        match config {
            MoveSelectorConfig::ChangeMoveSelector(_) => Some(Self::change(fn_ptrs)),
            MoveSelectorConfig::SwapMoveSelector(_) => Some(Self::swap(fn_ptrs)),
            MoveSelectorConfig::PillarChangeMoveSelector(_) => Some(Self::pillar_change(fn_ptrs)),
            MoveSelectorConfig::PillarSwapMoveSelector(_) => Some(Self::pillar_swap(fn_ptrs)),
            MoveSelectorConfig::RuinMoveSelector(cfg) => {
                Some(Self::ruin(fn_ptrs, cfg.ruin_count))
            }
            _ => None, // List variable configs not applicable
        }
    }

    /// Creates a selector from config for list variables.
    pub fn from_list_config(
        config: &solverforge_config::MoveSelectorConfig,
        fn_ptrs: ListVariableFnPtrs<S, V>,
    ) -> Option<Self> {
        use solverforge_config::MoveSelectorConfig;
        match config {
            MoveSelectorConfig::ListChangeMoveSelector(_) => Some(Self::list_change(fn_ptrs)),
            MoveSelectorConfig::ListSwapMoveSelector(_) => Some(Self::list_swap(fn_ptrs)),
            MoveSelectorConfig::ListReverseMoveSelector(cfg) => Some(Self::list_reverse(
                fn_ptrs,
                cfg.minimum_segment_length.unwrap_or(2),
                cfg.maximum_segment_length,
            )),
            MoveSelectorConfig::KOptMoveSelector(cfg) => {
                Some(Self::k_opt(fn_ptrs, cfg.k_value, 1))
            }
            MoveSelectorConfig::SubListChangeMoveSelector(cfg) => Some(Self::sublist_change(
                fn_ptrs,
                cfg.minimum_sub_list_size.unwrap_or(1),
                cfg.maximum_sub_list_size,
            )),
            MoveSelectorConfig::SubListSwapMoveSelector(cfg) => Some(Self::sublist_swap(
                fn_ptrs,
                cfg.minimum_sub_list_size.unwrap_or(1),
                cfg.maximum_sub_list_size,
            )),
            MoveSelectorConfig::ListRuinMoveSelector(cfg) => {
                Some(Self::list_ruin(fn_ptrs, cfg.ruin_count))
            }
            _ => None, // Basic variable configs not applicable
        }
    }
}

impl<S, V> Debug for MoveSelectorImpl<S, V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(fp) => f.debug_tuple("Change").field(fp).finish(),
            Self::Swap(fp) => f.debug_tuple("Swap").field(fp).finish(),
            Self::PillarChange(fp) => f.debug_tuple("PillarChange").field(fp).finish(),
            Self::PillarSwap(fp) => f.debug_tuple("PillarSwap").field(fp).finish(),
            Self::Ruin {
                fn_ptrs,
                ruin_count,
            } => f
                .debug_struct("Ruin")
                .field("fn_ptrs", fn_ptrs)
                .field("ruin_count", ruin_count)
                .finish(),
            Self::ListAssign(fp) => f.debug_tuple("ListAssign").field(fp).finish(),
            Self::ListChange(fp) => f.debug_tuple("ListChange").field(fp).finish(),
            Self::ListSwap(fp) => f.debug_tuple("ListSwap").field(fp).finish(),
            Self::ListReverse {
                fn_ptrs,
                min_segment_len,
                max_segment_len,
            } => f
                .debug_struct("ListReverse")
                .field("fn_ptrs", fn_ptrs)
                .field("min_segment_len", min_segment_len)
                .field("max_segment_len", max_segment_len)
                .finish(),
            Self::SubListChange {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
            } => f
                .debug_struct("SubListChange")
                .field("fn_ptrs", fn_ptrs)
                .field("min_sublist_len", min_sublist_len)
                .field("max_sublist_len", max_sublist_len)
                .finish(),
            Self::SubListSwap {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
            } => f
                .debug_struct("SubListSwap")
                .field("fn_ptrs", fn_ptrs)
                .field("min_sublist_len", min_sublist_len)
                .field("max_sublist_len", max_sublist_len)
                .finish(),
            Self::KOpt {
                fn_ptrs,
                k,
                min_segment_len,
            } => f
                .debug_struct("KOpt")
                .field("fn_ptrs", fn_ptrs)
                .field("k", k)
                .field("min_segment_len", min_segment_len)
                .finish(),
            Self::ListRuin {
                fn_ptrs,
                ruin_count,
            } => f
                .debug_struct("ListRuin")
                .field("fn_ptrs", fn_ptrs)
                .field("ruin_count", ruin_count)
                .finish(),
            Self::Union(selectors) => f.debug_tuple("Union").field(selectors).finish(),
        }
    }
}

// ============================================================================
// MoveSelector Implementation
// ============================================================================

impl<S, V> MoveSelector<S, MoveImpl<S, V>> for MoveSelectorImpl<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = MoveImpl<S, V>> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        match self {
            Self::Change(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let values = (fp.value_range)(solution);
                Box::new(ChangeMoveIterator::new(*fp, entity_count, values))
            }
            Self::Swap(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                Box::new(SwapMoveIterator::new(*fp, entity_count))
            }
            Self::PillarChange(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let values = (fp.value_range)(solution);
                let pillars = build_pillars(solution, entity_count, fp.getter);
                Box::new(PillarChangeMoveIterator::new(*fp, pillars, values))
            }
            Self::PillarSwap(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let pillars = build_pillars(solution, entity_count, fp.getter);
                Box::new(PillarSwapMoveIterator::new(*fp, pillars))
            }
            Self::Ruin {
                fn_ptrs,
                ruin_count,
            } => {
                let entity_count = (fn_ptrs.entity_count)(score_director.working_solution());
                Box::new(RuinMoveIterator::new(*fn_ptrs, entity_count, *ruin_count))
            }
            Self::ListAssign(_fp) => {
                // For construction, unassigned elements come from problem facts.
                // ListAssignMove generation is handled by the construction placer,
                // not by local search move selection.
                Box::new(std::iter::empty())
            }
            Self::ListChange(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fp.list_len)(solution, e))
                    .collect();
                Box::new(ListChangeMoveIterator::new(*fp, entity_count, list_lens))
            }
            Self::ListSwap(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fp.list_len)(solution, e))
                    .collect();
                Box::new(ListSwapMoveIterator::new(*fp, entity_count, list_lens))
            }
            Self::ListReverse {
                fn_ptrs,
                min_segment_len,
                max_segment_len,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                Box::new(ListReverseMoveIterator::new(
                    *fn_ptrs,
                    entity_count,
                    list_lens,
                    *min_segment_len,
                    *max_segment_len,
                ))
            }
            Self::SubListChange {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                Box::new(SubListChangeMoveIterator::new(
                    *fn_ptrs,
                    entity_count,
                    list_lens,
                    *min_sublist_len,
                    *max_sublist_len,
                ))
            }
            Self::SubListSwap {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                Box::new(SubListSwapMoveIterator::new(
                    *fn_ptrs,
                    entity_count,
                    list_lens,
                    *min_sublist_len,
                    *max_sublist_len,
                ))
            }
            Self::KOpt {
                fn_ptrs,
                k,
                min_segment_len,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                // Generate reconnection patterns for this k value
                let reconnections = enumerate_reconnections(*k);
                Box::new(KOptMoveIterator::new(
                    *fn_ptrs,
                    entity_count,
                    list_lens,
                    *k,
                    *min_segment_len,
                    reconnections,
                ))
            }
            Self::ListRuin {
                fn_ptrs,
                ruin_count,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                Box::new(ListRuinMoveIterator::new(
                    *fn_ptrs,
                    entity_count,
                    list_lens,
                    *ruin_count,
                ))
            }
            Self::Union(selectors) => {
                let iters: Vec<_> = selectors
                    .iter()
                    .map(|s| s.iter_moves(score_director))
                    .collect();
                Box::new(iters.into_iter().flatten())
            }
        }
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        match self {
            Self::Change(fp) => {
                let n = (fp.entity_count)(score_director.working_solution());
                let v = (fp.value_range)(score_director.working_solution()).len();
                n * v
            }
            Self::Swap(fp) => {
                let n = (fp.entity_count)(score_director.working_solution());
                n * (n.saturating_sub(1)) / 2
            }
            Self::PillarChange(_) | Self::PillarSwap(_) => {
                // Pillar size depends on solution state, estimate
                let fp = match self {
                    Self::PillarChange(fp) | Self::PillarSwap(fp) => fp,
                    _ => unreachable!(),
                };
                let n = (fp.entity_count)(score_director.working_solution());
                n
            }
            Self::Ruin {
                fn_ptrs,
                ruin_count,
            } => {
                let n = (fn_ptrs.entity_count)(score_director.working_solution());
                binomial(n, *ruin_count)
            }
            Self::ListAssign(fp) => {
                let solution = score_director.working_solution();
                let elem_count = (fp.element_count)(solution);
                let assigned = (fp.assigned_elements)(solution).len();
                let entity_count = (fp.entity_count)(solution);
                (elem_count.saturating_sub(assigned)) * entity_count
            }
            Self::ListChange(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let total_positions: usize =
                    (0..entity_count).map(|e| (fp.list_len)(solution, e)).sum();
                total_positions * total_positions
            }
            Self::ListSwap(fp) => {
                let solution = score_director.working_solution();
                let entity_count = (fp.entity_count)(solution);
                let total_positions: usize =
                    (0..entity_count).map(|e| (fp.list_len)(solution, e)).sum();
                total_positions * total_positions.saturating_sub(1) / 2
            }
            Self::ListReverse { fn_ptrs, .. } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                (0..entity_count)
                    .map(|e| {
                        let len = (fn_ptrs.list_len)(solution, e);
                        len * len / 2
                    })
                    .sum()
            }
            Self::SubListChange { fn_ptrs, .. } | Self::SubListSwap { fn_ptrs, .. } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                (0..entity_count)
                    .map(|e| {
                        let len = (fn_ptrs.list_len)(solution, e);
                        len * len
                    })
                    .sum()
            }
            Self::KOpt { fn_ptrs, k, .. } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let reconnection_count = enumerate_reconnections(*k).len();
                (0..entity_count)
                    .map(|e| {
                        let len = (fn_ptrs.list_len)(solution, e);
                        binomial(len, *k) * reconnection_count
                    })
                    .sum()
            }
            Self::ListRuin {
                fn_ptrs,
                ruin_count,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                (0..entity_count)
                    .map(|e| {
                        let len = (fn_ptrs.list_len)(solution, e);
                        binomial(len, *ruin_count)
                    })
                    .sum()
            }
            Self::Union(selectors) => selectors.iter().map(|s| s.size(score_director)).sum(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result.saturating_mul(n - i);
        result /= i + 1;
    }
    result
}

fn build_pillars<S, V: Clone + Eq + Hash>(
    solution: &S,
    entity_count: usize,
    getter: fn(&S, usize) -> Option<V>,
) -> Vec<(Option<V>, Vec<usize>)> {
    use std::collections::HashMap;

    let mut value_map: HashMap<Option<V>, Vec<usize>> = HashMap::new();
    for idx in 0..entity_count {
        let value = getter(solution, idx);
        value_map.entry(value).or_default().push(idx);
    }
    value_map.into_iter().collect()
}

// ============================================================================
// Move Iterators (no C parameter - data extracted at construction)
// ============================================================================

/// Iterator for ChangeMove generation.
struct ChangeMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    entity_count: usize,
    values: Vec<V>,
    entity_idx: usize,
    value_idx: usize,
}

impl<S, V> ChangeMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_count: usize, values: Vec<V>) -> Self {
        Self {
            fp,
            entity_count,
            values,
            entity_idx: 0,
            value_idx: 0,
        }
    }
}

impl<S, V> Iterator for ChangeMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.entity_idx < self.entity_count {
            if self.value_idx < self.values.len() {
                let entity_idx = self.entity_idx;
                let value = self.values[self.value_idx];
                self.value_idx += 1;

                let m = ChangeMove::new(
                    entity_idx,
                    Some(value),
                    self.fp.getter,
                    self.fp.setter,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                return Some(MoveImpl::Change(m));
            }
            self.value_idx = 0;
            self.entity_idx += 1;
        }
        None
    }
}

/// Iterator for SwapMove generation.
struct SwapMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    entity_count: usize,
    left_idx: usize,
    right_idx: usize,
}

impl<S, V> SwapMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_count: usize) -> Self {
        Self {
            fp,
            entity_count,
            left_idx: 0,
            right_idx: 1,
        }
    }
}

impl<S, V> Iterator for SwapMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.left_idx < self.entity_count {
            if self.right_idx < self.entity_count {
                let left = self.left_idx;
                let right = self.right_idx;
                self.right_idx += 1;

                let m = SwapMove::new(
                    left,
                    right,
                    self.fp.getter,
                    self.fp.setter,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                return Some(MoveImpl::Swap(m));
            }
            self.left_idx += 1;
            self.right_idx = self.left_idx + 1;
        }
        None
    }
}

/// Iterator for PillarChangeMove generation.
struct PillarChangeMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    pillars: Vec<(Option<V>, Vec<usize>)>,
    values: Vec<V>,
    pillar_idx: usize,
    value_idx: usize,
}

impl<S, V> PillarChangeMoveIterator<S, V> {
    fn new(
        fp: BasicVariableFnPtrs<S, V>,
        pillars: Vec<(Option<V>, Vec<usize>)>,
        values: Vec<V>,
    ) -> Self {
        Self {
            fp,
            pillars,
            values,
            pillar_idx: 0,
            value_idx: 0,
        }
    }
}

impl<S, V> Iterator for PillarChangeMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + PartialEq + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pillar_idx < self.pillars.len() {
            let (ref current_val, ref indices) = self.pillars[self.pillar_idx];
            if self.value_idx < self.values.len() {
                let to_value = &self.values[self.value_idx];
                self.value_idx += 1;

                // Skip if same value
                if current_val.as_ref() == Some(to_value) {
                    continue;
                }

                let m = PillarChangeMove::new(
                    indices.clone(),
                    Some(to_value.clone()),
                    self.fp.getter,
                    self.fp.setter,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                return Some(MoveImpl::PillarChange(m));
            }
            self.value_idx = 0;
            self.pillar_idx += 1;
        }
        None
    }
}

/// Iterator for PillarSwapMove generation.
struct PillarSwapMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    pillars: Vec<(Option<V>, Vec<usize>)>,
    left_idx: usize,
    right_idx: usize,
}

impl<S, V> PillarSwapMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, pillars: Vec<(Option<V>, Vec<usize>)>) -> Self {
        Self {
            fp,
            pillars,
            left_idx: 0,
            right_idx: 1,
        }
    }
}

impl<S, V> Iterator for PillarSwapMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.left_idx < self.pillars.len() {
            if self.right_idx < self.pillars.len() {
                let left = self.left_idx;
                let right = self.right_idx;
                self.right_idx += 1;

                let left_indices = self.pillars[left].1.clone();
                let right_indices = self.pillars[right].1.clone();

                let m = PillarSwapMove::new(
                    left_indices,
                    right_indices,
                    self.fp.getter,
                    self.fp.setter,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                return Some(MoveImpl::PillarSwap(m));
            }
            self.left_idx += 1;
            self.right_idx = self.left_idx + 1;
        }
        None
    }
}

/// Iterator for RuinMove generation.
struct RuinMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    entity_count: usize,
    ruin_count: usize,
    indices: Vec<usize>,
    done: bool,
}

impl<S, V> RuinMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_count: usize, ruin_count: usize) -> Self {
        let done = ruin_count > entity_count || ruin_count == 0;
        let indices = if done {
            vec![]
        } else {
            (0..ruin_count).collect()
        };
        Self {
            fp,
            entity_count,
            ruin_count,
            indices,
            done,
        }
    }

    fn advance_combination(&mut self) {
        if self.done {
            return;
        }

        let mut i = self.ruin_count;
        while i > 0 {
            i -= 1;
            if self.indices[i] < self.entity_count - self.ruin_count + i {
                self.indices[i] += 1;
                for j in (i + 1)..self.ruin_count {
                    self.indices[j] = self.indices[j - 1] + 1;
                }
                return;
            }
        }
        self.done = true;
    }
}

impl<S, V> Iterator for RuinMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let m = RuinMove::new(
            &self.indices,
            self.fp.getter,
            self.fp.setter,
            self.fp.variable_name,
            self.fp.descriptor_index,
        );

        self.advance_combination();
        Some(MoveImpl::Ruin(m))
    }
}

/// Iterator for ListChangeMove generation.
struct ListChangeMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    src_entity: usize,
    src_pos: usize,
    dst_entity: usize,
    dst_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListChangeMoveIterator<S, V> {
    fn new(fp: ListVariableFnPtrs<S, V>, entity_count: usize, list_lens: Vec<usize>) -> Self {
        Self {
            fp,
            entity_count,
            list_lens,
            src_entity: 0,
            src_pos: 0,
            dst_entity: 0,
            dst_pos: 0,
            _phantom: PhantomData,
        }
    }

    fn advance(&mut self) {
        self.dst_pos += 1;

        let max_dst = if self.src_entity == self.dst_entity {
            self.list_lens
                .get(self.dst_entity)
                .copied()
                .unwrap_or(0)
                .saturating_sub(1)
        } else {
            self.list_lens.get(self.dst_entity).copied().unwrap_or(0)
        };

        if self.dst_pos > max_dst {
            self.dst_pos = 0;
            self.dst_entity += 1;

            if self.dst_entity >= self.entity_count {
                self.dst_entity = 0;
                self.src_pos += 1;

                if self.src_pos >= self.list_lens.get(self.src_entity).copied().unwrap_or(0) {
                    self.src_pos = 0;
                    self.src_entity += 1;
                }
            }
        }
    }
}

impl<S, V> Iterator for ListChangeMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.src_entity >= self.entity_count {
                return None;
            }

            let src_len = self.list_lens.get(self.src_entity).copied().unwrap_or(0);
            if src_len == 0 {
                self.src_entity += 1;
                continue;
            }

            if self.src_pos >= src_len {
                self.src_pos = 0;
                self.src_entity += 1;
                continue;
            }

            // Skip no-op moves
            let is_noop = self.src_entity == self.dst_entity
                && (self.dst_pos == self.src_pos || self.dst_pos == self.src_pos + 1);

            if !is_noop {
                let m = ListChangeMove::new(
                    self.src_entity,
                    self.src_pos,
                    self.dst_entity,
                    self.dst_pos,
                    self.fp.list_len,
                    self.fp.list_remove,
                    self.fp.list_insert,
                    self.fp.list_get_element_idx,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                self.advance();
                return Some(MoveImpl::ListChange(m));
            }

            self.advance();
        }
    }
}

/// Iterator for ListSwapMove generation.
struct ListSwapMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    first_entity: usize,
    first_pos: usize,
    second_entity: usize,
    second_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListSwapMoveIterator<S, V> {
    fn new(fp: ListVariableFnPtrs<S, V>, entity_count: usize, list_lens: Vec<usize>) -> Self {
        Self {
            fp,
            entity_count,
            list_lens,
            first_entity: 0,
            first_pos: 0,
            second_entity: 0,
            second_pos: 1,
            _phantom: PhantomData,
        }
    }

    fn advance(&mut self) {
        self.second_pos += 1;

        let second_len = self.list_lens.get(self.second_entity).copied().unwrap_or(0);
        if self.second_pos >= second_len {
            self.second_entity += 1;
            self.second_pos = if self.first_entity == self.second_entity {
                self.first_pos + 1
            } else {
                0
            };

            if self.second_entity >= self.entity_count {
                self.first_pos += 1;
                let first_len = self.list_lens.get(self.first_entity).copied().unwrap_or(0);

                if self.first_pos >= first_len {
                    self.first_entity += 1;
                    self.first_pos = 0;
                }

                self.second_entity = self.first_entity;
                self.second_pos = self.first_pos + 1;
            }
        }
    }
}

impl<S, V> Iterator for ListSwapMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.first_entity >= self.entity_count {
                return None;
            }

            let first_len = self.list_lens.get(self.first_entity).copied().unwrap_or(0);
            if first_len == 0 {
                self.first_entity += 1;
                self.first_pos = 0;
                self.second_entity = self.first_entity;
                self.second_pos = 1;
                continue;
            }

            if self.first_pos >= first_len {
                self.first_entity += 1;
                self.first_pos = 0;
                self.second_entity = self.first_entity;
                self.second_pos = 1;
                continue;
            }

            let second_len = self.list_lens.get(self.second_entity).copied().unwrap_or(0);
            if self.second_entity >= self.entity_count || self.second_pos >= second_len {
                self.advance();
                continue;
            }

            let m = ListSwapMove::new(
                self.first_entity,
                self.first_pos,
                self.second_entity,
                self.second_pos,
                self.fp.list_len,
                self.fp.list_get,
                self.fp.list_set,
                self.fp.list_get_element_idx,
                self.fp.variable_name,
                self.fp.descriptor_index,
            );
            self.advance();
            return Some(MoveImpl::ListSwap(m));
        }
    }
}

/// Iterator for ListReverseMove generation.
struct ListReverseMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    min_segment_len: usize,
    max_segment_len: Option<usize>,
    entity_idx: usize,
    start: usize,
    end: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListReverseMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_count: usize,
        list_lens: Vec<usize>,
        min_segment_len: usize,
        max_segment_len: Option<usize>,
    ) -> Self {
        let min_segment_len = min_segment_len.max(2);
        Self {
            fp,
            entity_count,
            list_lens,
            min_segment_len,
            max_segment_len,
            entity_idx: 0,
            start: 0,
            end: min_segment_len,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Iterator for ListReverseMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.entity_idx >= self.entity_count {
                return None;
            }

            let list_len = self.list_lens.get(self.entity_idx).copied().unwrap_or(0);
            if list_len < self.min_segment_len {
                self.entity_idx += 1;
                self.start = 0;
                self.end = self.min_segment_len;
                continue;
            }

            let max_end = self
                .max_segment_len
                .map(|m| (self.start + m).min(list_len))
                .unwrap_or(list_len);

            if self.end > max_end {
                self.start += 1;
                self.end = self.start + self.min_segment_len;

                if self.start + self.min_segment_len > list_len {
                    self.entity_idx += 1;
                    self.start = 0;
                    self.end = self.min_segment_len;
                }
                continue;
            }

            let m = ListReverseMove::new(
                self.entity_idx,
                self.start,
                self.end,
                self.fp.list_len,
                self.fp.list_reverse,
                self.fp.list_get_element_idx,
                self.fp.variable_name,
                self.fp.descriptor_index,
            );

            self.end += 1;
            return Some(MoveImpl::ListReverse(m));
        }
    }
}

/// Iterator for SubListChangeMove generation.
struct SubListChangeMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    min_sublist_len: usize,
    max_sublist_len: Option<usize>,
    src_entity: usize,
    src_start: usize,
    src_end: usize,
    dst_entity: usize,
    dst_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> SubListChangeMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_count: usize,
        list_lens: Vec<usize>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        let min_sublist_len = min_sublist_len.max(1);
        Self {
            fp,
            entity_count,
            list_lens,
            min_sublist_len,
            max_sublist_len,
            src_entity: 0,
            src_start: 0,
            src_end: min_sublist_len,
            dst_entity: 0,
            dst_pos: 0,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Iterator for SubListChangeMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.src_entity >= self.entity_count {
                return None;
            }

            let src_len = self.list_lens.get(self.src_entity).copied().unwrap_or(0);
            if src_len < self.min_sublist_len || self.src_start + self.min_sublist_len > src_len {
                self.src_entity += 1;
                self.src_start = 0;
                self.src_end = self.min_sublist_len;
                self.dst_entity = 0;
                self.dst_pos = 0;
                continue;
            }

            let max_end = self
                .max_sublist_len
                .map(|m| (self.src_start + m).min(src_len))
                .unwrap_or(src_len);

            if self.src_end > max_end {
                self.src_start += 1;
                self.src_end = self.src_start + self.min_sublist_len;
                self.dst_entity = 0;
                self.dst_pos = 0;
                continue;
            }

            if self.dst_entity >= self.entity_count {
                self.src_end += 1;
                self.dst_entity = 0;
                self.dst_pos = 0;
                continue;
            }

            let dst_len = self.list_lens.get(self.dst_entity).copied().unwrap_or(0);
            let sublist_len = self.src_end - self.src_start;
            let max_dst = if self.src_entity == self.dst_entity {
                src_len.saturating_sub(sublist_len)
            } else {
                dst_len
            };

            if self.dst_pos > max_dst {
                self.dst_entity += 1;
                self.dst_pos = 0;
                continue;
            }

            // Skip no-op
            let is_noop = self.src_entity == self.dst_entity
                && self.dst_pos >= self.src_start
                && self.dst_pos <= self.src_end;

            if !is_noop {
                let m = SubListChangeMove::new(
                    self.src_entity,
                    self.src_start,
                    self.src_end,
                    self.dst_entity,
                    self.dst_pos,
                    self.fp.list_len,
                    self.fp.sublist_remove,
                    self.fp.sublist_insert,
                    self.fp.list_get_element_idx,
                    self.fp.variable_name,
                    self.fp.descriptor_index,
                );
                self.dst_pos += 1;
                return Some(MoveImpl::SubListChange(m));
            }

            self.dst_pos += 1;
        }
    }
}

/// Iterator for SubListSwapMove generation.
struct SubListSwapMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    min_sublist_len: usize,
    max_sublist_len: Option<usize>,
    first_entity: usize,
    first_start: usize,
    first_end: usize,
    second_entity: usize,
    second_start: usize,
    second_end: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> SubListSwapMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_count: usize,
        list_lens: Vec<usize>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        let min_sublist_len = min_sublist_len.max(1);
        Self {
            fp,
            entity_count,
            list_lens,
            min_sublist_len,
            max_sublist_len,
            first_entity: 0,
            first_start: 0,
            first_end: min_sublist_len,
            second_entity: 0,
            second_start: 0,
            second_end: min_sublist_len,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Iterator for SubListSwapMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.first_entity >= self.entity_count {
                return None;
            }

            let first_len = self.list_lens.get(self.first_entity).copied().unwrap_or(0);
            if first_len < self.min_sublist_len {
                self.first_entity += 1;
                self.first_start = 0;
                self.first_end = self.min_sublist_len;
                self.second_entity = 0;
                self.second_start = 0;
                self.second_end = self.min_sublist_len;
                continue;
            }

            // Advance to next valid pair
            self.second_end += 1;

            let second_len = self.list_lens.get(self.second_entity).copied().unwrap_or(0);
            let max_second_end = self
                .max_sublist_len
                .map(|m| (self.second_start + m).min(second_len))
                .unwrap_or(second_len);

            if self.second_end > max_second_end {
                self.second_start += 1;
                self.second_end = self.second_start + self.min_sublist_len;
            }

            if self.second_start + self.min_sublist_len > second_len {
                self.second_entity += 1;
                self.second_start = if self.first_entity == self.second_entity {
                    self.first_end
                } else {
                    0
                };
                self.second_end = self.second_start + self.min_sublist_len;
            }

            if self.second_entity >= self.entity_count {
                self.first_end += 1;
                let max_first_end = self
                    .max_sublist_len
                    .map(|m| (self.first_start + m).min(first_len))
                    .unwrap_or(first_len);

                if self.first_end > max_first_end {
                    self.first_start += 1;
                    self.first_end = self.first_start + self.min_sublist_len;
                }

                if self.first_start + self.min_sublist_len > first_len {
                    self.first_entity += 1;
                    self.first_start = 0;
                    self.first_end = self.min_sublist_len;
                }

                self.second_entity = self.first_entity;
                self.second_start = self.first_end;
                self.second_end = self.second_start + self.min_sublist_len;
                continue;
            }

            // Check for overlapping ranges in intra-list case
            if self.first_entity == self.second_entity {
                let overlaps =
                    self.first_start < self.second_end && self.second_start < self.first_end;
                if overlaps {
                    continue;
                }
            }

            let m = SubListSwapMove::new(
                self.first_entity,
                self.first_start,
                self.first_end,
                self.second_entity,
                self.second_start,
                self.second_end,
                self.fp.list_len,
                self.fp.sublist_remove,
                self.fp.sublist_insert,
                self.fp.list_get_element_idx,
                self.fp.variable_name,
                self.fp.descriptor_index,
            );
            return Some(MoveImpl::SubListSwap(m));
        }
    }
}

/// Iterator for KOptMove generation - supports any k value (2-5).
struct KOptMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    k: usize,
    min_segment_len: usize,
    reconnections: Vec<KOptReconnection>,
    entity_idx: usize,
    cuts: Vec<usize>,
    reconnection_idx: usize,
    done: bool,
    _phantom: PhantomData<V>,
}

impl<S, V> KOptMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_count: usize,
        list_lens: Vec<usize>,
        k: usize,
        min_segment_len: usize,
        reconnections: Vec<KOptReconnection>,
    ) -> Self {
        let cuts: Vec<usize> = (0..k).map(|i| (i + 1) * min_segment_len).collect();
        let done = !(2..=5).contains(&k) || reconnections.is_empty();

        Self {
            fp,
            entity_count,
            list_lens,
            k,
            min_segment_len,
            reconnections,
            entity_idx: 0,
            cuts,
            reconnection_idx: 0,
            done,
            _phantom: PhantomData,
        }
    }

    fn advance_cuts(&mut self, list_len: usize) -> bool {
        // Advance reconnection first
        self.reconnection_idx += 1;
        if self.reconnection_idx < self.reconnections.len() {
            return true;
        }
        self.reconnection_idx = 0;

        // Advance cut positions
        for i in (0..self.k).rev() {
            let max_pos = if i == self.k - 1 {
                list_len
            } else {
                self.cuts[i + 1].saturating_sub(self.min_segment_len)
            };

            if self.cuts[i] < max_pos {
                self.cuts[i] += 1;
                for j in (i + 1)..self.k {
                    self.cuts[j] = self.cuts[j - 1] + self.min_segment_len;
                }
                return true;
            }
        }

        false
    }
}

impl<S, V> Iterator for KOptMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            if self.entity_idx >= self.entity_count {
                return None;
            }

            let list_len = self.list_lens.get(self.entity_idx).copied().unwrap_or(0);
            let min_required = self.k * self.min_segment_len;

            if list_len < min_required {
                self.entity_idx += 1;
                self.cuts = (0..self.k)
                    .map(|i| (i + 1) * self.min_segment_len)
                    .collect();
                self.reconnection_idx = 0;
                continue;
            }

            // Check if current cuts are valid
            if self.cuts.last().copied().unwrap_or(0) > list_len {
                if !self.advance_cuts(list_len) {
                    self.entity_idx += 1;
                    self.cuts = (0..self.k)
                        .map(|i| (i + 1) * self.min_segment_len)
                        .collect();
                    self.reconnection_idx = 0;
                }
                continue;
            }

            let reconnection = match self.reconnections.get(self.reconnection_idx) {
                Some(r) => r,
                None => {
                    let _ = self.advance_cuts(list_len);
                    continue;
                }
            };

            let cut_points: Vec<CutPoint> = self
                .cuts
                .iter()
                .map(|&pos| CutPoint::new(self.entity_idx, pos))
                .collect();

            // Use leaked static reference for the reconnection pattern
            // This is safe because the reconnection patterns are generated once per k value
            let static_reconnection: &'static KOptReconnection = Box::leak(Box::new(*reconnection));

            let m = KOptMove::new(
                &cut_points,
                static_reconnection,
                self.fp.list_len,
                self.fp.sublist_remove,
                self.fp.sublist_insert,
                self.fp.variable_name,
                self.fp.descriptor_index,
            );

            if !self.advance_cuts(list_len) {
                self.entity_idx += 1;
                self.cuts = (0..self.k)
                    .map(|i| (i + 1) * self.min_segment_len)
                    .collect();
                self.reconnection_idx = 0;
            }

            return Some(MoveImpl::KOpt(m));
        }
    }
}

/// Iterator for ListRuinMove generation.
struct ListRuinMoveIterator<S, V> {
    fp: ListVariableFnPtrs<S, V>,
    entity_count: usize,
    list_lens: Vec<usize>,
    ruin_count: usize,
    entity_idx: usize,
    positions: Vec<usize>,
    done_for_entity: bool,
    _phantom: PhantomData<V>,
}

impl<S, V> ListRuinMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_count: usize,
        list_lens: Vec<usize>,
        ruin_count: usize,
    ) -> Self {
        let positions = (0..ruin_count).collect();

        Self {
            fp,
            entity_count,
            list_lens,
            ruin_count,
            entity_idx: 0,
            positions,
            done_for_entity: ruin_count == 0,
            _phantom: PhantomData,
        }
    }

    fn advance_combination(&mut self) {
        let list_len = self.list_lens.get(self.entity_idx).copied().unwrap_or(0);

        let mut i = self.ruin_count;
        while i > 0 {
            i -= 1;
            if self.positions[i] < list_len - self.ruin_count + i {
                self.positions[i] += 1;
                for j in (i + 1)..self.ruin_count {
                    self.positions[j] = self.positions[j - 1] + 1;
                }
                return;
            }
        }
        self.done_for_entity = true;
    }
}

impl<S, V> Iterator for ListRuinMoveIterator<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + Copy + Debug + 'static,
{
    type Item = MoveImpl<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.entity_idx >= self.entity_count {
                return None;
            }

            let list_len = self.list_lens.get(self.entity_idx).copied().unwrap_or(0);
            if list_len < self.ruin_count || self.done_for_entity {
                self.entity_idx += 1;
                self.positions = (0..self.ruin_count).collect();
                self.done_for_entity = self.ruin_count == 0;
                continue;
            }

            let m = ListRuinMove::new(
                self.entity_idx,
                &self.positions,
                self.fp.list_len,
                self.fp.list_remove,
                self.fp.list_insert,
                self.fp.list_get_element_idx,
                self.fp.variable_name,
                self.fp.descriptor_index,
            );

            self.advance_combination();
            return Some(MoveImpl::ListRuin(m));
        }
    }
}
