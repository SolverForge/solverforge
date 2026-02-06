//! Dynamic solution types with runtime-defined schemas.

use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use solverforge_core::domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
use solverforge_core::score::HardSoftScore;

use crate::descriptor::DynamicDescriptor;
use crate::NONE_SENTINEL;

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
            DynamicValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<i32> {
        match self {
            DynamicValue::Date(v) => Some(*v),
            DynamicValue::I64(v) => {
                Some(i32::try_from(*v).expect("i64 value exceeds i32 range for date conversion"))
            }
            _ => None,
        }
    }

    pub fn as_set(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::Set(v) => Some(v),
            DynamicValue::List(v) => Some(v),
            _ => None,
        }
    }

    pub fn contains(&self, value: &DynamicValue) -> bool {
        match self {
            DynamicValue::Set(items) | DynamicValue::List(items) => items.contains(value),
            _ => false,
        }
    }

    /// Flatten to i64 for the flat entity buffer.
    #[inline]
    pub fn to_flat_i64(&self) -> i64 {
        match self {
            DynamicValue::None => NONE_SENTINEL,
            DynamicValue::I64(v) => *v,
            DynamicValue::Bool(b) => *b as i64,
            DynamicValue::DateTime(ms) => *ms,
            DynamicValue::Date(days) => *days as i64,
            DynamicValue::F64(f) => f.to_bits() as i64,
            DynamicValue::Ref(_, idx) | DynamicValue::FactRef(_, idx) => *idx as i64,
            DynamicValue::String(s) => {
                let mut hash: u64 = 0xcbf29ce484222325;
                for byte in s.as_bytes() {
                    hash ^= *byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                hash as i64
            }
            DynamicValue::List(_) | DynamicValue::Set(_) => 0,
        }
    }
}

// ---- DynamicEntity ----

#[derive(Debug, Clone)]
pub struct DynamicEntity {
    pub id: i64,
    /// Accessible for reads. All mutation must go through `DynamicSolution::update_field`.
    pub(crate) fields: Vec<DynamicValue>,
}

impl DynamicEntity {
    pub fn new(id: i64, fields: Vec<DynamicValue>) -> Self {
        Self { id, fields }
    }

    pub fn get(&self, field_idx: usize) -> Option<&DynamicValue> {
        self.fields.get(field_idx)
    }

    pub fn fields(&self) -> &[DynamicValue] {
        &self.fields
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

// ---- DynamicFact ----

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

// ---- DynamicSolution ----

/// A solution where schema is defined at runtime.
///
/// Entity fields are stored twice:
/// - `entities[class_idx][entity_idx].fields` — tagged `DynamicValue` for the interpreter
/// - `flat_entities[class_idx]` — contiguous `i64` buffer for JIT, `entity_idx * field_count` offset
///
/// Both are updated atomically by `update_field`. No other mutation path exists.
#[derive(Debug, Clone)]
pub struct DynamicSolution {
    pub descriptor: DynamicDescriptor,
    pub entities: Vec<Vec<DynamicEntity>>,
    pub facts: Vec<Vec<DynamicFact>>,
    pub score: Option<HardSoftScore>,
    pub id_to_location: HashMap<i64, (usize, usize)>,
    flat_entities: Vec<Vec<i64>>,
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
            flat_entities: vec![Vec::new(); entity_count],
        }
    }

    /// Minimal solution with only descriptor, no entities/facts.
    /// Used by closure factories for key extraction where only the descriptor is needed.
    pub fn empty(descriptor: DynamicDescriptor) -> Self {
        Self::new(descriptor)
    }

    pub fn add_entity(&mut self, class_idx: usize, entity: DynamicEntity) {
        if class_idx < self.entities.len() {
            let entity_idx = self.entities[class_idx].len();
            self.id_to_location
                .insert(entity.id, (class_idx, entity_idx));
            for field in &entity.fields {
                self.flat_entities[class_idx].push(field.to_flat_i64());
            }
            self.entities[class_idx].push(entity);
        }
    }

    /// Remove an entity by index. Updates flat buffer and id_to_location.
    /// Indices of subsequent entities shift down by 1.
    pub fn remove_entity(&mut self, class_idx: usize, entity_idx: usize) {
        let field_count = self.descriptor.entity_classes[class_idx].fields.len();
        let flat_start = entity_idx * field_count;
        let flat_end = flat_start + field_count;

        // Remove from flat buffer
        self.flat_entities[class_idx].drain(flat_start..flat_end);

        // Remove from tagged entities
        let entity = self.entities[class_idx].remove(entity_idx);
        self.id_to_location.remove(&entity.id);

        // Fix id_to_location for shifted indices
        for shifted_idx in entity_idx..self.entities[class_idx].len() {
            let id = self.entities[class_idx][shifted_idx].id;
            self.id_to_location.insert(id, (class_idx, shifted_idx));
        }
    }

