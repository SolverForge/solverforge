# Python Model IR (Path 2: Codegen + Compile)

This document defines the Python-first model surface that lowers into typed SolverForge Rust code, then compiles as a Rust crate.

## Goals

- Preserve SolverForge zero-erasure and monomorphized hot paths.
- Let Python users model the same planning constructs and lifecycle workflows.
- Keep the Python/Rust bridge thin: compile once, run in Rust, stream lifecycle events.

## Modules

## Historical Context

Path 2 guardrails were derived from the removed `solverforge-py` experiment; see `docs/python-path2-postmortem.md`.

- IR schema and validation: `python/solverforge_ir/model.py`
- Rust code generation and project writer: `python/solverforge_ir/codegen.py`

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

`lambda_to_expr(fn, aliases)` lowers a restricted subset of Python lambda/function syntax into the expression AST:

- Attribute references from known stream aliases
- `==`, `!=`, `<`, `<=`, `>`, `>=`
- Boolean `and`, `or`, `not`
- Whitelisted calls (`contains`, `overlaps`, `len`)

Unsupported constructs fail fast with `LambdaLoweringError`.

## Validation

`validate_model(model)` performs structural validation:

- Unique entity/fact names
- Solution collection references target known entities/facts
- Constraint source and join collection references exist
- Join-specific required fields are present (`left_key/right_key` for keyed joins, predicate for predicate joins)

## Code Generation (Path 2)

`generate_rust_module(model)` emits Rust source with:

- Domain structs annotated by `#[problem_fact]`, `#[planning_entity]`, `#[planning_solution]`
- Typed `define_constraints()` function using `ConstraintFactory` and fluent stream builders
- Join lowering for `self_equal`, `cross_keyed`, and `cross_predicate`
- Filter/impact/name lowering per constraint

`write_rust_project(model, out_dir, crate_name)` writes a compilable crate:

- `Cargo.toml`
- `src/lib.rs`

Returned metadata (`GeneratedRustProject`) points to generated paths.

## Intended Lowering Contract

The IR lowers into the Rust stream API:

- `source(collection)` -> `ConstraintFactory::<S, Sc>::new().<collection>()`
- `join(self_equal|cross_keyed|cross_predicate)` -> `.join(...)`
- `filter(expr)` -> `.filter(...)`
- `impact` -> `.penalize_*()` / `.reward_*()`
- `name` -> `.named(...)`

This keeps solving and scoring in Rust while preserving Python modeling ergonomics.

## Current Limitations

- Codegen currently targets common standard-variable patterns.
- Advanced list-variable selectors/phases are not yet emitted.
- Project writing creates a Rust crate artifact; packaging via PyO3/maturin is a follow-up step.
