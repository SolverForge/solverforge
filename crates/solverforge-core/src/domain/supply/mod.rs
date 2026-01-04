//! Supply infrastructure for variable relationship tracking.
//!
//! Supplies provide efficient access to derived information about planning variables,
//! such as inverse relationships (who points to whom) and anchor tracking (chain roots).
//!
//! The supply pattern enables O(1) lookups that would otherwise require O(n) scans.
//!
//! # Architecture
//!
//! - [`Supply`]: Marker trait for all supply types
//! - [`SupplyDemand`]: Request a specific supply type from the manager
//! - [`SupplyManager`]: Registry holding all active supplies
//! - [`ListVariableStateSupply`]: Centralized tracking for list variable shadow state

mod inverse;
mod anchor;
mod list_state;

pub use inverse::{SingletonInverseVariableSupply, SingletonInverseVariableDemand, ExternalizedSingletonInverseVariableSupply};
pub use anchor::{AnchorVariableSupply, AnchorVariableDemand, ExternalizedAnchorVariableSupply};
pub use list_state::{
    ListVariableStateSupply, ListVariableStateDemand, ElementPosition,
    IndexVariableSupply, InverseVariableSupply,
};

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Marker trait for all supply types.
///
/// Supplies are automatically maintained data structures that track
/// relationships between entities and values. They provide O(1) lookups
/// for information that would otherwise require scanning all entities.
///
/// # Examples
///
/// - `SingletonInverseVariableSupply`: Given a value, find the entity pointing to it
/// - `AnchorVariableSupply`: Given an entity in a chain, find its anchor
pub trait Supply: Send + Sync + 'static {
    /// Returns the type ID of this supply for registration purposes.
    fn supply_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Trait for requesting a specific type of supply.
///
/// A demand describes what supply is needed. The `SupplyManager` uses
/// demands to create or retrieve the appropriate supply instance.
pub trait SupplyDemand: Send + Sync + 'static {
    /// The type of supply this demand requests.
    type Output: Supply;

    /// Returns a unique key identifying this specific demand.
    ///
    /// Demands for the same variable should return the same key.
    fn demand_key(&self) -> DemandKey;

    /// Creates a new supply instance for this demand.
    fn create_supply(&self) -> Self::Output;
}

/// A unique key identifying a supply demand.
///
/// The key combines the supply type with the variable it tracks,
/// allowing multiple supplies of the same type for different variables.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DemandKey {
    /// The type of supply being requested.
    pub supply_type: TypeId,
    /// The name of the variable this supply tracks.
    pub variable_name: String,
    /// Optional discriminator for multiple supplies of the same type/variable.
    pub discriminator: Option<String>,
}

impl DemandKey {
    /// Creates a new demand key.
    pub fn new<S: Supply>(variable_name: impl Into<String>) -> Self {
        Self {
            supply_type: TypeId::of::<S>(),
            variable_name: variable_name.into(),
            discriminator: None,
        }
    }

    /// Creates a demand key with a discriminator.
    pub fn with_discriminator<S: Supply>(
        variable_name: impl Into<String>,
        discriminator: impl Into<String>,
    ) -> Self {
        Self {
            supply_type: TypeId::of::<S>(),
            variable_name: variable_name.into(),
            discriminator: Some(discriminator.into()),
        }
    }
}

/// Manager that holds and provides supplies.
///
/// The `SupplyManager` is the central registry for all supplies used by the solver.
/// It creates supplies on-demand when first requested and caches them for reuse.
///
/// # Thread Safety
///
/// The manager uses `Arc` for shared ownership, allowing supplies to be accessed
/// from multiple threads during parallel solving.
#[derive(Default)]
pub struct SupplyManager {
    /// Cached supplies keyed by their demand key.
    supplies: HashMap<DemandKey, Arc<dyn Any + Send + Sync>>,
}

impl SupplyManager {
    /// Creates a new empty supply manager.
    pub fn new() -> Self {
        Self {
            supplies: HashMap::new(),
        }
    }

    /// Gets or creates a supply for the given demand.
    ///
    /// If the supply already exists, returns a clone of the Arc.
    /// Otherwise, creates a new supply using the demand's `create_supply` method.
    pub fn demand<D: SupplyDemand>(&mut self, demand: &D) -> Arc<D::Output> {
        let key = demand.demand_key();

        if let Some(supply) = self.supplies.get(&key) {
            // Safe: we only insert Arc<D::Output> for this key
            supply
                .clone()
                .downcast::<D::Output>()
                .expect("Supply type mismatch")
        } else {
            let supply = Arc::new(demand.create_supply());
            self.supplies
                .insert(key, supply.clone() as Arc<dyn Any + Send + Sync>);
            supply
        }
    }

