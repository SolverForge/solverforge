//! Variable descriptor.

use std::any::Any;

use super::VariableId;
use crate::domain::variable::{ShadowVariableKind, ValueRangeType, VariableType};

/// Reads a scalar variable's value index from a type-erased entity; `None` means unassigned.
pub type UsizeGetter = for<'a> fn(&'a dyn Any) -> Option<usize>;
/// Writes a scalar variable's value index on a type-erased entity; `None` unassigns it.
pub type UsizeSetter = fn(&mut dyn Any, Option<usize>);
/// Returns the legal value indexes declared on a type-erased entity.
pub type UsizeEntityValueProvider = for<'a> fn(&'a dyn Any) -> Vec<usize>;
/// Returns bounded candidate indexes for `(solution, entity_index, variable_index)`.
pub type UsizeCandidateValues = for<'a> fn(&'a dyn Any, usize, usize) -> &'a [usize];
/// Distance between an entity and a candidate value: `(solution, entity_index, value) -> distance`.
pub type UsizeNearbyValueDistanceMeter = fn(&dyn Any, usize, usize) -> f64;
/// Distance between two entities: `(solution, left_entity_index, right_entity_index) -> distance`.
pub type UsizeNearbyEntityDistanceMeter = fn(&dyn Any, usize, usize) -> f64;
/// Construction ranking key for an entity: `(solution, entity_index) -> key`.
pub type UsizeConstructionEntityOrderKey = fn(&dyn Any, usize) -> i64;
/// Construction ranking key for a candidate value: `(solution, entity_index, value) -> key`.
pub type UsizeConstructionValueOrderKey = fn(&dyn Any, usize, usize) -> i64;
/// Local-search ranking key for an entity: `(solution, entity_index) -> key`;
/// lower keys are tried first.
pub type UsizeSearchEntityOrderKey = fn(&dyn Any, usize) -> i64;
/// Local-search ranking key for a candidate value: `(solution, entity_index,
/// value) -> key`; lower keys are tried first.
pub type UsizeSearchValueOrderKey = fn(&dyn Any, usize, usize) -> i64;

/// Describes a planning variable at runtime.
#[derive(Debug, Clone)]
pub struct VariableDescriptor {
    /// Name of the variable (field name).
    pub name: &'static str,
    /// Logical variable ID for dynamic binding models.
    pub logical_id: Option<VariableId>,
    /// Type of the variable.
    pub variable_type: VariableType,
    /// Whether the variable can be unassigned (null).
    pub allows_unassigned: bool,
    /// Reference to the value range provider.
    pub value_range_provider: Option<&'static str>,
    /// The type of value range.
    pub value_range_type: ValueRangeType,
    /// For shadow variables: the source variable name.
    pub source_variable: Option<&'static str>,
    /// For shadow variables: the source entity type name.
    pub source_entity: Option<&'static str>,
    /// Dynamic accessors for canonical scalar-variable solving.
    pub usize_getter: Option<UsizeGetter>,
    /// Scalar value setter, paired with `usize_getter`.
    pub usize_setter: Option<UsizeSetter>,
    /// Legal values read from an entity field, when the value range provider is an entity field.
    pub entity_value_provider: Option<UsizeEntityValueProvider>,
    /// Bounded per-entity candidate values used instead of the full value range.
    pub candidate_values: Option<UsizeCandidateValues>,
    /// Bounded nearby-value candidates for local search.
    pub nearby_value_candidates: Option<UsizeCandidateValues>,
    /// Bounded nearby-entity candidates for local search; the slice holds entity indexes.
    pub nearby_entity_candidates: Option<UsizeCandidateValues>,
    /// Ranks nearby-value candidates; it does not bound them.
    pub nearby_value_distance_meter: Option<UsizeNearbyValueDistanceMeter>,
    /// Ranks nearby-entity candidates; it does not bound them.
    pub nearby_entity_distance_meter: Option<UsizeNearbyEntityDistanceMeter>,
    /// Entity ranking for construction heuristics that order entities.
    pub construction_entity_order_key: Option<UsizeConstructionEntityOrderKey>,
    /// Value ranking for construction heuristics that order values
    /// (weakest/strongest fit, queue allocation).
    pub construction_value_order_key: Option<UsizeConstructionValueOrderKey>,
    /// Entity ranking for local-search change and swap moves.
    pub search_entity_order_key: Option<UsizeSearchEntityOrderKey>,
    /// Value ranking for local-search change and swap moves.
    pub search_value_order_key: Option<UsizeSearchValueOrderKey>,
}

impl VariableDescriptor {
    /// Creates a genuine scalar variable that must be assigned and has no hooks.
    pub fn genuine(name: &'static str) -> Self {
        VariableDescriptor {
            name,
            logical_id: None,
            variable_type: VariableType::Genuine,
            allows_unassigned: false,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
            usize_getter: None,
            usize_setter: None,
            entity_value_provider: None,
            candidate_values: None,
            nearby_value_candidates: None,
            nearby_entity_candidates: None,
            nearby_value_distance_meter: None,
            nearby_entity_distance_meter: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
            search_entity_order_key: None,
            search_value_order_key: None,
        }
    }

