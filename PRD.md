# Project: SolverForge Dynamic Performance & Correctness Fixes

Fix performance bottlenecks and entity assignment bugs in the Python wrapper for solverforge-dynamic.

## Tasks

### Phase 1: Critical Performance - Entity Index Lookup O(n) → O(1)

- [x] Add entity ID to index mapping to DynamicSolution struct (crates/solverforge-dynamic/src/solution.rs)
- [x] Implement HashMap<i64, (usize, usize)> for id_to_location lookup (crates/solverforge-dynamic/src/solution.rs)
- [x] Update add_entity to maintain the id_to_location map (crates/solverforge-dynamic/src/solution.rs)
- [x] Add get_entity_location(id: i64) -> Option<(usize, usize)> method (crates/solverforge-dynamic/src/solution.rs)
- [x] Refactor make_bi_filter to use id_to_location instead of iter().position() (crates/solverforge-dynamic/src/constraint/closures_bi.rs)
- [x] Refactor make_tri_filter to use id_to_location lookup (crates/solverforge-dynamic/src/constraint/closures_tri.rs)
- [x] Refactor make_cross_filter to use id_to_location lookup (crates/solverforge-dynamic/src/constraint/closures_cross.rs)
- [x] Add tests verifying O(1) lookup behavior (crates/solverforge-dynamic/src/solution.rs)

### Phase 2: Moderate Performance - Eliminate Weight Function Cloning

- [x] Change DynBiWeight signature to accept solution reference and indices (crates/solverforge-dynamic/src/constraint/types.rs)
- [x] Refactor make_bi_weight to use indices instead of cloning entities (crates/solverforge-dynamic/src/constraint/closures_bi.rs)
- [x] Update IncrementalBiConstraint to pass solution to weight function (crates/solverforge-scoring/src/constraint/nary_incremental.rs)
- [x] Change DynTriWeight signature to accept solution reference and indices (crates/solverforge-dynamic/src/constraint/types.rs)
- [x] Refactor make_tri_weight to use indices instead of cloning (crates/solverforge-dynamic/src/constraint/closures_tri.rs)
- [x] Change DynCrossWeight signature to accept solution reference (crates/solverforge-dynamic/src/constraint/types.rs)
- [x] Refactor make_cross_weight to use solution reference (crates/solverforge-dynamic/src/constraint/closures_cross.rs)
- [x] Run existing constraint tests to verify no regressions (crates/solverforge-dynamic/src/constraint/tests.rs)

### Phase 3: Entity Assignment Bug - Field Lookup Correctness

- [x] Track source class index in ConstraintOp::ForEach and Join (crates/solverforge-py/src/constraint_builder.rs)
- [x] Add param_to_class_idx mapping during constraint building (crates/solverforge-py/src/constraint_builder.rs)
- [x] Refactor parse_simple_expr to use param's class for field lookup (crates/solverforge-py/src/constraint_builder.rs)
- [x] Add test for cross-class constraints with same-named fields (crates/solverforge-py/src/constraint_builder.rs)

### Phase 4: Key Extraction Context Fix

- [x] Document key expression limitations in make_cross_key_a/b docstrings (crates/solverforge-dynamic/src/constraint/closures_cross.rs)
- [x] Add runtime warning when key expr contains RefField or FactRef (crates/solverforge-dynamic/src/constraint/closures_cross.rs)
- [x] Consider passing full solution to key extractors for complex keys (crates/solverforge-dynamic/src/constraint/closures_cross.rs)

### Phase 5: Optional - Lazy Move Generation

- [x] Create DynamicMoveIterator struct implementing Iterator<Item=DynamicChangeMove> (crates/solverforge-dynamic/src/moves.rs)
- [x] Refactor generate_moves to return iterator instead of Vec (crates/solverforge-dynamic/src/moves.rs)
- [x] Update MoveSelector trait impl to use iterator (crates/solverforge-dynamic/src/moves.rs)
- [x] Benchmark move generation before/after change (crates/solverforge-dynamic/src/moves.rs)

### Phase 6: Testing & Validation

- [x] Create benchmark comparing old vs new filter performance (crates/solverforge-dynamic/benches/)
- [x] Add integration test with 1000+ entities verifying correct assignments (crates/solverforge-dynamic/src/solve/tests.rs)
- [x] Test Python wrapper with employee scheduling problem (crates/solverforge-py/)
- [x] Verify incremental scoring matches full recalculation after all changes (crates/solverforge-dynamic/src/constraint/tests.rs)

## Notes

### Context for Implementation

- The `DynamicEntity.id` is a sequential i64 assigned by `Solver.next_entity_id` in Python
- Entity indices are positions in `solution.entities[class_idx]` vectors
- The current O(n) lookup at closures_bi.rs:47-48 is called for EVERY filter evaluation
- Weight functions create temporary `DynamicSolution` with cloned descriptor - this is expensive
- The `IncrementalBiConstraint` in solverforge-scoring passes `&A` (entities) not indices to weight fn

### Key Files

- `crates/solverforge-dynamic/src/solution.rs` - DynamicSolution, DynamicEntity structs
- `crates/solverforge-dynamic/src/constraint/closures_bi.rs` - Bi-constraint filter/weight closures
- `crates/solverforge-dynamic/src/constraint/closures_tri.rs` - Tri-constraint closures
- `crates/solverforge-dynamic/src/constraint/closures_cross.rs` - Cross-join closures
- `crates/solverforge-dynamic/src/constraint/types.rs` - Closure type definitions
- `crates/solverforge-py/src/constraint_builder.rs` - Python constraint API
- `crates/solverforge-scoring/src/constraint/nary_incremental.rs` - Incremental constraint impls

### Breaking Change Considerations

- Changing weight function signatures affects `IncrementalBiConstraint` generic params
- May need to update factory functions in `factory_self.rs`, `factory_cross.rs`
- Python API should remain unchanged - changes are internal

### Root Cause Analysis

#### Performance Issue 1: O(n) Entity Lookup
```rust
// closures_bi.rs:47-48 - Called for EVERY filter evaluation
let a_idx = entities.iter().position(|e| e.id == a.id);
let b_idx = entities.iter().position(|e| e.id == b.id);
```
For N entities with M constraint matches, this is O(N * M) instead of O(M).

#### Performance Issue 2: Weight Function Cloning
```rust
// closures_bi.rs:109-122 - Clones descriptor and entities
let mut temp_solution = DynamicSolution {
    descriptor: descriptor.clone(),
    entities: Vec::new(),
    ...
};
temp_solution.entities[class_idx] = vec![a.clone(), b.clone()];
```

#### Correctness Issue: Field Lookup Ambiguity
```rust
// constraint_builder.rs:239-248 - Searches ALL classes for field name
for class_def in &descriptor.entity_classes {
    if let Some(field_idx) = class_def.field_index(field_name) {
        return Ok(Expr::field(param_idx, field_idx));
    }
}
```
If multiple classes have same-named fields with different indices, returns wrong field.

## Success Criteria

- All checkboxes marked `[x]`
- Existing tests in `constraint/tests.rs` pass
- Filter operations achieve O(1) entity lookup
- No temporary solution/entity cloning in weight functions
- Cross-class constraints with same-named fields resolve correctly
- Benchmark shows measurable performance improvement for 100+ entities
