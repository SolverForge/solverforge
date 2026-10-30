//! Dynamic model contracts for host-language integrations.

use std::fmt;
use std::sync::Arc;

use crate::domain::{EntityClassId, SolutionDescriptor, VariableId};
use crate::score::Score;

mod backend;
mod resolution;

use backend::{BackendListAccess, BackendScalarAccess};
use resolution::{resolve_dynamic_descriptor_indexes, DynamicVariableKind};

mod assignment;

pub use assignment::{
    DynamicScalarAssignmentMetadata, DynamicScalarAssignmentMetadataCapabilities,
};

mod list_slot;
pub use list_slot::DynamicListVariableSlot;

mod scalar_slot;
pub use scalar_slot::DynamicScalarVariableSlot;

/// Rust-owned dynamic planning model backend.
///
/// Binding crates implement this trait on their concrete dynamic solution
/// state. The trait is expressed in logical entity and variable IDs rather
/// than Rust `TypeId`s.
pub trait DynamicModelBackend: Clone + Send + Sync + 'static {
    /// Score type produced for this model.
    type Score: Score;

    /// Number of rows (entities) of class `entity`.
    fn entity_count(&self, entity: EntityClassId) -> usize;

    /// Value of a scalar variable on one row; `None` means unassigned.
    fn get_scalar(&self, entity: EntityClassId, row: usize, variable: VariableId) -> Option<usize>;

    /// Assigns (`Some`) or unassigns (`None`) a scalar variable on one row.
    fn set_scalar(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: Option<usize>,
    );

    /// Length of a list variable on one row.
    fn list_len(&self, entity: EntityClassId, row: usize, variable: VariableId) -> usize;

    /// Element at `pos` in a row's list; `None` when `pos` is out of range.
    fn list_get(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    /// Inserts `value` at `pos` in a row's list.
    fn list_insert(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
        value: usize,
    );

    /// Removes and returns the element at `pos` in a row's list; `None` when `pos` is out of range.
    fn list_remove(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    /// Candidate values for a scalar variable on one row.
    fn candidate_values(&self, entity: EntityClassId, row: usize, variable: VariableId)
        -> &[usize];

    /// Returns `true` when `value` is a legal assignment for a scalar variable on one row.
    fn scalar_value_is_legal(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: usize,
    ) -> bool;

    /// Number of declared elements a list variable distributes; defaults to `0`.
    fn list_element_count(&self, _entity: EntityClassId, _variable: VariableId) -> usize {
        0
    }

    /// Element value at declared source index `element_index`; `None` when the
    /// element is missing. Defaults to the index itself.
    fn list_element(
        &self,
        _entity: EntityClassId,
        _variable: VariableId,
        element_index: usize,
    ) -> Option<usize> {
        Some(element_index)
    }

    /// Elements currently placed in any row's list; defaults to none.
    fn list_assigned_elements(&self, _entity: EntityClassId, _variable: VariableId) -> Vec<usize> {
        Vec::new()
    }
}

/// Object-safe dynamic scalar variable access.
pub trait DynamicScalarAccess<S>: Send + Sync {
    /// Entity class this access is bound to.
    fn entity_class(&self) -> EntityClassId;
    /// Variable this access is bound to.
    fn variable(&self) -> VariableId;
    /// Number of rows of the bound entity class.
    fn entity_count(&self, solution: &S) -> usize;
    /// Value on one row; `None` means unassigned.
    fn get(&self, solution: &S, row: usize) -> Option<usize>;
    /// Assigns (`Some`) or unassigns (`None`) the value on one row.
    fn set(&self, solution: &mut S, row: usize, value: Option<usize>);
    /// Candidate values for one row.
    fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize];
    /// Returns `true` when `value` is a legal assignment for one row.
    fn value_is_legal(&self, solution: &S, row: usize, value: usize) -> bool;

    /// Whether this slot structurally supplies an ordered nearby-value source.
    ///
    /// This reports schema capability rather than a row result. The runtime
    /// still calls [`Self::visit_nearby_value_candidates`] for each row, since
    /// a row-local source or callback may intentionally be absent for one row.
    fn has_nearby_value_candidates(&self) -> bool {
        false
    }

    /// Visits the ordered nearby-value source for one row.
    ///
    /// `limit` is a source-consumption limit, not a result limit. Implementors
    /// that bridge lazy host-language iterables must not consume values after
    /// that limit. Return `true` when this row supplied a source, including an
    /// empty source. Return `false` when no row source is available so the
    /// solver can use the ordinary candidate-value fallback.
    fn visit_nearby_value_candidates(
        &self,
        _solution: &S,
        _row: usize,
        _limit: usize,
        _visit: &mut dyn FnMut(usize),
    ) -> bool {
        false
    }

    /// Returns the nearby-value distance for a source candidate when one is
    /// available. `None` preserves source order as the distance fallback.
    fn nearby_value_distance(&self, _solution: &S, _row: usize, _candidate: usize) -> Option<f64> {
        None
    }

    /// Whether this slot structurally supplies an ordered nearby-entity source.
    fn has_nearby_entity_candidates(&self) -> bool {
        false
    }

    /// Visits the ordered nearby-entity source for one left-hand row.
    ///
    /// `limit` has the same source-consumption contract as
    /// [`Self::visit_nearby_value_candidates`]. Return `true` when a row
    /// source was supplied, including an empty source; otherwise return
    /// `false` to request the all-entity fallback.
    fn visit_nearby_entity_candidates(
        &self,
        _solution: &S,
        _left_row: usize,
        _limit: usize,
        _visit: &mut dyn FnMut(usize),
    ) -> bool {
        false
    }

    /// Returns the nearby-entity distance for a source candidate when one is
    /// available. `None` preserves source order as the distance fallback.
    fn nearby_entity_distance(
        &self,
        _solution: &S,
        _left_row: usize,
        _right_row: usize,
    ) -> Option<f64> {
        None
    }
}

