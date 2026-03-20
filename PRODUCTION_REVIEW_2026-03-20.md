# SolverForge production review — 2026-03-20

## Scope

This review audits every workspace crate individually for completeness, coherence, production readiness, and obvious dead code / wrappers / stubs / duplicates.

## Workspace-level assessment

The workspace architecture is coherent: the top-level manifest cleanly separates core types, proc macros, scoring, solver engine, config, console output, façade crate, CVRP helpers, shared test fixtures, and the CLI. As a codebase, it feels organized and test-heavy. As a product, it is close, but not uniformly production-ready across all crates.

### Global strengths
- Clear crate boundaries and layered architecture.
- Strong automated test coverage in the core solver/scoring crates.
- Passing workspace tests and doctests.

### Global blockers
1. Strict Clippy does not currently pass because of `module_inception` warnings in two CLI test modules.
2. The CLI still exposes intentionally incomplete commands and templates.
3. Generated code still contains `todo!()` placeholders/stubs that require manual cleanup.
4. Some smaller/public helper crates are lightly or indirectly tested compared with the core runtime crates.

## Crate-by-crate audit

### 1. `solverforge-core`
**Role:** foundational domain traits, descriptors, errors, and score types.

**Assessment:** strongest crate in the workspace; looks production-ready from a structure and validation standpoint.

**Why:**
- Small, focused public surface.
- Broad internal test coverage under the crate.
- No obvious stubs or placeholder logic found.

**Suggested follow-up:** keep this crate as the quality bar for the rest of the workspace.

### 2. `solverforge-macros`
**Role:** attribute/derive macros for planning entities, facts, and solutions.

**Assessment:** coherent and likely usable, but less directly validated than the runtime crates.

**Why:**
- Public API is compact and understandable.
- No obvious placeholders/stubs in the macro entrypoints.
- Validation appears to come mostly from downstream tests rather than macro-crate-local tests.

**Suggested follow-up:** add direct macro-focused golden tests / compile-fail tests to reduce regression risk.

### 3. `solverforge-scoring`
**Role:** incremental scoring engine and fluent constraint stream API.

**Assessment:** high-confidence crate; appears close to production-ready.

**Why:**
- Clear zero-erasure design and public exports.
- Large number of focused tests across constraints, directors, streams, analysis, and collectors.
- No placeholder/stub code found.

**Suggested follow-up:** mostly documentation/performance hardening, not architectural cleanup.

### 4. `solverforge-solver`
**Role:** solver engine, phases, heuristics, selectors, runtime orchestration.

**Assessment:** technically strong and heavily exercised; likely production-capable, though large and therefore deserving continued hardening.

**Why:**
- Broad feature set with a correspondingly large test surface.
- Good modular separation between moves, selectors, phases, terminations, manager/builder layers, and realtime hooks.
- No obvious placeholder/stub logic found.

**Suggested follow-up:** keep investing in docs/examples/benchmarks because this is the highest-complexity crate.

### 5. `solverforge-config`
**Role:** TOML/YAML-driven solver configuration.

**Assessment:** small, coherent, and likely production-ready for its scope.

**Why:**
- Good crate fit: narrowly scoped and easy to reason about.
- Has examples and tests.
- No placeholder/stub logic found.

**Suggested follow-up:** add more malformed-config cases over time, but no major red flags.

### 6. `solverforge-console`
**Role:** tracing-based console/log formatting layer.

**Assessment:** useful but under-validated.

**Why:**
- Public surface is tiny and conceptually coherent.
- The crate has no visible test module/files.
- `init()` configures global subscriber state, which is exactly the sort of code that benefits from focused smoke tests.

**Suggested follow-up:** add tests for idempotent init, filter wiring, and formatting smoke coverage.

### 7. `solverforge`
**Role:** façade crate that re-exports the main user-facing API.

**Assessment:** coherent and convenient, but inherits the uneven readiness of the crates beneath it.

**Why:**
- Strong façade design: re-exports macros, scores, stream API, solver types, optional console, and CVRP helpers.
- Includes user-facing docs and integration tests.
- Its production readiness is constrained by weaker subcomponents such as CLI-generated experience and CVRP helpers.

**Suggested follow-up:** keep tightening the public contract and document which modules/features are stable.

### 8. `solverforge-cvrp`
**Role:** domain helpers for list-based vehicle-routing examples/solvers.

**Assessment:** weakest runtime/helper crate in the workspace; not yet at the same production bar as core/scoring/solver.

**Why:**
- Entire crate is a single file with no visible test module/files.
- Uses `*const ProblemData` plus repeated `unsafe` dereferences based on trait-level guarantees.
- Contains duplicate thin wrappers: `assign_route` and `set_route` do the same thing; `get_route` is also a minimal convenience wrapper.

**Suggested follow-up:**
- Add direct unit tests.
- Consolidate duplicate route helpers.
- Consider a safer ownership/reference pattern or, at minimum, much stronger safety documentation.

### 9. `solverforge-test`
**Role:** shared fixtures for testing other crates.

**Assessment:** coherent internal-support crate; fit for purpose.

**Why:**
- Clear purpose and narrow scope.
- Contains self-tests for fixture behavior.
- No obvious dead code/stub issues.

**Suggested follow-up:** low priority unless fixture complexity grows.

### 10. `solverforge-cli`
**Role:** scaffolding and project-management CLI.

**Assessment:** least production-ready crate in the workspace.

**Why:**
- Strict Clippy fails because of `module_inception` warnings in two test modules.
- The CLI advertises an interactive console command that is explicitly a stub.
- The generic `--list` scaffold is advertised as “coming soon” and intentionally unavailable.
- Generated constraint/domain code includes multiple `todo!()` placeholders and a stub data loader.

**Dead code / duplication / polish issues:**
- The nested `mod tests` pattern is a lint-only issue but is easy cleanup.
- The public CLI surface currently includes unfinished paths that should be hidden, gated, or finished.
- Generated output is useful for scaffolding but not yet polished enough to feel production-grade.

**Suggested follow-up:**
- Make Clippy pass.
- Remove/gate unfinished commands.
- Improve generated code so new projects compile cleanly or fail with much more guided placeholders.

## Production-readiness ranking

### Ready or nearly ready
- `solverforge-core`
- `solverforge-scoring`
- `solverforge-solver`
- `solverforge-config`
- `solverforge-test`

### Usable but needs more validation/polish
- `solverforge-macros`
- `solverforge-console`
- `solverforge`

### Not yet at the same production bar
- `solverforge-cvrp`
- `solverforge-cli`

## Highest-value improvements

1. Fix the existing Clippy failures in `solverforge-cli`.
2. Remove or gate incomplete CLI functionality (`console`, generic list scaffold).
3. Upgrade scaffolding output so generated projects are closer to compile-and-run quality.
4. Add direct tests for `solverforge-console`, `solverforge-cvrp`, and macro expansion edge cases.
5. Reduce unnecessary API surface in `solverforge-cvrp` by consolidating duplicate wrappers.
6. Publish a stability/readiness matrix for crates and major features.