    /// Creates a list variable with no hooks.
    pub fn list(name: &'static str) -> Self {
        VariableDescriptor {
            name,
            logical_id: None,
            variable_type: VariableType::List,
            allows_unassigned: false,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
            usize_getter: None,
            usize_setter: None,
            entity_value_provider: None,
            candidate_values: None,
            nearby_value_candidates: None,
            nearby_entity_candidates: None,
            nearby_value_distance_meter: None,
            nearby_entity_distance_meter: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
            search_entity_order_key: None,
            search_value_order_key: None,
        }
    }

    /// Creates a shadow variable of the given kind; shadow variables may be unassigned.
    pub fn shadow(name: &'static str, kind: ShadowVariableKind) -> Self {
        VariableDescriptor {
            name,
            logical_id: None,
            variable_type: VariableType::Shadow(kind),
            allows_unassigned: true,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
            usize_getter: None,
            usize_setter: None,
            entity_value_provider: None,
            candidate_values: None,
            nearby_value_candidates: None,
            nearby_entity_candidates: None,
            nearby_value_distance_meter: None,
            nearby_entity_distance_meter: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
            search_entity_order_key: None,
            search_value_order_key: None,
        }
    }

    /// Names the value range provider that supplies this variable's values.
    pub fn with_value_range(mut self, provider: &'static str) -> Self {
        self.value_range_provider = Some(provider);
        self
    }

    /// Sets whether the variable may stay unassigned.
    pub fn with_allows_unassigned(mut self, allows: bool) -> Self {
        self.allows_unassigned = allows;
        self
    }

    /// Sets the logical variable ID used by dynamic bindings.
    pub fn with_logical_id(mut self, logical_id: VariableId) -> Self {
        self.logical_id = Some(logical_id);
        self
    }

    /// Creates a piggyback shadow variable descriptor.
    ///
    /// Piggyback shadows ride on another shadow variable's listener,
    /// updated as a side-effect without their own dedicated listener.
    pub fn piggyback(name: &'static str, source_shadow: &'static str) -> Self {
        VariableDescriptor {
            name,
            logical_id: None,
            variable_type: VariableType::Shadow(ShadowVariableKind::Piggyback),
            allows_unassigned: true,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: Some(source_shadow),
            source_entity: None,
            usize_getter: None,
            usize_setter: None,
            entity_value_provider: None,
            candidate_values: None,
            nearby_value_candidates: None,
            nearby_entity_candidates: None,
            nearby_value_distance_meter: None,
            nearby_entity_distance_meter: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
            search_entity_order_key: None,
            search_value_order_key: None,
        }
    }

    /// Sets the kind of value range.
    pub fn with_value_range_type(mut self, value_range_type: ValueRangeType) -> Self {
        self.value_range_type = value_range_type;
        self
    }

    /// Sets the source entity type and variable a shadow variable derives from.
    pub fn with_source(mut self, entity: &'static str, variable: &'static str) -> Self {
        self.source_entity = Some(entity);
        self.source_variable = Some(variable);
        self
    }

    /// Sets the scalar getter and setter.
    pub fn with_usize_accessors(mut self, getter: UsizeGetter, setter: UsizeSetter) -> Self {
        self.usize_getter = Some(getter);
        self.usize_setter = Some(setter);
        self
    }

    /// Sets the entity-field value provider.
    pub fn with_entity_value_provider(mut self, provider: UsizeEntityValueProvider) -> Self {
        self.entity_value_provider = Some(provider);
        self
    }

    /// Sets the bounded candidate-values hook.
    pub fn with_candidate_values(mut self, provider: UsizeCandidateValues) -> Self {
        self.candidate_values = Some(provider);
        self
    }

    /// Sets the nearby-value candidates hook.
    pub fn with_nearby_value_candidates(mut self, provider: UsizeCandidateValues) -> Self {
        self.nearby_value_candidates = Some(provider);
        self
    }

    /// Sets the nearby-entity candidates hook.
    pub fn with_nearby_entity_candidates(mut self, provider: UsizeCandidateValues) -> Self {
        self.nearby_entity_candidates = Some(provider);
        self
    }

    /// Sets the nearby-value distance meter.
    pub fn with_nearby_value_distance_meter(
        mut self,
        meter: UsizeNearbyValueDistanceMeter,
    ) -> Self {
        self.nearby_value_distance_meter = Some(meter);
        self
    }

    /// Sets the nearby-entity distance meter.
    pub fn with_nearby_entity_distance_meter(
        mut self,
        meter: UsizeNearbyEntityDistanceMeter,
    ) -> Self {
        self.nearby_entity_distance_meter = Some(meter);
        self
    }

    /// Sets the construction entity order key.
    pub fn with_construction_entity_order_key(
        mut self,
        order_key: UsizeConstructionEntityOrderKey,
    ) -> Self {
        self.construction_entity_order_key = Some(order_key);
        self
    }

    /// Sets the construction value order key.
    pub fn with_construction_value_order_key(
        mut self,
        order_key: UsizeConstructionValueOrderKey,
    ) -> Self {
        self.construction_value_order_key = Some(order_key);
        self
    }
}
