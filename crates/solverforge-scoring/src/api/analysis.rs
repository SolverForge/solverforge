//! Score analysis types for detailed constraint tracking.
//!
//! This module provides types for analyzing constraint matches in detail,
//! including which entities are involved in each match, score explanations,
//! and entity-level indictments.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

/// Reference to an entity involved in a constraint match.
///
/// Uses type erasure to allow storing references to different entity types
/// in a single collection.
#[derive(Clone)]
pub struct EntityRef {
    /// Type name of the entity (e.g., "Shift", "Employee").
    pub type_name: String,
    /// String representation for display.
    pub display: String,
    /// Type-erased entity for programmatic access.
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
pub struct DetailedConstraintMatch<Sc: Score> {
    /// Reference to the constraint that matched.
    pub constraint_ref: ConstraintRef,
    /// Score impact of this match.
    pub score: Sc,
    /// Justification with involved entities.
    pub justification: ConstraintJustification,
}

impl<Sc: Score> DetailedConstraintMatch<Sc> {
    /// Creates a new detailed constraint match.
    pub fn new(
        constraint_ref: ConstraintRef,
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

/// Extended constraint evaluation with detailed match information.
#[derive(Debug, Clone)]
pub struct DetailedConstraintEvaluation<Sc: Score> {
    /// Total score impact from all matches.
    pub total_score: Sc,
    /// Number of matches found.
    pub match_count: usize,
    /// Detailed information for each match.
    pub matches: Vec<DetailedConstraintMatch<Sc>>,
}

impl<Sc: Score> DetailedConstraintEvaluation<Sc> {
    /// Creates a new detailed evaluation.
    pub fn new(total_score: Sc, matches: Vec<DetailedConstraintMatch<Sc>>) -> Self {
        let match_count = matches.len();
        Self {
            total_score,
            match_count,
            matches,
        }
    }

    /// Creates an empty evaluation (no matches).
    pub fn empty() -> Self {
        Self {
            total_score: Sc::zero(),
            match_count: 0,
            matches: Vec::new(),
        }
    }
}

/// Per-constraint breakdown in a score explanation.
#[derive(Debug, Clone)]
pub struct ConstraintAnalysis<Sc: Score> {
    /// Constraint reference.
    pub constraint_ref: ConstraintRef,
    /// Constraint weight (score per match).
    pub weight: Sc,
    /// Total score from this constraint.
    pub score: Sc,
    /// All matches for this constraint.
    pub matches: Vec<DetailedConstraintMatch<Sc>>,
    /// Whether this is a hard constraint.
    pub is_hard: bool,
}

impl<Sc: Score> ConstraintAnalysis<Sc> {
    /// Creates a new constraint analysis.
    pub fn new(
        constraint_ref: ConstraintRef,
        weight: Sc,
        score: Sc,
        matches: Vec<DetailedConstraintMatch<Sc>>,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            weight,
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

/// Complete score explanation with per-constraint breakdown.
#[derive(Debug, Clone)]
pub struct ScoreExplanation<Sc: Score> {
    /// The total score.
    pub score: Sc,
    /// Per-constraint breakdown.
    pub constraint_analyses: Vec<ConstraintAnalysis<Sc>>,
}

impl<Sc: Score> ScoreExplanation<Sc> {
    /// Creates a new score explanation.
    pub fn new(score: Sc, constraint_analyses: Vec<ConstraintAnalysis<Sc>>) -> Self {
        Self {
            score,
            constraint_analyses,
        }
    }

    /// Returns the total match count across all constraints.
    pub fn total_match_count(&self) -> usize {
        self.constraint_analyses.iter().map(|a| a.match_count()).sum()
    }

    /// Returns constraints with non-zero scores.
    pub fn non_zero_constraints(&self) -> Vec<&ConstraintAnalysis<Sc>> {
        self.constraint_analyses
            .iter()
            .filter(|a| a.score != Sc::zero())
            .collect()
    }

    /// Returns all detailed matches across all constraints.
    pub fn all_matches(&self) -> Vec<&DetailedConstraintMatch<Sc>> {
        self.constraint_analyses
            .iter()
            .flat_map(|a| &a.matches)
            .collect()
    }
}

/// Analysis of how a single entity impacts the score.
#[derive(Debug, Clone)]
pub struct Indictment<Sc: Score> {
    /// The entity being analyzed.
    pub entity: EntityRef,
    /// Total score impact from this entity.
    pub score: Sc,
    /// Matches involving this entity, grouped by constraint.
    pub constraint_matches: HashMap<ConstraintRef, Vec<DetailedConstraintMatch<Sc>>>,
}

impl<Sc: Score> Indictment<Sc> {
    /// Creates a new indictment for an entity.
    pub fn new(entity: EntityRef) -> Self {
        Self {
            entity,
            score: Sc::zero(),
            constraint_matches: HashMap::new(),
        }
    }

    /// Adds a match to this indictment.
    pub fn add_match(&mut self, constraint_match: DetailedConstraintMatch<Sc>) {
        self.score = self.score.clone() + constraint_match.score.clone();
        self.constraint_matches
            .entry(constraint_match.constraint_ref.clone())
            .or_default()
            .push(constraint_match);
    }

    /// Returns the total number of constraint violations.
    pub fn match_count(&self) -> usize {
        self.constraint_matches.values().map(|v| v.len()).sum::<usize>()
    }

    /// Returns the constraint refs for all violated constraints.
    pub fn violated_constraints(&self) -> Vec<&ConstraintRef> {
        self.constraint_matches.keys().collect()
    }

    /// Returns the number of distinct constraints violated.
    pub fn constraint_count(&self) -> usize {
        self.constraint_matches.len()
    }
}

/// Map of entity indictments for analyzing which entities cause violations.
#[derive(Debug, Clone)]
pub struct IndictmentMap<Sc: Score> {
    /// Indictments keyed by entity reference.
    pub indictments: HashMap<EntityRef, Indictment<Sc>>,
}

impl<Sc: Score> IndictmentMap<Sc> {
    /// Creates an empty indictment map.
    pub fn new() -> Self {
        Self {
            indictments: HashMap::new(),
        }
    }

    /// Builds an indictment map from a collection of detailed matches.
    pub fn from_matches(matches: Vec<DetailedConstraintMatch<Sc>>) -> Self {
        let mut map = Self::new();
        for m in matches {
            for entity in &m.justification.entities {
                map.indictments
                    .entry(entity.clone())
                    .or_insert_with(|| Indictment::new(entity.clone()))
                    .add_match(m.clone());
            }
        }
        map
    }

    /// Gets the indictment for a specific entity.
    pub fn get(&self, entity: &EntityRef) -> Option<&Indictment<Sc>> {
        self.indictments.get(entity)
    }

    /// Returns all indicted entities.
    pub fn entities(&self) -> impl Iterator<Item = &EntityRef> {
        self.indictments.keys()
    }

    /// Returns entities sorted by worst score impact (most negative first).
    pub fn worst_entities(&self) -> Vec<&EntityRef> {
        let mut entities: Vec<_> = self.indictments.keys().collect();
        entities.sort_by(|a, b| {
            let score_a = &self.indictments[*a].score;
            let score_b = &self.indictments[*b].score;
            score_a.cmp(score_b)
        });
        entities
    }

    /// Returns the number of indicted entities.
    pub fn len(&self) -> usize {
        self.indictments.len()
    }

    /// Returns true if no entities are indicted.
    pub fn is_empty(&self) -> bool {
        self.indictments.is_empty()
    }
}

impl<Sc: Score> Default for IndictmentMap<Sc> {
    fn default() -> Self {
        Self::new()
    }
}
