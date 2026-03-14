/* Pillar selector for selecting groups of entities with the same variable value.

A pillar is a group of entities that share the same planning variable value.
Pillar moves operate on entire pillars, changing or swapping all entities
in the pillar atomically.
*/

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::entity::{EntityReference, EntitySelector};

/* A pillar is a group of entity references that share the same variable value.

All entities in a pillar have the same value for a specific planning variable,
which allows them to be moved together atomically.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pillar {
    // The entity references in this pillar.
    pub entities: Vec<EntityReference>,
}

impl Pillar {
    pub fn new(entities: Vec<EntityReference>) -> Self {
        Self { entities }
    }

    pub fn size(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn first(&self) -> Option<&EntityReference> {
        self.entities.first()
    }

    /// Returns an iterator over the entity references.
    pub fn iter(&self) -> impl Iterator<Item = &EntityReference> {
        self.entities.iter()
    }
}

/// Trait for selecting pillars of entities.
///
/// A pillar selector groups entities by their variable values and returns
/// groups (pillars) that can be moved together.
///
/// # Type Parameters
/// * `S` - The planning solution type
pub trait PillarSelector<S: PlanningSolution>: Send + Debug {
    /* Returns an iterator over pillars.

    Each pillar contains entity references for entities that share
    the same variable value.
    */
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = Pillar> + 'a;

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    // Returns true if this selector may return the same pillar multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }

    fn descriptor_index(&self) -> usize;
}

// Configuration for sub-pillar selection.
#[derive(Debug, Clone)]
pub struct SubPillarConfig {
    // Whether sub-pillar selection is enabled.
    pub enabled: bool,
    // Minimum size of a sub-pillar (default: 1).
    pub minimum_size: usize,
    // Maximum size of a sub-pillar (default: usize::MAX).
    pub maximum_size: usize,
}

impl Default for SubPillarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            minimum_size: 1,
            maximum_size: usize::MAX,
        }
    }
}

impl SubPillarConfig {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn all() -> Self {
        Self {
            enabled: true,
            minimum_size: 1,
            maximum_size: usize::MAX,
        }
    }

    pub fn with_minimum_size(mut self, size: usize) -> Self {
        self.minimum_size = size.max(1);
        self
    }

    pub fn with_maximum_size(mut self, size: usize) -> Self {
        self.maximum_size = size;
        self
    }
}

/// A pillar selector that groups entities by their variable value.
///
/// This selector uses an entity selector to get entities, then groups them
/// by the value of a specified variable using a value extractor function.
///
/// # Zero-Erasure Design
///
/// Both the entity selector `ES` and the extractor function `E` are stored as
/// concrete generic type parameters, eliminating all virtual dispatch overhead.
///
/// **Note**: The value extractor closure uses `&dyn Director<S>` intentionally.
/// This is a scorer-agnostic boundary - the closure doesn't need the concrete `D` type.
pub struct DefaultPillarSelector<S, V, ES, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    ES: EntitySelector<S>,
    E: Fn(&dyn Director<S>, usize, usize) -> Option<V> + Send + Sync,
{
    // The underlying entity selector (zero-erasure).
    entity_selector: ES,
    // The descriptor index.
    descriptor_index: usize,
    // The variable name for grouping.
    variable_name: &'static str,
    // Function to extract the value from an entity for grouping (zero-erasure).
    value_extractor: E,
    // Sub-pillar configuration.
    sub_pillar_config: SubPillarConfig,
    // Marker for solution and value types.
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, ES, E> Debug for DefaultPillarSelector<S, V, ES, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    ES: EntitySelector<S> + Debug,
    E: Fn(&dyn Director<S>, usize, usize) -> Option<V> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPillarSelector")
            .field("entity_selector", &self.entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("sub_pillar_config", &self.sub_pillar_config)
            .finish()
    }
}

impl<S, V, ES, E> DefaultPillarSelector<S, V, ES, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    ES: EntitySelector<S>,
    E: Fn(&dyn Director<S>, usize, usize) -> Option<V> + Send + Sync,
{
    /// Creates a new default pillar selector.
    ///
    /// # Arguments
    ///
    /// * `entity_selector` - The entity selector to get entities from
    /// * `descriptor_index` - The entity descriptor index
    /// * `variable_name` - The variable name for grouping
    /// * `value_extractor` - Function to extract the grouping value from an entity
    pub fn new(
        entity_selector: ES,
        descriptor_index: usize,
        variable_name: &'static str,
        value_extractor: E,
    ) -> Self {
        Self {
            entity_selector,
            descriptor_index,
            variable_name,
            value_extractor,
            sub_pillar_config: SubPillarConfig::default(),
            _phantom: PhantomData,
        }
    }

    pub fn with_sub_pillar_config(mut self, config: SubPillarConfig) -> Self {
        self.sub_pillar_config = config;
        self
    }

    pub fn variable_name(&self) -> &str {
        self.variable_name
    }

    // Builds the pillar list from the current solution state.
    fn build_pillars<D: Director<S>>(&self, score_director: &D) -> Vec<Pillar> {
        // Group entities by their value
        let mut value_to_entities: HashMap<Option<V>, Vec<EntityReference>> = HashMap::new();

        for entity_ref in self.entity_selector.iter(score_director) {
            // Use dyn Director for the extractor (intentional type-erasure boundary)
            let value = (self.value_extractor)(
                score_director,
                entity_ref.descriptor_index,
                entity_ref.entity_index,
            );
            value_to_entities.entry(value).or_default().push(entity_ref);
        }

        // Filter by minimum size and create pillars
        let min_size = self.sub_pillar_config.minimum_size;
        value_to_entities
            .into_values()
            .filter(|entities| entities.len() >= min_size)
            .map(Pillar::new)
            .collect()
    }
}

impl<S, V, ES, E> PillarSelector<S> for DefaultPillarSelector<S, V, ES, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    ES: EntitySelector<S>,
    E: Fn(&dyn Director<S>, usize, usize) -> Option<V> + Send + Sync,
{
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = Pillar> + 'a {
        let pillars = self.build_pillars(score_director);
        pillars.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.build_pillars(score_director).len()
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }
}

#[cfg(test)]
#[path = "pillar_tests.rs"]
mod tests;
