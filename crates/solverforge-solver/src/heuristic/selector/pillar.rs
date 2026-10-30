//! Pillar selector for selecting groups of entities with the same variable value.
//!
//! A pillar is a group of entities that share the same planning variable value.
//! Pillar moves operate on entire pillars, changing or swapping all entities
//! in the pillar atomically.

use std::fmt::Debug;

use super::entity::EntityReference;

/// A pillar is a group of entity references that share the same variable value.
///
/// All entities in a pillar have the same value for a specific planning variable,
/// which allows them to be moved together atomically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pillar {
    /// The entity references in this pillar.
    pub entities: Vec<EntityReference>,
}

impl Pillar {
    pub(crate) fn new(entities: Vec<EntityReference>) -> Self {
        Self { entities }
    }

    pub(crate) fn size(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn first(&self) -> Option<&EntityReference> {
        self.entities.first()
    }

    /// Returns an iterator over the entity references.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &EntityReference> {
        self.entities.iter()
    }
}

/// Configuration for sub-pillar selection.
#[derive(Debug, Clone)]
pub(crate) struct SubPillarConfig {
    /// Whether sub-pillar selection is enabled.
    pub enabled: bool,
    /// Minimum size of a sub-pillar (default: 1).
    pub minimum_size: usize,
    /// Maximum size of a sub-pillar (default: usize::MAX).
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
    pub(crate) fn none() -> Self {
        Self::default()
    }
}
