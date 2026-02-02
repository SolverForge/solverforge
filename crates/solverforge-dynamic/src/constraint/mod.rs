//! Dynamic constraint system using expression trees with true incremental scoring.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use solverforge_core::score::{HardSoftScore, Score};
use solverforge_core::ConstraintRef;
use solverforge_scoring::api::analysis::DetailedConstraintMatch;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;

use crate::descriptor::DynamicDescriptor;
use crate::eval::{eval_expr, EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Match tuple type: (class_a, idx_a, class_b, idx_b) - indices only, no cloning.
type MatchTuple = (usize, usize, usize, usize);

// =============================================================================
// Type aliases for boxed closures used with monomorphized constraints
// =============================================================================

/// Extractor: retrieves entity slice from solution.
pub type DynExtractor =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Key extractor: extracts join key from entity.
pub type DynKeyExtractor =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Bi-constraint filter: checks if pair of entities matches.
pub type DynBiFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Tri-constraint filter: checks if triple of entities matches.
pub type DynTriFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Quad-constraint filter: checks if quadruple of entities matches.
pub type DynQuadFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Penta-constraint filter: checks if quintuple of entities matches.
pub type DynPentaFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Bi-constraint weight: computes score for pair.
pub type DynBiWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Tri-constraint weight: computes score for triple.
pub type DynTriWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Quad-constraint weight: computes score for quadruple.
pub type DynQuadWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Penta-constraint weight: computes score for quintuple.
pub type DynPentaWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

// Cross-join constraint closures (for joining two different entity classes)

/// Cross-join extractor A: extracts first entity class slice from solution.
pub type DynCrossExtractorA =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join extractor B: extracts second entity class slice from solution.
pub type DynCrossExtractorB =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join key extractor A: extracts join key from entity of class A.
pub type DynCrossKeyA =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join key extractor B: extracts join key from entity of class B.
pub type DynCrossKeyB =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join filter: checks if pair of entities from different classes matches.
pub type DynCrossFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Cross-join weight: computes score for cross-join pair.
pub type DynCrossWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

// Flattened constraint closures (for constraints that expand entities into collections)

/// Flatten function: expands entity B into a slice of C items.
pub type DynFlatten =
    Box<dyn Fn(&DynamicEntity) -> &[DynamicValue] + Send + Sync>;

/// C key function: extracts index key from flattened item C.
pub type DynCKeyFn =
    Box<dyn Fn(&DynamicValue) -> DynamicValue + Send + Sync>;

/// A lookup function: extracts lookup key from entity A for O(1) index access.
pub type DynALookup =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Flattened filter: checks if pair of (A entity, C item) matches.
pub type DynFlattenedFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicValue) -> bool + Send + Sync>;

/// Flattened weight: computes score for (A entity, C item) pair.
pub type DynFlattenedWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicValue) -> HardSoftScore + Send + Sync>;

// =============================================================================
// Closure builder functions for self-joins
// =============================================================================

