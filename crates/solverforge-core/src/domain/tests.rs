//! Tests for domain types (entity_ref, value_range, variable).

use std::any::{Any, TypeId};

// ============================================================================
// Entity Ref Tests
// ============================================================================

mod entity_ref_tests {
    use super::*;
    use crate::domain::{EntityExtractor, TypedEntityExtractor};

    #[derive(Clone, Debug)]
    struct TestEntity {
        id: i64,
        value: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TestSolution {
        entities: Vec<TestEntity>,
    }

    fn get_entities(s: &TestSolution) -> &Vec<TestEntity> {
        &s.entities
    }

    fn get_entities_mut(s: &mut TestSolution) -> &mut Vec<TestEntity> {
        &mut s.entities
    }

    #[test]
    fn test_typed_entity_extractor_count() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let solution = TestSolution {
            entities: vec![
                TestEntity {
                    id: 1,
                    value: Some(10),
                },
                TestEntity {
                    id: 2,
                    value: Some(20),
                },
                TestEntity { id: 3, value: None },
            ],
        };

        let count = extractor.count(&solution as &dyn Any);
        assert_eq!(count, Some(3));
    }

    #[test]
    fn test_typed_entity_extractor_get() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let solution = TestSolution {
            entities: vec![
                TestEntity {
                    id: 1,
                    value: Some(10),
                },
                TestEntity {
                    id: 2,
                    value: Some(20),
                },
            ],
        };

        let entity = extractor.get(&solution as &dyn Any, 0);
        assert!(entity.is_some());
        let entity = entity.unwrap().downcast_ref::<TestEntity>().unwrap();
        assert_eq!(entity.id, 1);
        assert_eq!(entity.value, Some(10));

        // Out of bounds
        let entity = extractor.get(&solution as &dyn Any, 5);
        assert!(entity.is_none());
    }

    #[test]
    fn test_typed_entity_extractor_get_mut() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let mut solution = TestSolution {
            entities: vec![TestEntity {
                id: 1,
                value: Some(10),
            }],
        };

        let entity = extractor.get_mut(&mut solution as &mut dyn Any, 0);
        assert!(entity.is_some());
        let entity = entity.unwrap().downcast_mut::<TestEntity>().unwrap();
        entity.value = Some(100);

        assert_eq!(solution.entities[0].value, Some(100));
    }

    #[test]
    fn test_typed_entity_extractor_entity_refs() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let solution = TestSolution {
            entities: vec![
                TestEntity {
                    id: 1,
                    value: Some(10),
                },
                TestEntity {
                    id: 2,
                    value: Some(20),
                },
            ],
        };

        let refs = extractor.entity_refs(&solution as &dyn Any);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].index, 0);
        assert_eq!(refs[0].type_name, "TestEntity");
        assert_eq!(refs[0].collection_field, "entities");
        assert_eq!(refs[1].index, 1);
    }

    #[test]
    fn test_extractor_wrong_solution_type() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let wrong_solution = "not a solution";
        let count = extractor.count(&wrong_solution as &dyn Any);
        assert!(count.is_none());
    }

    #[test]
    fn test_extractor_clone() {
        let extractor: Box<dyn EntityExtractor> = Box::new(TypedEntityExtractor::new(
            "TestEntity",
            "entities",
            get_entities,
            get_entities_mut,
        ));

        let cloned = extractor.clone();

        let solution = TestSolution {
            entities: vec![TestEntity {
                id: 1,
                value: Some(10),
            }],
        };

        assert_eq!(cloned.count(&solution as &dyn Any), Some(1));
    }

    #[test]
    fn test_clone_entity_boxed() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        let solution = TestSolution {
            entities: vec![
                TestEntity {
                    id: 1,
                    value: Some(10),
                },
                TestEntity {
                    id: 2,
                    value: Some(20),
                },
            ],
        };

        // Clone first entity
        let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 0);
        assert!(boxed.is_some());
        let boxed_entity = boxed.unwrap();
        let entity = boxed_entity.downcast_ref::<TestEntity>().unwrap();
        assert_eq!(entity.id, 1);
        assert_eq!(entity.value, Some(10));

        // Clone second entity
        let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 1);
        assert!(boxed.is_some());
        let boxed_entity = boxed.unwrap();
        let entity = boxed_entity.downcast_ref::<TestEntity>().unwrap();
        assert_eq!(entity.id, 2);
        assert_eq!(entity.value, Some(20));

        // Out of bounds returns None
        let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 5);
        assert!(boxed.is_none());
    }

    #[test]
    fn test_entity_type_id() {
        let extractor =
            TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

        assert_eq!(
            extractor.entity_type_id(),
            std::any::TypeId::of::<TestEntity>()
        );
    }
}

