//! Anchor variable supply for O(1) "what's the chain root?" lookups.
//!
//! In chained variables, each entity belongs to a chain with a root anchor.
//! The anchor supply provides O(1) lookup of an entity's anchor, which is
//! essential for operations like determining if two entities are in the same chain.

use std::any::TypeId;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

use super::{DemandKey, Supply, SupplyDemand};

/// Supply that provides O(1) lookup of an entity's chain anchor.
///
/// For chained variables, entities form chains rooted at anchors (problem facts).
/// This supply answers: "Given an entity, what anchor is at the root of its chain?"
///
/// # Example Chain
///
/// ```text
/// Anchor ← Entity1 ← Entity2 ← Entity3
/// ```
///
/// For all three entities, `get_anchor()` returns the same Anchor.
pub trait AnchorVariableSupply<E, A>: Supply
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    /// Gets the anchor for an entity.
    ///
    /// Returns `None` if the entity is not in any chain (uninitialized).
    fn get_anchor(&self, entity: &E) -> Option<A>;

    /// Sets the anchor for an entity.
    ///
    /// Called when an entity's chain membership changes.
    fn set_anchor(&self, entity: E, anchor: A);

    /// Removes the anchor mapping for an entity.
    fn remove(&self, entity: &E) -> Option<A>;

    /// Clears all mappings.
    fn clear(&self);

    /// Returns the number of tracked entities.
    fn len(&self) -> usize;

    /// Returns true if no entities are tracked.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Hash-based implementation of anchor variable supply.
///
/// Uses a `HashMap` internally for O(1) average-case lookups.
/// Thread-safe via `RwLock`.
pub struct ExternalizedAnchorVariableSupply<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    /// The variable name this supply tracks.
    variable_name: String,
    /// Mapping from entity to its anchor.
    anchor_map: RwLock<HashMap<E, A>>,
}

impl<E, A> ExternalizedAnchorVariableSupply<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    /// Creates a new externalized anchor variable supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            anchor_map: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the variable name this supply tracks.
    pub fn variable_name(&self) -> &str {
        &self.variable_name
    }

    /// Updates anchors for all entities in a chain segment.
    ///
    /// When an entity changes its chain position, all downstream entities
    /// in the chain need their anchor updated. This method cascades the update.
    ///
    /// # Arguments
    ///
    /// * `entities` - Iterator of entities to update (in chain order)
    /// * `new_anchor` - The new anchor for all entities
    pub fn cascade_anchor<I>(&self, entities: I, new_anchor: A)
    where
        I: IntoIterator<Item = E>,
    {
        let mut map = self.anchor_map.write().unwrap();
        for entity in entities {
            map.insert(entity, new_anchor.clone());
        }
    }
}

impl<E, A> Supply for ExternalizedAnchorVariableSupply<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    fn supply_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl<E, A> AnchorVariableSupply<E, A> for ExternalizedAnchorVariableSupply<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    fn get_anchor(&self, entity: &E) -> Option<A> {
        self.anchor_map.read().unwrap().get(entity).cloned()
    }

    fn set_anchor(&self, entity: E, anchor: A) {
        self.anchor_map.write().unwrap().insert(entity, anchor);
    }

    fn remove(&self, entity: &E) -> Option<A> {
        self.anchor_map.write().unwrap().remove(entity)
    }

    fn clear(&self) {
        self.anchor_map.write().unwrap().clear();
    }

    fn len(&self) -> usize {
        self.anchor_map.read().unwrap().len()
    }
}

impl<E, A> std::fmt::Debug for ExternalizedAnchorVariableSupply<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalizedAnchorVariableSupply")
            .field("variable_name", &self.variable_name)
            .field("size", &self.len())
            .finish()
    }
}

/// Demand for an anchor variable supply.
pub struct AnchorVariableDemand<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    /// The variable name to track.
    pub variable_name: String,
    /// Phantom data for type parameters.
    _phantom: std::marker::PhantomData<(E, A)>,
}

