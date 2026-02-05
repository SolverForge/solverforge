# Project: solverforge-scoring refactor

Clean up test organization, remove doc comments, extract code from mod.rs files.

## Tasks

### Phase 1: Fix test directory structure
- [x] Fix crates/solverforge-scoring/src/api/constraint_set/tests/constraint_set.rs imports
- [x] Fix crates/solverforge-scoring/src/stream/collector/tests/collector.rs imports
- [x] Fix crates/solverforge-scoring/src/stream/filter/tests/filter.rs imports
- [x] Run cargo test -p solverforge-scoring to verify
- [ ] Commit: fix test imports

### Phase 2: Convert /// to // or /* */
- [ ] Convert /// comments in crates/solverforge-scoring/src/lib.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/api/analysis.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/api/weight_overrides.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/api/constraint_set/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/balance.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/complemented.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/cross_bi_incremental.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/flattened_bi.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/grouped.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/if_exists.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/incremental.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/macros.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/nary_incremental/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/nary_incremental/bi.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/nary_incremental/tri.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/nary_incremental/quad.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/constraint/nary_incremental/penta.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/simple.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/traits.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/typed.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/recording.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/shadow_aware.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/director/factory.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/factory.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/uni_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/bi_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/tri_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/quad_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/penta_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/balance_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/complemented_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/cross_bi_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/flattened_bi_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/grouped_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/if_exists_stream.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/collector/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/collector/count.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/collector/sum.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/collector/load_balance.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/filter/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/filter/traits.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/filter/wrappers.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/filter/composition.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/filter/adapters.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/joiner/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/joiner/equal.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/joiner/comparison.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/joiner/filtering.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/joiner/overlapping.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/arity_stream_macros/mod.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/arity_stream_macros/bi.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/arity_stream_macros/tri.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/arity_stream_macros/quad.rs
- [ ] Convert /// comments in crates/solverforge-scoring/src/stream/arity_stream_macros/penta.rs
- [ ] Run cargo test -p solverforge-scoring to verify
- [ ] Commit: convert doc comments to regular comments

### Phase 3: Extract code from mod.rs files
- [ ] Extract UniCollector and Accumulator traits from crates/solverforge-scoring/src/stream/collector/mod.rs to crates/solverforge-scoring/src/stream/collector/uni.rs
- [ ] Update crates/solverforge-scoring/src/stream/collector/mod.rs to only have mod declarations and re-exports
- [ ] Extract Joiner trait, AndJoiner, FnJoiner from crates/solverforge-scoring/src/stream/joiner/mod.rs to crates/solverforge-scoring/src/stream/joiner/match_condition.rs
- [ ] Update crates/solverforge-scoring/src/stream/joiner/mod.rs to only have mod declarations and re-exports
- [ ] Extract IncrementalConstraint, ConstraintSet, ConstraintResult, macro from crates/solverforge-scoring/src/api/constraint_set/mod.rs to crates/solverforge-scoring/src/api/constraint_set/incremental.rs
- [ ] Update crates/solverforge-scoring/src/api/constraint_set/mod.rs to only have mod declarations and re-exports
- [ ] Run cargo test -p solverforge-scoring to verify
- [ ] Commit: extract code from mod.rs files

### Phase 4: Scan for repetition and refactor
- [ ] Identify duplicate test fixtures across test files
- [ ] Identify repeated code patterns
- [ ] Refactor if opportunities found
- [ ] Commit: refactor to reduce repetition (if changes made)

### Phase 5: Final verification
- [ ] Run cargo test -p solverforge-scoring
- [ ] Run cargo clippy -p solverforge-scoring
- [ ] Verify zero-erasure: no Box<dyn> or Arc<dyn> in constraint evaluation hot paths

## Notes

- mod.rs files should ONLY contain: mod declarations, pub use re-exports, #[cfg(test)] mod tests
- No /// or //! doc comments anywhere in this crate - it's internal, no rustdoc needed
- Single-line comments become //, multi-line blocks become /* */
- Test files live in tests/ subdirectories, not *_tests.rs suffixes
- Zero-erasure = no dynamic dispatch in scoring hot paths
- Already confirmed: zero-erasure is maintained, Box<dyn> only in undo stack (not hot path)

## Success Criteria

- All checkboxes marked [x]
- cargo test passes
- cargo clippy passes
- No /// or //! comments in crate
- No code in mod.rs files (only mod/use statements)
- All tests in tests/ subdirectories