// ============================================================================
// Value Range Tests
// ============================================================================

mod value_range_tests {
    use crate::domain::{
        ComputedValueRangeProvider, FieldValueRangeProvider, IntegerRange, StaticValueRange,
        ValueRangeProvider,
    };

    struct TestSolution {
        n: i32,
        values: Vec<i32>,
    }

    #[test]
    fn test_static_value_range() {
        let range = StaticValueRange::new(vec![1, 2, 3, 4, 5]);
        let solution = TestSolution {
            n: 5,
            values: vec![],
        };

        assert_eq!(range.get_values(&solution), vec![1, 2, 3, 4, 5]);
        assert_eq!(range.value_count(&solution), 5);
        assert!(!range.is_empty(&solution));
    }

    #[test]
    fn test_field_value_range_provider() {
        let provider = FieldValueRangeProvider::new(|s: &TestSolution| &s.values);
        let solution = TestSolution {
            n: 3,
            values: vec![10, 20, 30],
        };

        assert_eq!(provider.get_values(&solution), vec![10, 20, 30]);
        assert_eq!(provider.value_count(&solution), 3);
    }

    #[test]
    fn test_computed_value_range_provider() {
        let provider = ComputedValueRangeProvider::new(|s: &TestSolution| (0..s.n).collect());
        let solution = TestSolution {
            n: 4,
            values: vec![],
        };

        assert_eq!(provider.get_values(&solution), vec![0, 1, 2, 3]);
        assert_eq!(provider.value_count(&solution), 4);
    }

    #[test]
    fn test_computed_value_range_type() {
        use crate::domain::variable::ValueRangeType;
        type TestProvider =
            ComputedValueRangeProvider<TestSolution, i32, fn(&TestSolution) -> Vec<i32>>;
        assert_eq!(
            TestProvider::value_range_type(),
            ValueRangeType::EntityDependent
        );
    }

    #[test]
    fn test_integer_range() {
        let range = IntegerRange::new(5, 10);
        let solution = TestSolution {
            n: 0,
            values: vec![],
        };

        let values: Vec<i64> =
            ValueRangeProvider::<TestSolution, i64>::get_values(&range, &solution);
        assert_eq!(values, vec![5, 6, 7, 8, 9]);
        assert_eq!(
            ValueRangeProvider::<TestSolution, i64>::value_count(&range, &solution),
            5
        );
    }

    #[test]
    fn test_integer_range_value_range_type() {
        use crate::domain::variable::ValueRangeType;

        let range = IntegerRange::new(5, 10);
        assert_eq!(
            range.value_range_type(),
            ValueRangeType::CountableRange { from: 5, to: 10 }
        );
    }

    #[test]
    fn test_integer_range_i32() {
        let range = IntegerRange::from_zero(3);
        let solution = TestSolution {
            n: 0,
            values: vec![],
        };

        let values: Vec<i32> =
            ValueRangeProvider::<TestSolution, i32>::get_values(&range, &solution);
        assert_eq!(values, vec![0, 1, 2]);
    }
}

// ============================================================================
// Variable Tests
// ============================================================================

mod variable_tests {
    use super::*;
    use crate::domain::{ChainedVariableInfo, ShadowVariableKind, VariableType};

