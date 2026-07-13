//! Shared typed/dynamic scalar access carrier for runtime graph compilation.
//!
//! This is deliberately below selector/provider policy.  A graph node carries
//! one `RuntimeScalarSlot`; scalar leaves and generic provider moves use
//! the same identity, legality, and metadata dispatch rather than rebuilding
//! typed and dynamic selector trees.

use std::fmt;
use std::sync::Arc;

use solverforge_core::domain::{DynamicScalarVariableSlot, EntityClassId, VariableId};

use super::{ScalarVariableSlot, ValueSource};

/// Immutable scalar slot identity used by compiler errors, provider
/// normalization, candidate ownership, and trace provenance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeScalarSlotId {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub entity_class: Arc<str>,
    pub variable_name: Arc<str>,
    pub dynamic_identity: Option<(EntityClassId, VariableId)>,
}

impl RuntimeScalarSlotId {
    pub fn from_static_slot<S>(slot: &ScalarVariableSlot<S>) -> Self {
        Self {
            descriptor_index: slot.descriptor_index,
            variable_index: slot.variable_index,
            entity_class: Arc::from(slot.entity_type_name),
            variable_name: Arc::from(slot.variable_name),
            dynamic_identity: None,
        }
    }

    pub fn from_dynamic_slot<S>(slot: &DynamicScalarVariableSlot<S>) -> Self {
        Self {
            descriptor_index: slot.descriptor_index(),
            variable_index: slot.descriptor_variable_index(),
            entity_class: Arc::from(slot.entity_type_name),
            variable_name: Arc::from(slot.variable_name),
            dynamic_identity: Some((slot.entity, slot.variable)),
        }
    }
}

impl fmt::Display for RuntimeScalarSlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{} (descriptor {}, variable {})",
            self.entity_class, self.variable_name, self.descriptor_index, self.variable_index
        )
    }
}

/// Structural scalar capability.  It answers whether an immutable binding
/// supplies a source, not whether a particular row produces a candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarAccessCapability {
    Candidates,
    NearbyValue,
    NearbyEntity,
    ConstructionEntityOrder,
    ConstructionValueOrder,
}

/// Physical payload carrier.  This is the only typed/dynamic distinction
/// retained by the shared scalar/provider kernels.
pub enum RuntimeScalarSlot<S> {
    Static(ScalarVariableSlot<S>),
    Dynamic(DynamicScalarVariableSlot<S>),
}

impl<S> Clone for RuntimeScalarSlot<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(slot) => Self::Static(*slot),
            Self::Dynamic(slot) => Self::Dynamic(slot.clone()),
        }
    }
}

impl<S> fmt::Debug for RuntimeScalarSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeScalarSlot")
            .field("id", &self.id())
            .field("dynamic", &self.is_dynamic())
            .finish()
    }
}

impl<S> RuntimeScalarSlot<S> {
    pub fn id(&self) -> RuntimeScalarSlotId {
        match self {
            Self::Static(slot) => RuntimeScalarSlotId::from_static_slot(slot),
            Self::Dynamic(slot) => RuntimeScalarSlotId::from_dynamic_slot(slot),
        }
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        match self {
            Self::Static(slot) => slot.matches_target(entity_class, variable_name),
            Self::Dynamic(slot) => slot.matches_target(entity_class, variable_name),
        }
    }

    pub fn has_capability(&self, capability: ScalarAccessCapability) -> bool {
        match (self, capability) {
            // Every scalar binding exposes an ordinary stream, even when a
            // row's stream is empty.
            (_, ScalarAccessCapability::Candidates) => true,
            (Self::Static(slot), ScalarAccessCapability::NearbyValue) => {
                slot.nearby_value_candidates.is_some()
            }
            (Self::Static(slot), ScalarAccessCapability::NearbyEntity) => {
                slot.nearby_entity_candidates.is_some()
            }
            (Self::Static(slot), ScalarAccessCapability::ConstructionEntityOrder) => {
                slot.construction_entity_order_key.is_some()
            }
            (Self::Static(slot), ScalarAccessCapability::ConstructionValueOrder) => {
                slot.construction_value_order_key.is_some()
            }
            (Self::Dynamic(slot), ScalarAccessCapability::NearbyValue) => {
                slot.has_nearby_value_candidates()
            }
            (Self::Dynamic(slot), ScalarAccessCapability::NearbyEntity) => {
                slot.has_nearby_entity_candidates()
            }
            // Dynamic scalar decreasing/queue construction requires immutable
            // order metadata. It is intentionally not recreated by a wrapper
            // callback or ad hoc model scan.
            (Self::Dynamic(_), ScalarAccessCapability::ConstructionEntityOrder)
            | (Self::Dynamic(_), ScalarAccessCapability::ConstructionValueOrder) => false,
        }
    }

