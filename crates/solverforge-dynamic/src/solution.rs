//! Dynamic solution types with runtime-defined schemas.

use std::any::Any;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use solverforge_core::domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
use solverforge_core::score::HardSoftScore;

use crate::descriptor::DynamicDescriptor;

/// A value in a dynamic solution.
#[derive(Debug, Clone)]
pub enum DynamicValue {
    /// No value assigned (uninitialized planning variable).
    None,
    /// 64-bit signed integer.
    I64(i64),
    /// 64-bit floating point.
    F64(f64),
    /// String value.
    String(Arc<str>),
    /// Boolean value.
    Bool(bool),
    /// Reference to another entity or fact: (class_idx, item_idx).
    Ref(usize, usize),
    /// Reference to a fact: (class_idx, fact_idx).
    FactRef(usize, usize),
    /// List of values.
    List(Vec<DynamicValue>),
    /// DateTime as Unix timestamp in milliseconds.
    DateTime(i64),
    /// Date as days since Unix epoch.
    Date(i32),
    /// Set of values.
    Set(Vec<DynamicValue>),
}

impl PartialEq for DynamicValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DynamicValue::None, DynamicValue::None) => true,
            (DynamicValue::I64(a), DynamicValue::I64(b)) => a == b,
            (DynamicValue::F64(a), DynamicValue::F64(b)) => {
                (a - b).abs() < f64::EPSILON || (a.is_nan() && b.is_nan())
            }
            (DynamicValue::String(a), DynamicValue::String(b)) => a == b,
            (DynamicValue::Bool(a), DynamicValue::Bool(b)) => a == b,
            (DynamicValue::Ref(c1, e1), DynamicValue::Ref(c2, e2)) => c1 == c2 && e1 == e2,
            (DynamicValue::FactRef(c1, f1), DynamicValue::FactRef(c2, f2)) => c1 == c2 && f1 == f2,
            (DynamicValue::List(a), DynamicValue::List(b)) => a == b,
            (DynamicValue::DateTime(a), DynamicValue::DateTime(b)) => a == b,
            (DynamicValue::Date(a), DynamicValue::Date(b)) => a == b,
            (DynamicValue::Set(a), DynamicValue::Set(b)) => {
                // Sets are equal if they contain the same elements (order-independent)
                if a.len() != b.len() {
                    return false;
                }
                a.iter().all(|item| b.contains(item))
            }
            _ => false,
        }
    }
}

impl Eq for DynamicValue {}

impl Hash for DynamicValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            DynamicValue::None => {}
            DynamicValue::I64(v) => v.hash(state),
            DynamicValue::F64(v) => v.to_bits().hash(state),
            DynamicValue::String(v) => v.hash(state),
            DynamicValue::Bool(v) => v.hash(state),
            DynamicValue::Ref(c, e) => {
                c.hash(state);
                e.hash(state);
            }
            DynamicValue::FactRef(c, f) => {
                c.hash(state);
                f.hash(state);
            }
            DynamicValue::List(v) => v.hash(state),
            DynamicValue::DateTime(v) => v.hash(state),
            DynamicValue::Date(v) => v.hash(state),
            DynamicValue::Set(v) => {
                // Hash as a sorted set for consistency
                let mut sorted: Vec<_> = v.iter().collect();
                sorted.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
                for item in sorted {
                    item.hash(state);
                }
            }
        }
    }
}

impl DynamicValue {
    /// Returns true if this value is None.
    pub fn is_none(&self) -> bool {
        matches!(self, DynamicValue::None)
    }

    /// Attempts to extract an i64 value.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DynamicValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract an f64 value.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DynamicValue::F64(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Attempts to extract a bool value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DynamicValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract a string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DynamicValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Attempts to extract a reference.
    pub fn as_ref(&self) -> Option<(usize, usize)> {
        match self {
            DynamicValue::Ref(class_idx, entity_idx) => Some((*class_idx, *entity_idx)),
            _ => None,
        }
    }

