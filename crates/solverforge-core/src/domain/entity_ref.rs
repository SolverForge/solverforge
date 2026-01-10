//! Entity reference types for dynamic entity access.
//!
//! These types enable the solver to work with entities at runtime without
//! knowing their concrete types at compile time.

use std::any::Any;
use std::fmt::Debug;

/// A reference to a planning entity with its index in the solution.
///
/// This struct provides a way to identify and access entities during solving
/// without needing to know the concrete entity type.
#[derive(Debug, Clone)]
pub struct EntityRef {
    /// Index of this entity in its collection.
    pub index: usize,
    /// Name of the entity type.
    pub type_name: &'static str,
    /// Name of the collection field in the solution.
    pub collection_field: &'static str,
}

impl EntityRef {
    /// Creates a new entity reference.
    pub fn new(index: usize, type_name: &'static str, collection_field: &'static str) -> Self {
        Self {
            index,
            type_name,
            collection_field,
        }
    }
}

/// Trait for extracting entities from a planning solution.
///
/// This trait is implemented by closures/functions that can extract
/// entity references from a solution of a specific type.
pub trait EntityExtractor: Send + Sync {
    /// Returns the number of entities in the collection.
    fn count(&self, solution: &dyn Any) -> Option<usize>;

    /// Gets a reference to an entity by index.
    fn get<'a>(&self, solution: &'a dyn Any, index: usize) -> Option<&'a dyn Any>;

    /// Gets a mutable reference to an entity by index.
    fn get_mut<'a>(&self, solution: &'a mut dyn Any, index: usize) -> Option<&'a mut dyn Any>;

    /// Returns an iterator over entity references.
    fn entity_refs(&self, solution: &dyn Any) -> Vec<EntityRef>;

    /// Clone this extractor.
    fn clone_box(&self) -> Box<dyn EntityExtractor>;

    /// Clones an entity as a boxed value for insertion into the constraint session.
    ///
    /// This is used for incremental scoring where entities need to be inserted
    /// into the BAVET session as owned, type-erased values.
    fn clone_entity_boxed(
        &self,
        solution: &dyn Any,
        index: usize,
    ) -> Option<Box<dyn Any + Send + Sync>>;

    /// Returns the TypeId of the entity type.
    fn entity_type_id(&self) -> std::any::TypeId;
}

impl Clone for Box<dyn EntityExtractor> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A concrete entity extractor for a specific solution and entity type.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `E` - The entity type
pub struct TypedEntityExtractor<S, E> {
    /// Name of the entity type.
    type_name: &'static str,
    /// Name of the collection field in the solution.
    collection_field: &'static str,
    /// Function to get the entity collection from a solution.
    get_collection: fn(&S) -> &Vec<E>,
    /// Function to get the mutable entity collection from a solution.
    get_collection_mut: fn(&mut S) -> &mut Vec<E>,
}

impl<S, E> TypedEntityExtractor<S, E>
where
    S: 'static,
    E: 'static,
{
    /// Creates a new typed entity extractor.
    pub fn new(
        type_name: &'static str,
        collection_field: &'static str,
        get_collection: fn(&S) -> &Vec<E>,
        get_collection_mut: fn(&mut S) -> &mut Vec<E>,
    ) -> Self {
        Self {
            type_name,
            collection_field,
            get_collection,
            get_collection_mut,
        }
    }
}

impl<S, E> EntityExtractor for TypedEntityExtractor<S, E>
where
    S: Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn count(&self, solution: &dyn Any) -> Option<usize> {
        let solution = solution.downcast_ref::<S>()?;
        Some((self.get_collection)(solution).len())
    }

    fn get<'a>(&self, solution: &'a dyn Any, index: usize) -> Option<&'a dyn Any> {
        let solution = solution.downcast_ref::<S>()?;
        let collection = (self.get_collection)(solution);
        collection.get(index).map(|e| e as &dyn Any)
    }

    fn get_mut<'a>(&self, solution: &'a mut dyn Any, index: usize) -> Option<&'a mut dyn Any> {
        let solution = solution.downcast_mut::<S>()?;
        let collection = (self.get_collection_mut)(solution);
        collection.get_mut(index).map(|e| e as &mut dyn Any)
    }

    fn entity_refs(&self, solution: &dyn Any) -> Vec<EntityRef> {
        let Some(solution) = solution.downcast_ref::<S>() else {
            return Vec::new();
        };
        let collection = (self.get_collection)(solution);
        (0..collection.len())
            .map(|i| EntityRef::new(i, self.type_name, self.collection_field))
            .collect()
    }

    fn clone_box(&self) -> Box<dyn EntityExtractor> {
        Box::new(Self {
            type_name: self.type_name,
            collection_field: self.collection_field,
            get_collection: self.get_collection,
            get_collection_mut: self.get_collection_mut,
        })
    }

    fn clone_entity_boxed(
        &self,
        solution: &dyn Any,
        index: usize,
    ) -> Option<Box<dyn Any + Send + Sync>> {
        let solution = solution.downcast_ref::<S>()?;
        let collection = (self.get_collection)(solution);
        let entity = collection.get(index)?;
        Some(Box::new(entity.clone()) as Box<dyn Any + Send + Sync>)
    }

    fn entity_type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<E>()
    }
}

impl<S, E> Debug for TypedEntityExtractor<S, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedEntityExtractor")
            .field("type_name", &self.type_name)
            .field("collection_field", &self.collection_field)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