/// Object-safe dynamic list variable access.
///
/// Basic list mutation capabilities are explicit. `false`/`None` from the
/// optional range operations means the access implementation did not bind that
/// operation; it is not permission for a phase to emulate it piecemeal. A
/// canonical list kernel must validate these bits before creating a reverse,
/// sublist, permute, swap, ruin, or route phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicListAccessCapabilities {
    /// [`DynamicListAccess::set`] is bound.
    pub set: bool,
    /// [`DynamicListAccess::replace`] is bound.
    pub replace: bool,
    /// [`DynamicListAccess::reverse`] is bound.
    pub reverse: bool,
    /// [`DynamicListAccess::sublist_remove`] and [`DynamicListAccess::sublist_insert`] are bound.
    pub sublist: bool,
}

/// Object-safe dynamic list variable access.
///
/// Optional operations return `false`/`None` when they are not bound; check
/// [`capabilities`](Self::capabilities) before relying on them.
pub trait DynamicListAccess<S>: Send + Sync {
    /// Entity class this access is bound to.
    fn entity_class(&self) -> EntityClassId;
    /// Variable this access is bound to.
    fn variable(&self) -> VariableId;
    /// Number of rows of the bound entity class.
    fn entity_count(&self, solution: &S) -> usize;
    /// Number of declared elements the list variable distributes.
    fn element_count(&self, solution: &S) -> usize;
    /// Element value at declared source index `element_index`; `None` when the element is missing.
    fn element(&self, solution: &S, element_index: usize) -> Option<usize>;
    /// Elements currently placed in any row's list.
    fn assigned_elements(&self, solution: &S) -> Vec<usize>;
    /// Length of one row's list.
    fn len(&self, solution: &S, row: usize) -> usize;
    /// Element at `pos` in one row's list; `None` when `pos` is out of range.
    fn get(&self, solution: &S, row: usize, pos: usize) -> Option<usize>;
    /// Inserts `value` at `pos` in one row's list.
    fn insert(&self, solution: &mut S, row: usize, pos: usize, value: usize);
    /// Removes and returns the element at `pos` in one row's list; `None` when
    /// `pos` is out of range.
    fn remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize>;

    /// Optional operations this access binds; defaults to none.
    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities::default()
    }

    /// Overwrites the element at `pos`; returns `false` when the operation is not bound.
    fn set(&self, _solution: &mut S, _row: usize, _pos: usize, _value: usize) -> bool {
        false
    }

    /// Replaces one complete row list as one logical mutation.
    fn replace(&self, _solution: &mut S, _row: usize, _values: Vec<usize>) -> bool {
        false
    }

    /// Reverses positions `start..end` of one row's list; returns `false` when
    /// the operation is not bound.
    fn reverse(&self, _solution: &mut S, _row: usize, _start: usize, _end: usize) -> bool {
        false
    }

    /// Removes and returns positions `start..end` of one row's list; `None`
    /// when the operation is not bound.
    fn sublist_remove(
        &self,
        _solution: &mut S,
        _row: usize,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<usize>> {
        None
    }

    /// Inserts `values` at `pos` in one row's list; returns `false` when the
    /// operation is not bound.
    fn sublist_insert(
        &self,
        _solution: &mut S,
        _row: usize,
        _pos: usize,
        _values: Vec<usize>,
    ) -> bool {
        false
    }
}