impl<E, A> AnchorVariableDemand<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    /// Creates a new demand for an anchor variable supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E, A> SupplyDemand for AnchorVariableDemand<E, A>
where
    E: Eq + Hash + Clone + Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    type Output = ExternalizedAnchorVariableSupply<E, A>;

    fn demand_key(&self) -> DemandKey {
        DemandKey::new::<Self::Output>(&self.variable_name)
    }

    fn create_supply(&self) -> Self::Output {
        ExternalizedAnchorVariableSupply::new(&self.variable_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Entity {
        id: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Anchor {
        name: String,
    }

    #[test]
    fn test_anchor_supply_set_and_get() {
        let supply: ExternalizedAnchorVariableSupply<Entity, Anchor> =
            ExternalizedAnchorVariableSupply::new("previous");

        let entity1 = Entity { id: 1 };
        let entity2 = Entity { id: 2 };
        let anchor = Anchor {
            name: "vehicle_1".to_string(),
        };

        supply.set_anchor(entity1.clone(), anchor.clone());
        supply.set_anchor(entity2.clone(), anchor.clone());

        assert_eq!(supply.get_anchor(&entity1), Some(anchor.clone()));
        assert_eq!(supply.get_anchor(&entity2), Some(anchor));
    }

    #[test]
    fn test_anchor_supply_remove() {
        let supply: ExternalizedAnchorVariableSupply<Entity, Anchor> =
            ExternalizedAnchorVariableSupply::new("previous");

        let entity = Entity { id: 1 };
        let anchor = Anchor {
            name: "anchor".to_string(),
        };

        supply.set_anchor(entity.clone(), anchor.clone());
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(&entity);
        assert_eq!(removed, Some(anchor));
        assert!(supply.is_empty());
    }

    #[test]
    fn test_anchor_supply_cascade() {
        let supply: ExternalizedAnchorVariableSupply<Entity, Anchor> =
            ExternalizedAnchorVariableSupply::new("previous");

        let entities = vec![Entity { id: 1 }, Entity { id: 2 }, Entity { id: 3 }];
        let anchor = Anchor {
            name: "root".to_string(),
        };

        supply.cascade_anchor(entities.clone(), anchor.clone());

        for entity in &entities {
            assert_eq!(supply.get_anchor(entity), Some(anchor.clone()));
        }
    }

    #[test]
    fn test_anchor_supply_update_anchor() {
        let supply: ExternalizedAnchorVariableSupply<Entity, Anchor> =
            ExternalizedAnchorVariableSupply::new("previous");

        let entity = Entity { id: 1 };
        let anchor1 = Anchor {
            name: "anchor1".to_string(),
        };
        let anchor2 = Anchor {
            name: "anchor2".to_string(),
        };

        supply.set_anchor(entity.clone(), anchor1);
        assert_eq!(
            supply.get_anchor(&entity).unwrap().name,
            "anchor1".to_string()
        );

        supply.set_anchor(entity.clone(), anchor2);
        assert_eq!(
            supply.get_anchor(&entity).unwrap().name,
            "anchor2".to_string()
        );

        // Only one mapping exists
        assert_eq!(supply.len(), 1);
    }

    #[test]
    fn test_anchor_supply_demand() {
        let mut manager = super::super::SupplyManager::new();
        let demand: AnchorVariableDemand<Entity, Anchor> =
            AnchorVariableDemand::new("previous");

        let supply = manager.demand(&demand);
        let entity = Entity { id: 42 };
        let anchor = Anchor {
            name: "test".to_string(),
        };

        supply.set_anchor(entity.clone(), anchor.clone());

        // Get the same supply again
        let supply2 = manager.demand(&demand);
        assert_eq!(supply2.get_anchor(&entity), Some(anchor));
    }

    #[test]
    fn test_anchor_supply_clear() {
        let supply: ExternalizedAnchorVariableSupply<Entity, Anchor> =
            ExternalizedAnchorVariableSupply::new("previous");

        let anchor = Anchor {
            name: "anchor".to_string(),
        };

        for i in 0..10 {
            supply.set_anchor(Entity { id: i }, anchor.clone());
        }

        assert_eq!(supply.len(), 10);

        supply.clear();
        assert!(supply.is_empty());
    }
}