    /// Registers a pre-created supply.
    ///
    /// This is useful when supplies are created externally and need to be
    /// registered with the manager.
    pub fn register<S: Supply>(&mut self, key: DemandKey, supply: Arc<S>) {
        self.supplies.insert(key, supply as Arc<dyn Any + Send + Sync>);
    }

    /// Gets an existing supply without creating one.
    ///
    /// Returns `None` if the supply has not been created yet.
    pub fn get<S: Supply>(&self, key: &DemandKey) -> Option<Arc<S>> {
        self.supplies
            .get(key)
            .and_then(|s| s.clone().downcast::<S>().ok())
    }

    /// Removes a supply from the manager.
    pub fn remove(&mut self, key: &DemandKey) -> bool {
        self.supplies.remove(key).is_some()
    }

    /// Clears all supplies from the manager.
    pub fn clear(&mut self) {
        self.supplies.clear();
    }

    /// Returns the number of registered supplies.
    pub fn len(&self) -> usize {
        self.supplies.len()
    }

    /// Returns true if no supplies are registered.
    pub fn is_empty(&self) -> bool {
        self.supplies.is_empty()
    }
}

impl std::fmt::Debug for SupplyManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupplyManager")
            .field("supply_count", &self.supplies.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test supply implementation
    struct TestSupply {
        value: i32,
    }

    impl Supply for TestSupply {}

    // Test demand implementation
    struct TestDemand {
        variable_name: String,
        initial_value: i32,
    }

    impl SupplyDemand for TestDemand {
        type Output = TestSupply;

        fn demand_key(&self) -> DemandKey {
            DemandKey::new::<TestSupply>(&self.variable_name)
        }

        fn create_supply(&self) -> TestSupply {
            TestSupply {
                value: self.initial_value,
            }
        }
    }

    #[test]
    fn test_supply_manager_demand_creates_supply() {
        let mut manager = SupplyManager::new();
        let demand = TestDemand {
            variable_name: "test_var".to_string(),
            initial_value: 42,
        };

        let supply = manager.demand(&demand);
        assert_eq!(supply.value, 42);
    }

    #[test]
    fn test_supply_manager_demand_returns_cached() {
        let mut manager = SupplyManager::new();
        let demand = TestDemand {
            variable_name: "test_var".to_string(),
            initial_value: 42,
        };

        let supply1 = manager.demand(&demand);
        let supply2 = manager.demand(&demand);

        // Should return the same Arc
        assert!(Arc::ptr_eq(&supply1, &supply2));
    }

    #[test]
    fn test_supply_manager_different_variables() {
        let mut manager = SupplyManager::new();

        let demand1 = TestDemand {
            variable_name: "var1".to_string(),
            initial_value: 1,
        };
        let demand2 = TestDemand {
            variable_name: "var2".to_string(),
            initial_value: 2,
        };

        let supply1 = manager.demand(&demand1);
        let supply2 = manager.demand(&demand2);

        assert_eq!(supply1.value, 1);
        assert_eq!(supply2.value, 2);
        assert!(!Arc::ptr_eq(&supply1, &supply2));
    }

    #[test]
    fn test_supply_manager_register() {
        let mut manager = SupplyManager::new();
        let key = DemandKey::new::<TestSupply>("registered_var");
        let supply = Arc::new(TestSupply { value: 100 });

        manager.register(key.clone(), supply);

        let retrieved = manager.get::<TestSupply>(&key);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, 100);
    }

    #[test]
    fn test_supply_manager_remove() {
        let mut manager = SupplyManager::new();
        let demand = TestDemand {
            variable_name: "to_remove".to_string(),
            initial_value: 1,
        };

        let _ = manager.demand(&demand);
        assert_eq!(manager.len(), 1);

        let key = demand.demand_key();
        assert!(manager.remove(&key));
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_supply_manager_clear() {
        let mut manager = SupplyManager::new();

        let demand1 = TestDemand {
            variable_name: "var1".to_string(),
            initial_value: 1,
        };
        let demand2 = TestDemand {
            variable_name: "var2".to_string(),
            initial_value: 2,
        };

        let _ = manager.demand(&demand1);
        let _ = manager.demand(&demand2);
        assert_eq!(manager.len(), 2);

        manager.clear();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_demand_key_with_discriminator() {
        let key1 = DemandKey::new::<TestSupply>("var");
        let key2 = DemandKey::with_discriminator::<TestSupply>("var", "disc");

        assert_ne!(key1, key2);
        assert!(key1.discriminator.is_none());
        assert_eq!(key2.discriminator, Some("disc".to_string()));
    }
}
