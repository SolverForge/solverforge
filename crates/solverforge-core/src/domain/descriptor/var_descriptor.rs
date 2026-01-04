//! Variable descriptor.

use crate::domain::variable::{ShadowVariableKind, ValueRangeType, VariableType};

/// Describes a planning variable at runtime.
#[derive(Debug, Clone)]
pub struct VariableDescriptor {
    /// Name of the variable (field name).
    pub name: &'static str,
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
}

impl VariableDescriptor {
    /// Creates a new genuine variable descriptor.
    pub fn genuine(name: &'static str) -> Self {
        VariableDescriptor {
            name,
            variable_type: VariableType::Genuine,
            allows_unassigned: false,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
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
        }
    }

    /// Creates a new list variable descriptor.
    pub fn list(name: &'static str) -> Self {
        VariableDescriptor {
            name,
            variable_type: VariableType::List,
            allows_unassigned: false,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
        }
    }

    /// Creates a new shadow variable descriptor.
    pub fn shadow(name: &'static str, kind: ShadowVariableKind) -> Self {
        VariableDescriptor {
            name,
            variable_type: VariableType::Shadow(kind),
            allows_unassigned: true,
            value_range_provider: None,
            value_range_type: ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
        }
    }

    /// Sets the value range provider reference.
    pub fn with_value_range(mut self, provider: &'static str) -> Self {
        self.value_range_provider = Some(provider);
        self
    }

    /// Sets whether unassigned values are allowed.
    pub fn with_allows_unassigned(mut self, allows: bool) -> Self {
        self.allows_unassigned = allows;
        self
    }

    /// Sets the source variable for shadow variables.
    pub fn with_source(mut self, entity: &'static str, variable: &'static str) -> Self {
        self.source_entity = Some(entity);
        self.source_variable = Some(variable);
        self
    }
}
