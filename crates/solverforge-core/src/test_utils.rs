//! Test utilities for solverforge-core
//!
//! Provides common test fixtures used across the crate's test modules.

use crate::domain::{EntityExtractor, TypedEntityExtractor};

/// A simple test entity with an id and optional value.
#[derive(Clone, Debug, PartialEq)]
pub struct TestEntity {
    pub id: i64,
    pub value: Option<i32>,
}

impl TestEntity {
    /// Creates a new test entity with the given id and value.
    pub fn new(id: i64, value: Option<i32>) -> Self {
        Self { id, value }
    }

    /// Creates a test entity with an assigned value.
    pub fn assigned(id: i64, value: i32) -> Self {
        Self {
            id,
            value: Some(value),
        }
    }

    /// Creates a test entity with no value assigned.
    pub fn unassigned(id: i64) -> Self {
        Self { id, value: None }
    }
}

/// A simple test solution containing a vector of test entities.
#[derive(Clone, Debug)]
pub struct TestSolution {
    pub entities: Vec<TestEntity>,
}

impl TestSolution {
    /// Creates an empty test solution.
    pub fn empty() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Creates a test solution with the given entities.
    pub fn with_entities(entities: Vec<TestEntity>) -> Self {
        Self { entities }
    }

    /// Returns a reference to the entities.
    pub fn entities(&self) -> &Vec<TestEntity> {
        &self.entities
    }

    /// Returns a mutable reference to the entities.
    pub fn entities_mut(&mut self) -> &mut Vec<TestEntity> {
        &mut self.entities
    }
}

/// Gets a reference to the entities vector from a TestSolution.
pub fn get_test_entities(s: &TestSolution) -> &Vec<TestEntity> {
    &s.entities
}

/// Gets a mutable reference to the entities vector from a TestSolution.
pub fn get_test_entities_mut(s: &mut TestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

/// Creates a TypedEntityExtractor for TestEntity within TestSolution.
pub fn create_test_entity_extractor() -> Box<dyn EntityExtractor> {
    Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_test_entities,
        get_test_entities_mut,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[test]
    fn test_entity_creation() {
        let e1 = TestEntity::new(1, Some(10));
        assert_eq!(e1.id, 1);
        assert_eq!(e1.value, Some(10));

        let e2 = TestEntity::assigned(2, 20);
        assert_eq!(e2.id, 2);
        assert_eq!(e2.value, Some(20));

        let e3 = TestEntity::unassigned(3);
        assert_eq!(e3.id, 3);
        assert_eq!(e3.value, None);
    }

    #[test]
    fn test_solution_creation() {
        let empty = TestSolution::empty();
        assert!(empty.entities.is_empty());

        let with_entities = TestSolution::with_entities(vec![
            TestEntity::assigned(1, 10),
            TestEntity::assigned(2, 20),
        ]);
        assert_eq!(with_entities.entities.len(), 2);
    }

    #[test]
    fn test_extractor_creation() {
        let extractor = create_test_entity_extractor();
        let solution = TestSolution::with_entities(vec![
            TestEntity::assigned(1, 10),
            TestEntity::assigned(2, 20),
        ]);

        assert_eq!(extractor.count(&solution as &dyn Any), Some(2));
    }
}