    /// Descriptor coordinate without materializing an owned identity. Runtime
    /// moves use this in their hot apply/undo and tabu paths.
    pub(crate) fn descriptor_index(&self) -> usize {
        match self {
            Self::Static(slot) => slot.descriptor_index,
            Self::Dynamic(slot) => slot.descriptor_index(),
        }
    }

    /// Descriptor-local scalar coordinate without materializing an owned
    /// identity.
    pub(crate) fn variable_index(&self) -> usize {
        match self {
            Self::Static(slot) => slot.variable_index,
            Self::Dynamic(slot) => slot.descriptor_variable_index(),
        }
    }

    /// Stable slot name held by the frozen model. Callback reasons remain
    /// owned separately; this accessor is only the planning-variable identity
    /// needed by existing core telemetry/tabu primitives.
    pub(crate) fn variable_name(&self) -> &'static str {
        match self {
            Self::Static(slot) => slot.variable_name,
            Self::Dynamic(slot) => slot.variable_name,
        }
    }

    pub(crate) fn entity_count(&self, solution: &S) -> usize {
        match self {
            Self::Static(slot) => (slot.entity_count)(solution),
            Self::Dynamic(slot) => slot.entity_count(solution),
        }
    }

    /// Whether a construction placement may retain an unassigned value. This
    /// is immutable slot metadata, shared by the descriptor-placement and
    /// global runtime-slot construction schedules.
    pub(crate) fn allows_unassigned(&self) -> bool {
        match self {
            Self::Static(slot) => slot.allows_unassigned,
            Self::Dynamic(slot) => slot.allows_unassigned,
        }
    }

    pub(crate) fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        match self {
            Self::Static(slot) => slot.current_value(solution, entity_index),
            Self::Dynamic(slot) => slot.current_value(solution, entity_index),
        }
    }

    pub(crate) fn set_value(&self, solution: &mut S, entity_index: usize, value: Option<usize>) {
        match self {
            Self::Static(slot) => slot.set_value(solution, entity_index, value),
            Self::Dynamic(slot) => slot.set_value(solution, entity_index, value),
        }
    }

    pub(crate) fn value_is_legal(
        &self,
        solution: &S,
        entity_index: usize,
        value: Option<usize>,
    ) -> bool {
        match self {
            Self::Static(slot) => slot.value_is_legal(solution, entity_index, value),
            Self::Dynamic(slot) => slot.value_is_legal(solution, entity_index, value),
        }
    }

    /// Applies scalar-swap legality through the shared carrier. Native empty
    /// value sources permit exchanging assigned values;
    /// every other source uses its declared per-row legality predicate.
    pub(crate) fn swap_destination_is_legal(
        &self,
        solution: &S,
        entity_index: usize,
        value: Option<usize>,
    ) -> bool {
        match self {
            Self::Static(slot) if matches!(slot.value_source, ValueSource::Empty) => {
                value.is_some()
            }
            Self::Static(slot) => slot.value_is_legal(solution, entity_index, value),
            Self::Dynamic(slot) => slot.value_is_legal(solution, entity_index, value),
        }
    }

    /// Visits ordinary candidates in their declared source order.  `limit`
    /// limits source consumption, never a post-hoc result set, so lazy dynamic
    /// bindings and static value ranges share one ordering contract.
    pub(crate) fn visit_candidate_values(
        &self,
        solution: &S,
        entity_index: usize,
        limit: Option<usize>,
        visit: &mut dyn FnMut(usize),
    ) {
        let limit = limit.unwrap_or(usize::MAX);
        match self {
            Self::Static(slot) => {
                for value in slot.candidate_values_for_entity(solution, entity_index, Some(limit)) {
                    visit(value);
                }
            }
            Self::Dynamic(slot) => {
                for &value in slot
                    .candidate_values(solution, entity_index)
                    .iter()
                    .take(limit)
                {
                    visit(value);
                }
            }
        }
    }

    /// Visits the declared nearby-value source, returning false only when the
    /// binding has no such source and a caller may select its explicit normal
    /// candidate behavior.  It never synthesizes a wrapper-local fallback.
    pub(crate) fn visit_nearby_value_candidates(
        &self,
        solution: &S,
        entity_index: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        match self {
            Self::Static(slot) => {
                let Some(source) = slot.nearby_value_candidates else {
                    return false;
                };
                for &value in source(solution, entity_index, slot.variable_index)
                    .iter()
                    .take(limit)
                {
                    visit(value);
                }
                true
            }
            Self::Dynamic(slot) => {
                slot.visit_nearby_value_candidates(solution, entity_index, limit, visit)
            }
        }
    }

    pub(crate) fn nearby_value_distance(
        &self,
        solution: &S,
        entity_index: usize,
        candidate: usize,
    ) -> Option<f64> {
        match self {
            Self::Static(slot) => slot.nearby_value_distance(solution, entity_index, candidate),
            Self::Dynamic(slot) => slot.nearby_value_distance(solution, entity_index, candidate),
        }
    }

    pub(crate) fn visit_nearby_entity_candidates(
        &self,
        solution: &S,
        entity_index: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        match self {
            Self::Static(slot) => {
                let Some(source) = slot.nearby_entity_candidates else {
                    return false;
                };
                for &candidate in source(solution, entity_index, slot.variable_index)
                    .iter()
                    .take(limit)
                {
                    visit(candidate);
                }
                true
            }
            Self::Dynamic(slot) => {
                slot.visit_nearby_entity_candidates(solution, entity_index, limit, visit)
            }
        }
    }

    pub(crate) fn nearby_entity_distance(
        &self,
        solution: &S,
        left_entity_index: usize,
        right_entity_index: usize,
    ) -> Option<f64> {
        match self {
            Self::Static(slot) => {
                slot.nearby_entity_distance(solution, left_entity_index, right_entity_index)
            }
            Self::Dynamic(slot) => {
                slot.nearby_entity_distance(solution, left_entity_index, right_entity_index)
            }
        }
    }

    pub(crate) fn construction_entity_order_key(
        &self,
        solution: &S,
        entity_index: usize,
    ) -> Option<i64> {
        match self {
            Self::Static(slot) => slot.construction_entity_order_key(solution, entity_index),
            Self::Dynamic(_) => None,
        }
    }

    pub(crate) fn construction_value_order_key(
        &self,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Option<i64> {
        match self {
            Self::Static(slot) => slot.construction_value_order_key(solution, entity_index, value),
            Self::Dynamic(_) => None,
        }
    }

    pub(crate) fn edit(
        &self,
        entity_index: usize,
        to_value: Option<usize>,
    ) -> RuntimeScalarEdit<S> {
        RuntimeScalarEdit {
            slot: self.clone(),
            entity_index,
            to_value,
        }
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }
}

/// A neutral scalar edit used by the shared provider/scalar kernels before it
/// is lowered to the outer move payload.  The kind/reason policy lives above
/// this type; apply/undo ownership never needs a second typed/dynamic edit
/// implementation.
pub struct RuntimeScalarEdit<S> {
    pub slot: RuntimeScalarSlot<S>,
    pub entity_index: usize,
    pub to_value: Option<usize>,
}

impl<S> Clone for RuntimeScalarEdit<S> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            entity_index: self.entity_index,
            to_value: self.to_value,
        }
    }
}

impl<S> fmt::Debug for RuntimeScalarEdit<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeScalarEdit")
            .field("slot", &self.slot)
            .field("entity_index", &self.entity_index)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S> RuntimeScalarEdit<S> {
    pub fn id(&self) -> RuntimeScalarSlotId {
        self.slot.id()
    }

    pub(crate) fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    pub(crate) fn variable_index(&self) -> usize {
        self.slot.variable_index()
    }

    pub(crate) fn variable_name(&self) -> &'static str {
        self.slot.variable_name()
    }
}
