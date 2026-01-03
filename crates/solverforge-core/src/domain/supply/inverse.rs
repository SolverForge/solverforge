//! Inverse variable supply for O(1) "who points to this value?" lookups.
//!
//! In chained variables, we often need to know what entity points TO a given value.
//! Without an inverse supply, this requires scanning all entities (O(n)).
//! The inverse supply maintains a mapping for O(1) lookups.

use std::any::TypeId;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

use super::{DemandKey, Supply, SupplyDemand};

/// Supply that provides O(1) lookup of the entity pointing to a value.
///
/// For a chained variable where `entity.previous = value`, this supply answers:
/// "Given `value`, which `entity` has `entity.previous == value`?"
///
/// This is essential for efficient chain manipulation in moves.
pub trait SingletonInverseVariableSupply<V, E>: Supply
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Gets the entity that points to the given value, if any.
    ///
    /// Returns `None` if no entity currently points to this value.
    fn get_inverse_singleton(&self, value: &V) -> Option<E>;

    /// Registers that an entity now points to a value.
    ///
    /// Called when a variable change causes `entity.var = value`.
    fn insert(&self, value: V, entity: E);

    /// Removes the mapping for a value.
    ///
    /// Called when the entity that pointed to this value changes.
    fn remove(&self, value: &V) -> Option<E>;

    /// Updates the mapping: removes old value mapping, adds new.
    fn update(&self, old_value: Option<&V>, new_value: V, entity: E) {
        if let Some(old) = old_value {
            self.remove(old);
        }
        self.insert(new_value, entity);
    }

    /// Clears all mappings.
    fn clear(&self);

    /// Returns the number of tracked mappings.
    fn len(&self) -> usize;

    /// Returns true if no mappings exist.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Hash-based implementation of inverse variable supply.
///
/// Uses a `HashMap` internally for O(1) average-case lookups.
/// Thread-safe via `RwLock`.
pub struct ExternalizedSingletonInverseVariableSupply<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// The variable name this supply tracks.
    variable_name: String,
    /// Mapping from value to the entity pointing to it.
    inverse_map: RwLock<HashMap<V, E>>,
}

impl<V, E> ExternalizedSingletonInverseVariableSupply<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new externalized inverse variable supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            inverse_map: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the variable name this supply tracks.
    pub fn variable_name(&self) -> &str {
        &self.variable_name
    }
}

impl<V, E> Supply for ExternalizedSingletonInverseVariableSupply<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn supply_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl<V, E> SingletonInverseVariableSupply<V, E>
    for ExternalizedSingletonInverseVariableSupply<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn get_inverse_singleton(&self, value: &V) -> Option<E> {
        self.inverse_map.read().unwrap().get(value).cloned()
    }

    fn insert(&self, value: V, entity: E) {
        self.inverse_map.write().unwrap().insert(value, entity);
    }

    fn remove(&self, value: &V) -> Option<E> {
        self.inverse_map.write().unwrap().remove(value)
    }

    fn clear(&self) {
        self.inverse_map.write().unwrap().clear();
    }

    fn len(&self) -> usize {
        self.inverse_map.read().unwrap().len()
    }
}

impl<V, E> std::fmt::Debug for ExternalizedSingletonInverseVariableSupply<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalizedSingletonInverseVariableSupply")
            .field("variable_name", &self.variable_name)
            .field("size", &self.len())
            .finish()
    }
}

/// Demand for a singleton inverse variable supply.
pub struct SingletonInverseVariableDemand<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// The variable name to track.
    pub variable_name: String,
    /// Phantom data for type parameters.
    _phantom: std::marker::PhantomData<(V, E)>,
}

impl<V, E> SingletonInverseVariableDemand<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new demand for an inverse variable supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<V, E> SupplyDemand for SingletonInverseVariableDemand<V, E>
where
    V: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    type Output = ExternalizedSingletonInverseVariableSupply<V, E>;

    fn demand_key(&self) -> DemandKey {
        DemandKey::new::<Self::Output>(&self.variable_name)
    }

    fn create_supply(&self) -> Self::Output {
        ExternalizedSingletonInverseVariableSupply::new(&self.variable_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverse_supply_insert_and_get() {
        let supply: ExternalizedSingletonInverseVariableSupply<i32, String> =
            ExternalizedSingletonInverseVariableSupply::new("previous");

        supply.insert(1, "entity_a".to_string());
        supply.insert(2, "entity_b".to_string());

        assert_eq!(
            supply.get_inverse_singleton(&1),
            Some("entity_a".to_string())
        );
        assert_eq!(
            supply.get_inverse_singleton(&2),
            Some("entity_b".to_string())
        );
        assert_eq!(supply.get_inverse_singleton(&3), None);
    }

    #[test]
    fn test_inverse_supply_remove() {
        let supply: ExternalizedSingletonInverseVariableSupply<i32, String> =
            ExternalizedSingletonInverseVariableSupply::new("previous");

        supply.insert(1, "entity_a".to_string());
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(&1);
        assert_eq!(removed, Some("entity_a".to_string()));
        assert_eq!(supply.len(), 0);
        assert_eq!(supply.get_inverse_singleton(&1), None);
    }

    #[test]
    fn test_inverse_supply_update() {
        let supply: ExternalizedSingletonInverseVariableSupply<i32, String> =
            ExternalizedSingletonInverseVariableSupply::new("previous");

        supply.insert(1, "entity_a".to_string());
        supply.update(Some(&1), 2, "entity_a".to_string());

        assert_eq!(supply.get_inverse_singleton(&1), None);
        assert_eq!(
            supply.get_inverse_singleton(&2),
            Some("entity_a".to_string())
        );
    }

    #[test]
    fn test_inverse_supply_clear() {
        let supply: ExternalizedSingletonInverseVariableSupply<i32, String> =
            ExternalizedSingletonInverseVariableSupply::new("previous");

        supply.insert(1, "a".to_string());
        supply.insert(2, "b".to_string());
        supply.insert(3, "c".to_string());

        assert_eq!(supply.len(), 3);

        supply.clear();

        assert!(supply.is_empty());
    }

    #[test]
    fn test_inverse_supply_demand() {
        let mut manager = super::super::SupplyManager::new();
        let demand: SingletonInverseVariableDemand<i32, String> =
            SingletonInverseVariableDemand::new("previous");

        let supply = manager.demand(&demand);
        supply.insert(42, "test_entity".to_string());

        // Get the same supply again
        let supply2 = manager.demand(&demand);
        assert_eq!(
            supply2.get_inverse_singleton(&42),
            Some("test_entity".to_string())
        );
    }

    #[test]
    fn test_inverse_supply_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let supply: Arc<ExternalizedSingletonInverseVariableSupply<i32, i32>> =
            Arc::new(ExternalizedSingletonInverseVariableSupply::new("var"));

        let supply_clone = supply.clone();
        let handle = thread::spawn(move || {
            for i in 0..100 {
                supply_clone.insert(i, i * 10);
            }
        });

        handle.join().unwrap();

        assert_eq!(supply.len(), 100);
        assert_eq!(supply.get_inverse_singleton(&50), Some(500));
    }
}
