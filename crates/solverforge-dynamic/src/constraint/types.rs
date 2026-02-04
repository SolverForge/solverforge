//! Type aliases for boxed closures used with monomorphized constraints.

use solverforge_core::score::HardSoftScore;

use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

// =============================================================================
// Type aliases for boxed closures used with monomorphized constraints
// =============================================================================

/// Extractor: retrieves entity slice from solution.
pub type DynExtractor = Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Key extractor: extracts join key from entity.
pub type DynKeyExtractor = Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Uni-constraint filter: checks if a single entity matches.
pub type DynUniFilter = Box<dyn Fn(&DynamicSolution, &DynamicEntity) -> bool + Send + Sync>;

/// Uni-constraint weight: computes score for a single entity.
pub type DynUniWeight = Box<dyn Fn(&DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Bi-constraint filter: checks if pair of entities matches.
pub type DynBiFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Tri-constraint filter: checks if triple of entities matches.
pub type DynTriFilter = Box<
    dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync,
>;

/// Quad-constraint filter: checks if quadruple of entities matches.
pub type DynQuadFilter = Box<
    dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool
        + Send
        + Sync,
>;

/// Penta-constraint filter: checks if quintuple of entities matches.
pub type DynPentaFilter = Box<
    dyn Fn(
            &DynamicSolution,
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
        ) -> bool
        + Send
        + Sync,
>;

/// Bi-constraint weight: computes score for pair using solution reference and entity indices.
///
/// Takes the full solution and indices into the entity slice, avoiding entity cloning.
/// The indices are positions within `solution.entities[class_idx]`.
pub type DynBiWeight =
    Box<dyn Fn(&DynamicSolution, usize, usize) -> HardSoftScore + Send + Sync>;

/// Tri-constraint weight: computes score for triple using solution reference and entity indices.
///
/// Takes the full solution and three indices into the entity slice, avoiding entity cloning.
/// The indices are positions within `solution.entities[class_idx]`.
pub type DynTriWeight =
    Box<dyn Fn(&DynamicSolution, usize, usize, usize) -> HardSoftScore + Send + Sync>;

/// Quad-constraint weight: computes score for quadruple.
pub type DynQuadWeight = Box<
    dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore
        + Send
        + Sync,
>;

/// Penta-constraint weight: computes score for quintuple.
pub type DynPentaWeight = Box<
    dyn Fn(
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
            &DynamicEntity,
        ) -> HardSoftScore
        + Send
        + Sync,
>;

// Cross-join constraint closures (for joining two different entity classes)

/// Cross-join extractor A: extracts first entity class slice from solution.
pub type DynCrossExtractorA = Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join extractor B: extracts second entity class slice from solution.
pub type DynCrossExtractorB = Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join key extractor A: extracts join key from entity of class A.
pub type DynCrossKeyA = Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join key extractor B: extracts join key from entity of class B.
pub type DynCrossKeyB = Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join filter: checks if pair of entities from different classes matches.
pub type DynCrossFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Cross-join weight: computes score for cross-join pair using solution reference and entity indices.
///
/// Takes the full solution and indices into each entity class slice, avoiding entity cloning.
/// The first index is a position within `solution.entities[class_idx_a]`, the second is within
/// `solution.entities[class_idx_b]`.
pub type DynCrossWeight =
    Box<dyn Fn(&DynamicSolution, usize, usize) -> HardSoftScore + Send + Sync>;

// Flattened constraint closures (for constraints that expand entities into collections)

/// Flatten function: expands entity B into a slice of C items.
pub type DynFlatten = Box<dyn Fn(&DynamicEntity) -> &[DynamicValue] + Send + Sync>;

/// C key function: extracts index key from flattened item C.
pub type DynCKeyFn = Box<dyn Fn(&DynamicValue) -> DynamicValue + Send + Sync>;

/// A lookup function: extracts lookup key from entity A for O(1) index access.
pub type DynALookup = Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Flattened filter: checks if pair of (A entity, C item) matches.
pub type DynFlattenedFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicValue) -> bool + Send + Sync>;

/// Flattened weight: computes score for (A entity, C item) pair.
pub type DynFlattenedWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicValue) -> HardSoftScore + Send + Sync>;
