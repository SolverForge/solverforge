# Naming Contract Audit and Unification Plan

Date: 2026-03-31

## Goal

Audit selector, extractor, descriptor, and move-family names that drifted into helper-role prefixes or implementation-boundary compounds. Record the canonical names kept by the current repo without preserving aliases, migration paths, or transitional APIs.

## Scope audited

- `crates/solverforge-core`
- `crates/solverforge-solver`
- `crates/solverforge-scoring`
- Macro-generated call sites in `crates/solverforge-macros`
- Public facade re-exports in `crates/solverforge`
- User-facing docs, wireframes, tests, fixtures, and examples

## Current contract

- Extractor adapters use neutral role names such as `EntityCollectionExtractor`.
- Value selectors use `ValueSelector`, `StaticValueSelector`, `FromSolutionValueSelector`, and `PerEntityValueSelector`.
- Move selectors use `MoveSelector`, cursor-owned candidate storage, `open_cursor(...)`, and `take_candidate(...)`.
- Descriptor-backed scalar-only construction and selector assembly live under `descriptor/*`.
- Generated scalar helper methods use scalar-variable names, not helper-role suffixes.
- Incremental scoring lives under `score_director/incremental.rs`.
- User constraints enter streams through `ConstraintFactory::for_each(Solution::field())`.

## Consolidated surfaces

The current repo has one naming path for each concept:

- Type-qualified extractor and selector names are consolidated into neutral selector/extractor contracts.
- Type-qualified module names are consolidated into role names such as `value_selector.rs`, `move_selector.rs`, and `score_director/incremental.rs`.
- Descriptor-plus-variable-family compounds are consolidated into `descriptor/*` when the boundary is descriptor-owned, and scalar/list names only appear where the API is directly about a variable family.
- Generated public suffix traits are replaced by inherent solution source methods and stable fluent stream methods.
- Generated private helper methods use scalar/list metadata names that match the runtime field they expose.

## Migration notes

- This remains a single breaking sweep with no compatibility shims.
- Macro expansion paths and facade re-exports are updated in the same change as the core and solver renames.
- Relevant wireframes are updated in lockstep so the documented public surface matches the implemented API.
- The value-selector method is `iter(...)` across the public value-selector surface.
- Cursor-owned move storage and selected-winner materialization are part of the canonical `MoveSelector` boundary.
- Scalar construction order hooks mean live model hooks, not phase-start snapshots. Weakest-fit, strongest-fit, decreasing, and queue-style scalar heuristics re-evaluate the current working solution at each construction step. Local-search scalar selectors do not consume construction order hooks.

## Risk summary

- Runtime risk is low because this is primarily a symbol, module, and documentation cleanup.
- Integration risk is medium because downstream imports and generated helper names change atomically.
- Mechanical scope is high because tests, macro-generated paths, re-exports, docs, and wireframes all move together.

## Conclusion

The repository now has one neutral selector/extractor naming model with the intentional descriptor boundary preserved. The selector contract is cursor-owned, monomorphized, and coherent across runtime assembly, generated code, and public docs.
