//! Schema definition types for dynamic solutions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::solution::DynamicValue;

/// Field type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    I64,
    F64,
    String,
    Bool,
    Ref,
    List,
    /// DateTime stored as Unix timestamp in milliseconds.
    DateTime,
    /// Date stored as days since Unix epoch.
    Date,
    /// Set of values (with contains semantics).
    Set,
}

/// Definition of a field within an entity or fact class.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name.
    pub name: Arc<str>,
    /// Field type.
    pub field_type: FieldType,
    /// If this is a planning variable, the name of its value range.
    pub value_range: Option<Arc<str>>,
    /// If this is a Ref type, the class name it references.
    pub ref_class: Option<Arc<str>>,
}

impl FieldDef {
    /// Creates a new non-planning field.
    pub fn new(name: impl Into<Arc<str>>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            value_range: None,
            ref_class: None,
        }
    }

    /// Creates a new planning variable field.
    pub fn planning_variable(
        name: impl Into<Arc<str>>,
        field_type: FieldType,
        value_range: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            name: name.into(),
            field_type,
            value_range: Some(value_range.into()),
            ref_class: None,
        }
    }

    /// Creates a reference field pointing to another class.
    pub fn reference(name: impl Into<Arc<str>>, ref_class: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            field_type: FieldType::Ref,
            value_range: None,
            ref_class: Some(ref_class.into()),
        }
    }

    /// Creates a planning variable that references another class.
    pub fn reference_variable(
        name: impl Into<Arc<str>>,
        ref_class: impl Into<Arc<str>>,
        value_range: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            name: name.into(),
            field_type: FieldType::Ref,
            value_range: Some(value_range.into()),
            ref_class: Some(ref_class.into()),
        }
    }

    /// Returns true if this field is a planning variable.
    pub fn is_planning_variable(&self) -> bool {
        self.value_range.is_some()
    }
}

/// Definition of an entity class.
#[derive(Debug, Clone)]
pub struct EntityClassDef {
    /// Class name.
    pub name: Arc<str>,
    /// Field definitions.
    pub fields: Vec<FieldDef>,
    /// Indices of fields that are planning variables.
    pub planning_variable_indices: Vec<usize>,
}

impl EntityClassDef {
    /// Creates a new entity class definition.
    pub fn new(name: impl Into<Arc<str>>, fields: Vec<FieldDef>) -> Self {
        let name = name.into();
        let planning_variable_indices: Vec<usize> = fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_planning_variable())
            .map(|(i, _)| i)
            .collect();
        Self {
            name,
            fields,
            planning_variable_indices,
        }
    }

    /// Finds a field index by name.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.as_ref() == name)
    }
}

/// Definition of a fact class.
#[derive(Debug, Clone)]
pub struct FactClassDef {
    /// Class name.
    pub name: Arc<str>,
    /// Field definitions.
    pub fields: Vec<FieldDef>,
}

impl FactClassDef {
    /// Creates a new fact class definition.
    pub fn new(name: impl Into<Arc<str>>, fields: Vec<FieldDef>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }

    /// Finds a field index by name.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.as_ref() == name)
    }
}

/// Definition of a value range for planning variables.
#[derive(Debug, Clone)]
pub enum ValueRangeDef {
    /// Explicit list of values.
    Explicit(Vec<DynamicValue>),
    /// Integer range [start, end).
    IntRange { start: i64, end: i64 },
    /// Reference to all entities of a class.
    EntityClass(usize),
    /// Reference to all facts of a class.
    FactClass(usize),
}

impl ValueRangeDef {
    /// Creates a value range from explicit integer values.
    pub fn from_ints(values: impl IntoIterator<Item = i64>) -> Self {
        ValueRangeDef::Explicit(values.into_iter().map(DynamicValue::I64).collect())
    }

    /// Creates an integer range [start, end).
    pub fn int_range(start: i64, end: i64) -> Self {
        ValueRangeDef::IntRange { start, end }
    }

    /// Creates a value range referencing all entities of a class.
    pub fn entity_class(class_idx: usize) -> Self {
        ValueRangeDef::EntityClass(class_idx)
    }

    /// Creates a value range referencing all facts of a class.
    pub fn fact_class(class_idx: usize) -> Self {
        ValueRangeDef::FactClass(class_idx)
    }

