# Typed Contract Audit and Unification Plan

Date: 2026-03-31

## Goal

Audit the former `Typed*` / `typed_*` public surface, explain the architectural boundary that remains intentional, and record the canonical naming adopted in this refactor.

## Scope audited

- `crates/solverforge-core`
- `crates/solverforge-solver`
- `crates/solverforge-scoring`
- Macro-generated call sites in `crates/solverforge-macros`
- Public re-exports in `crates/solverforge`

## What existed before this refactor

### Core descriptor boundary

- `TypedEntityExtractor<S, E>` was the concrete implementation behind the dynamic `EntityExtractor` descriptor boundary.
- That adapter existed to bridge runtime descriptor storage (`Box<dyn EntityExtractor>`) with strongly typed solution field access.

### Solver selector layer

- `typed_value.rs` exposed:
  - `TypedValueSelector<S, V>`
  - `StaticTypedValueSelector<S, V>`
  - `FromSolutionTypedValueSelector<S, V>`
- `release/0.7.0` had also introduced `PerEntityTypedValueSelector<S, V>`, which followed the same naming pattern and therefore belongs in the same cleanup.
- `typed_move_selector.rs` already exposed neutral trait names such as `MoveSelector<S, M>`, so the drift there was primarily in module/file naming rather than the public trait names themselves.

### Scoring references to "typed"

- Remaining scoring references were descriptive prose rather than separate prefixed public contracts.

## Architectural assessment

- Prefix-free naming fits the current architecture better. In generic Rust APIs, the type information is already explicit in the signatures.
- The extractor adapter remains necessary. `EntityDescriptor` still depends on an object-safe erased boundary, so the refactor renames that adapter rather than removing it.
- The move-selector layer was already conceptually single-path; the cleanup there is about removing naming drift in modules and re-exports.

## Adopted canonical naming

- `TypedEntityExtractor<S, E>` -> `EntityCollectionExtractor<S, E>`
- `TypedValueSelector<S, V>` -> `ValueSelector<S, V>`
- `StaticTypedValueSelector<S, V>` -> `StaticValueSelector<S, V>`
- `FromSolutionTypedValueSelector<S, V>` -> `FromSolutionValueSelector<S, V>`
- `PerEntityTypedValueSelector<S, V>` -> `PerEntityValueSelector<S, V>`
- `typed_value.rs` -> `value_selector.rs`
- `typed_move_selector.rs` -> `move_selector.rs`

## Migration notes

- This remains a single breaking sweep with no compatibility shims, consistent with repository policy.
- Macro expansion paths and facade re-exports are updated in the same change as the core and solver renames.
- Relevant wireframes are updated in lockstep so the documented public surface matches the implemented API.
- The selector method name is now `iter(...)` across the public value-selector surface. The old `iter_typed(...)` name is removed as part of the same breaking sweep so the typed/scalar naming model stays consistent.
- The move-selector surface now uses cursor/materialization semantics: `open_cursor(...)`
  yields stable candidate indices plus borrowable move views, and ownership
  transfers only through `take_candidate(...)` after the solver selects a
  winner. Cartesian composition reuses that contract to expose preview-safe
  sequential candidates without cloning move storage, and it is intentionally
  not a general owned-stream selector contract through `iter_moves(...)` or
  `append_moves(...)`.
- Scalar construction order hooks now mean live model hooks, not phase-start
  snapshots. Weakest-fit, strongest-fit, decreasing, and queue-style scalar
  heuristics re-evaluate the current working solution at each construction step.
  Local-search scalar selectors do not consume construction order hooks.

## Risk summary

- Runtime risk is low because this is primarily a symbol and module rename.
- Integration risk is medium because downstream imports and type names change atomically.
- Mechanical scope is high because tests, macro-generated paths, re-exports, and wireframes all move together.

## Conclusion

- The repository now has one neutral selector/extractor naming model with the intentional erased descriptor boundary preserved.
- The selector contract is no longer "owned iterator first". Cursor-owned
  storage and selected-winner materialization are now part of the canonical
  `MoveSelector` boundary.
- The synthesis branch combines the implementation intent from the code draft with the explanatory audit intent from the two documentation drafts, while also covering the additional `PerEntityTypedValueSelector` surface that existed on `release/0.7.0`.