/// Immutable capabilities bound to one dynamic planning-list slot.
///
/// A `true` bit means the corresponding method on
/// [`DynamicListMetadata`] is structurally available. It does *not* install a
/// semantic fallback: phase assembly must reject a selected heuristic when a
/// required capability is absent. Optional owner restriction remains distinct
/// from a missing owner capability: when `element_owner` is available, an
/// individual element may still return `None` to mean unrestricted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicListMetadataCapabilities {
    /// [`DynamicListMetadata::element_owner`] is available.
    pub element_owner: bool,
    /// [`DynamicListMetadata::construction_order_key`] is available.
    pub construction_order_key: bool,
    /// Duration metadata. Full precedence-aware construction requires this
    /// together with `precedence_successors`; successor-only metadata remains
    /// a valid ListRuin contract.
    pub precedence_duration: bool,
    /// Ordered successor metadata. This is intentionally independent from
    /// duration so dynamic bindings preserve the same successor-only policy
    /// as native list slots.
    pub precedence_successors: bool,
    /// [`DynamicListMetadata::cross_position_distance`] is available.
    pub cross_position_distance: bool,
    /// [`DynamicListMetadata::intra_position_distance`] is available.
    pub intra_position_distance: bool,
    /// The route bundle (`route_depot`, `route_distance`, `route_feasible`) is available.
    pub route: bool,
    /// The savings bundle (`savings_depot`, `savings_metric_class`,
    /// `savings_distance`, `savings_feasible`) is available.
    pub savings: bool,
}

/// Slot-bound immutable metadata used by canonical dynamic-list phases.
///
/// This deliberately contains no discovery API, current-phase lookup, or
/// global/TLS state. Binding crates compile schema fields and callbacks once
/// into one implementation for a specific `(entity_class, variable)` pair,
/// then attach it to the corresponding [`DynamicListVariableSlot`]. Runtime
/// callers validate [`Self::capabilities`] before invoking a selected family;
/// an implementation must not synthesize depot, metric, distance, feasibility,
/// precedence, or ownership defaults.
pub trait DynamicListMetadata<S>: Send + Sync {
    /// Entity class this metadata is bound to.
    fn entity_class(&self) -> EntityClassId;
    /// Variable this metadata is bound to.
    fn variable(&self) -> VariableId;
    /// Metadata this implementation declares.
    fn capabilities(&self) -> DynamicListMetadataCapabilities;

    /// Fixed owner for one element when owner restriction is declared.
    /// `None` means that element is unrestricted; callers must consult
    /// `capabilities().element_owner` to distinguish an unavailable owner
    /// source from an unrestricted element.
    fn element_owner(&self, solution: &S, element: usize) -> Option<usize>;

    /// Deterministic construction ordering key for one element.
    fn construction_order_key(&self, solution: &S, element: usize) -> Option<i64>;

    /// Duration used by full precedence-aware construction and scheduling.
    /// It may be absent when this slot deliberately supplies successor-only
    /// metadata for ListRuin.
    fn precedence_duration(&self, solution: &S, element: usize) -> Option<usize>;

    /// Extends `successors` with fixed successors in declared order. Returns
    /// `false` only when successor metadata is unavailable for this slot.
    ///
    /// The concrete output buffer preserves the object-safe dynamic metadata
    /// seam without putting a `dyn FnMut` dispatch in a precedence candidate
    /// loop.
    fn extend_precedence_successors(
        &self,
        solution: &S,
        element: usize,
        successors: &mut Vec<usize>,
    ) -> bool;

    /// Explicit cross-route position metric for nearby list neighborhoods.
    fn cross_position_distance(
        &self,
        solution: &S,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Option<f64>;

    /// Explicit within-route position metric for nearby K-opt/neighborhoods.
    fn intra_position_distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Option<f64>;

    /// Strict route-local bundle used by K-opt and other route phases.
    ///
    /// Reading and replacing a route are deliberately *not* metadata
    /// callbacks. They come from the slot's [`DynamicListAccess`] (`get` and
    /// one logical `replace` operation) so every route mutation follows the
    /// same dirty-row/revision path as every other list mutation. A canonical
    /// phase must require both that base access and this complete metadata
    /// bundle: depot, distance, and feasibility.
    fn route_depot(&self, solution: &S, entity: usize) -> Option<usize>;
    /// Distance from element `from` to element `to` on one route; `None` when unavailable.
    fn route_distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> Option<i64>;
    /// Whether a complete candidate route is feasible for one entity; `None` when unavailable.
    fn route_feasible(&self, solution: &S, entity: usize, route: &[usize]) -> Option<bool>;

    /// Savings-specific bundle used only by Clarke-Wright construction.
    fn savings_depot(&self, solution: &S, entity: usize) -> Option<usize>;
    /// Savings metric class of one owner; owners in the same class must share
    /// depot and distance behavior.
    fn savings_metric_class(&self, solution: &S, entity: usize) -> Option<usize>;
    /// Distance from element `from` to element `to` used for savings; `None` when unavailable.
    fn savings_distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> Option<i64>;
    /// Whether a merged candidate route is feasible for savings construction;
    /// `None` when unavailable.
    fn savings_feasible(&self, solution: &S, entity: usize, route: &[usize]) -> Option<bool>;
}
