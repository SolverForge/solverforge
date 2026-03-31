# Typed Contract Audit

Date: 2026-03-31

## Goal

Audit the current `Typed*` / `typed_*` surface and evaluate whether we can move to one unified model without prefix/suffix naming and wrapper-style APIs.

## What is actually public today

The *public* `Typed*` API names are concentrated in two areas:

1. `solverforge-core`
   - `TypedEntityExtractor<S, E>` in `domain/entity_ref.rs`.
2. `solverforge-solver`
   - `TypedValueSelector<S, V>` trait.
   - `StaticTypedValueSelector<S, V>`.
   - `FromSolutionTypedValueSelector<S, V>`.

Most other `typed` occurrences are comments, test names, filenames, or descriptive prose (e.g. “typed getter”, “fully typed”), not additional public type wrappers.

## Inventory (high-signal)

### Core descriptor boundary

- `EntityExtractor` is the trait-object boundary used by descriptors.
- `TypedEntityExtractor<S, E>` is a concrete generic implementation used to adapt typed closures into that boundary.
- Wireframe explicitly documents this as an intentional erasure boundary.

### Solver selector layer

- Module filenames currently encode `typed_`:
  - `heuristic/selector/typed_value.rs`
  - `heuristic/selector/typed_move_selector.rs`
- Public API currently carries “Typed” names for value selectors, while move selector trait is already unified as `MoveSelector`.

### Scoring layer

- Scoring terminology still frequently says “typed” in docs/comments, but the public primary name is already `ScoreDirector` (without a `Typed` prefix).

## Assessment of your claim

Your claim is directionally correct for naming:

- The prefix is mostly historical signaling (“this is the zero-erasure path”) and no longer adds much disambiguation where this has already become the default architecture.
- The solver already demonstrates the desired end-state in places (`MoveSelector`, `ScoreDirector`) where generic APIs exist without `Typed` prefixes.

However, there is one structural nuance:

- `TypedEntityExtractor` is not cargo-cult inheritance; it is an adapter from strongly typed field accessors to the intentional runtime-erased descriptor boundary (`Box<dyn EntityExtractor>`).
- That adapter can be renamed to a neutral name, but removing the adapter concept entirely would require changing descriptor storage and runtime polymorphism design.

## Recommended unified naming target

If we pursue cleanup, the target model can be:

- `TypedEntityExtractor` -> `EntityCollectionExtractor` (or `EntityFieldExtractor`).
- `TypedValueSelector` -> `ValueSelector`.
- `StaticTypedValueSelector` -> `StaticValueSelector`.
- `FromSolutionTypedValueSelector` -> `FromSolutionValueSelector`.
- Module file rename:
  - `typed_value.rs` -> `value_selector.rs`
  - `typed_move_selector.rs` -> `move_selector.rs`
- Method rename:
  - `iter_typed(...)` -> `iter_values(...)` (or simply `iter(...)` if unambiguous).

This keeps zero-erasure guarantees but removes terminology noise.

## Migration strategy (no compatibility shims)

Because repository policy forbids transitional shims, migration should be done as a single breaking sweep:

1. Rename types/modules/methods in `solverforge-core` and `solverforge-solver`.
2. Update all internal call sites and tests in the same commit.
3. Update crate wireframes in the same commit (required by AGENTS policy).
4. Update facade re-exports and macro-generated paths.
5. Update README/examples/changelog references.

## Risk summary

- **Low runtime risk**: mostly symbol/file renaming and call-site updates.
- **Medium integration risk**: downstream users will need to migrate imports/method calls in one release.
- **High mechanical scope**: many files reference the names directly, especially tests and wireframes.

## Conclusion

- Yes: we can and likely should converge on prefix-free API naming.
- No: we should not remove the extractor adapter *concept* unless we also redesign descriptor polymorphism.
- Best next step is an atomic, no-shim rename sweep across core + solver + macros + wireframes.
