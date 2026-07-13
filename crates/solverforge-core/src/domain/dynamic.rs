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

/// Rust-owned dynamic planning model backend.
///
/// Binding crates implement this trait on their concrete dynamic solution
/// state. The trait is expressed in logical entity and variable IDs rather
/// than Rust `TypeId`s.
pub trait DynamicModelBackend: Clone + Send + Sync + 'static {
    type Score: Score;

    fn entity_count(&self, entity: EntityClassId) -> usize;

    fn get_scalar(&self, entity: EntityClassId, row: usize, variable: VariableId) -> Option<usize>;

    fn set_scalar(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: Option<usize>,
    );

    fn list_len(&self, entity: EntityClassId, row: usize, variable: VariableId) -> usize;

    fn list_get(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    fn list_insert(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
        value: usize,
    );

    fn list_remove(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    fn candidate_values(&self, entity: EntityClassId, row: usize, variable: VariableId)
        -> &[usize];

    fn scalar_value_is_legal(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: usize,
    ) -> bool;

    fn list_element_count(&self, _entity: EntityClassId, _variable: VariableId) -> usize {
        0
    }

    fn list_element(
        &self,
        _entity: EntityClassId,
        _variable: VariableId,
        element_index: usize,
    ) -> Option<usize> {
        Some(element_index)
    }

    fn list_assigned_elements(&self, _entity: EntityClassId, _variable: VariableId) -> Vec<usize> {
        Vec::new()
    }
}

/// Object-safe dynamic scalar variable access.
pub trait DynamicScalarAccess<S>: Send + Sync {
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
    fn entity_count(&self, solution: &S) -> usize;
    fn get(&self, solution: &S, row: usize) -> Option<usize>;
    fn set(&self, solution: &mut S, row: usize, value: Option<usize>);
    fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize];
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
    pub set: bool,
    pub replace: bool,
    pub reverse: bool,
    pub sublist: bool,
}

pub trait DynamicListAccess<S>: Send + Sync {
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
    fn entity_count(&self, solution: &S) -> usize;
    fn element_count(&self, solution: &S) -> usize;
    fn element(&self, solution: &S, element_index: usize) -> Option<usize>;
    fn assigned_elements(&self, solution: &S) -> Vec<usize>;
    fn len(&self, solution: &S, row: usize) -> usize;
    fn get(&self, solution: &S, row: usize, pos: usize) -> Option<usize>;
    fn insert(&self, solution: &mut S, row: usize, pos: usize, value: usize);
    fn remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize>;

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities::default()
    }

    fn set(&self, _solution: &mut S, _row: usize, _pos: usize, _value: usize) -> bool {
        false
    }

    /// Replaces one complete row list as one logical mutation.
    fn replace(&self, _solution: &mut S, _row: usize, _values: Vec<usize>) -> bool {
        false
    }

    fn reverse(&self, _solution: &mut S, _row: usize, _start: usize, _end: usize) -> bool {
        false
    }

    fn sublist_remove(
        &self,
        _solution: &mut S,
        _row: usize,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<usize>> {
        None
    }

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
    pub element_owner: bool,
    pub construction_order_key: bool,
    /// Duration metadata. Full precedence-aware construction requires this
    /// together with `precedence_successors`; successor-only metadata remains
    /// a valid ListRuin contract.
    pub precedence_duration: bool,
    /// Ordered successor metadata. This is intentionally independent from
    /// duration so dynamic bindings preserve the same successor-only policy
    /// as native list slots.
    pub precedence_successors: bool,
    pub cross_position_distance: bool,
    pub intra_position_distance: bool,
    pub route: bool,
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
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
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
    fn route_distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> Option<i64>;
    fn route_feasible(&self, solution: &S, entity: usize, route: &[usize]) -> Option<bool>;

    /// Savings-specific bundle used only by Clarke-Wright construction.
    fn savings_depot(&self, solution: &S, entity: usize) -> Option<usize>;
    fn savings_metric_class(&self, solution: &S, entity: usize) -> Option<usize>;
    fn savings_distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> Option<i64>;
    fn savings_feasible(&self, solution: &S, entity: usize, route: &[usize]) -> Option<bool>;
}

