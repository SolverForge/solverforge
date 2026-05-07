# Naming Charter

Since SolverForge v0.9.0, `scalar` is the only canonical opposite of `list`.

## Core rules

- Use `scalar` for the non-list planning-variable family.
- Keep established base move names when the structure is already obvious.
- Use explicit contrast names when the surface would otherwise hide scalar-vs-list meaning.
- Do not keep dual naming, compatibility aliases, or legacy synonyms.
- Macro-generated public names must not encode helper roles by prefix or suffix when an inherent method or stable fluent API can carry the concept.
- Do not combine implementation-boundary words with variable-family words into compound namespaces. A descriptor boundary stays `descriptor`; `scalar` and `list` appear only where the API is actually about variable families.

## Canonical names

Internal runtime assembly uses slot/binding names for hidden model metadata:

- `ScalarVariableSlot`
- `ListVariableSlot`
- `VariableSlot::Scalar`
- `VariableSlot::List`

Public grouped scalar declarations use target/group names:

- `ScalarTarget`
- `ScalarEdit`
- `ScalarCandidate`
- `ScalarGroup`

Move-family unions keep their family contrast:

- `ScalarMoveUnion`
- `ListMoveUnion`

## Preserved base-case names

- `ChangeMove`
- `SwapMove`
- `ChangeMoveSelector`
- `SwapMoveSelector`

## Consolidated naming

- Scalar-variable solving uses `scalar` only when it contrasts list variables; otherwise it uses base names such as `ChangeMove`, `SwapMove`, `ChangeMoveSelector`, and `SwapMoveSelector`.
- Descriptor-boundary code lives under `descriptor`.
- Move-family union code lives under `ScalarMoveUnion` or `ListMoveUnion`.
- List move union code uses `ListMoveUnion`.
- Sublist naming is written as one word in identifiers and file names.

## List selector lifting rule

List selector lifting happens by assembling `ListMoveUnion` directly when each
leaf selector opens its cursor. Cartesian-safe selector decorators stay
same-type and cursor-native: filtering, sorting, shuffling, and probability
operate on an existing selector surface instead of type-lifting moves through a
generic map adapter.
