/* Generic entity test fixtures.

Provides a generic entity/solution pair for testing domain infrastructure
like entity extractors, descriptors, and solution management.

# Example

```
use solverforge_test::entity::{TestEntity, TestSolution, create_test_descriptor};

let solution = TestSolution::with_entities(vec![
TestEntity::assigned(1, 10),
TestEntity::unassigned(2),
]);
let descriptor = create_test_descriptor();
```
*/

use solverforge_core::domain::{
    EntityDescriptor, EntityExtractor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SoftScore;
use std::any::TypeId;

// A simple test entity with an id and optional value.
#[derive(Clone, Debug, PartialEq)]
pub struct TestEntity {
    pub id: i64,
    pub value: Option<i32>,
}

impl TestEntity {
    pub fn new(id: i64, value: Option<i32>) -> Self {
        Self { id, value }
    }

    pub fn assigned(id: i64, value: i32) -> Self {
        Self {
            id,
            value: Some(value),
        }
    }

    pub fn unassigned(id: i64) -> Self {
        Self { id, value: None }
    }
}

// A test solution containing a vector of test entities and an optional score.
#[derive(Clone, Debug)]
pub struct TestSolution {
    pub entities: Vec<TestEntity>,
    pub score: Option<SoftScore>,
}

impl TestSolution {
    pub fn empty() -> Self {
        Self {
            entities: Vec::new(),
            score: None,
        }
    }

    pub fn with_entities(entities: Vec<TestEntity>) -> Self {
        Self {
            entities,
            score: None,
        }
    }

    pub fn with_score(score: SoftScore) -> Self {
        Self {
            entities: Vec::new(),
            score: Some(score),
        }
    }

    pub fn entities(&self) -> &Vec<TestEntity> {
        &self.entities
    }

    pub fn entities_mut(&mut self) -> &mut Vec<TestEntity> {
        &mut self.entities
    }
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub fn get_test_entities(s: &TestSolution) -> &Vec<TestEntity> {
    &s.entities
}

pub fn get_test_entities_mut(s: &mut TestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

pub fn get_entity_value(s: &TestSolution, idx: usize) -> Option<i32> {
    s.entities.get(idx).and_then(|e| e.value)
}

pub fn set_entity_value(s: &mut TestSolution, idx: usize, v: Option<i32>) {
    if let Some(entity) = s.entities.get_mut(idx) {
        entity.value = v;
    }
}

pub fn create_test_entity_extractor() -> Box<dyn EntityExtractor> {
    Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_test_entities,
        get_test_entities_mut,
    ))
}

pub fn create_test_descriptor() -> SolutionDescriptor {
    let extractor = create_test_entity_extractor();
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>()).with_entity(entity_desc)
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

    #[test]
    fn test_get_set_value() {
        let mut solution =
            TestSolution::with_entities(vec![TestEntity::unassigned(1), TestEntity::unassigned(2)]);

        assert_eq!(get_entity_value(&solution, 0), None);
        set_entity_value(&mut solution, 0, Some(42));
        assert_eq!(get_entity_value(&solution, 0), Some(42));
    }
}