    #[test]
    fn test_variable_type_is_genuine() {
        assert!(VariableType::Genuine.is_genuine());
        assert!(VariableType::Chained.is_genuine());
        assert!(VariableType::List.is_genuine());
        assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_genuine());
    }

    #[test]
    fn test_variable_type_is_shadow() {
        assert!(!VariableType::Genuine.is_shadow());
        assert!(!VariableType::Chained.is_shadow());
        assert!(!VariableType::List.is_shadow());
        assert!(VariableType::Shadow(ShadowVariableKind::Custom).is_shadow());
        assert!(VariableType::Shadow(ShadowVariableKind::InverseRelation).is_shadow());
    }

    #[test]
    fn test_variable_type_is_chained() {
        assert!(!VariableType::Genuine.is_chained());
        assert!(VariableType::Chained.is_chained());
        assert!(!VariableType::List.is_chained());
        assert!(!VariableType::Shadow(ShadowVariableKind::Anchor).is_chained());
    }

    #[test]
    fn test_variable_type_is_list() {
        assert!(!VariableType::Genuine.is_list());
        assert!(!VariableType::Chained.is_list());
        assert!(VariableType::List.is_list());
        assert!(!VariableType::Shadow(ShadowVariableKind::Index).is_list());
    }

    #[test]
    fn test_variable_type_is_basic() {
        assert!(VariableType::Genuine.is_basic());
        assert!(!VariableType::Chained.is_basic());
        assert!(!VariableType::List.is_basic());
        assert!(!VariableType::Shadow(ShadowVariableKind::Custom).is_basic());
    }

    #[test]
    fn test_shadow_variable_kind_requires_listener() {
        assert!(ShadowVariableKind::Custom.requires_listener());
        assert!(ShadowVariableKind::Cascading.requires_listener());
        assert!(!ShadowVariableKind::InverseRelation.requires_listener());
        assert!(!ShadowVariableKind::Index.requires_listener());
        assert!(!ShadowVariableKind::Anchor.requires_listener());
        assert!(!ShadowVariableKind::Piggyback.requires_listener());
    }

    #[test]
    fn test_shadow_variable_kind_is_automatic() {
        assert!(!ShadowVariableKind::Custom.is_automatic());
        assert!(!ShadowVariableKind::Cascading.is_automatic());
        assert!(ShadowVariableKind::InverseRelation.is_automatic());
        assert!(ShadowVariableKind::Index.is_automatic());
        assert!(ShadowVariableKind::NextElement.is_automatic());
        assert!(ShadowVariableKind::PreviousElement.is_automatic());
        assert!(ShadowVariableKind::Anchor.is_automatic());
        assert!(!ShadowVariableKind::Piggyback.is_automatic());
    }

    #[test]
    fn test_shadow_variable_kind_is_piggyback() {
        assert!(ShadowVariableKind::Piggyback.is_piggyback());
        assert!(!ShadowVariableKind::Custom.is_piggyback());
        assert!(!ShadowVariableKind::Anchor.is_piggyback());
    }

    struct TestAnchor;
    struct TestEntity;

    #[test]
    fn test_chained_variable_info_new() {
        let info = ChainedVariableInfo::new::<TestAnchor, TestEntity>();

        assert_eq!(info.anchor_type_id, TypeId::of::<TestAnchor>());
        assert_eq!(info.entity_type_id, TypeId::of::<TestEntity>());
        assert!(!info.has_anchor_shadow);
    }

    #[test]
    fn test_chained_variable_info_with_anchor_shadow() {
        let info = ChainedVariableInfo::with_anchor_shadow::<TestAnchor, TestEntity>();

        assert_eq!(info.anchor_type_id, TypeId::of::<TestAnchor>());
        assert_eq!(info.entity_type_id, TypeId::of::<TestEntity>());
        assert!(info.has_anchor_shadow);
    }

    #[test]
    fn test_chained_variable_info_type_checks() {
        let info = ChainedVariableInfo::new::<TestAnchor, TestEntity>();

        assert!(info.is_anchor_type(TypeId::of::<TestAnchor>()));
        assert!(!info.is_anchor_type(TypeId::of::<TestEntity>()));

        assert!(info.is_entity_type(TypeId::of::<TestEntity>()));
        assert!(!info.is_entity_type(TypeId::of::<TestAnchor>()));
    }
}