    /// The single mutation point. Updates both the tagged `DynamicValue` and the flat i64 slot.
    #[inline]
    pub fn update_field(
        &mut self,
        class_idx: usize,
        entity_idx: usize,
        field_idx: usize,
        value: DynamicValue,
    ) {
        let field_count = self.descriptor.entity_classes[class_idx].fields.len();
        let flat_offset = entity_idx * field_count + field_idx;
        self.flat_entities[class_idx][flat_offset] = value.to_flat_i64();
        self.entities[class_idx][entity_idx].fields[field_idx] = value;
    }

    /// Read a field value.
    #[inline]
    pub fn get_field(
        &self,
        class_idx: usize,
        entity_idx: usize,
        field_idx: usize,
    ) -> Option<&DynamicValue> {
        self.entities
            .get(class_idx)?
            .get(entity_idx)?
            .fields
            .get(field_idx)
    }

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

    /// Mutable entity access for internal use. Callers must NOT mutate fields directly;
    /// use `update_field` instead. This exists for `PlanningEntity::as_any_mut` compatibility.
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

    /// Pointer to entity's flat i64 data. Valid until next `add_entity` on same class.
    #[inline]
    pub fn flat_entity_ptr(&self, class_idx: usize, entity_idx: usize) -> *const i64 {
        let field_count = self.descriptor.entity_classes[class_idx].fields.len();
        let offset = entity_idx * field_count;
        unsafe { self.flat_entities[class_idx].as_ptr().add(offset) }
    }

    pub fn flat_field_count(&self, class_idx: usize) -> usize {
        self.descriptor.entity_classes[class_idx].fields.len()
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
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Test",
            vec![
                FieldDef::new("a", FieldType::I64),
                FieldDef::new("b", FieldType::I64),
            ],
        ));
        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::None]),
        );

        assert_eq!(solution.get_entity(0, 0).unwrap().planning_id(), 1);
        assert_eq!(solution.get_field(0, 0, 0).unwrap().as_i64(), Some(0));
        assert!(solution.get_field(0, 0, 1).unwrap().is_none());

        solution.update_field(0, 0, 1, DynamicValue::I64(3));
        assert_eq!(solution.get_field(0, 0, 1).unwrap().as_i64(), Some(3));

        // Verify flat buffer is in sync
        let ptr = solution.flat_entity_ptr(0, 0);
        unsafe {
            assert_eq!(*ptr, 0); // field 0
            assert_eq!(*ptr.add(1), 3); // field 1 (was None, now 3)
        }
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

        solution.update_field(0, 0, 1, DynamicValue::I64(2));
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
        solution.add_entity(0, DynamicEntity::new(100, vec![DynamicValue::I64(1)]));
        solution.add_entity(0, DynamicEntity::new(101, vec![DynamicValue::I64(2)]));
        solution.add_entity(1, DynamicEntity::new(200, vec![DynamicValue::I64(3)]));
        solution.add_entity(1, DynamicEntity::new(201, vec![DynamicValue::I64(4)]));
        solution.add_entity(0, DynamicEntity::new(102, vec![DynamicValue::I64(5)]));

        assert_eq!(solution.get_entity_location(100), Some((0, 0)));
        assert_eq!(solution.get_entity_location(101), Some((0, 1)));
        assert_eq!(solution.get_entity_location(102), Some((0, 2)));
        assert_eq!(solution.get_entity_location(200), Some((1, 0)));
        assert_eq!(solution.get_entity_location(201), Some((1, 1)));
        assert_eq!(solution.get_entity_location(999), None);

        let (class_idx, entity_idx) = solution.get_entity_location(201).unwrap();
        let entity = solution.get_entity(class_idx, entity_idx).unwrap();
        assert_eq!(entity.id, 201);
        assert_eq!(entity.get(0).unwrap().as_i64(), Some(4));
    }

    #[test]
    fn test_flat_buffer_sync() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Item",
            vec![
                FieldDef::new("id", FieldType::I64),
                FieldDef::planning_variable("slot", FieldType::I64, "slots"),
            ],
        ));

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::None]),
        );
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::None]),
        );

        // Flat buffer: [0, SENTINEL, 1, SENTINEL]
        let ptr0 = solution.flat_entity_ptr(0, 0);
        let ptr1 = solution.flat_entity_ptr(0, 1);
        unsafe {
            assert_eq!(*ptr0, 0);
            assert_eq!(*ptr0.add(1), NONE_SENTINEL);
            assert_eq!(*ptr1, 1);
            assert_eq!(*ptr1.add(1), NONE_SENTINEL);
        }

        // Mutate via update_field
        solution.update_field(0, 0, 1, DynamicValue::I64(42));
        solution.update_field(0, 1, 1, DynamicValue::I64(99));

        // Both tagged and flat must be in sync
        assert_eq!(solution.get_field(0, 0, 1).unwrap().as_i64(), Some(42));
        assert_eq!(solution.get_field(0, 1, 1).unwrap().as_i64(), Some(99));
        unsafe {
            assert_eq!(*solution.flat_entity_ptr(0, 0).add(1), 42);
            assert_eq!(*solution.flat_entity_ptr(0, 1).add(1), 99);
        }
    }
}
