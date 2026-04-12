# Python Model IR (Path 2: Codegen + Compile)

This document defines the proposed Python-first model surface that should lower into typed SolverForge Rust code, then compile as a Rust crate in a standalone integration repository.

## Goals

- Preserve SolverForge zero-erasure and monomorphized hot paths.
- Let Python users model the same planning constructs and lifecycle workflows.
- Keep the Python/Rust bridge thin: compile once, run in Rust, stream lifecycle events.

## Historical Context

Path 2 guardrails were derived from the removed `solverforge-py` experiment; see `docs/python-path2-postmortem.md`.

This workspace intentionally keeps Path 2 at the documentation level. Any Python implementation should live outside the SolverForge Rust workspace and consume the public `solverforge` API as a client.

## Planned Modules

A standalone Python integration should roughly split into:

- IR schema and validation
- Expression lowering
- Rust code generation and project writing
- Build/runtime bridge around compiled generated crates

## Design

The IR is declarative and typed:

- Domain declarations (`FactDef`, `EntityDef`, `VariableDef`, `SolutionDef`)
- Constraint declarations (`ConstraintDef`, `JoinSpec`, `FilterSpec`, `ImpactSpec`)
- Runtime configuration (`TerminationDef`, `SolverDef`)
- Top-level container (`ModelDef`)

Expressions are represented as an AST (not executable Python callbacks):

- `RefExpr`
- `ConstExpr`
- `CompareExpr`
- `BoolExpr`
- `CallExpr`

## Lambda Lowering

A convenience helper such as `lambda_to_expr(fn, aliases)` can lower a restricted subset of Python lambda/function syntax into the expression AST:

- Attribute references from known stream aliases
- `==`, `!=`, `<`, `<=`, `>`, `>=`
- Boolean `and`, `or`, `not`
- Whitelisted calls (`contains`, `overlaps`, `len`)

Unsupported constructs fail fast with `LambdaLoweringError`.

## Validation

`validate_model(model)` should perform structural validation:

- Unique entity/fact names
- Solution collection references target known entities/facts
- Constraint source and join collection references exist
- Join-specific required fields are present (`left_key/right_key` for keyed joins, predicate for predicate joins)

## Code Generation (Path 2)

A generator such as `generate_rust_module(model)` should emit Rust source with:

- Domain structs annotated by `#[problem_fact]`, `#[planning_entity]`, `#[planning_solution]`
- Typed `define_constraints()` function using `ConstraintFactory` and fluent stream builders
- Join lowering for `self_equal`, `cross_keyed`, and `cross_predicate`
- Filter/impact/name lowering per constraint

A project writer such as `write_rust_project(model, out_dir, crate_name)` should write a compilable crate:

- `Cargo.toml`
- `src/lib.rs`

Returned metadata should point to generated paths and build artifacts.

## Intended Lowering Contract

The IR lowers into the Rust stream API:

- `source(collection)` -> `ConstraintFactory::<S, Sc>::new().<collection>()`
- `join(self_equal|cross_keyed|cross_predicate)` -> `.join(...)`
- `filter(expr)` -> `.filter(...)`
- `impact` -> `.penalize_*()` / `.reward_*()`
- `name` -> `.named(...)`

This keeps solving and scoring in Rust while preserving Python modeling ergonomics.

## Current Limitations

- The first production scope should target common standard-variable patterns only.
- Advanced list-variable selectors/phases should be designed as a separate lowering track, not implied by the initial IR.
- Packaging, native build/import flow, and lifecycle bridging should live in the standalone Python integration repo rather than this Rust workspace.
