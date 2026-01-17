//! Selectors for entities, values, and moves.
//!
//! Use `MoveSelectorImpl` for config-driven move selection.

pub(crate) mod decorator;
pub(crate) mod entity;
pub(crate) mod k_opt;
pub(crate) mod list_change;
pub(crate) mod list_ruin;
pub(crate) mod list_swap;
pub(crate) mod mimic;
pub(crate) mod sublist_change;
pub(crate) mod sublist_swap;
pub(crate) mod move_selector_impl;
pub(crate) mod nearby;
pub(crate) mod pillar;
pub(crate) mod ruin;
pub(crate) mod selection_order;
pub(crate) mod typed_move_selector;
pub(crate) mod typed_value;

pub use move_selector_impl::MoveSelectorImpl;
pub use typed_move_selector::MoveSelector;