    /// Returns the values in this range as an iterator.
    pub fn values(&self) -> Vec<DynamicValue> {
        match self {
            ValueRangeDef::Explicit(values) => values.clone(),
            ValueRangeDef::IntRange { start, end } => {
                (*start..*end).map(DynamicValue::I64).collect()
            }
            ValueRangeDef::EntityClass(_) | ValueRangeDef::FactClass(_) => {
                // Entity/fact references must be resolved against the solution
                Vec::new()
            }
        }
    }

    /// Returns the number of values in this range.
    pub fn len(&self) -> usize {
        match self {
            ValueRangeDef::Explicit(values) => values.len(),
            ValueRangeDef::IntRange { start, end } => (*end - *start) as usize,
            ValueRangeDef::EntityClass(_) | ValueRangeDef::FactClass(_) => 0, // Must be resolved against solution
        }
    }

    /// Returns true if the range is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Complete schema descriptor for a dynamic solution.
#[derive(Debug, Clone, Default)]
pub struct DynamicDescriptor {
    /// Entity class definitions.
    pub entity_classes: Vec<EntityClassDef>,
    /// Fact class definitions.
    pub fact_classes: Vec<FactClassDef>,
    /// Value range definitions by name.
    pub value_ranges: HashMap<Arc<str>, ValueRangeDef>,
    /// Mapping from entity class name to index.
    entity_class_indices: HashMap<Arc<str>, usize>,
    /// Mapping from fact class name to index.
    fact_class_indices: HashMap<Arc<str>, usize>,
}

impl DynamicDescriptor {
    /// Creates a new empty descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entity class definition.
    pub fn add_entity_class(&mut self, class: EntityClassDef) {
        let idx = self.entity_classes.len();
        self.entity_class_indices.insert(class.name.clone(), idx);
        self.entity_classes.push(class);
    }

    /// Adds a fact class definition.
    pub fn add_fact_class(&mut self, class: FactClassDef) {
        let idx = self.fact_classes.len();
        self.fact_class_indices.insert(class.name.clone(), idx);
        self.fact_classes.push(class);
    }

    /// Adds a value range definition.
    pub fn add_value_range(&mut self, name: impl Into<Arc<str>>, range: ValueRangeDef) {
        self.value_ranges.insert(name.into(), range);
    }

    /// Gets an entity class index by name.
    pub fn entity_class_index(&self, name: &str) -> Option<usize> {
        self.entity_class_indices.get(name).copied()
    }

    /// Gets a fact class index by name.
    pub fn fact_class_index(&self, name: &str) -> Option<usize> {
        self.fact_class_indices.get(name).copied()
    }

    /// Gets an entity class by index.
    pub fn entity_class(&self, idx: usize) -> Option<&EntityClassDef> {
        self.entity_classes.get(idx)
    }

    /// Gets a fact class by index.
    pub fn fact_class(&self, idx: usize) -> Option<&FactClassDef> {
        self.fact_classes.get(idx)
    }

    /// Gets a value range by name.
    pub fn value_range(&self, name: &str) -> Option<&ValueRangeDef> {
        self.value_ranges.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_def() {
        let field = FieldDef::new("column", FieldType::I64);
        assert_eq!(field.name.as_ref(), "column");
        assert!(!field.is_planning_variable());

        let pv = FieldDef::planning_variable("row", FieldType::I64, "rows");
        assert!(pv.is_planning_variable());
        assert_eq!(pv.value_range.as_deref(), Some("rows"));
    }

    #[test]
    fn test_entity_class_def() {
        let class = EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        );
        assert_eq!(class.name.as_ref(), "Queen");
        assert_eq!(class.planning_variable_indices, vec![1]);
        assert_eq!(class.field_index("column"), Some(0));
        assert_eq!(class.field_index("row"), Some(1));
    }

    #[test]
    fn test_value_range_def() {
        let range = ValueRangeDef::from_ints([0, 1, 2, 3]);
        assert_eq!(range.len(), 4);

        let range = ValueRangeDef::int_range(0, 8);
        assert_eq!(range.len(), 8);
        let values = range.values();
        assert_eq!(values.len(), 8);
    }

    #[test]
    fn test_dynamic_descriptor() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 8));

        assert_eq!(desc.entity_class_index("Queen"), Some(0));
        assert!(desc.entity_class(0).is_some());
        assert!(desc.value_range("rows").is_some());
    }
}
