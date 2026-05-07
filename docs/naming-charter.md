# Naming Charter

Since SolverForge v0.9.0, `scalar` is the only canonical opposite of `list`.

## Core rules

- Use `scalar` for the non-list planning-variable family.
- Keep established base move names when the structure is already obvious.
- Use explicit contrast names when the surface would otherwise hide scalar-vs-list meaning.
- Do not keep dual naming, compatibility aliases, or legacy synonyms.
- Macro-generated public names must not encode helper roles by prefix or suffix when an inherent method or stable fluent API can carry the concept.

## Canonical names

- `ScalarVariableContext`
- `ListVariableContext`
- `VariableContext::Scalar`
- `VariableContext::List`
- `ScalarMoveUnion`
- `ListMoveUnion`
- `descriptor_scalar`

## Preserved base-case names

- `ChangeMove`
- `SwapMove`
- `ChangeMoveSelector`
- `SwapMoveSelector`

## Removed naming

- semantic `standard*` names for scalar-variable solving
- `descriptor_standard`
- `EitherMove`
- `ListMoveImpl`
- `SubList*`
- `sub_list_*`
- `iter_typed(...)`

## List selector lifting rule

List selector lifting happens by assembling `ListMoveUnion` directly when each
leaf selector opens its cursor. Cartesian-safe selector decorators stay
same-type and cursor-native: filtering, sorting, shuffling, and probability
operate on an existing selector surface instead of type-lifting moves through a
generic map adapter.
