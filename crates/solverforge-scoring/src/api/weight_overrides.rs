// Runtime constraint weight configuration.
//
// Allows dynamic adjustment of constraint weights without recompiling.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use solverforge_core::Score;

// Holds runtime overrides for constraint weights.
//
// Use this to adjust constraint weights without recompiling. Weights can be
// changed between solver runs or even during solving (if you rebuild constraints).
#[derive(Clone)]
pub struct ConstraintWeightOverrides<Sc: Score> {
    weights: HashMap<String, Sc>,
}

impl<Sc: Score> Debug for ConstraintWeightOverrides<Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstraintWeightOverrides")
            .field("count", &self.weights.len())
            .finish()
    }
}

impl<Sc: Score> Default for ConstraintWeightOverrides<Sc> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Sc: Score> ConstraintWeightOverrides<Sc> {
    // Creates an empty overrides container.
    pub fn new() -> Self {
        Self {
            weights: HashMap::new(),
        }
    }

    // Creates overrides from an iterator of (name, weight) pairs.
    pub fn from_pairs<I, N>(iter: I) -> Self
    where
        I: IntoIterator<Item = (N, Sc)>,
        N: Into<String>,
    {
        let weights = iter.into_iter().map(|(n, w)| (n.into(), w)).collect();
        Self { weights }
    }

    // Sets the weight for a constraint.
    pub fn put<N: Into<String>>(&mut self, name: N, weight: Sc) {
        self.weights.insert(name.into(), weight);
    }

    // Removes the override for a constraint.
    pub fn remove(&mut self, name: &str) -> Option<Sc> {
        self.weights.remove(name)
    }

    // Gets the overridden weight, or returns the default if not overridden.
    pub fn get_or_default(&self, name: &str, default: Sc) -> Sc {
        self.weights.get(name).cloned().unwrap_or(default)
    }

    // Gets the overridden weight if present.
    pub fn get(&self, name: &str) -> Option<&Sc> {
        self.weights.get(name)
    }

    // Returns true if this constraint has an override.
    pub fn contains(&self, name: &str) -> bool {
        self.weights.contains_key(name)
    }

    // Returns the number of overrides.
    pub fn len(&self) -> usize {
        self.weights.len()
    }

    // Returns true if there are no overrides.
    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    // Clears all overrides.
    pub fn clear(&mut self) {
        self.weights.clear();
    }

    // Creates an Arc-wrapped version for sharing across threads.
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

// Helper trait for creating weight functions from overrides.
// This enables zero-erasure constraint building with runtime weight lookup.
pub trait WeightProvider<Sc: Score>: Send + Sync {
    // Gets the weight for a constraint by name.
    fn weight(&self, name: &str) -> Option<Sc>;

    // Gets the weight or returns the default.
    fn weight_or_default(&self, name: &str, default: Sc) -> Sc {
        self.weight(name).unwrap_or(default)
    }
}

impl<Sc: Score> WeightProvider<Sc> for ConstraintWeightOverrides<Sc> {
    fn weight(&self, name: &str) -> Option<Sc> {
        self.get(name).cloned()
    }
}

impl<Sc: Score> WeightProvider<Sc> for Arc<ConstraintWeightOverrides<Sc>> {
    fn weight(&self, name: &str) -> Option<Sc> {
        self.get(name).cloned()
    }
}