    /// Attempts to extract a list.
    pub fn as_list(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::List(v) => Some(v),
            _ => None,
        }
    }

    /// Attempts to extract a fact reference.
    pub fn as_fact_ref(&self) -> Option<(usize, usize)> {
        match self {
            DynamicValue::FactRef(class_idx, fact_idx) => Some((*class_idx, *fact_idx)),
            _ => None,
        }
    }

    /// Attempts to extract a datetime (milliseconds since epoch).
    pub fn as_datetime(&self) -> Option<i64> {
        match self {
            DynamicValue::DateTime(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v), // Allow i64 to be treated as datetime
            _ => None,
        }
    }

    /// Attempts to extract a date (days since epoch).
    pub fn as_date(&self) -> Option<i32> {
        match self {
            DynamicValue::Date(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v as i32),
            _ => None,
        }
    }

    /// Attempts to extract a set.
    pub fn as_set(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::Set(v) => Some(v),
            DynamicValue::List(v) => Some(v), // Lists can be treated as sets
            _ => None,
        }
    }

    /// Checks if this set/list contains the given value.
    pub fn contains(&self, value: &DynamicValue) -> bool {
        match self {
            DynamicValue::Set(items) | DynamicValue::List(items) => items.contains(value),
            _ => false,
        }
    }
}

/// A dynamic entity with runtime-defined fields.
#[derive(Debug, Clone)]
pub struct DynamicEntity {
    /// Unique identifier for this entity.
    pub id: i64,
    /// Field values in order matching the class definition.
    pub fields: Vec<DynamicValue>,
}

impl DynamicEntity {
    /// Creates a new dynamic entity with the given ID and fields.
    pub fn new(id: i64, fields: Vec<DynamicValue>) -> Self {
        Self { id, fields }
    }

    /// Gets a field value by index.
    pub fn get(&self, field_idx: usize) -> Option<&DynamicValue> {
        self.fields.get(field_idx)
    }

    /// Gets a mutable field value by index.
    pub fn get_mut(&mut self, field_idx: usize) -> Option<&mut DynamicValue> {
        self.fields.get_mut(field_idx)
    }

    /// Sets a field value by index.
    pub fn set(&mut self, field_idx: usize, value: DynamicValue) {
        if field_idx < self.fields.len() {
            self.fields[field_idx] = value;
        }
    }
}

impl PlanningEntity for DynamicEntity {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl PlanningId for DynamicEntity {
    type Id = i64;

    fn planning_id(&self) -> i64 {
        self.id
    }
}

/// A dynamic fact with runtime-defined fields.
#[derive(Debug, Clone)]
pub struct DynamicFact {
    /// Unique identifier for this fact.
    pub id: i64,
    /// Field values in order matching the class definition.
    pub fields: Vec<DynamicValue>,
}

impl DynamicFact {
    /// Creates a new dynamic fact with the given ID and fields.
    pub fn new(id: i64, fields: Vec<DynamicValue>) -> Self {
        Self { id, fields }
    }

    /// Gets a field value by index.
    pub fn get(&self, field_idx: usize) -> Option<&DynamicValue> {
        self.fields.get(field_idx)
    }
}

impl ProblemFact for DynamicFact {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl PlanningId for DynamicFact {
    type Id = i64;

    fn planning_id(&self) -> i64 {
        self.id
    }
}

/// A solution where schema is defined at runtime.
///
/// This is the core type that allows dynamic problem definitions while
/// implementing the real `PlanningSolution` trait.
#[derive(Debug, Clone)]
pub struct DynamicSolution {
    /// Schema descriptor.
    pub descriptor: DynamicDescriptor,
    /// Entities organized by class index: entities[class_idx][entity_idx].
    pub entities: Vec<Vec<DynamicEntity>>,
    /// Facts organized by class index: facts[class_idx][fact_idx].
    pub facts: Vec<Vec<DynamicFact>>,
    /// Current score if calculated.
    pub score: Option<HardSoftScore>,
}

impl DynamicSolution {
    /// Creates a new dynamic solution with the given descriptor.
    pub fn new(descriptor: DynamicDescriptor) -> Self {
        let entity_count = descriptor.entity_classes.len();
        let fact_count = descriptor.fact_classes.len();
        Self {
            descriptor,
            entities: vec![Vec::new(); entity_count],
            facts: vec![Vec::new(); fact_count],
            score: None,
        }
    }