/// Public dynamic scalar variable slot.
pub struct DynamicScalarVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub allows_unassigned: bool,
    descriptor_index: Option<usize>,
    descriptor_variable_index: Option<usize>,
    access: Arc<dyn DynamicScalarAccess<S>>,
}

impl<S> Clone for DynamicScalarVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            allows_unassigned: self.allows_unassigned,
            descriptor_index: self.descriptor_index,
            descriptor_variable_index: self.descriptor_variable_index,
            access: Arc::clone(&self.access),
        }
    }
}

impl<S> fmt::Debug for DynamicScalarVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicScalarVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .field("descriptor_index", &self.descriptor_index)
            .field("descriptor_variable_index", &self.descriptor_variable_index)
            .finish()
    }
}

impl<S> PartialEq for DynamicScalarVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.allows_unassigned == other.allows_unassigned
            && self.descriptor_index == other.descriptor_index
            && self.descriptor_variable_index == other.descriptor_variable_index
    }
}

impl<S> Eq for DynamicScalarVariableSlot<S> {}

impl<S> DynamicScalarVariableSlot<S> {
    pub fn with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
        access: Arc<dyn DynamicScalarAccess<S>>,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            descriptor_index: None,
            descriptor_variable_index: None,
            access,
        }
    }

    pub fn resolve_descriptor_index(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let indexes = resolve_dynamic_descriptor_indexes(
            descriptor,
            self.entity,
            self.variable,
            self.entity_type_name,
            self.variable_name,
            DynamicVariableKind::Scalar,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != indexes.descriptor_index {
                return Err(format!(
                    "dynamic scalar variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {}",
                    self.entity_type_name,
                    self.variable_name,
                    self.entity.0,
                    indexes.descriptor_index
                ));
            }
        }
        self.descriptor_index = Some(indexes.descriptor_index);
        self.descriptor_variable_index = Some(indexes.variable_index);
        Ok(())
    }

    pub fn resolved_against(mut self, descriptor: &SolutionDescriptor) -> Result<Self, String> {
        self.resolve_descriptor_index(descriptor)?;
        Ok(self)
    }

    pub fn is_descriptor_resolved(&self) -> bool {
        self.descriptor_index.is_some() && self.descriptor_variable_index.is_some()
    }

    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index.unwrap_or_else(|| {
            panic!(
                "dynamic scalar variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn descriptor_variable_index(&self) -> usize {
        self.descriptor_variable_index.unwrap_or_else(|| {
            panic!(
                "dynamic scalar variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|entity| entity == self.entity_type_name)
            && variable_name.is_none_or(|variable| variable == self.variable_name)
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        self.access.entity_count(solution)
    }

    pub fn current_value(&self, solution: &S, row: usize) -> Option<usize> {
        self.access.get(solution, row)
    }

    pub fn set_value(&self, solution: &mut S, row: usize, value: Option<usize>) {
        self.access.set(solution, row, value);
    }

    pub fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize] {
        self.access.candidate_values(solution, row)
    }

    pub fn has_nearby_value_candidates(&self) -> bool {
        self.access.has_nearby_value_candidates()
    }

    pub fn visit_nearby_value_candidates(
        &self,
        solution: &S,
        row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.access
            .visit_nearby_value_candidates(solution, row, limit, visit)
    }

    pub fn nearby_value_distance(&self, solution: &S, row: usize, candidate: usize) -> Option<f64> {
        self.access.nearby_value_distance(solution, row, candidate)
    }

    pub fn has_nearby_entity_candidates(&self) -> bool {
        self.access.has_nearby_entity_candidates()
    }

    pub fn visit_nearby_entity_candidates(
        &self,
        solution: &S,
        left_row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.access
            .visit_nearby_entity_candidates(solution, left_row, limit, visit)
    }

    pub fn nearby_entity_distance(
        &self,
        solution: &S,
        left_row: usize,
        right_row: usize,
    ) -> Option<f64> {
        self.access
            .nearby_entity_distance(solution, left_row, right_row)
    }

    pub fn value_is_legal(&self, solution: &S, row: usize, value: Option<usize>) -> bool {
        let Some(value) = value else {
            return self.allows_unassigned;
        };
        self.access.value_is_legal(solution, row, value)
    }
}

impl<S> DynamicScalarVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
    ) -> Self {
        Self::with_access(
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            Arc::new(BackendScalarAccess { entity, variable }),
        )
    }
}

/// Public dynamic list variable slot.
pub struct DynamicListVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    descriptor_index: Option<usize>,
    descriptor_variable_index: Option<usize>,
    access: Arc<dyn DynamicListAccess<S>>,
    metadata: Option<Arc<dyn DynamicListMetadata<S>>>,
}

