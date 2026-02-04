//! Schema definition types for dynamic solutions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::solution::DynamicValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    I64,
    F64,
    String,
    Bool,
    Ref,
    List,
    // DateTime stored as Unix timestamp in milliseconds.
    DateTime,
    // Date stored as days since Unix epoch.
    Date,
    Set,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: Arc<str>,
    pub field_type: FieldType,
    pub value_range: Option<Arc<str>>,
    pub ref_class: Option<Arc<str>>,
}

impl FieldDef {
    pub fn new(name: impl Into<Arc<str>>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            value_range: None,
            ref_class: None,
        }
    }

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

    pub fn reference(name: impl Into<Arc<str>>, ref_class: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            field_type: FieldType::Ref,
            value_range: None,
            ref_class: Some(ref_class.into()),
        }
    }

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

    pub fn is_planning_variable(&self) -> bool {
        self.value_range.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct EntityClassDef {
    pub name: Arc<str>,
    pub fields: Vec<FieldDef>,
    pub planning_variable_indices: Vec<usize>,
}

impl EntityClassDef {
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

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.as_ref() == name)
    }
}

#[derive(Debug, Clone)]
pub struct FactClassDef {
    pub name: Arc<str>,
    pub fields: Vec<FieldDef>,
}

impl FactClassDef {
    pub fn new(name: impl Into<Arc<str>>, fields: Vec<FieldDef>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.as_ref() == name)
    }
}

#[derive(Debug, Clone)]
pub enum ValueRangeDef {
    Explicit(Vec<DynamicValue>),
    IntRange { start: i64, end: i64 },
    EntityClass(usize),
    FactClass(usize),
}

impl ValueRangeDef {
    pub fn from_ints(values: impl IntoIterator<Item = i64>) -> Self {
        ValueRangeDef::Explicit(values.into_iter().map(DynamicValue::I64).collect())
    }

    pub fn int_range(start: i64, end: i64) -> Self {
        ValueRangeDef::IntRange { start, end }
    }

    pub fn entity_class(class_idx: usize) -> Self {
        ValueRangeDef::EntityClass(class_idx)
    }

    pub fn fact_class(class_idx: usize) -> Self {
        ValueRangeDef::FactClass(class_idx)
    }

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

    pub fn len(&self) -> usize {
        match self {
            ValueRangeDef::Explicit(values) => values.len(),
            ValueRangeDef::IntRange { start, end } => (*end - *start) as usize,
            ValueRangeDef::EntityClass(_) | ValueRangeDef::FactClass(_) => 0, // Must be resolved against solution
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct DynamicDescriptor {
    pub entity_classes: Vec<EntityClassDef>,
    pub fact_classes: Vec<FactClassDef>,
    pub value_ranges: HashMap<Arc<str>, ValueRangeDef>,
    entity_class_indices: HashMap<Arc<str>, usize>,
    fact_class_indices: HashMap<Arc<str>, usize>,
}

impl DynamicDescriptor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity_class(&mut self, class: EntityClassDef) {
        let idx = self.entity_classes.len();
        self.entity_class_indices.insert(class.name.clone(), idx);
        self.entity_classes.push(class);
    }

    pub fn add_fact_class(&mut self, class: FactClassDef) {
        let idx = self.fact_classes.len();
        self.fact_class_indices.insert(class.name.clone(), idx);
        self.fact_classes.push(class);
    }

    pub fn add_value_range(&mut self, name: impl Into<Arc<str>>, range: ValueRangeDef) {
        self.value_ranges.insert(name.into(), range);
    }

    pub fn entity_class_index(&self, name: &str) -> Option<usize> {
        self.entity_class_indices.get(name).copied()
    }

    pub fn fact_class_index(&self, name: &str) -> Option<usize> {
        self.fact_class_indices.get(name).copied()
    }

    pub fn entity_class(&self, idx: usize) -> Option<&EntityClassDef> {
        self.entity_classes.get(idx)
    }

    pub fn fact_class(&self, idx: usize) -> Option<&FactClassDef> {
        self.fact_classes.get(idx)
    }

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
