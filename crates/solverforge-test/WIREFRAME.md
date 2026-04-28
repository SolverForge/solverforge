# solverforge-test WIREFRAME

Shared test fixtures for SolverForge workspace crates.

**Location:** `crates/solverforge-test/`
**Workspace Release:** `0.10.0`

This crate is intended for workspace and downstream test code. It is not part of
the runtime facade API.

## Dependencies

- `solverforge-core` (workspace) — score, descriptor, and domain traits used by fixtures

## File Map

```
src/
├── lib.rs             — Crate root; fixture modules and convenience re-exports
├── entity.rs          — Generic `TestEntity` and `TestSolution`
├── entity_tests.rs    — Generic entity fixture tests
├── nqueens.rs         — N-Queens solution, queen entity, and conflict helpers
├── nqueens_tests.rs   — N-Queens fixture tests
├── shadow.rs          — Shadow-variable solution fixture
├── shadow_tests.rs    — Shadow fixture tests
├── task.rs            — Task scheduling fixture types
└── task_tests.rs      — Task fixture tests
```

## Public Modules

```rust
pub mod entity;
pub mod nqueens;
pub mod shadow;
pub mod task;
```

## Public Re-exports

```rust
pub use entity::{TestEntity, TestSolution};
pub use nqueens::{NQueensSolution, Queen};
pub use shadow::ShadowSolution;
pub use task::{Task, TaskSolution};
```

## Public Fixture Families

### `entity`

Generic entity and solution fixtures used by selector, move, descriptor, and
score-director tests.

### `nqueens`

N-Queens fixture types plus conflict-calculation helpers and descriptor support
for solver/scoring tests.

### `shadow`

Fixture solution for testing shadow update behavior and score-director
notification paths.

### `task`

Task scheduling fixture types used by construction, selector, and scoring tests.

## Architectural Notes

- **Dev-support crate.** It centralizes reusable test domains so runtime crates do not duplicate fixture models.
- **No scoring dependency.** The crate depends on `solverforge-core` only, avoiding circular dependencies with `solverforge-scoring`.
- **Current release aligned.** The crate participates in the workspace version so test fixtures remain aligned with public trait signatures.