impl<S> Clone for DynamicListVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            descriptor_variable_index: self.descriptor_variable_index,
            access: Arc::clone(&self.access),
            metadata: self.metadata.as_ref().map(Arc::clone),
        }
    }
}

impl<S> fmt::Debug for DynamicListVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicListVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .field("descriptor_variable_index", &self.descriptor_variable_index)
            .field(
                "metadata_capabilities",
                &self
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.capabilities()),
            )
            .finish()
    }
}

impl<S> PartialEq for DynamicListVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.descriptor_index == other.descriptor_index
            && self.descriptor_variable_index == other.descriptor_variable_index
    }
}

impl<S> Eq for DynamicListVariableSlot<S> {}

impl<S> DynamicListVariableSlot<S> {
    /// Fallible access constructor for integrations that compile a slot from
    /// runtime schema. This verifies that the object supplying mutations is
    /// bound to exactly the same logical entity and variable as the slot.
    ///
    /// Integrations requiring immutable metadata can use
    /// [`Self::with_access_and_metadata`] to validate both bindings together.
    pub fn try_with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        access: Arc<dyn DynamicListAccess<S>>,
    ) -> Result<Self, String> {
        if access.entity_class() != entity || access.variable() != variable {
            return Err(format!(
                "dynamic list access is bound to entity ID {} variable ID {}, but slot {}.{} is entity ID {} variable ID {}",
                access.entity_class().0,
                access.variable().0,
                entity_type_name,
                variable_name,
                entity.0,
                variable.0,
            ));
        }
        Ok(Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            descriptor_index: None,
            descriptor_variable_index: None,
            access,
            metadata: None,
        })
    }

    /// Constructs a slot with its immutable, structurally bound metadata.
    ///
    /// Metadata identity is checked immediately; a binding implementation
    /// cannot accidentally be reused for a same-named variable in another
    /// entity class. Phase assembly remains responsible for rejecting a
    /// selected family whose specific capability bit is absent.
    pub fn with_access_and_metadata(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        access: Arc<dyn DynamicListAccess<S>>,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        Self::try_with_access(entity, variable, entity_type_name, variable_name, access)?
            .with_metadata(metadata)
    }

    /// Attaches immutable metadata after access construction while preserving
    /// the same structural identity validation as
    /// [`Self::with_access_and_metadata`].
    pub fn with_metadata(
        mut self,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        if metadata.entity_class() != self.entity || metadata.variable() != self.variable {
            return Err(format!(
                "dynamic list metadata is bound to entity ID {} variable ID {}, but slot {}.{} is entity ID {} variable ID {}",
                metadata.entity_class().0,
                metadata.variable().0,
                self.entity_type_name,
                self.variable_name,
                self.entity.0,
                self.variable.0,
            ));
        }
        self.metadata = Some(metadata);
        Ok(self)
    }

    pub fn resolve_descriptor_index(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let indexes = resolve_dynamic_descriptor_indexes(
            descriptor,
            self.entity,
            self.variable,
            self.entity_type_name,
            self.variable_name,
            DynamicVariableKind::List,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != indexes.descriptor_index {
                return Err(format!(
                    "dynamic list variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {}",
                    self.entity_type_name,
                    self.variable_name,
                    self.entity.0,
                    indexes.descriptor_index
                ));
            }
        }
        self.descriptor_index = Some(indexes.descriptor_index);
        self.descriptor_variable_index = Some(indexes.variable_index);
        Ok(())
    }

    pub fn resolved_against(mut self, descriptor: &SolutionDescriptor) -> Result<Self, String> {
        self.resolve_descriptor_index(descriptor)?;
        Ok(self)
    }

    pub fn is_descriptor_resolved(&self) -> bool {
        self.descriptor_index.is_some() && self.descriptor_variable_index.is_some()
    }

    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index.unwrap_or_else(|| {
            panic!(
                "dynamic list variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn descriptor_variable_index(&self) -> usize {
        self.descriptor_variable_index.unwrap_or_else(|| {
            panic!(
                "dynamic list variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|entity| entity == self.entity_type_name)
            && variable_name.is_none_or(|variable| variable == self.variable_name)
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        self.access.entity_count(solution)
    }

    pub fn element_count(&self, solution: &S) -> usize {
        self.access.element_count(solution)
    }

    pub fn element(&self, solution: &S, element_index: usize) -> Option<usize> {
        self.access.element(solution, element_index)
    }

    pub fn assigned_elements(&self, solution: &S) -> Vec<usize> {
        self.access.assigned_elements(solution)
    }

    pub fn list_len(&self, solution: &S, row: usize) -> usize {
        self.access.len(solution, row)
    }

    pub fn list_get(&self, solution: &S, row: usize, pos: usize) -> Option<usize> {
        self.access.get(solution, row, pos)
    }

    pub fn list_insert(&self, solution: &mut S, row: usize, pos: usize, value: usize) {
        self.access.insert(solution, row, pos, value);
    }

    pub fn list_remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize> {
        self.access.remove(solution, row, pos)
    }

    pub fn access_capabilities(&self) -> DynamicListAccessCapabilities {
        self.access.capabilities()
    }

    pub fn list_set(&self, solution: &mut S, row: usize, pos: usize, value: usize) -> bool {
        self.access.set(solution, row, pos, value)
    }

    /// Replaces a complete list row as one logical mutation. Dynamic kernels
    /// use this for route replacement rather than composing remove/insert and
    /// accidentally publishing multiple dirty/revision transitions.
    pub fn list_replace(&self, solution: &mut S, row: usize, values: Vec<usize>) -> bool {
        self.access.replace(solution, row, values)
    }

    pub fn list_reverse(&self, solution: &mut S, row: usize, start: usize, end: usize) -> bool {
        self.access.reverse(solution, row, start, end)
    }

    pub fn sublist_remove(
        &self,
        solution: &mut S,
        row: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        self.access.sublist_remove(solution, row, start, end)
    }

    pub fn sublist_insert(
        &self,
        solution: &mut S,
        row: usize,
        pos: usize,
        values: Vec<usize>,
    ) -> bool {
        self.access.sublist_insert(solution, row, pos, values)
    }

    /// Returns slot-bound metadata when the integration compiled it.
    ///
    /// Absence is intentional and must cause a selected metadata-dependent
    /// phase to fail validation; it is never a request to synthesize defaults.
    pub fn metadata(&self) -> Option<&dyn DynamicListMetadata<S>> {
        self.metadata.as_deref()
    }

    pub fn metadata_capabilities(&self) -> Option<DynamicListMetadataCapabilities> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.capabilities())
    }

    pub fn require_metadata(&self) -> Result<&dyn DynamicListMetadata<S>, String> {
        self.metadata().ok_or_else(|| {
            format!(
                "dynamic list variable {}.{} has no immutable list metadata; selected list phases must declare and bind their required capability bundle before solve",
                self.entity_type_name, self.variable_name,
            )
        })
    }
}

impl<S> DynamicListVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            descriptor_index: None,
            descriptor_variable_index: None,
            access: Arc::new(BackendListAccess { entity, variable }),
            metadata: None,
        }
    }

    /// Backend-access convenience constructor with immutable metadata.
    pub fn new_with_metadata(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        Self::with_access_and_metadata(
            entity,
            variable,
            entity_type_name,
            variable_name,
            Arc::new(BackendListAccess { entity, variable }),
            metadata,
        )
    }
}