    /// Adds an entity to the specified class.
    pub fn add_entity(&mut self, class_idx: usize, entity: DynamicEntity) {
        if class_idx < self.entities.len() {
            self.entities[class_idx].push(entity);
        }
    }

    /// Adds a fact to the specified class.
    pub fn add_fact(&mut self, class_idx: usize, fact: DynamicFact) {
        if class_idx < self.facts.len() {
            self.facts[class_idx].push(fact);
        }
    }

    /// Gets an entity by class index and entity index.
    pub fn get_entity(&self, class_idx: usize, entity_idx: usize) -> Option<&DynamicEntity> {
        self.entities.get(class_idx)?.get(entity_idx)
    }

    /// Gets a mutable entity by class index and entity index.
    pub fn get_entity_mut(
        &mut self,
        class_idx: usize,
        entity_idx: usize,
    ) -> Option<&mut DynamicEntity> {
        self.entities.get_mut(class_idx)?.get_mut(entity_idx)
    }

    /// Gets a fact by class index and fact index.
    pub fn get_fact(&self, class_idx: usize, fact_idx: usize) -> Option<&DynamicFact> {
        self.facts.get(class_idx)?.get(fact_idx)
    }

    /// Returns an iterator over all entities in a class.
    pub fn entities_in_class(&self, class_idx: usize) -> impl Iterator<Item = &DynamicEntity> {
        self.entities
            .get(class_idx)
            .map(|v| v.iter())
            .into_iter()
            .flatten()
    }

    /// Returns an iterator over all entity references (class_idx, entity_idx) in a class.
    pub fn entity_refs_in_class(&self, class_idx: usize) -> impl Iterator<Item = (usize, usize)> {
        let count = self.entities.get(class_idx).map(|v| v.len()).unwrap_or(0);
        (0..count).map(move |i| (class_idx, i))
    }

    /// Returns true if the solution is initialized (all planning variables assigned).
    pub fn check_initialized(&self) -> bool {
        for (class_idx, class) in self.descriptor.entity_classes.iter().enumerate() {
            for entity in self.entities.get(class_idx).into_iter().flatten() {
                for &var_idx in &class.planning_variable_indices {
                    if let Some(value) = entity.fields.get(var_idx) {
                        if value.is_none() {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl PlanningSolution for DynamicSolution {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn is_initialized(&self) -> bool {
        self.check_initialized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{EntityClassDef, FieldDef, FieldType};

    #[test]
    fn test_dynamic_value_conversions() {
        let v = DynamicValue::I64(42);
        assert_eq!(v.as_i64(), Some(42));
        assert_eq!(v.as_f64(), Some(42.0));
        assert!(!v.is_none());

        let v = DynamicValue::None;
        assert!(v.is_none());
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn test_dynamic_entity() {
        let mut entity = DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::None]);
        assert_eq!(entity.planning_id(), 1);
        assert_eq!(entity.get(0).unwrap().as_i64(), Some(0));
        assert!(entity.get(1).unwrap().is_none());

        entity.set(1, DynamicValue::I64(3));
        assert_eq!(entity.get(1).unwrap().as_i64(), Some(3));
    }

    #[test]
    fn test_dynamic_solution() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef {
            name: "Queen".into(),
            fields: vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
            planning_variable_indices: vec![1],
        });

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::None]),
        );

        assert!(!solution.is_initialized());

        solution
            .get_entity_mut(0, 0)
            .unwrap()
            .set(1, DynamicValue::I64(2));
        assert!(solution.is_initialized());
    }
}
