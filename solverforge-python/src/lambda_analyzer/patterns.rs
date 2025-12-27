//! Pattern extraction types and helpers for lambda analysis.
//!
//! This module contains types and simple pattern-checking functions used
//! during method body analysis.

use pyo3::prelude::*;

/// Information about a mutable variable tracked across loop iterations.
/// Used to detect "previous element" patterns like:
/// ```python
/// prev = self.init_field
/// for item in collection:
///     use(prev)
///     prev = item.field
/// ```
#[derive(Debug, Clone)]
pub(crate) struct MutableLoopVar {
    /// The variable name (e.g., "previous_location")
    pub name: String,
    /// The initialization expression (e.g., self.home_location)
    pub init_expr: Bound<'static, PyAny>,
    /// The field being tracked from each item (e.g., "location" from visit.location)
    pub item_field: String,
}

/// Context from loop extraction, needed for post-loop term processing.
pub(crate) struct LoopContext {
    /// The Sum expression for the loop
    pub sum_expr: solverforge_core::wasm::Expression,
    /// The collection being iterated
    pub collection_expr: solverforge_core::wasm::Expression,
    /// The item class name
    pub item_class_name: String,
    /// Mutable loop variables (for post-loop term substitution)
    pub mutable_vars: Vec<MutableLoopVar>,
}
