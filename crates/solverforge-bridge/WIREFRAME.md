# solverforge-bridge

Public contracts for dynamic host-language bindings.

This crate is the boundary between monomorphized Rust models and dynamic
binding models. It is additive: the macro-generated Rust path remains in
`solverforge-core`, `solverforge-scoring`, and `solverforge-solver` and should
continue to be the performance ceiling.

## Scope

- Stable logical IDs for dynamic entity, fact, and variable classes.
- A dynamic planning-model backend trait that binding crates can implement with
  Rust-owned state.
- Dynamic score support for the binding path.
- Public runner helpers that avoid requiring macro-style descriptor and
  constraint factory functions.

## Non-Goals

- Python-specific types.
- Generated Rust.
- String-parsed constraints.
- A second solver engine.
