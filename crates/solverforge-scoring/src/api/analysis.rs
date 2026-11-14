//! Score analysis types for detailed constraint tracking.
//!
//! This module provides types for analyzing constraint matches in detail:
//! each match's score and the entities that caused it.

use std::fmt::Debug;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

/// An entity involved in a constraint match, as constraint analysis reports
/// it: the entity's type name, its `Debug` rendering and, when its type has a
/// `#[planning_id]`, its planning id, all taken when the match is explained.
/// It holds no copy of the entity, so explaining a match requires only
/// `Debug` on the entity type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EntityRef {
    /// Type name of the entity (e.g., "Shift", "Employee").
    pub type_name: String,
    /// String representation for display. It is a rendering, not an
    /// identifier: look records up by [`planning_id`](Self::planning_id).
    pub display: String,
    /// The entity's planning id, rendered as a string: the id's `Display`
    /// output when its type implements `Display` (`7`, `shift-7`), otherwise
    /// its `Debug` output (`ShiftId(7)`). Equal ids render equally, so it is
    /// a lookup key for the record within its type.
    ///
    /// Set for entities streamed from a planning model collection whose type
    /// has a `#[planning_id]` field (`#[planning_entity]`, `#[problem_fact]`);
    /// `None` for every other entity, including plain values streamed through
    /// a hand-built source.
    pub planning_id: Option<String>,
}

impl EntityRef {
    /// Describes `entity` by its type name and `Debug` rendering, with no
    /// planning id.
    pub fn new<T: Debug + ?Sized>(entity: &T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            display: format!("{entity:?}"),
            planning_id: None,
        }
    }

    /// Sets the entity's rendered planning id (see
    /// [`planning_id`](Self::planning_id)).
    pub fn with_planning_id(mut self, planning_id: Option<String>) -> Self {
        self.planning_id = planning_id;
        self
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
            .field("planning_id", &self.planning_id)
            .finish()
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
