# Typed Contract Audit and Unification Plan

## Goal

Audit all `Typed*` / `typed_*` contracts and evaluate whether we can move to a unified model with neutral names (no type-signaling prefixes/suffixes).

## Scope audited

- `crates/solverforge-core`
- `crates/solverforge-solver`
- `crates/solverforge-scoring`
- Macro-generated call sites in `crates/solverforge-macros`
- Public re-exports in `crates/solverforge`

## What exists today

### 1) `TypedEntityExtractor` (core)

- Concrete implementation of the dynamic `EntityExtractor` boundary.
- Exists to bridge:
  - runtime descriptor storage (`Box<dyn EntityExtractor>`), and
  - concrete solution/entity access via `fn(&S) -> &Vec<E>` and `fn(&mut S) -> &mut Vec<E>`.
- This is an **intentional type-erasure boundary** in current architecture, not accidental abstraction.

### 2) `typed_value.rs` (solver)

- Defines:
  - `TypedValueSelector<S, V>`
  - `StaticTypedValueSelector<S, V>`
  - `FromSolutionTypedValueSelector<S, V>`
  - `RangeValueSelector<S>`
- Functionally this is the default value selector model in the hot path. The `Typed` prefix is mostly naming noise.

### 3) `typed_move_selector.rs` (solver)

- File/module name contains `typed_`, but the key trait is already neutral (`MoveSelector<S, M>`).
- This is already aligned with a unified model semantically; naming drift remains in module/file names and comments.

### 4) Scoring references to “typed”

- Mostly descriptive text/comments (`typed director`, `typed undo`) rather than separate prefixed public contract types.
- This is conceptual language and can be normalized independently from API renaming.

## Assessment of the claim

Your claim is directionally correct for naming and public ergonomics:

- The “typed” prefix is largely redundant in a generic Rust API where static typing is already explicit in signatures.
- Prefix-heavy naming obscures the true model: there is one selector/extractor model with an intentional erased boundary where needed.

However, one caveat is important:

- We **cannot remove all wrappers** outright. `EntityDescriptor` requires an object-safe boundary (`Box<dyn EntityExtractor>`). A concrete adapter type is still needed; only its name/placement should change.

## Recommended unified naming

### Core

- `TypedEntityExtractor<S, E>` → `EntityCollectionExtractor<S, E>`

Rationale: describes the concrete responsibility (extracting an entity collection) without repeating Rust's type system.

### Solver value selectors

- `TypedValueSelector<S, V>` → `ValueSelector<S, V>`
- `StaticTypedValueSelector<S, V>` → `StaticValueSelector<S, V>`
- `FromSolutionTypedValueSelector<S, V>` → `SolutionValueSelector<S, V>`
- `typed_value.rs` → `value_selector.rs`

### Solver move selector module

- `typed_move_selector.rs` → `move_selector.rs`
- Keep trait names already neutral (`MoveSelector`, `ChangeMoveSelector`, `SwapMoveSelector`, etc.).

## Migration strategy (no compatibility shims)

Because repository policy disallows transitional APIs, do this in a single coherent refactor:

1. Rename files/modules first (`typed_value` → `value_selector`, `typed_move_selector` → `move_selector`).
2. Rename public types above and update all imports/usages.
3. Update macro expansion references in `planning_solution.rs` from `TypedEntityExtractor` to the new core name.
4. Update facade re-exports (`solverforge/src/lib.rs`).
5. Update wireframes:
   - `crates/solverforge-core/WIREFRAME.md`
   - `crates/solverforge-solver/WIREFRAME.md`
   - `crates/solverforge/WIREFRAME.md`
6. Sweep docs/comments to remove wording that suggests two separate models (“typed vs non-typed”) where there is only one runtime path.

## Risk profile

- **Compile-time breakage:** high during rename, low after full migration.
- **Runtime behavior risk:** low if mechanical rename only.
- **API break:** intentional and acceptable per change policy (no shims).

## Conclusion

- A unified, prefix-free surface is feasible and aligns with the architecture.
- Keep the concrete extractor adapter, but rename it to reflect purpose rather than “typedness”.
- The largest value comes from renaming solver selector modules/types and updating wireframes in lockstep.
