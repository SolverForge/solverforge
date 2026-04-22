# Naming Charter

SolverForge v0.9.0 uses `scalar` as the only canonical opposite of `list`.

## Core rules

- Use `scalar` for the non-list planning-variable family.
- Keep established base move names when the structure is already obvious.
- Use explicit contrast names when the surface would otherwise hide scalar-vs-list meaning.
- Do not keep dual naming, compatibility aliases, or legacy synonyms.

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

## Mapping decorator rule

List selector lifting uses the generic `MapMoveSelector` decorator. SolverForge does not keep one adapter type per list selector family.
