//! Score analysis types for detailed constraint tracking.
//!
//! This module provides types for analyzing constraint matches in detail,
//! including which entities are involved in each match, score explanations,
//! and entity-level indictments.

use std::any::Any;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

/// Reference to an entity involved in a constraint match.
/// Uses type erasure to allow storing references to different entity types
/// in a single collection.
#[derive(Clone)]
pub struct EntityRef {
    /// Type name of the entity (e.g., "Shift", "Employee").
    pub type_name: String,
    /// String representation for display.
    pub display: String,
    // Type-erased entity for programmatic access.
    entity: Arc<dyn Any + Send + Sync>,
}

impl EntityRef {
    /// Creates a new entity reference from a concrete entity.
    pub fn new<T: Clone + Debug + Send + Sync + 'static>(entity: &T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            display: format!("{:?}", entity),
            entity: Arc::new(entity.clone()),
        }
    }

    /// Creates an entity reference with a custom display string.
    pub fn with_display<T: Clone + Send + Sync + 'static>(entity: &T, display: String) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            display,
            entity: Arc::new(entity.clone()),
        }
    }

    /// Attempts to downcast to the concrete entity type.
    pub fn as_entity<T: 'static>(&self) -> Option<&T> {
        self.entity.downcast_ref::<T>()
    }

    /// Returns the short type name (without module path).
    pub fn short_type_name(&self) -> &str {
        self.type_name
            .rsplit("::")
            .next()
            .unwrap_or(&self.type_name)
    }
}

impl Debug for EntityRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityRef")
            .field("type", &self.short_type_name())
            .field("display", &self.display)
            .finish()
    }
}

impl PartialEq for EntityRef {
    fn eq(&self, other: &Self) -> bool {
        self.type_name == other.type_name && self.display == other.display
    }
}

impl Eq for EntityRef {}

impl Hash for EntityRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_name.hash(state);
        self.display.hash(state);
    }
}

/// Justification for why a constraint matched.
#[derive(Debug, Clone)]
pub struct ConstraintJustification {
    /// Entities involved in the match.
    pub entities: Vec<EntityRef>,
    /// Human-readable description of why the constraint matched.
    pub description: String,
}

impl ConstraintJustification {
    /// Creates a justification from entities, auto-generating description.
    pub fn new(entities: Vec<EntityRef>) -> Self {
        let description = if entities.is_empty() {
            "No entities".to_string()
        } else {
            entities
                .iter()
                .map(|e| e.display.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        Self {
            entities,
            description,
        }
    }

    /// Creates a justification with a custom description.
    pub fn with_description(entities: Vec<EntityRef>, description: String) -> Self {
        Self {
            entities,
            description,
        }
    }
}

/// A detailed constraint match with entity information.
#[derive(Debug, Clone)]
pub struct DetailedConstraintMatch<'a, Sc: Score> {
    /// Reference to the constraint that matched.
    pub constraint_ref: &'a ConstraintRef,
    /// Score impact of this match.
    pub score: Sc,
    /// Justification with involved entities.
    pub justification: ConstraintJustification,
}

impl<'a, Sc: Score> DetailedConstraintMatch<'a, Sc> {
    /// Creates a new detailed constraint match.
    pub fn new(
        constraint_ref: &'a ConstraintRef,
        score: Sc,
        justification: ConstraintJustification,
    ) -> Self {
        Self {
            constraint_ref,
            score,
            justification,
        }
    }
}

/// Per-constraint breakdown in a score explanation.
#[derive(Debug, Clone)]
pub struct ConstraintAnalysis<'a, Sc: Score> {
    /// Constraint reference.
    pub constraint_ref: &'a ConstraintRef,
    /// Bound runtime weight multiplier; `0` means the constraint is disabled.
    pub weight_multiplier: u32,
    /// Total (weighted) score from this constraint.
    pub score: Sc,
    /// All matches for this constraint.
    pub matches: Vec<DetailedConstraintMatch<'a, Sc>>,
    /// Whether this is a hard constraint.
    pub is_hard: bool,
}

impl<'a, Sc: Score> ConstraintAnalysis<'a, Sc> {
    /// Creates a new constraint analysis.
    pub fn new(
        constraint_ref: &'a ConstraintRef,
        weight_multiplier: u32,
        score: Sc,
        matches: Vec<DetailedConstraintMatch<'a, Sc>>,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            weight_multiplier,
            score,
            matches,
            is_hard,
        }
    }

    /// Returns the number of matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Returns the constraint name.
    pub fn name(&self) -> &str {
        &self.constraint_ref.name
    }
}
