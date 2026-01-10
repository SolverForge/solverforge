//! Solution descriptor.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

use super::{EntityDescriptor, ProblemFactDescriptor, VariableDescriptor};
use crate::domain::entity_ref::EntityRef;

/// Describes a planning solution at runtime.
///
/// Contains metadata about:
/// - Entity types and their descriptors
/// - Problem fact types
/// - Score type
pub struct SolutionDescriptor {
    /// Name of the solution type.
    pub type_name: &'static str,
    /// TypeId of the solution type.
    pub type_id: TypeId,
    /// Descriptors for all entity types in this solution.
    pub entity_descriptors: Vec<EntityDescriptor>,
    /// Descriptors for all problem fact types.
    pub problem_fact_descriptors: Vec<ProblemFactDescriptor>,
    /// Name of the score field.
    pub score_field: &'static str,
    /// Whether the score type is Option<Score>.
    pub score_is_optional: bool,
    /// Index mapping entity TypeId to descriptor index for O(1) lookup.
    entity_type_index: HashMap<TypeId, usize>,
}

impl SolutionDescriptor {
    /// Creates a new SolutionDescriptor.
    pub fn new(type_name: &'static str, type_id: TypeId) -> Self {
        SolutionDescriptor {
            type_name,
            type_id,
            entity_descriptors: Vec::new(),
            problem_fact_descriptors: Vec::new(),
            score_field: "score",
            score_is_optional: true,
            entity_type_index: HashMap::new(),
        }
    }

    /// Adds an entity descriptor and indexes it by TypeId for O(1) lookup.
    pub fn with_entity(mut self, descriptor: EntityDescriptor) -> Self {
        let index = self.entity_descriptors.len();
        let type_id = descriptor.type_id;
        self.entity_descriptors.push(descriptor);
        self.entity_type_index.insert(type_id, index);
        self
    }

    /// Adds a problem fact descriptor.
    pub fn with_problem_fact(mut self, descriptor: ProblemFactDescriptor) -> Self {
        self.problem_fact_descriptors.push(descriptor);
        self
    }

    /// Sets the score field name.
    pub fn with_score_field(mut self, field: &'static str) -> Self {
        self.score_field = field;
        self
    }

    /// Finds an entity descriptor by type name.
    pub fn find_entity_descriptor(&self, type_name: &str) -> Option<&EntityDescriptor> {
        self.entity_descriptors
            .iter()
            .find(|d| d.type_name == type_name)
    }

    /// Finds an entity descriptor by type ID (O(1) lookup).
    pub fn find_entity_descriptor_by_type(&self, type_id: TypeId) -> Option<&EntityDescriptor> {
        self.entity_type_index
            .get(&type_id)
            .and_then(|&idx| self.entity_descriptors.get(idx))
    }

    /// Returns all genuine variable descriptors across all entities.
    pub fn genuine_variable_descriptors(&self) -> Vec<&VariableDescriptor> {
        self.entity_descriptors
            .iter()
            .flat_map(|e| e.genuine_variable_descriptors())
            .collect()
    }

    /// Returns all shadow variable descriptors across all entities.
    pub fn shadow_variable_descriptors(&self) -> Vec<&VariableDescriptor> {
        self.entity_descriptors
            .iter()
            .flat_map(|e| e.shadow_variable_descriptors())
            .collect()
    }

    /// Returns the total number of entities across all entity collections.
    ///
    /// Returns `None` if any entity descriptor lacks an extractor or the solution
    /// type doesn't match.
    pub fn total_entity_count(&self, solution: &dyn Any) -> Option<usize> {
        let mut total = 0;
        for desc in &self.entity_descriptors {
            total += desc.entity_count(solution)?;
        }
        Some(total)
    }

    /// Returns all entity references across all entity collections.
    pub fn all_entity_refs(&self, solution: &dyn Any) -> Vec<(usize, EntityRef)> {
        let mut refs = Vec::new();
        for (desc_idx, desc) in self.entity_descriptors.iter().enumerate() {
            for entity_ref in desc.entity_refs(solution) {
                refs.push((desc_idx, entity_ref));
            }
        }
        refs
    }

    /// Iterates over all entities in all collections.
    ///
    /// The callback receives:
    /// - The entity descriptor index
    /// - The entity index within its collection
    /// - A reference to the entity
    pub fn for_each_entity<F>(&self, solution: &dyn Any, mut f: F)
    where
        F: FnMut(usize, usize, &dyn Any),
    {
        for (desc_idx, desc) in self.entity_descriptors.iter().enumerate() {
            desc.for_each_entity(solution, |entity_idx, entity| {
                f(desc_idx, entity_idx, entity);
            });
        }
    }

    /// Gets an entity by descriptor index and entity index.
    pub fn get_entity<'a>(
        &self,
        solution: &'a dyn Any,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Option<&'a dyn Any> {
        self.entity_descriptors
            .get(descriptor_index)?
            .get_entity(solution, entity_index)
    }

    /// Gets a mutable entity by descriptor index and entity index.
    pub fn get_entity_mut<'a>(
        &self,
        solution: &'a mut dyn Any,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Option<&'a mut dyn Any> {
        self.entity_descriptors
            .get(descriptor_index)?
            .get_entity_mut(solution, entity_index)
    }

    /// Returns the number of entity descriptors.
    pub fn entity_descriptor_count(&self) -> usize {
        self.entity_descriptors.len()
    }

    /// Returns the number of problem fact descriptors.
    pub fn problem_fact_descriptor_count(&self) -> usize {
        self.problem_fact_descriptors.len()
    }

    /// Returns whether all entity descriptors have extractors configured.
    pub fn all_extractors_configured(&self) -> bool {
        self.entity_descriptors.iter().all(|d| d.has_extractor())
    }
}

impl Clone for SolutionDescriptor {
    fn clone(&self) -> Self {
        Self {
            type_name: self.type_name,
            type_id: self.type_id,
            entity_descriptors: self.entity_descriptors.clone(),
            problem_fact_descriptors: self.problem_fact_descriptors.clone(),
            score_field: self.score_field,
            score_is_optional: self.score_is_optional,
            entity_type_index: self.entity_type_index.clone(),
        }
    }
}

impl fmt::Debug for SolutionDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SolutionDescriptor")
            .field("type_name", &self.type_name)
            .field("entities", &self.entity_descriptors.len())
            .field("problem_facts", &self.problem_fact_descriptors.len())
            .finish()
    }
}
