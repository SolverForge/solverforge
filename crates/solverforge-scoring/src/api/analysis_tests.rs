//! Tests for score analysis types.

use super::analysis::*;
use solverforge_core::score::SimpleScore;
use solverforge_core::ConstraintRef;

#[derive(Clone, Debug, PartialEq)]
struct TestEntity {
    id: i32,
    name: String,
}

#[test]
fn test_entity_ref_creation() {
    let entity = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity_ref = EntityRef::new(&entity);

    assert!(entity_ref.type_name.contains("TestEntity"));
    assert!(entity_ref.display.contains("Test"));
    assert!(entity_ref.display.contains("1"));
}

#[test]
fn test_entity_ref_downcast() {
    let entity = TestEntity {
        id: 42,
        name: "Answer".to_string(),
    };
    let entity_ref = EntityRef::new(&entity);

    let recovered: Option<&TestEntity> = entity_ref.as_entity();
    assert!(recovered.is_some());
    assert_eq!(recovered.unwrap().id, 42);
}

#[test]
fn test_entity_ref_equality() {
    let entity1 = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity2 = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity3 = TestEntity {
        id: 2,
        name: "Other".to_string(),
    };

    let ref1 = EntityRef::new(&entity1);
    let ref2 = EntityRef::new(&entity2);
    let ref3 = EntityRef::new(&entity3);

    assert_eq!(ref1, ref2);
    assert_ne!(ref1, ref3);
}

#[test]
fn test_constraint_justification() {
    let entity1 = TestEntity {
        id: 1,
        name: "Alice".to_string(),
    };
    let entity2 = TestEntity {
        id: 2,
        name: "Bob".to_string(),
    };

    let just =
        ConstraintJustification::new(vec![EntityRef::new(&entity1), EntityRef::new(&entity2)]);

    assert_eq!(just.entities.len(), 2);
    assert!(just.description.contains("Alice"));
    assert!(just.description.contains("Bob"));
}

#[test]
fn test_detailed_evaluation() {
    let constraint_ref = ConstraintRef::new("", "TestConstraint");
    let entity = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };

    let match1 = DetailedConstraintMatch::new(
        constraint_ref.clone(),
        SimpleScore::of(-1),
        ConstraintJustification::new(vec![EntityRef::new(&entity)]),
    );

    let eval = DetailedConstraintEvaluation::new(SimpleScore::of(-1), vec![match1]);

    assert_eq!(eval.total_score, SimpleScore::of(-1));
    assert_eq!(eval.match_count, 1);
    assert_eq!(eval.matches.len(), 1);
}

#[test]
fn test_indictment_map() {
    let constraint_ref = ConstraintRef::new("", "TestConstraint");
    let entity1 = TestEntity {
        id: 1,
        name: "Alice".to_string(),
    };
    let entity2 = TestEntity {
        id: 2,
        name: "Bob".to_string(),
    };

    let match1 = DetailedConstraintMatch::new(
        constraint_ref.clone(),
        SimpleScore::of(-1),
        ConstraintJustification::new(vec![EntityRef::new(&entity1), EntityRef::new(&entity2)]),
    );

    let map = IndictmentMap::from_matches(vec![match1]);

    assert_eq!(map.len(), 2);

    let alice_ref = EntityRef::new(&entity1);
    let alice_indictment = map.get(&alice_ref).unwrap();
    assert_eq!(alice_indictment.match_count(), 1);
    assert_eq!(alice_indictment.score, SimpleScore::of(-1));
}

#[test]
fn test_score_explanation() {
    let constraint_ref = ConstraintRef::new("", "TestConstraint");

    let analysis = ConstraintAnalysis::new(
        constraint_ref,
        SimpleScore::of(1),
        SimpleScore::of(-3),
        vec![],
        false,
    );

    let explanation = ScoreExplanation::new(SimpleScore::of(-3), vec![analysis]);

    assert_eq!(explanation.score, SimpleScore::of(-3));
    assert_eq!(explanation.constraint_analyses.len(), 1);
    assert_eq!(explanation.non_zero_constraints().len(), 1);
}
