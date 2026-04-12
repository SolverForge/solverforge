# Python Path 2 vs Historical `solverforge-py` (Postmortem)

This note compares the current Python Path 2 direction (IR -> Rust codegen -> compile) with the removed `crates/solverforge-py` experiment.

## Historical Reference Point

The latest commit where `crates/solverforge-py` still existed was:

- `be76aaf` (2026-02-06) `refactor(py): remove Solver pyclass and unify API under SolverManager`

The deletion happened at:

- `559c57d` (2026-03-08) `chore: delete dynamic, py + all cranelift stuff; delete stub dotfiles that were used with zoyd`

## Why the Old Experiment Failed (Structural Issues)

### 1) Dynamic runtime model instead of typed compile-time model

Old `solverforge-py` built solutions with dynamic descriptors and dynamic values at runtime:

- `DynamicDescriptor`, `DynamicEntity`, `DynamicSolution`, `DynamicConstraintSet`
- runtime-defined classes and value ranges

This created a separate dynamic execution path that diverged from the typed core.

### 2) String-expression constraints

Old constraints were built from string expressions like:

- `"A.row == B.row"`
- `"field + 1"`

and parsed at runtime with ad-hoc parsing logic (`parse_expr`, `parse_simple_expr`).

This was fragile, hard to validate statically, and not aligned with typed stream APIs.

### 3) Python API drift from Rust public API

The old interface (`entity_class`, `add_entities`, string joins/filters) did not match the typed Rust modeling surface and lifecycle contracts.

### 4) Lifecycle/telemetry mismatch

The old manager API exposed coarse status strings and custom async controls, which did not map to retained `job/snapshot/checkpoint` semantics used in modern SolverForge.

## Path 2 Correctives

Path 2 intentionally avoids the above failure modes:

1. **Typed IR, not dynamic runtime objects**
   - Python describes model structure and expressions as a typed AST.

2. **Compile to Rust, do not interpret in Python**
   - Emit Rust structs/macros/constraint streams and compile.

3. **No string DSL at runtime**
   - Expressions are AST nodes lowered into Rust source.

4. **Keep Rust as the only execution path**
   - Scoring/moves/phases run in generated Rust.

5. **Align with retained lifecycle contracts**
   - Future bindings should expose `job`/`snapshot`/`events` directly rather than inventing a parallel lifecycle model.

## Non-Negotiable Guardrails

- No runtime expression parser for user strings.
- No dynamic scoring/move engine fork for Python.
- No Python callback execution in hot scoring/move loops.
- Generated code must target the same public SolverForge contracts used by Rust users.

## Current Status

The repository now contains a Python IR + codegen prototype under `python/solverforge_ir` and docs in `docs/python-model-ir.md`.

Remaining work includes:

- pyproject/maturin packaging for produced crates,
- list-variable parity,
- lifecycle bridge that forwards retained runtime events as web/SSE-friendly payloads.
