//! Tests for descriptor types.

mod entity_descriptor;
mod solution_descriptor;
mod variable_descriptor;

use crate::domain::descriptor::*;
use crate::domain::TypedEntityExtractor;
use std::any::Any;

// Shared test helpers used across descriptor test modules.

#[derive(Clone, Debug)]
pub(super) struct TestEntity {
    pub id: i64,
    pub row: Option<i32>,
}

#[derive(Clone, Debug)]
pub(super) struct TestSolution {
    pub entities: Vec<TestEntity>,
}

pub(super) fn get_entities(s: &TestSolution) -> &Vec<TestEntity> {
    &s.entities
}

pub(super) fn get_entities_mut(s: &mut TestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

pub(super) fn create_test_entity_descriptor() -> EntityDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    EntityDescriptor::new(
        "TestEntity",
        std::any::TypeId::of::<TestEntity>(),
        "entities",
    )
    .with_extractor(extractor)
}