/// Creates an extractor that retrieves the entity slice for a specific class.
///
/// # Arguments
/// * `class_idx` - The entity class index to extract from the solution
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` and returns a slice of entities
/// for the specified class.
pub fn make_extractor(class_idx: usize) -> DynExtractor {
    Box::new(move |solution: &DynamicSolution| {
        solution.entities.get(class_idx)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    })
}

/// Creates a key extractor that evaluates an expression against an entity to extract a join key.
///
/// # Arguments
/// * `key_expr` - The expression to evaluate against each entity to produce the join key
/// * `descriptor` - The schema descriptor, cloned into the closure for minimal solution context
///
/// # Returns
/// A boxed closure that takes a `DynamicEntity` reference and returns a `DynamicValue`
/// representing the join key extracted from that entity.
///
/// # Notes
/// The returned closure uses `eval_entity_expr` to evaluate the expression in a single-entity
/// context where `Param(0)` refers to the entity itself.
///
/// **Important**: Join key expressions should only reference entity fields (`Param(0)` and
/// `Field { param_idx: 0, ... }`). References to facts or other solution state will not work
/// correctly because the closure only has access to the entity and a minimal solution context.
/// This is an intentional design constraint - join keys should be stable entity attributes.
pub fn make_key_extractor(key_expr: Expr, descriptor: DynamicDescriptor) -> DynKeyExtractor {
    // Create a minimal solution context with only the descriptor, cloned once into the closure.
    // This is sufficient for entity field access, which is all that join keys should need.
    // Fact lookups and other solution-dependent operations will not work in this context.
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
    };

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&key_expr, &minimal_solution, entity)
    })
}

/// Creates a bi-constraint filter that evaluates an expression against a pair of entities.
///
/// # Arguments
/// * `filter_expr` - The expression to evaluate against the entity pair (returns bool)
/// * `class_idx` - The entity class index (for self-join constraints, both entities are from this class)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and two `DynamicEntity` references,
/// evaluates the filter expression in a bi-entity context, and returns whether the pair matches.
///
/// # Expression Context
/// The filter expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
/// - The full solution is available for fact lookups and other operations
///
/// The expression should return a boolean value. Non-boolean results are treated as `false`.
///
/// # Implementation Note
/// The filter searches the entity slice to find entity indices, which is O(n) per call.
/// This is acceptable because filtering is done on already-matched entities (by join key),
/// not on the full entity set.
pub fn make_bi_filter(filter_expr: Expr, class_idx: usize) -> DynBiFilter {
    Box::new(move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
        // Find entity indices by searching the entity slice.
        // For self-join constraints, both entities are from class_idx.
        // We search by entity ID which is unique.
        let entities = solution.entities.get(class_idx).map(|v| v.as_slice()).unwrap_or(&[]);

        let a_idx = entities.iter().position(|e| e.id == a.id);
        let b_idx = entities.iter().position(|e| e.id == b.id);

        let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) else {
            // Entities not found in solution - shouldn't happen, but return false defensively
            return false;
        };

        // Build entity tuple for evaluation context
        let tuple = vec![
            EntityRef::new(class_idx, a_idx),
            EntityRef::new(class_idx, b_idx),
        ];

        // Evaluate expression in bi-entity context
        let ctx = EvalContext::new(solution, &tuple);
        let result = eval_expr(&filter_expr, &ctx);

        // Convert result to bool (default to false if not a bool)
        result.as_bool().unwrap_or(false)
    })
}

/// Creates a bi-constraint weight function that evaluates an expression against a pair of entities.
///
/// # Arguments
/// * `weight_expr` - The expression to evaluate against the entity pair (returns numeric weight)
/// * `class_idx` - The entity class index (both entities are from this class for self-join)
/// * `descriptor` - The schema descriptor for building evaluation context
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes two `DynamicEntity` references, evaluates the weight expression
/// in a bi-entity context, and returns a `HardSoftScore`.
///
/// # Expression Evaluation
/// The weight expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
///
/// The expression should return a numeric value (i64). Non-numeric results default to 0.
///
/// **Design constraint**: Weight expressions should only reference entity fields and perform
/// arithmetic/comparisons. References to facts or other solution state will NOT work correctly
/// because the evaluation uses a temporary solution context with only the two entities.
///
/// # Weight Application
/// The resulting numeric value is applied to either the hard or soft score component based on
/// the `is_hard` parameter. The constraint's impact type (penalty vs reward) is NOT applied
/// here - that's handled by the monomorphized constraint wrapper's `compute_score` method.
pub fn make_bi_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynBiWeight {
    Box::new(move |a: &DynamicEntity, b: &DynamicEntity| {
        // Create a temporary solution context with just these two entities
        // This allows us to use EvalContext with proper entity indices
        let mut temp_solution = DynamicSolution {
            descriptor: descriptor.clone(),
            entities: Vec::new(),
            facts: Vec::new(),
            score: None,
        };

        // Ensure the entities vec is large enough
        while temp_solution.entities.len() <= class_idx {
            temp_solution.entities.push(Vec::new());
        }

        // Add the two entities at indices 0 and 1
        temp_solution.entities[class_idx] = vec![a.clone(), b.clone()];

        // Build entity tuple for evaluation context (indices 0 and 1)
        let tuple = vec![
            EntityRef::new(class_idx, 0),
            EntityRef::new(class_idx, 1),
        ];

        // Evaluate expression in bi-entity context
        let ctx = EvalContext::new(&temp_solution, &tuple);
        let result = eval_expr(&weight_expr, &ctx);

        // Convert to numeric value (default to 0 if not numeric)
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft component
        if is_hard {
            HardSoftScore::hard(weight_num)
        } else {
            HardSoftScore::soft(weight_num)
        }
    })
}

/// Creates a tri-entity filter closure from a filter expression.
///
/// Returns a boxed closure that evaluates the filter expression against three entities
/// from the same class (self-join with three-way matching).
///
/// # Parameters
/// - `filter_expr`: Expression to evaluate (should return bool)
/// - `class_idx`: Entity class index (all three entities must be from this class)
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
/// - The full solution is available for fact lookups
///
/// # Implementation
/// Searches the entity slice by entity ID to find indices (O(n) per entity).
/// This is acceptable because filtering is performed only on entities already matched
/// by join key, not on the full entity set.
pub fn make_tri_filter(filter_expr: Expr, class_idx: usize) -> DynTriFilter {
    Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity| {
            // Find entity indices by searching the entity slice using entity IDs.
            let entities = solution
                .entities
                .get(class_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let a_idx = entities.iter().position(|e| e.id == a.id);
            let b_idx = entities.iter().position(|e| e.id == b.id);
            let c_idx = entities.iter().position(|e| e.id == c.id);

            if a_idx.is_none() || b_idx.is_none() || c_idx.is_none() {
                return false;
            }

            // Build EntityRef tuple: all three entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
            ];

            let ctx = EvalContext::new(solution, &tuple);
            crate::eval::eval_expr(&filter_expr, &ctx)
                .as_bool()
                .unwrap_or(false)
        },
    )
}

/// Creates a tri-entity weight closure from a weight expression.
///
/// Returns a boxed closure that evaluates the weight expression against three entities
/// and returns a `HardSoftScore`.
///
/// # Parameters
/// - `weight_expr`: Expression to evaluate (should return numeric value)
/// - `class_idx`: Entity class index (all three entities must be from this class)
/// - `descriptor`: Problem descriptor for creating temporary solution context
/// - `is_hard`: If true, weight is applied to hard score; otherwise soft score
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
/// - Arithmetic and comparison operations work across all three entities
///
/// # Implementation
/// Creates a temporary `DynamicSolution` with all three entities for proper evaluation context.
/// This enables full tri-entity expression evaluation via `EvalContext`.
///
/// Note: This approach clones entities into a temporary solution. While this violates the
/// zero-clone principle, it's necessary because the `DynTriWeight` signature doesn't provide
/// access to the solution or entity indices. The clone happens only for matched triples
/// (bounded by match count, not total entity count).
pub fn make_tri_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynTriWeight {
    Box::new(
        move |a: &DynamicEntity, b: &DynamicEntity, c: &DynamicEntity| {
            // Create a temporary solution with the descriptor and the three entities.
            let mut temp_solution = DynamicSolution {
                descriptor: descriptor.clone(),
                entities: vec![Vec::new(); descriptor.entity_classes.len()],
                facts: Vec::new(),
                score: None,
            };

            // Place all three entities at indices 0, 1, 2 in the class entity slice.
            temp_solution.entities[class_idx] = vec![a.clone(), b.clone(), c.clone()];

            // Build EntityRef tuple: all three entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, 0),
                EntityRef::new(class_idx, 1),
                EntityRef::new(class_idx, 2),
            ];

            let ctx = EvalContext::new(&temp_solution, &tuple);
            let result = crate::eval::eval_expr(&weight_expr, &ctx);

            // Convert result to numeric value and apply to hard or soft score.
            let weight_num = result.as_i64().unwrap_or(0) as f64;
            if is_hard {
                HardSoftScore::hard(weight_num)
            } else {
                HardSoftScore::soft(weight_num)
            }
        },
    )
}

/// Operations in a constraint stream pipeline.
#[derive(Debug, Clone)]
pub enum StreamOp {
    /// Iterate over all entities of a class.
    ForEach { class_idx: usize },

    /// Filter entities using a predicate expression.
    Filter { predicate: Expr },

    /// Join with another class using join conditions.
    Join {
        class_idx: usize,
        /// Join conditions that must all be true.
        conditions: Vec<Expr>,
    },

    /// Filter distinct pairs (ensuring A < B to avoid duplicates).
    DistinctPair {
        /// Expression to compare (e.g., entity IDs or indices).
        ordering_expr: Expr,
    },

    /// Penalize matching tuples.
    Penalize { weight: HardSoftScore },

    /// Penalize with a configurable amount based on expression.
    PenalizeConfigurable { match_weight: Expr },

    /// Reward matching tuples.
    Reward { weight: HardSoftScore },

    /// Reward with a configurable amount based on expression.
    RewardConfigurable { match_weight: Expr },

    /// Flatten a set/list field, creating one tuple per element.
    FlattenLast {
        /// Expression to get the set/list to flatten.
        set_expr: Expr,
    },
}

/// A constraint defined using expression trees and stream operations.
///
/// Supports true incremental scoring: on_insert/on_retract compute deltas
/// by tracking active matches and updating only affected tuples.
#[derive(Debug)]
pub struct DynamicConstraint {
    /// Constraint name.
    pub name: Arc<str>,
    /// Base weight (for simple penalize/reward).
    pub weight: HardSoftScore,
    /// Stream operations pipeline.
    pub ops: Vec<StreamOp>,
    /// Whether this is a hard constraint.
    pub is_hard: bool,

    // Incremental state - indices only, no cloning
    /// Active matches: set of (class_a, idx_a, class_b, idx_b) tuples.
    matches: HashSet<MatchTuple>,
    /// Reverse index: (class_idx, entity_idx) -> matches involving this entity.
    entity_to_matches: HashMap<(usize, usize), Vec<MatchTuple>>,
    /// Join key index: join_key_value -> list of (class_idx, entity_idx) with that value.
    /// Used for O(1) lookup on insert instead of O(n) scan.
    join_key_index: HashMap<i64, Vec<(usize, usize)>>,
    /// Cached score from all current matches.
    cached_score: HardSoftScore,
    /// Whether initialized.
    initialized: bool,
    /// Cached distinct_pair expression for on_insert.
    distinct_expr: Option<Expr>,
    /// Cached join conditions for on_insert.
    join_conditions: Vec<Expr>,
    /// Cached filter predicates for on_insert.
    filter_predicates: Vec<Expr>,
    /// Cached foreach class index.
    foreach_class: Option<usize>,
    /// Cached join class index.
    join_class: Option<usize>,
}

impl Clone for DynamicConstraint {
    fn clone(&self) -> Self {
        // Clone resets incremental state - will be reinitialized on first use
        Self {
            name: self.name.clone(),
            weight: self.weight,
            ops: self.ops.clone(),
            is_hard: self.is_hard,
            matches: HashSet::new(),
            entity_to_matches: HashMap::new(),
            join_key_index: HashMap::new(),
            cached_score: HardSoftScore::ZERO,
            initialized: false,
            distinct_expr: None,
            join_conditions: Vec::new(),
            filter_predicates: Vec::new(),
            foreach_class: None,
            join_class: None,
        }
    }
}

impl DynamicConstraint {
    /// Creates a new dynamic constraint.
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            weight: HardSoftScore::ZERO,
            ops: Vec::new(),
            is_hard: false,
            matches: HashSet::new(),
            entity_to_matches: HashMap::new(),
            join_key_index: HashMap::new(),
            cached_score: HardSoftScore::ZERO,
            initialized: false,
            distinct_expr: None,
            join_conditions: Vec::new(),
            filter_predicates: Vec::new(),
            foreach_class: None,
            join_class: None,
        }
    }

    /// Sets the constraint weight.
    pub fn with_weight(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self
    }

    /// Adds a ForEach operation.
    pub fn for_each(mut self, class_idx: usize) -> Self {
        self.foreach_class = Some(class_idx);
        self.ops.push(StreamOp::ForEach { class_idx });
        self
    }

    /// Adds a Filter operation.
    pub fn filter(mut self, predicate: Expr) -> Self {
        self.filter_predicates.push(predicate.clone());
        self.ops.push(StreamOp::Filter { predicate });
        self
    }

    /// Adds a Join operation.
    pub fn join(mut self, class_idx: usize, conditions: Vec<Expr>) -> Self {
        self.join_class = Some(class_idx);
        self.join_conditions = conditions.clone();
        self.ops.push(StreamOp::Join {
            class_idx,
            conditions,
        });
        self
    }

    /// Adds a DistinctPair filter to avoid duplicate pairs (A,B) and (B,A).
    pub fn distinct_pair(mut self, ordering_expr: Expr) -> Self {
        self.distinct_expr = Some(ordering_expr.clone());
        self.ops.push(StreamOp::DistinctPair { ordering_expr });
        self
    }

    /// Adds a Penalize operation.
    pub fn penalize(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self.ops.push(StreamOp::Penalize { weight });
        self
    }

    /// Adds a Reward operation.
    pub fn reward(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self.ops.push(StreamOp::Reward { weight });
        self
    }

    /// Adds a FlattenLast operation to expand a set/list into individual tuples.
    pub fn flatten_last(mut self, set_expr: Expr) -> Self {
        self.ops.push(StreamOp::FlattenLast { set_expr });
        self
    }

    /// Adds a PenalizeConfigurable operation with dynamic weight.
    pub fn penalize_configurable(mut self, base_weight: HardSoftScore, match_weight: Expr) -> Self {
        self.weight = base_weight;
        self.is_hard = base_weight.hard() != 0;
        self.ops
            .push(StreamOp::PenalizeConfigurable { match_weight });
        self
    }

    /// Adds a RewardConfigurable operation with dynamic weight.
    pub fn reward_configurable(mut self, base_weight: HardSoftScore, match_weight: Expr) -> Self {
        self.weight = base_weight;
        self.is_hard = base_weight.hard() != 0;
        self.ops.push(StreamOp::RewardConfigurable { match_weight });
        self
    }

    /// Returns the cached score. Must call initialize() first.
    pub fn cached_score(&self) -> HardSoftScore {
        self.cached_score
    }

    /// Returns the match count from incremental state.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    // =========================================================================
    // Incremental scoring helpers
    // =========================================================================

    /// Finds all matching index tuples (for bi-joins only currently).
    /// Returns (class_a, idx_a, class_b, idx_b) tuples.
    /// Called once during initialize() - O(n^2) is acceptable here.
    fn find_match_indices(&self, solution: &DynamicSolution) -> HashSet<MatchTuple> {
        let mut result = HashSet::new();

        // Parse ops to find the constraint structure
        let mut foreach_class = None;
        let mut join_class = None;
        let mut join_conditions = Vec::new();
        let mut distinct_expr = None;
        let mut filter_predicates = Vec::new();

        for op in &self.ops {
            match op {
                StreamOp::ForEach { class_idx } => {
                    foreach_class = Some(*class_idx);
                }
                StreamOp::Join {
                    class_idx,
                    conditions,
                } => {
                    join_class = Some(*class_idx);
                    join_conditions = conditions.clone();
                }
                StreamOp::DistinctPair { ordering_expr } => {
                    distinct_expr = Some(ordering_expr.clone());
                }
                StreamOp::Filter { predicate } => {
                    filter_predicates.push(predicate.clone());
                }
                _ => {}
            }
        }

        let Some(class_a) = foreach_class else {
            return result;
        };
        let Some(class_b) = join_class else {
            return result;
        };

        // Iterate all pairs (A, B) - O(n^2) but only called once at init
        for (_, a_idx) in solution.entity_refs_in_class(class_a) {
            for (_, b_idx) in solution.entity_refs_in_class(class_b) {
                if self.check_join_match(
                    solution,
                    class_a,
                    a_idx,
                    class_b,
                    b_idx,
                    &join_conditions,
                    &distinct_expr,
                    &filter_predicates,
                ) {
                    result.insert((class_a, a_idx, class_b, b_idx));
                }
            }
        }

        result
    }

    /// Gets the join key value for an entity (extracts the field used in equality join).
    fn get_join_key(
        &self,
        solution: &DynamicSolution,
        class_idx: usize,
        entity_idx: usize,
    ) -> Option<i64> {
        // Parse ops to find join condition field
        for op in &self.ops {
            if let StreamOp::Join { conditions, .. } = op {
                // Look for equality condition like A.field == B.field
                for cond in conditions {
                    if let Expr::Eq(left, right) = cond {
                        // Verify both sides are field references
                        if let (
                            Expr::Field {
                                param_idx: 0,
                                field_idx: left_field,
                            },
                            Expr::Field {
                                param_idx: 1,
                                field_idx: right_field,
                            },
                        ) = (left.as_ref(), right.as_ref())
                        {
                            if left_field == right_field {
                                let entity = solution.get_entity(class_idx, entity_idx)?;
                                return entity.fields.get(*left_field)?.as_i64();
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Checks if a pair of entities matches the join conditions and filter predicates.
    fn check_join_match(
        &self,
        solution: &DynamicSolution,
        class_a: usize,
        idx_a: usize,
        class_b: usize,
        idx_b: usize,
        conditions: &[Expr],
        distinct_expr: &Option<Expr>,
        filter_predicates: &[Expr],
    ) -> bool {
        let tuple = vec![
            EntityRef::new(class_a, idx_a),
            EntityRef::new(class_b, idx_b),
        ];
        let ctx = EvalContext::new(solution, &tuple);

        // Check all join conditions
        let conditions_match = conditions
            .iter()
            .all(|cond| eval_expr(cond, &ctx).as_bool().unwrap_or(false));

        if !conditions_match {
            return false;
        }

        // Check distinct pair if present
        if let Some(ordering) = distinct_expr {
            if !eval_expr(ordering, &ctx).as_bool().unwrap_or(false) {
                return false;
            }
        }

        // Check all filter predicates
        for pred in filter_predicates {
            if !eval_expr(pred, &ctx).as_bool().unwrap_or(false) {
                return false;
            }
        }

        true
    }

    /// Computes score for a match by looking up entities (no clone).
    fn score_for_match(&self, solution: &DynamicSolution, m: MatchTuple) -> HardSoftScore {
        let (ca, ia, cb, ib) = m;
        let tuple = vec![EntityRef::new(ca, ia), EntityRef::new(cb, ib)];

        // Find terminal op and compute score
        for op in &self.ops {
            match op {
                StreamOp::Penalize { weight } => {
                    return -*weight;
                }
                StreamOp::PenalizeConfigurable { match_weight } => {
                    let ctx = EvalContext::new(solution, &tuple);
                    let weight_val = eval_expr(match_weight, &ctx);
                    if let Some(w) = weight_val.as_i64() {
                        return -self.weight.multiply(w as f64);
                    }
                }
                StreamOp::Reward { weight } => {
                    return *weight;
                }
                StreamOp::RewardConfigurable { match_weight } => {
                    let ctx = EvalContext::new(solution, &tuple);
                    let weight_val = eval_expr(match_weight, &ctx);
                    if let Some(w) = weight_val.as_i64() {
                        return self.weight.multiply(w as f64);
                    }
                }
                _ => {}
            }
        }

        HardSoftScore::ZERO
    }
}

/// Implement IncrementalConstraint for individual DynamicConstraint.
impl IncrementalConstraint<DynamicSolution, HardSoftScore> for DynamicConstraint {
    fn evaluate(&self, _solution: &DynamicSolution) -> HardSoftScore {
        // Use cached score from incremental state
        self.cached_score
    }

    fn match_count(&self, _solution: &DynamicSolution) -> usize {
        // Use cached match count from incremental state
        self.matches.len()
    }

    fn initialize(&mut self, solution: &DynamicSolution) -> HardSoftScore {
        self.matches.clear();
        self.entity_to_matches.clear();
        self.join_key_index.clear();

        // Find foreach class
        let mut foreach_class = None;
        for op in &self.ops {
            if let StreamOp::ForEach { class_idx } = op {
                foreach_class = Some(*class_idx);
                break;
            }
        }

        // Find all matching index pairs
        let all_matches = self.find_match_indices(solution);

        // Build join key index for O(1) lookup on insert
        if let Some(class_idx) = foreach_class {
            for (_, entity_idx) in solution.entity_refs_in_class(class_idx) {
                if let Some(key_val) = self.get_join_key(solution, class_idx, entity_idx) {
                    self.join_key_index
                        .entry(key_val)
                        .or_default()
                        .push((class_idx, entity_idx));
                }
            }
        }

        let mut total = HardSoftScore::ZERO;
        for m in &all_matches {
            let (ca, ia, cb, ib) = *m;

            // Add to reverse index
            self.entity_to_matches.entry((ca, ia)).or_default().push(*m);
            self.entity_to_matches.entry((cb, ib)).or_default().push(*m);

            // Compute score by index lookup (no clone)
            total = total + self.score_for_match(solution, *m);
        }

        self.matches = all_matches;
        self.cached_score = total;
        self.initialized = true;
        total
    }

    fn on_insert(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if !self.initialized {
            return HardSoftScore::ZERO;
        }

        // Get candidate partners to check
        let others: Vec<(usize, usize)> =
            if let Some(key_val) = self.get_join_key(solution, descriptor_index, entity_index) {
                // O(1) lookup using join key index
                let candidates = self
                    .join_key_index
                    .get(&key_val)
                    .cloned()
                    .unwrap_or_default();

                // Add to join key index
                self.join_key_index
                    .entry(key_val)
                    .or_default()
                    .push((descriptor_index, entity_index));

                candidates
            } else {
                // Complex join condition - O(n) scan of join class
                let Some(join_class) = self.join_class else {
                    return HardSoftScore::ZERO;
                };
                solution
                    .entity_refs_in_class(join_class)
                    .map(|(_, idx)| (join_class, idx))
                    .filter(|&(c, i)| !(c == descriptor_index && i == entity_index))
                    .collect()
            };

        let mut delta = HardSoftScore::ZERO;
        for (other_class, other_idx) in others {
            // Check full join match with correct ordering for distinct_pair
            if self.check_join_match(
                solution,
                descriptor_index,
                entity_index,
                other_class,
                other_idx,
                &self.join_conditions,
                &self.distinct_expr,
                &self.filter_predicates,
            ) {
                let m = (descriptor_index, entity_index, other_class, other_idx);
                if self.matches.insert(m) {
                    self.entity_to_matches
                        .entry((descriptor_index, entity_index))
                        .or_default()
                        .push(m);
                    self.entity_to_matches
                        .entry((other_class, other_idx))
                        .or_default()
                        .push(m);
                    delta = delta + self.score_for_match(solution, m);
                }
            } else if self.check_join_match(
                solution,
                other_class,
                other_idx,
                descriptor_index,
                entity_index,
                &self.join_conditions,
                &self.distinct_expr,
                &self.filter_predicates,
            ) {
                let m = (other_class, other_idx, descriptor_index, entity_index);
                if self.matches.insert(m) {
                    self.entity_to_matches
                        .entry((other_class, other_idx))
                        .or_default()
                        .push(m);
                    self.entity_to_matches
                        .entry((descriptor_index, entity_index))
                        .or_default()
                        .push(m);
                    delta = delta + self.score_for_match(solution, m);
                }
            }
        }

        self.cached_score = self.cached_score + delta;
        delta
    }

    fn on_retract(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if !self.initialized {
            return HardSoftScore::ZERO;
        }

        // Remove from join key index
        if let Some(key_val) = self.get_join_key(solution, descriptor_index, entity_index) {
            if let Some(list) = self.join_key_index.get_mut(&key_val) {
                list.retain(|&(c, e)| !(c == descriptor_index && e == entity_index));
            }
        }

        let key = (descriptor_index, entity_index);
        let Some(affected) = self.entity_to_matches.get_mut(&key) else {
            return HardSoftScore::ZERO;
        };
        let affected = std::mem::take(affected);

        let mut delta = HardSoftScore::ZERO;
        for m in affected {
            if self.matches.remove(&m) {
                let (ca, ia, cb, ib) = m;

                // Remove from other entity's reverse index
                let other_key = if (ca, ia) == key { (cb, ib) } else { (ca, ia) };
                if let Some(list) = self.entity_to_matches.get_mut(&other_key) {
                    list.retain(|t| *t != m);
                }

                // Score delta (negative - match removed)
                delta = delta - self.score_for_match(solution, m);
            }
        }

        self.cached_score = self.cached_score + delta;
        delta
    }

    fn reset(&mut self) {
        self.matches.clear();
        self.entity_to_matches.clear();
        self.join_key_index.clear();
        self.cached_score = HardSoftScore::ZERO;
        self.initialized = false;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        ConstraintRef::new("", &*self.name)
    }

    fn get_matches(
        &self,
        _solution: &DynamicSolution,
    ) -> Vec<DetailedConstraintMatch<HardSoftScore>> {
        Vec::new()
    }

    fn weight(&self) -> HardSoftScore {
        self.weight
    }
}
