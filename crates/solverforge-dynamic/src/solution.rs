//! Dynamic solution types with runtime-defined schemas.

use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use solverforge_core::domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
use solverforge_core::score::HardSoftScore;

use crate::descriptor::DynamicDescriptor;

#[derive(Debug, Clone)]
pub enum DynamicValue {
    None,
    I64(i64),
    F64(f64),
    String(Arc<str>),
    Bool(bool),
    Ref(usize, usize),
    FactRef(usize, usize),
    List(Vec<DynamicValue>),
    DateTime(i64),
    Date(i32),
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
    pub fn is_none(&self) -> bool {
        matches!(self, DynamicValue::None)
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DynamicValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DynamicValue::F64(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DynamicValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DynamicValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<(usize, usize)> {
        match self {
            DynamicValue::Ref(class_idx, entity_idx) => Some((*class_idx, *entity_idx)),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::List(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_fact_ref(&self) -> Option<(usize, usize)> {
        match self {
            DynamicValue::FactRef(class_idx, fact_idx) => Some((*class_idx, *fact_idx)),
            _ => None,
        }
    }

    pub fn as_datetime(&self) -> Option<i64> {
        match self {
            DynamicValue::DateTime(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v), // Allow i64 to be treated as datetime
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<i32> {
        match self {
            DynamicValue::Date(v) => Some(*v),
            DynamicValue::I64(v) => Some(*v as i32),
            _ => None,
        }
    }

    pub fn as_set(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::Set(v) => Some(v),
            DynamicValue::List(v) => Some(v), // Lists can be treated as sets
            _ => None,
        }
    }

    pub fn contains(&self, value: &DynamicValue) -> bool {
        match self {
            DynamicValue::Set(items) | DynamicValue::List(items) => items.contains(value),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynamicEntity {
    pub id: i64,
    pub fields: Vec<DynamicValue>,
}

impl DynamicEntity {
    pub fn new(id: i64, fields: Vec<DynamicValue>) -> Self {
        Self { id, fields }
    }

    pub fn get(&self, field_idx: usize) -> Option<&DynamicValue> {
        self.fields.get(field_idx)
    }

    pub fn get_mut(&mut self, field_idx: usize) -> Option<&mut DynamicValue> {
        self.fields.get_mut(field_idx)
    }

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

#[derive(Debug, Clone)]
pub struct DynamicFact {
    pub id: i64,
    pub fields: Vec<DynamicValue>,
}

impl DynamicFact {
    pub fn new(id: i64, fields: Vec<DynamicValue>) -> Self {
        Self { id, fields }
    }

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
    pub descriptor: DynamicDescriptor,
    pub entities: Vec<Vec<DynamicEntity>>,
    pub facts: Vec<Vec<DynamicFact>>,
    pub score: Option<HardSoftScore>,
    /// Maps entity ID to its location (class_idx, entity_idx) for O(1) lookup.
    pub id_to_location: HashMap<i64, (usize, usize)>,
}

impl DynamicSolution {
    pub fn new(descriptor: DynamicDescriptor) -> Self {
        let entity_count = descriptor.entity_classes.len();
        let fact_count = descriptor.fact_classes.len();
        Self {
            descriptor,
            entities: vec![Vec::new(); entity_count],
            facts: vec![Vec::new(); fact_count],
            score: None,
            id_to_location: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self, class_idx: usize, entity: DynamicEntity) {
        if class_idx < self.entities.len() {
            let entity_idx = self.entities[class_idx].len();
            self.id_to_location.insert(entity.id, (class_idx, entity_idx));
            self.entities[class_idx].push(entity);
        }
    }

    /// Look up an entity's location by its ID in O(1) time.
    ///
    /// Returns `Some((class_idx, entity_idx))` if the entity exists, `None` otherwise.
    pub fn get_entity_location(&self, id: i64) -> Option<(usize, usize)> {
        self.id_to_location.get(&id).copied()
    }

    pub fn add_fact(&mut self, class_idx: usize, fact: DynamicFact) {
        if class_idx < self.facts.len() {
            self.facts[class_idx].push(fact);
        }
    }

    pub fn get_entity(&self, class_idx: usize, entity_idx: usize) -> Option<&DynamicEntity> {
        self.entities.get(class_idx)?.get(entity_idx)
    }

    pub fn get_entity_mut(
        &mut self,
        class_idx: usize,
        entity_idx: usize,
    ) -> Option<&mut DynamicEntity> {
        self.entities.get_mut(class_idx)?.get_mut(entity_idx)
    }

    pub fn get_fact(&self, class_idx: usize, fact_idx: usize) -> Option<&DynamicFact> {
        self.facts.get(class_idx)?.get(fact_idx)
    }

    pub fn entities_in_class(&self, class_idx: usize) -> impl Iterator<Item = &DynamicEntity> {
        self.entities
            .get(class_idx)
            .map(|v| v.iter())
            .into_iter()
            .flatten()
    }

    pub fn entity_refs_in_class(&self, class_idx: usize) -> impl Iterator<Item = (usize, usize)> {
        let count = self.entities.get(class_idx).map(|v| v.len()).unwrap_or(0);
        (0..count).map(move |i| (class_idx, i))
    }

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

    #[test]
    fn test_id_to_location_lookup() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef {
            name: "ClassA".into(),
            fields: vec![FieldDef::new("value", FieldType::I64)],
            planning_variable_indices: vec![],
        });
        desc.add_entity_class(EntityClassDef {
            name: "ClassB".into(),
            fields: vec![FieldDef::new("value", FieldType::I64)],
            planning_variable_indices: vec![],
        });

        let mut solution = DynamicSolution::new(desc);

        // Add entities with various IDs to different classes
        solution.add_entity(0, DynamicEntity::new(100, vec![DynamicValue::I64(1)]));
        solution.add_entity(0, DynamicEntity::new(101, vec![DynamicValue::I64(2)]));
        solution.add_entity(1, DynamicEntity::new(200, vec![DynamicValue::I64(3)]));
        solution.add_entity(1, DynamicEntity::new(201, vec![DynamicValue::I64(4)]));
        solution.add_entity(0, DynamicEntity::new(102, vec![DynamicValue::I64(5)]));

        // Verify O(1) lookup returns correct (class_idx, entity_idx)
        assert_eq!(solution.get_entity_location(100), Some((0, 0)));
        assert_eq!(solution.get_entity_location(101), Some((0, 1)));
        assert_eq!(solution.get_entity_location(102), Some((0, 2)));
        assert_eq!(solution.get_entity_location(200), Some((1, 0)));
        assert_eq!(solution.get_entity_location(201), Some((1, 1)));

        // Non-existent ID returns None
        assert_eq!(solution.get_entity_location(999), None);

        // Verify we can retrieve entity using the location
        let (class_idx, entity_idx) = solution.get_entity_location(201).unwrap();
        let entity = solution.get_entity(class_idx, entity_idx).unwrap();
        assert_eq!(entity.id, 201);
        assert_eq!(entity.get(0).unwrap().as_i64(), Some(4));
    }
}
