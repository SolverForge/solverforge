// Variable descriptor.

use std::any::Any;

use crate::domain::variable::{ShadowVariableKind, ValueRangeType, VariableType};

pub type UsizeGetter = for<'a> fn(&'a dyn Any) -> Option<usize>;
pub type UsizeSetter = fn(&mut dyn Any, Option<usize>);
pub type UsizeEntityValueProvider = for<'a> fn(&'a dyn Any) -> Vec<usize>;
pub type UsizeCandidateValues = for<'a> fn(&'a dyn Any, usize, usize) -> &'a [usize];
pub type UsizeNearbyValueDistanceMeter = fn(&dyn Any, usize, usize) -> f64;
pub type UsizeNearbyEntityDistanceMeter = fn(&dyn Any, usize, usize) -> f64;
pub type UsizeConstructionEntityOrderKey = fn(&dyn Any, usize) -> i64;
pub type UsizeConstructionValueOrderKey = fn(&dyn Any, usize, usize) -> i64;

// Describes a planning variable at runtime.
#[derive(Debug, Clone)]
pub struct VariableDescriptor {
    // Name of the variable (field name).
    pub name: &'static str,
    // Type of the variable.
    pub variable_type: VariableType,
    // Whether the variable can be unassigned (null).
    pub allows_unassigned: bool,
    // Reference to the value range provider.
    pub value_range_provider: Option<&'static str>,
    // The type of value range.
    pub value_range_type: ValueRangeType,
    // For shadow variables: the source variable name.
    pub source_variable: Option<&'static str>,
    // For shadow variables: the source entity type name.
    pub source_entity: Option<&'static str>,
    // Dynamic accessors for canonical scalar-variable solving.
    pub usize_getter: Option<UsizeGetter>,
    pub usize_setter: Option<UsizeSetter>,
    pub entity_value_provider: Option<UsizeEntityValueProvider>,
    pub candidate_values: Option<UsizeCandidateValues>,
    pub nearby_value_candidates: Option<UsizeCandidateValues>,
    pub nearby_entity_candidates: Option<UsizeCandidateValues>,
    pub nearby_value_distance_meter: Option<UsizeNearbyValueDistanceMeter>,
    pub nearby_entity_distance_meter: Option<UsizeNearbyEntityDistanceMeter>,
    pub construction_entity_order_key: Option<UsizeConstructionEntityOrderKey>,
    pub construction_value_order_key: Option<UsizeConstructionValueOrderKey>,
}

impl VariableDescriptor {
    pub fn genuine(name: &'static str) -> Self {
        VariableDescriptor {
            name,
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
        }
    }

    /// Creates a new chained variable descriptor.
    ///
    /// Chained variables form chains rooted at anchor problem facts.
    /// For example, in vehicle routing: Vehicle ← Customer1 ← Customer2
    pub fn chained(name: &'static str) -> Self {
        VariableDescriptor {
            name,
            variable_type: VariableType::Chained,
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
        }
    }

    pub fn list(name: &'static str) -> Self {
        VariableDescriptor {
            name,
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
        }
    }

    pub fn shadow(name: &'static str, kind: ShadowVariableKind) -> Self {
        VariableDescriptor {
            name,
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
        }
    }

    pub fn with_value_range(mut self, provider: &'static str) -> Self {
        self.value_range_provider = Some(provider);
        self
    }

    pub fn with_allows_unassigned(mut self, allows: bool) -> Self {
        self.allows_unassigned = allows;
        self
    }

    /// Creates a piggyback shadow variable descriptor.
    ///
    /// Piggyback shadows ride on another shadow variable's listener,
    /// updated as a side-effect without their own dedicated listener.
    pub fn piggyback(name: &'static str, source_shadow: &'static str) -> Self {
        VariableDescriptor {
            name,
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
        }
    }

    pub fn with_value_range_type(mut self, value_range_type: ValueRangeType) -> Self {
        self.value_range_type = value_range_type;
        self
    }

    pub fn with_source(mut self, entity: &'static str, variable: &'static str) -> Self {
        self.source_entity = Some(entity);
        self.source_variable = Some(variable);
        self
    }

    pub fn with_usize_accessors(mut self, getter: UsizeGetter, setter: UsizeSetter) -> Self {
        self.usize_getter = Some(getter);
        self.usize_setter = Some(setter);
        self
    }

    pub fn with_entity_value_provider(mut self, provider: UsizeEntityValueProvider) -> Self {
        self.entity_value_provider = Some(provider);
        self
    }

    pub fn with_candidate_values(mut self, provider: UsizeCandidateValues) -> Self {
        self.candidate_values = Some(provider);
        self
    }

    pub fn with_nearby_value_candidates(mut self, provider: UsizeCandidateValues) -> Self {
        self.nearby_value_candidates = Some(provider);
        self
    }

    pub fn with_nearby_entity_candidates(mut self, provider: UsizeCandidateValues) -> Self {
        self.nearby_entity_candidates = Some(provider);
        self
    }

    pub fn with_nearby_value_distance_meter(
        mut self,
        meter: UsizeNearbyValueDistanceMeter,
    ) -> Self {
        self.nearby_value_distance_meter = Some(meter);
        self
    }

    pub fn with_nearby_entity_distance_meter(
        mut self,
        meter: UsizeNearbyEntityDistanceMeter,
    ) -> Self {
        self.nearby_entity_distance_meter = Some(meter);
        self
    }

    pub fn with_construction_entity_order_key(
        mut self,
        order_key: UsizeConstructionEntityOrderKey,
    ) -> Self {
        self.construction_entity_order_key = Some(order_key);
        self
    }

    pub fn with_construction_value_order_key(
        mut self,
        order_key: UsizeConstructionValueOrderKey,
    ) -> Self {
        self.construction_value_order_key = Some(order_key);
        self
    }
}
