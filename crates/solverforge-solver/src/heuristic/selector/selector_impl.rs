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
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::{BasicVariableFnPtrs, MoveSelectorImpl};
//!
//! // Create function pointers for a basic variable
//! let fn_ptrs: BasicVariableFnPtrs<(), i32> = BasicVariableFnPtrs {
//!     entity_count: |_| 5,
//!     value_range: |_| vec![1, 2, 3],
//!     getter: |_, _| None,
//!     setter: |_, _, _| {},
//!     variable_name: "x",
//!     descriptor_index: 0,
//! };
//!
//! // Create a Change selector using constructor
//! let selector = MoveSelectorImpl::change(fn_ptrs);
//!
//! // Or create directly from config
//! use solverforge_config::{MoveSelectorConfig, ChangeMoveConfig};
//! let config = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig::default());
//! let selector = MoveSelectorImpl::from_basic_config(&config, fn_ptrs);
//! assert!(selector.is_some());
//! ```

use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng as _;
use solverforge_core::domain::PlanningSolution;

use super::SelectionOrder;
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
    Change {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates SwapMove: exchanges values between entities.
    Swap {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates PillarChangeMove: changes all entities with same value.
    PillarChange {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates PillarSwapMove: swaps between entity groups.
    PillarSwap {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates RuinMove: unassigns multiple entities (LNS).
    Ruin {
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        /// Number of entities to unassign per move.
        ruin_count: usize,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    // ========================================================================
    // List Variable Selectors
    // ========================================================================
    /// Generates ListAssignMove: assigns element to list (construction).
    ListAssign(ListVariableFnPtrs<S, V>),

    /// Generates ListChangeMove: relocates element within/between lists.
    ListChange {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates ListSwapMove: swaps two elements.
    ListSwap {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates ListReverseMove: reverses segment (2-opt style).
    ListReverse {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum segment length (default: 2).
        min_segment_len: usize,
        /// Maximum segment length (None = entire list).
        max_segment_len: Option<usize>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates SubListChangeMove: relocates contiguous sublist.
    SubListChange {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum sublist length.
        min_sublist_len: usize,
        /// Maximum sublist length (None = entire list).
        max_sublist_len: Option<usize>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates SubListSwapMove: swaps two sublists.
    SubListSwap {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Minimum sublist length.
        min_sublist_len: usize,
        /// Maximum sublist length (None = entire list).
        max_sublist_len: Option<usize>,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates KOptMove: k-opt tour optimization for any k (2-5).
    KOpt {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// k value (2-5).
        k: usize,
        /// Minimum segment length between cuts.
        min_segment_len: usize,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
    },

    /// Generates ListRuinMove: removes elements from lists (LNS).
    ListRuin {
        fn_ptrs: ListVariableFnPtrs<S, V>,
        /// Number of elements to remove per move.
        ruin_count: usize,
        selection_order: SelectionOrder,
        rng: RefCell<StdRng>,
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

/// Creates a default RNG for selectors.
fn default_rng() -> RefCell<StdRng> {
    RefCell::new(StdRng::from_os_rng())
}

impl<S, V> MoveSelectorImpl<S, V> {
    /// Creates a Change selector from function pointers with default Original order.
    pub fn change(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::change_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a Change selector with specified selection order.
    pub fn change_with_order(
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::Change {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a Swap selector from function pointers with default Original order.
    pub fn swap(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::swap_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a Swap selector with specified selection order.
    pub fn swap_with_order(
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::Swap {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a PillarChange selector from function pointers with default Original order.
    pub fn pillar_change(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::pillar_change_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a PillarChange selector with specified selection order.
    pub fn pillar_change_with_order(
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::PillarChange {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a PillarSwap selector from function pointers with default Original order.
    pub fn pillar_swap(fn_ptrs: BasicVariableFnPtrs<S, V>) -> Self {
        Self::pillar_swap_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a PillarSwap selector with specified selection order.
    pub fn pillar_swap_with_order(
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::PillarSwap {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a Ruin selector from function pointers with default Original order.
    pub fn ruin(fn_ptrs: BasicVariableFnPtrs<S, V>, ruin_count: usize) -> Self {
        Self::ruin_with_order(fn_ptrs, ruin_count, SelectionOrder::Original)
    }

    /// Creates a Ruin selector with specified selection order.
    pub fn ruin_with_order(
        fn_ptrs: BasicVariableFnPtrs<S, V>,
        ruin_count: usize,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::Ruin {
            fn_ptrs,
            ruin_count,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a ListAssign selector from function pointers.
    pub fn list_assign(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::ListAssign(fn_ptrs)
    }

    /// Creates a ListChange selector from function pointers with default Original order.
    pub fn list_change(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::list_change_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a ListChange selector with specified selection order.
    pub fn list_change_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::ListChange {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a ListSwap selector from function pointers with default Original order.
    pub fn list_swap(fn_ptrs: ListVariableFnPtrs<S, V>) -> Self {
        Self::list_swap_with_order(fn_ptrs, SelectionOrder::Original)
    }

    /// Creates a ListSwap selector with specified selection order.
    pub fn list_swap_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::ListSwap {
            fn_ptrs,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a ListReverse selector from function pointers with default Original order.
    pub fn list_reverse(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_segment_len: usize,
        max_segment_len: Option<usize>,
    ) -> Self {
        Self::list_reverse_with_order(
            fn_ptrs,
            min_segment_len,
            max_segment_len,
            SelectionOrder::Original,
        )
    }

    /// Creates a ListReverse selector with specified selection order.
    pub fn list_reverse_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_segment_len: usize,
        max_segment_len: Option<usize>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::ListReverse {
            fn_ptrs,
            min_segment_len,
            max_segment_len,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a SubListChange selector from function pointers with default Original order.
    pub fn sublist_change(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        Self::sublist_change_with_order(
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
            SelectionOrder::Original,
        )
    }

    /// Creates a SubListChange selector with specified selection order.
    pub fn sublist_change_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::SubListChange {
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a SubListSwap selector from function pointers with default Original order.
    pub fn sublist_swap(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        Self::sublist_swap_with_order(
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
            SelectionOrder::Original,
        )
    }

    /// Creates a SubListSwap selector with specified selection order.
    pub fn sublist_swap_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::SubListSwap {
            fn_ptrs,
            min_sublist_len,
            max_sublist_len,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a KOpt selector from function pointers with default Original order.
    pub fn k_opt(fn_ptrs: ListVariableFnPtrs<S, V>, k: usize, min_segment_len: usize) -> Self {
        Self::k_opt_with_order(fn_ptrs, k, min_segment_len, SelectionOrder::Original)
    }

    /// Creates a KOpt selector with specified selection order.
    pub fn k_opt_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        k: usize,
        min_segment_len: usize,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::KOpt {
            fn_ptrs,
            k,
            min_segment_len,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a ListRuin selector from function pointers with default Original order.
    pub fn list_ruin(fn_ptrs: ListVariableFnPtrs<S, V>, ruin_count: usize) -> Self {
        Self::list_ruin_with_order(fn_ptrs, ruin_count, SelectionOrder::Original)
    }

    /// Creates a ListRuin selector with specified selection order.
    pub fn list_ruin_with_order(
        fn_ptrs: ListVariableFnPtrs<S, V>,
        ruin_count: usize,
        selection_order: SelectionOrder,
    ) -> Self {
        Self::ListRuin {
            fn_ptrs,
            ruin_count,
            selection_order,
            rng: default_rng(),
        }
    }

    /// Creates a Union selector from multiple selectors.
    ///
    /// Union selectors combine moves from multiple sources, enabling mixed
    /// move neighborhoods (e.g., Change + Swap, or ListChange + ListSwap + KOpt).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::heuristic::selector::{BasicVariableFnPtrs, MoveSelectorImpl};
    ///
    /// // Create function pointers for a basic variable
    /// let fn_ptrs: BasicVariableFnPtrs<(), i32> = BasicVariableFnPtrs {
    ///     entity_count: |_| 3,
    ///     value_range: |_| vec![1, 2],
    ///     getter: |_, _| None,
    ///     setter: |_, _, _| {},
    ///     variable_name: "x",
    ///     descriptor_index: 0,
    /// };
    ///
    /// // Create a union of Change and Swap selectors
    /// let union_selector = MoveSelectorImpl::union(vec![
    ///     MoveSelectorImpl::change(fn_ptrs),
    ///     MoveSelectorImpl::swap(fn_ptrs),
    /// ]);
    ///
    /// // Union combines moves from both selectors
    /// match &union_selector {
    ///     MoveSelectorImpl::Union(selectors) => assert_eq!(selectors.len(), 2),
    ///     _ => panic!("Expected Union variant"),
    /// }
    /// ```
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
            MoveSelectorConfig::ChangeMoveSelector(cfg) => {
                Some(Self::change_with_order(fn_ptrs, cfg.selection_order.into()))
            }
            MoveSelectorConfig::SwapMoveSelector(cfg) => {
                Some(Self::swap_with_order(fn_ptrs, cfg.selection_order.into()))
            }
            MoveSelectorConfig::PillarChangeMoveSelector(cfg) => Some(
                Self::pillar_change_with_order(fn_ptrs, cfg.selection_order.into()),
            ),
            MoveSelectorConfig::PillarSwapMoveSelector(cfg) => Some(Self::pillar_swap_with_order(
                fn_ptrs,
                cfg.selection_order.into(),
            )),
            MoveSelectorConfig::RuinMoveSelector(cfg) => Some(Self::ruin_with_order(
                fn_ptrs,
                cfg.ruin_count,
                cfg.selection_order.into(),
            )),
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
            MoveSelectorConfig::ListChangeMoveSelector(cfg) => Some(Self::list_change_with_order(
                fn_ptrs,
                cfg.selection_order.into(),
            )),
            MoveSelectorConfig::ListSwapMoveSelector(cfg) => Some(Self::list_swap_with_order(
                fn_ptrs,
                cfg.selection_order.into(),
            )),
            MoveSelectorConfig::ListReverseMoveSelector(cfg) => {
                Some(Self::list_reverse_with_order(
                    fn_ptrs,
                    cfg.minimum_segment_length.unwrap_or(2),
                    cfg.maximum_segment_length,
                    cfg.selection_order.into(),
                ))
            }
            MoveSelectorConfig::KOptMoveSelector(cfg) => Some(Self::k_opt_with_order(
                fn_ptrs,
                cfg.k_value,
                1,
                cfg.selection_order.into(),
            )),
            MoveSelectorConfig::SubListChangeMoveSelector(cfg) => {
                Some(Self::sublist_change_with_order(
                    fn_ptrs,
                    cfg.minimum_sub_list_size.unwrap_or(1),
                    cfg.maximum_sub_list_size,
                    cfg.selection_order.into(),
                ))
            }
            MoveSelectorConfig::SubListSwapMoveSelector(cfg) => {
                Some(Self::sublist_swap_with_order(
                    fn_ptrs,
                    cfg.minimum_sub_list_size.unwrap_or(1),
                    cfg.maximum_sub_list_size,
                    cfg.selection_order.into(),
                ))
            }
            MoveSelectorConfig::ListRuinMoveSelector(cfg) => Some(Self::list_ruin_with_order(
                fn_ptrs,
                cfg.ruin_count,
                cfg.selection_order.into(),
            )),
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
            Self::Change {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("Change")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::Swap {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("Swap")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::PillarChange {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("PillarChange")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::PillarSwap {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("PillarSwap")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::Ruin {
                fn_ptrs,
                ruin_count,
                selection_order,
                ..
            } => f
                .debug_struct("Ruin")
                .field("fn_ptrs", fn_ptrs)
                .field("ruin_count", ruin_count)
                .field("selection_order", selection_order)
                .finish(),
            Self::ListAssign(fp) => f.debug_tuple("ListAssign").field(fp).finish(),
            Self::ListChange {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("ListChange")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::ListSwap {
                fn_ptrs,
                selection_order,
                ..
            } => f
                .debug_struct("ListSwap")
                .field("fn_ptrs", fn_ptrs)
                .field("selection_order", selection_order)
                .finish(),
            Self::ListReverse {
                fn_ptrs,
                min_segment_len,
                max_segment_len,
                selection_order,
                ..
            } => f
                .debug_struct("ListReverse")
                .field("fn_ptrs", fn_ptrs)
                .field("min_segment_len", min_segment_len)
                .field("max_segment_len", max_segment_len)
                .field("selection_order", selection_order)
                .finish(),
            Self::SubListChange {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
                selection_order,
                ..
            } => f
                .debug_struct("SubListChange")
                .field("fn_ptrs", fn_ptrs)
                .field("min_sublist_len", min_sublist_len)
                .field("max_sublist_len", max_sublist_len)
                .field("selection_order", selection_order)
                .finish(),
            Self::SubListSwap {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
                selection_order,
                ..
            } => f
                .debug_struct("SubListSwap")
                .field("fn_ptrs", fn_ptrs)
                .field("min_sublist_len", min_sublist_len)
                .field("max_sublist_len", max_sublist_len)
                .field("selection_order", selection_order)
                .finish(),
            Self::KOpt {
                fn_ptrs,
                k,
                min_segment_len,
                selection_order,
                ..
            } => f
                .debug_struct("KOpt")
                .field("fn_ptrs", fn_ptrs)
                .field("k", k)
                .field("min_segment_len", min_segment_len)
                .field("selection_order", selection_order)
                .finish(),
            Self::ListRuin {
                fn_ptrs,
                ruin_count,
                selection_order,
                ..
            } => f
                .debug_struct("ListRuin")
                .field("fn_ptrs", fn_ptrs)
                .field("ruin_count", ruin_count)
                .field("selection_order", selection_order)
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
            Self::Change {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let values = (fn_ptrs.value_range)(solution);
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(ChangeMoveIterator::new(*fn_ptrs, entity_order, values))
            }
            Self::Swap {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(SwapMoveIterator::new(*fn_ptrs, entity_order))
            }
            Self::PillarChange {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let values = (fn_ptrs.value_range)(solution);
                let mut pillars = build_pillars(solution, entity_count, fn_ptrs.getter);
                shuffle_pillars(&mut pillars, *selection_order, rng);
                Box::new(PillarChangeMoveIterator::new(*fn_ptrs, pillars, values))
            }
            Self::PillarSwap {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let mut pillars = build_pillars(solution, entity_count, fn_ptrs.getter);
                shuffle_pillars(&mut pillars, *selection_order, rng);
                Box::new(PillarSwapMoveIterator::new(*fn_ptrs, pillars))
            }
            Self::Ruin {
                fn_ptrs,
                ruin_count,
                selection_order,
                rng,
            } => {
                let entity_count = (fn_ptrs.entity_count)(score_director.working_solution());
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(RuinMoveIterator::new(*fn_ptrs, entity_order, *ruin_count))
            }
            Self::ListAssign(_fp) => {
                // For construction, unassigned elements come from problem facts.
                // ListAssignMove generation is handled by the construction placer,
                // not by local search move selection.
                Box::new(std::iter::empty())
            }
            Self::ListChange {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(ListChangeMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
                    list_lens,
                ))
            }
            Self::ListSwap {
                fn_ptrs,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(ListSwapMoveIterator::new(*fn_ptrs, entity_order, list_lens))
            }
            Self::ListReverse {
                fn_ptrs,
                min_segment_len,
                max_segment_len,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(ListReverseMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
                    list_lens,
                    *min_segment_len,
                    *max_segment_len,
                ))
            }
            Self::SubListChange {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(SubListChangeMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
                    list_lens,
                    *min_sublist_len,
                    *max_sublist_len,
                ))
            }
            Self::SubListSwap {
                fn_ptrs,
                min_sublist_len,
                max_sublist_len,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(SubListSwapMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
                    list_lens,
                    *min_sublist_len,
                    *max_sublist_len,
                ))
            }
            Self::KOpt {
                fn_ptrs,
                k,
                min_segment_len,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                // Generate reconnection patterns for this k value
                let reconnections = enumerate_reconnections(*k);
                Box::new(KOptMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
                    list_lens,
                    *k,
                    *min_segment_len,
                    reconnections,
                ))
            }
            Self::ListRuin {
                fn_ptrs,
                ruin_count,
                selection_order,
                rng,
            } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let list_lens: Vec<_> = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .collect();
                let entity_order = create_entity_order(entity_count, *selection_order, rng);
                Box::new(ListRuinMoveIterator::new(
                    *fn_ptrs,
                    entity_order,
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
            Self::Change { fn_ptrs, .. } => {
                let n = (fn_ptrs.entity_count)(score_director.working_solution());
                let v = (fn_ptrs.value_range)(score_director.working_solution()).len();
                n * v
            }
            Self::Swap { fn_ptrs, .. } => {
                let n = (fn_ptrs.entity_count)(score_director.working_solution());
                n * (n.saturating_sub(1)) / 2
            }
            Self::PillarChange { fn_ptrs, .. } | Self::PillarSwap { fn_ptrs, .. } => {
                // Pillar size depends on solution state, estimate
                let n = (fn_ptrs.entity_count)(score_director.working_solution());
                n
            }
            Self::Ruin {
                fn_ptrs,
                ruin_count,
                ..
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
            Self::ListChange { fn_ptrs, .. } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let total_positions: usize = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .sum();
                total_positions * total_positions
            }
            Self::ListSwap { fn_ptrs, .. } => {
                let solution = score_director.working_solution();
                let entity_count = (fn_ptrs.entity_count)(solution);
                let total_positions: usize = (0..entity_count)
                    .map(|e| (fn_ptrs.list_len)(solution, e))
                    .sum();
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
                ..
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

/// Entity order representation: either a sequential range (zero allocation) or shuffled Vec.
///
/// For `Original` selection order, we use `Range` to avoid allocating a Vec.
/// For `Shuffled`/`Random`, we allocate and shuffle once.
#[derive(Clone)]
enum EntityOrder {
    /// Sequential order: 0, 1, 2, ..., n-1 (no allocation)
    Range(std::ops::Range<usize>),
    /// Shuffled order: pre-shuffled indices
    Shuffled(Vec<usize>),
}

impl EntityOrder {
    /// Returns the number of entities.
    #[inline]
    fn len(&self) -> usize {
        match self {
            EntityOrder::Range(r) => r.len(),
            EntityOrder::Shuffled(v) => v.len(),
        }
    }

    /// Gets entity at logical index.
    #[inline]
    fn get(&self, idx: usize) -> usize {
        match self {
            EntityOrder::Range(r) => r.start + idx,
            EntityOrder::Shuffled(v) => v[idx],
        }
    }
}

/// Creates an entity order based on selection order.
///
/// - `Original`: sequential range (0, 1, 2, ...) - no allocation
/// - `Shuffled`: shuffled once, then iterate sequentially
/// - `Random`: same as Shuffled for finite iterators
fn create_entity_order(
    entity_count: usize,
    selection_order: SelectionOrder,
    rng: &RefCell<StdRng>,
) -> EntityOrder {
    if matches!(
        selection_order,
        SelectionOrder::Shuffled | SelectionOrder::Random
    ) {
        let mut order: Vec<usize> = (0..entity_count).collect();
        order.shuffle(&mut *rng.borrow_mut());
        EntityOrder::Shuffled(order)
    } else {
        EntityOrder::Range(0..entity_count)
    }
}

/// Shuffles pillars based on selection order.
fn shuffle_pillars<V>(
    pillars: &mut [(Option<V>, Vec<usize>)],
    selection_order: SelectionOrder,
    rng: &RefCell<StdRng>,
) {
    if matches!(
        selection_order,
        SelectionOrder::Shuffled | SelectionOrder::Random
    ) {
        pillars.shuffle(&mut *rng.borrow_mut());
    }
}

// ============================================================================
// Move Iterators (no C parameter - data extracted at construction)
// ============================================================================

/// Iterator for ChangeMove generation.
struct ChangeMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    entity_order: EntityOrder,
    values: Vec<V>,
    entity_order_idx: usize,
    value_idx: usize,
}

impl<S, V> ChangeMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_order: EntityOrder, values: Vec<V>) -> Self {
        Self {
            fp,
            entity_order,
            values,
            entity_order_idx: 0,
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
        while self.entity_order_idx < self.entity_order.len() {
            if self.value_idx < self.values.len() {
                let entity_idx = self.entity_order.get(self.entity_order_idx);
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
            self.entity_order_idx += 1;
        }
        None
    }
}

/// Iterator for SwapMove generation.
struct SwapMoveIterator<S, V> {
    fp: BasicVariableFnPtrs<S, V>,
    entity_order: EntityOrder,
    left_order_idx: usize,
    right_order_idx: usize,
}

impl<S, V> SwapMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_order: EntityOrder) -> Self {
        Self {
            fp,
            entity_order,
            left_order_idx: 0,
            right_order_idx: 1,
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
        let entity_count = self.entity_order.len();
        while self.left_order_idx < entity_count {
            if self.right_order_idx < entity_count {
                let left = self.entity_order.get(self.left_order_idx);
                let right = self.entity_order.get(self.right_order_idx);
                self.right_order_idx += 1;

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
            self.left_order_idx += 1;
            self.right_order_idx = self.left_order_idx + 1;
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
    entity_order: EntityOrder,
    ruin_count: usize,
    combination_indices: Vec<usize>,
    done: bool,
}

impl<S, V> RuinMoveIterator<S, V> {
    fn new(fp: BasicVariableFnPtrs<S, V>, entity_order: EntityOrder, ruin_count: usize) -> Self {
        let entity_count = entity_order.len();
        let done = ruin_count > entity_count || ruin_count == 0;
        let combination_indices = if done {
            vec![]
        } else {
            (0..ruin_count).collect()
        };
        Self {
            fp,
            entity_order,
            ruin_count,
            combination_indices,
            done,
        }
    }

    fn advance_combination(&mut self) {
        if self.done {
            return;
        }

        let entity_count = self.entity_order.len();
        let mut i = self.ruin_count;
        while i > 0 {
            i -= 1;
            if self.combination_indices[i] < entity_count - self.ruin_count + i {
                self.combination_indices[i] += 1;
                for j in (i + 1)..self.ruin_count {
                    self.combination_indices[j] = self.combination_indices[j - 1] + 1;
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

        // Map combination indices through entity_order
        let entity_indices: Vec<usize> = self
            .combination_indices
            .iter()
            .map(|&i| self.entity_order.get(i))
            .collect();

        let m = RuinMove::new(
            &entity_indices,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    src_order_idx: usize,
    src_pos: usize,
    dst_order_idx: usize,
    dst_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListChangeMoveIterator<S, V> {
    fn new(fp: ListVariableFnPtrs<S, V>, entity_order: EntityOrder, list_lens: Vec<usize>) -> Self {
        Self {
            fp,
            entity_order,
            list_lens,
            src_order_idx: 0,
            src_pos: 0,
            dst_order_idx: 0,
            dst_pos: 0,
            _phantom: PhantomData,
        }
    }

    fn src_entity(&self) -> usize {
        self.entity_order.get(self.src_order_idx)
    }

    fn dst_entity(&self) -> usize {
        self.entity_order.get(self.dst_order_idx)
    }

    fn advance(&mut self) {
        self.dst_pos += 1;

        let max_dst = if self.src_order_idx == self.dst_order_idx {
            self.list_lens
                .get(self.dst_entity())
                .copied()
                .unwrap_or(0)
                .saturating_sub(1)
        } else {
            self.list_lens.get(self.dst_entity()).copied().unwrap_or(0)
        };

        if self.dst_pos > max_dst {
            self.dst_pos = 0;
            self.dst_order_idx += 1;

            if self.dst_order_idx >= self.entity_order.len() {
                self.dst_order_idx = 0;
                self.src_pos += 1;

                if self.src_pos >= self.list_lens.get(self.src_entity()).copied().unwrap_or(0) {
                    self.src_pos = 0;
                    self.src_order_idx += 1;
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
            if self.src_order_idx >= self.entity_order.len() {
                return None;
            }

            let src_entity = self.src_entity();
            let src_len = self.list_lens.get(src_entity).copied().unwrap_or(0);
            if src_len == 0 {
                self.src_order_idx += 1;
                continue;
            }

            if self.src_pos >= src_len {
                self.src_pos = 0;
                self.src_order_idx += 1;
                continue;
            }

            let dst_entity = self.dst_entity();
            // Skip no-op moves
            let is_noop = src_entity == dst_entity
                && (self.dst_pos == self.src_pos || self.dst_pos == self.src_pos + 1);

            if !is_noop {
                let m = ListChangeMove::new(
                    src_entity,
                    self.src_pos,
                    dst_entity,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    first_order_idx: usize,
    first_pos: usize,
    second_order_idx: usize,
    second_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListSwapMoveIterator<S, V> {
    fn new(fp: ListVariableFnPtrs<S, V>, entity_order: EntityOrder, list_lens: Vec<usize>) -> Self {
        Self {
            fp,
            entity_order,
            list_lens,
            first_order_idx: 0,
            first_pos: 0,
            second_order_idx: 0,
            second_pos: 1,
            _phantom: PhantomData,
        }
    }

    fn first_entity(&self) -> usize {
        self.entity_order.get(self.first_order_idx)
    }

    fn second_entity(&self) -> usize {
        self.entity_order.get(self.second_order_idx)
    }

    fn advance(&mut self) {
        self.second_pos += 1;

        let second_len = self
            .list_lens
            .get(self.second_entity())
            .copied()
            .unwrap_or(0);
        if self.second_pos >= second_len {
            self.second_order_idx += 1;
            self.second_pos = if self.first_order_idx == self.second_order_idx {
                self.first_pos + 1
            } else {
                0
            };

            if self.second_order_idx >= self.entity_order.len() {
                self.first_pos += 1;
                let first_len = self
                    .list_lens
                    .get(self.first_entity())
                    .copied()
                    .unwrap_or(0);

                if self.first_pos >= first_len {
                    self.first_order_idx += 1;
                    self.first_pos = 0;
                }

                self.second_order_idx = self.first_order_idx;
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
            if self.first_order_idx >= self.entity_order.len() {
                return None;
            }

            let first_entity = self.first_entity();
            let first_len = self.list_lens.get(first_entity).copied().unwrap_or(0);
            if first_len == 0 {
                self.first_order_idx += 1;
                self.first_pos = 0;
                self.second_order_idx = self.first_order_idx;
                self.second_pos = 1;
                continue;
            }

            if self.first_pos >= first_len {
                self.first_order_idx += 1;
                self.first_pos = 0;
                self.second_order_idx = self.first_order_idx;
                self.second_pos = 1;
                continue;
            }

            let second_entity = self.second_entity();
            let second_len = self.list_lens.get(second_entity).copied().unwrap_or(0);
            if self.second_order_idx >= self.entity_order.len() || self.second_pos >= second_len {
                self.advance();
                continue;
            }

            let m = ListSwapMove::new(
                first_entity,
                self.first_pos,
                second_entity,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    min_segment_len: usize,
    max_segment_len: Option<usize>,
    entity_order_idx: usize,
    start: usize,
    end: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> ListReverseMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_order: EntityOrder,
        list_lens: Vec<usize>,
        min_segment_len: usize,
        max_segment_len: Option<usize>,
    ) -> Self {
        let min_segment_len = min_segment_len.max(2);
        Self {
            fp,
            entity_order,
            list_lens,
            min_segment_len,
            max_segment_len,
            entity_order_idx: 0,
            start: 0,
            end: min_segment_len,
            _phantom: PhantomData,
        }
    }

    fn entity(&self) -> usize {
        self.entity_order.get(self.entity_order_idx)
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
            if self.entity_order_idx >= self.entity_order.len() {
                return None;
            }

            let entity_idx = self.entity();
            let list_len = self.list_lens.get(entity_idx).copied().unwrap_or(0);
            if list_len < self.min_segment_len {
                self.entity_order_idx += 1;
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
                    self.entity_order_idx += 1;
                    self.start = 0;
                    self.end = self.min_segment_len;
                }
                continue;
            }

            let m = ListReverseMove::new(
                entity_idx,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    min_sublist_len: usize,
    max_sublist_len: Option<usize>,
    src_order_idx: usize,
    src_start: usize,
    src_end: usize,
    dst_order_idx: usize,
    dst_pos: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> SubListChangeMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_order: EntityOrder,
        list_lens: Vec<usize>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        let min_sublist_len = min_sublist_len.max(1);
        Self {
            fp,
            entity_order,
            list_lens,
            min_sublist_len,
            max_sublist_len,
            src_order_idx: 0,
            src_start: 0,
            src_end: min_sublist_len,
            dst_order_idx: 0,
            dst_pos: 0,
            _phantom: PhantomData,
        }
    }

    fn src_entity(&self) -> usize {
        self.entity_order.get(self.src_order_idx)
    }

    fn dst_entity(&self) -> usize {
        self.entity_order.get(self.dst_order_idx)
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
            if self.src_order_idx >= self.entity_order.len() {
                return None;
            }

            let src_entity = self.src_entity();
            let src_len = self.list_lens.get(src_entity).copied().unwrap_or(0);
            if src_len < self.min_sublist_len || self.src_start + self.min_sublist_len > src_len {
                self.src_order_idx += 1;
                self.src_start = 0;
                self.src_end = self.min_sublist_len;
                self.dst_order_idx = 0;
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
                self.dst_order_idx = 0;
                self.dst_pos = 0;
                continue;
            }

            if self.dst_order_idx >= self.entity_order.len() {
                self.src_end += 1;
                self.dst_order_idx = 0;
                self.dst_pos = 0;
                continue;
            }

            let dst_entity = self.dst_entity();
            let dst_len = self.list_lens.get(dst_entity).copied().unwrap_or(0);
            let sublist_len = self.src_end - self.src_start;
            let max_dst = if src_entity == dst_entity {
                src_len.saturating_sub(sublist_len)
            } else {
                dst_len
            };

            if self.dst_pos > max_dst {
                self.dst_order_idx += 1;
                self.dst_pos = 0;
                continue;
            }

            // Skip no-op
            let is_noop = src_entity == dst_entity
                && self.dst_pos >= self.src_start
                && self.dst_pos <= self.src_end;

            if !is_noop {
                let m = SubListChangeMove::new(
                    src_entity,
                    self.src_start,
                    self.src_end,
                    dst_entity,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    min_sublist_len: usize,
    max_sublist_len: Option<usize>,
    first_order_idx: usize,
    first_start: usize,
    first_end: usize,
    second_order_idx: usize,
    second_start: usize,
    second_end: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> SubListSwapMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_order: EntityOrder,
        list_lens: Vec<usize>,
        min_sublist_len: usize,
        max_sublist_len: Option<usize>,
    ) -> Self {
        let min_sublist_len = min_sublist_len.max(1);
        Self {
            fp,
            entity_order,
            list_lens,
            min_sublist_len,
            max_sublist_len,
            first_order_idx: 0,
            first_start: 0,
            first_end: min_sublist_len,
            second_order_idx: 0,
            second_start: 0,
            second_end: min_sublist_len,
            _phantom: PhantomData,
        }
    }

    fn first_entity(&self) -> usize {
        self.entity_order.get(self.first_order_idx)
    }

    fn second_entity(&self) -> usize {
        self.entity_order.get(self.second_order_idx)
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
            if self.first_order_idx >= self.entity_order.len() {
                return None;
            }

            let first_entity = self.first_entity();
            let first_len = self.list_lens.get(first_entity).copied().unwrap_or(0);
            if first_len < self.min_sublist_len {
                self.first_order_idx += 1;
                self.first_start = 0;
                self.first_end = self.min_sublist_len;
                self.second_order_idx = 0;
                self.second_start = 0;
                self.second_end = self.min_sublist_len;
                continue;
            }

            // Advance to next valid pair
            self.second_end += 1;

            let second_entity = self.second_entity();
            let second_len = self.list_lens.get(second_entity).copied().unwrap_or(0);
            let max_second_end = self
                .max_sublist_len
                .map(|m| (self.second_start + m).min(second_len))
                .unwrap_or(second_len);

            if self.second_end > max_second_end {
                self.second_start += 1;
                self.second_end = self.second_start + self.min_sublist_len;
            }

            if self.second_start + self.min_sublist_len > second_len {
                self.second_order_idx += 1;
                self.second_start = if self.first_order_idx == self.second_order_idx {
                    self.first_end
                } else {
                    0
                };
                self.second_end = self.second_start + self.min_sublist_len;
            }

            if self.second_order_idx >= self.entity_order.len() {
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
                    self.first_order_idx += 1;
                    self.first_start = 0;
                    self.first_end = self.min_sublist_len;
                }

                self.second_order_idx = self.first_order_idx;
                self.second_start = self.first_end;
                self.second_end = self.second_start + self.min_sublist_len;
                continue;
            }

            // Check for overlapping ranges in intra-list case
            if first_entity == second_entity {
                let overlaps =
                    self.first_start < self.second_end && self.second_start < self.first_end;
                if overlaps {
                    continue;
                }
            }

            let m = SubListSwapMove::new(
                first_entity,
                self.first_start,
                self.first_end,
                second_entity,
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    k: usize,
    min_segment_len: usize,
    reconnections: Vec<KOptReconnection>,
    entity_order_idx: usize,
    cuts: Vec<usize>,
    reconnection_idx: usize,
    done: bool,
    _phantom: PhantomData<V>,
}

impl<S, V> KOptMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_order: EntityOrder,
        list_lens: Vec<usize>,
        k: usize,
        min_segment_len: usize,
        reconnections: Vec<KOptReconnection>,
    ) -> Self {
        let cuts: Vec<usize> = (0..k).map(|i| (i + 1) * min_segment_len).collect();
        let done = !(2..=5).contains(&k) || reconnections.is_empty();

        Self {
            fp,
            entity_order,
            list_lens,
            k,
            min_segment_len,
            reconnections,
            entity_order_idx: 0,
            cuts,
            reconnection_idx: 0,
            done,
            _phantom: PhantomData,
        }
    }

    fn entity(&self) -> usize {
        self.entity_order.get(self.entity_order_idx)
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
            if self.entity_order_idx >= self.entity_order.len() {
                return None;
            }

            let entity_idx = self.entity();
            let list_len = self.list_lens.get(entity_idx).copied().unwrap_or(0);
            let min_required = self.k * self.min_segment_len;

            if list_len < min_required {
                self.entity_order_idx += 1;
                self.cuts = (0..self.k)
                    .map(|i| (i + 1) * self.min_segment_len)
                    .collect();
                self.reconnection_idx = 0;
                continue;
            }

            // Check if current cuts are valid
            if self.cuts.last().copied().unwrap_or(0) > list_len {
                if !self.advance_cuts(list_len) {
                    self.entity_order_idx += 1;
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
                .map(|&pos| CutPoint::new(entity_idx, pos))
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
                self.entity_order_idx += 1;
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
    entity_order: EntityOrder,
    list_lens: Vec<usize>,
    ruin_count: usize,
    entity_order_idx: usize,
    positions: Vec<usize>,
    done_for_entity: bool,
    _phantom: PhantomData<V>,
}

impl<S, V> ListRuinMoveIterator<S, V> {
    fn new(
        fp: ListVariableFnPtrs<S, V>,
        entity_order: EntityOrder,
        list_lens: Vec<usize>,
        ruin_count: usize,
    ) -> Self {
        let positions = (0..ruin_count).collect();

        Self {
            fp,
            entity_order,
            list_lens,
            ruin_count,
            entity_order_idx: 0,
            positions,
            done_for_entity: ruin_count == 0,
            _phantom: PhantomData,
        }
    }

    fn entity(&self) -> usize {
        self.entity_order.get(self.entity_order_idx)
    }

    fn advance_combination(&mut self) {
        let list_len = self.list_lens.get(self.entity()).copied().unwrap_or(0);

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
            if self.entity_order_idx >= self.entity_order.len() {
                return None;
            }

            let entity_idx = self.entity();
            let list_len = self.list_lens.get(entity_idx).copied().unwrap_or(0);
            if list_len < self.ruin_count || self.done_for_entity {
                self.entity_order_idx += 1;
                self.positions = (0..self.ruin_count).collect();
                self.done_for_entity = self.ruin_count == 0;
                continue;
            }

            let m = ListRuinMove::new(
                entity_idx,
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
