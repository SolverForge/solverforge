# Refactor Audit (2026-04-20)

## Scope and method

This audit reviewed the entire Rust workspace under `crates/**` (57,387 non-comment, non-blank LOC) and focused on repetition patterns in production code and tests.

Method used:

1. Workspace-wide LOC and module concentration scan.
2. Similarity scans on high-churn directories (`solverforge-solver`, `solverforge-scoring`, `solverforge-macros`).
3. Repetition hotspot review by hand in representative files.
4. Conservative LOC-saving estimates (net reduction after introducing shared abstractions).

## Baseline footprint

- Workspace Rust LOC baseline: **57,387**.
- Largest crates by LOC:
  - `solverforge-solver`: **32,409**
  - `solverforge-scoring`: **14,537**
  - `solverforge-core`: **3,920**
  - `solverforge-macros`: **3,378**

These four crates make up the vast majority of repetition opportunities.

## Repetition hotspots and proposals

### 1) Constraint-stream arity family consolidation (scoring)

**Hotspot evidence**

- `bi_stream.rs` (280), `tri_stream.rs` (295), `quad_stream.rs` (238), `penta_stream.rs` (239) are structurally similar and implement near-identical fluent operations across arities.
- `existence_stream.rs` (440) and `complemented_stream.rs` (392) repeat operation plumbing with slight type-shape differences.

**Proposal**

Create an internal arity-kernel layer (generic traits + helper macros) that centralizes:

- propagation wiring,
- stream builder transitions,
- collector/weighting pass-through,
- shared impl blocks for transform/filter/penalize/reward style operators.

Then keep arity-specific files as thin type aliases + minimal ergonomics wrappers.

**Estimated net savings**: **900 LOC**.

**Risk**: Medium (generic complexity). Keep public API unchanged; refactor internal glue only.

---

### 2) Move-selector neighborhood unification (solver)

**Hotspot evidence**

Large, parallel implementations across neighborhood families:

- move layer: `list_change.rs` (281), `list_swap.rs` (268), `sublist_change.rs` (299), `sublist_swap.rs` (402), `pillar_change.rs` (176), `pillar_swap.rs` (200), `ruin.rs` (181), `list_ruin.rs` (330).
- selector layer: `list_change.rs` (246), `list_swap.rs` (277), `sublist_change.rs` (318), `sublist_swap.rs` (330), `pillar.rs` (260), `ruin.rs` (252), `list_ruin.rs` (280), `nearby_list_change.rs` (383), `nearby_list_swap.rs` (319).

**Proposal**

Introduce a shared neighborhood traversal kernel with static-dispatch strategy structs:

- candidate enumeration,
- filtering predicates,
- score impact hooks,
- move materialization adapters.

Keep concrete move/selector types for API clarity, but delegate to shared kernels.

**Estimated net savings**: **1,050 LOC**.

**Risk**: Medium-high (performance-sensitive). Must benchmark and preserve monomorphization.

---

### 3) Solver tests: fixture and assertion DSL extraction

**Hotspot evidence**

Solver test surface is large and repetitive (5,031 LOC sampled across move/selector/manager/realtime tests), especially in:

- `heuristic/move/tests/*.rs`
- `heuristic/move/*_tests.rs`
- `heuristic/selector/*_tests.rs`
- `manager/*_tests.rs`

Repeated patterns include:

- almost-identical entity/route setup,
- repeated selector construction boilerplate,
- repeated step-and-assert sequences.

**Proposal**

Build a dedicated internal test harness module:

- fixture builders (problem graph, entities, anchors),
- one-line scenario macros for common setup paths,
- assertion helpers for deterministic move sets / score deltas.

Migrate existing tests incrementally without reducing coverage.

**Estimated net savings**: **1,250 LOC**.

**Risk**: Low-medium (test-only). Highest immediate ROI.

---

### 4) Manager + realtime lifecycle scenario consolidation

**Hotspot evidence**

Scenario-style tests in manager/realtime duplicate orchestration setup and expected transitions:

- `manager/mod_tests.rs` (309)
- `manager/phase_factory_tests.rs` (268)
- `manager/builder_tests.rs` (96)
- `realtime/problem_change_tests.rs` (89)
- `realtime/solver_handle_tests.rs` (94)

**Proposal**

Add lifecycle scenario builders:

- `given_solver_manager()` presets,
- phase-pipeline scenario templates,
- reusable assertions for pause/resume/change behavior.

Co-locate with existing `test_utils` to avoid fragmentation.

**Estimated net savings**: **300 LOC**.

**Risk**: Low.

---

### 5) Scoring stream builder internals: flattened/existence/complemented boilerplate reduction

**Hotspot evidence**

Internal boilerplate for weighting and builder transitions is repeated in:

- `flattened_bi_stream/base.rs` (190)
- `flattened_bi_stream/builder.rs` (158)
- `flattened_bi_stream/weighting.rs` (353)
- plus repeated fragments in `existence_stream.rs` and `complemented_stream.rs`.

**Proposal**

Introduce private helper macros for:

- weighted stream factory wiring,
- collector construction patterns,
- common state-carrier structs and impl templates.

Avoid public macro exposure; keep internal to scoring crate.

**Estimated net savings**: **420 LOC**.

**Risk**: Medium.

---

### 6) Proc-macro codegen utility extraction (macros crate)

**Hotspot evidence**

TokenStream assembly patterns recur across planning entity/solution codegen paths:

- `planning_entity/expand.rs` (493)
- `planning_entity/list_variable.rs` (369)
- `planning_solution/runtime.rs` (470)
- `planning_solution/list_operations.rs` (476)
- `planning_solution/expand.rs` (159)
- `planning_solution/shadow.rs` (174)

**Proposal**

Create internal codegen helper modules for:

- repeated token fragments,
- common error emission helpers,
- shared parsing + normalization adapters.

Retain explicit public macro behavior while reducing duplicate quote! blocks.

**Estimated net savings**: **450 LOC**.

**Risk**: Medium (macro diagnostics quality must be preserved).

## Estimated total reduction

Estimated aggregate net reduction if all 6 initiatives land:

- 900 + 1,050 + 1,250 + 300 + 420 + 450 = **4,370 LOC**.

Relative to current Rust LOC baseline (57,387), that is approximately **7.6%** reduction.

## Recommended execution order

1. **Test harness extraction** (initiative 3) — fastest, lowest risk.
2. **Manager/realtime scenario consolidation** (initiative 4) — low risk, immediate cleanup.
3. **Scoring builder boilerplate reduction** (initiative 5) — contained internal refactor.
4. **Stream arity family consolidation** (initiative 1) — larger architectural cleanup.
5. **Move-selector neighborhood unification** (initiative 2) — performance-sensitive, benchmark-gated.
6. **Proc-macro utility extraction** (initiative 6) — do after other internals stabilize.

## Guardrails

- Preserve zero-erasure and static dispatch in hot paths.
- Avoid introducing `Arc`, `Rc`, `Box<dyn Trait>` in solver/scoring paths.
- Validate each phase with `make build`, `make test`, and targeted microbenchmarks for selector/move refactors.
- Keep wireframes/docs in sync if any public API signatures move.
