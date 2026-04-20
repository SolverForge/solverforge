# Repetition Audit (2026-04-20)

This document records a full-repo repetition audit focused on Rust sources and a refactor plan that minimizes repeated logic while preserving SolverForge architecture constraints (zero-erasure, ownership transfer, no `Arc`/`Rc`/`dyn` in hot paths).

## Methodology

1. Scanned all Rust files in `crates/**`.
2. Used sliding-window duplicate detection (normalized, comment-stripped) to find high-confidence repeated blocks.
3. Manually reviewed top clone clusters and separated:
   - intentional repetition (acceptable tradeoff),
   - accidental/legacy repetition (high-priority refactor),
   - structural repetition suitable for macro/helper extraction.
4. Estimated net LOC savings conservatively for each proposal.

## Priority 1 — Remove legacy duplicate test suites in solver selectors/moves

### Evidence

The move and selector modules include direct `#[path = "*_tests.rs"]` module wiring in production files, while equivalent suites also exist under `tests/` module trees.

Examples:
- `k_opt.rs` includes `k_opt_tests.rs`. (`crates/solverforge-solver/src/heuristic/move/k_opt.rs`)
- `list_change.rs` includes `list_change_tests.rs`. (`crates/solverforge-solver/src/heuristic/move/list_change.rs`)
- `sublist_change.rs` includes `sublist_change_tests.rs`. (`crates/solverforge-solver/src/heuristic/move/sublist_change.rs`)
- `move_selector.rs`, `pillar.rs`, and `mimic.rs` mirror the same pattern in selector code.

Parallel suites already exist under:
- `crates/solverforge-solver/src/heuristic/move/tests/*.rs`
- `crates/solverforge-solver/src/heuristic/selector/tests/*.rs`

### Refactor

- Keep a single canonical test layout (`tests/` submodule tree).
- Remove `*_tests.rs` legacy files and inline `#[path = ...]` hooks.
- Preserve any unique assertions by migrating them into the canonical files before deletion.

### LOC savings estimate

- Deleting duplicate legacy files: **~1,652 LOC** (sum of seven confirmed duplicated `*_tests.rs` files).
- Removing legacy `#[path = ...]` hooks + cleanup: **~10 LOC**.
- **Estimated net savings: ~1,660 LOC**.

## Priority 2 — Unify repeated weighting DSL builders across stream types

### Evidence

`penalize*`/`reward*` families are repeated with near-identical control flow and `ImpactType` handling across multiple stream types:

- `cross_bi_stream/weighting.rs`
- `flattened_bi_stream/weighting.rs`
- `uni_stream/weighting.rs`
- `existence_stream.rs`
- `complemented_stream.rs`

All repeat the same pattern:
- compute `is_hard` from weight,
- build typed builder struct,
- duplicate hard/soft convenience wrappers.

### Refactor

- Introduce an internal macro/helper layer for weighting family generation.
- Keep concrete builder types and monomorphized closures (no type erasure).
- Consolidate hard/soft convenience methods via shared expansion points.

### LOC savings estimate

- Current repeated weighting-heavy files total roughly **~1,330 LOC**.
- Conservative 30–35% reduction from shared generation layer.
- **Estimated net savings: ~400–470 LOC**.

## Priority 3 — Collapse repeated n-ary arity stream macro definitions into one meta-macro

### Evidence

`arity_stream_macros/nary_stream/{bi,tri,quad,penta}.rs` are structurally near-identical and mostly differ by arity-specific filter/closure signatures.

### Refactor

- Replace four top-level arity macro files with one meta-macro file parameterized by:
  - arity token set,
  - filter trait family,
  - closure signature tuple.
- Retain explicit generated APIs for ergonomics and docs.

### LOC savings estimate

- Current four files: **1,325 LOC**.
- Keep one meta-macro + concise arity invocations.
- **Estimated net savings: ~420–520 LOC**.

## Priority 4 — Consolidate higher-arity incremental constraints (tri/quad/penta)

### Evidence

`constraint/nary_incremental/higher_arity/{tri,quad,penta}.rs` repeat lifecycle and delta-application flow with arity-specific tuple plumbing.

### Refactor

- Introduce shared macro-driven skeleton for:
  - index maintenance,
  - score impact update path,
  - event handling branches.
- Keep explicit type signatures generated per arity to preserve compile-time clarity.

### LOC savings estimate

- Current three files: **1,101 LOC**.
- Conservative reduction with shared generation layer: 25–35%.
- **Estimated net savings: ~275–385 LOC**.

## Priority 5 — Extract reusable list-neighborhood enumeration kernels for selector families

### Evidence

Move selector implementations for list neighborhoods contain duplicated iteration skeletons:
- `list_swap.rs`
- `sublist_change.rs`
- `sublist_swap.rs`

Repeated mechanics include:
- entity materialization,
- precomputed lengths,
- nested start/size/destination loops,
- symmetric pair filtering.

### Refactor

- Introduce internal zero-erasure iterator kernels (generic over callback emitters), e.g.:
  - `for_each_entity_pair(...)`
  - `for_each_segment(...)`
  - `for_each_non_overlapping_segment_pair(...)`
- Keep selector public APIs unchanged.

### LOC savings estimate

- Current candidate files total about **~830 LOC**.
- Conservative extraction savings: 20–30%.
- **Estimated net savings: ~165–250 LOC**.

## Priority 6 — Deduplicate reusable test fixture builders across solver/scoring tests

### Evidence

Multiple suites repeat near-identical mini-domain setup (solution structs, descriptor creation, extractors, director wiring), notably in move/selector tests and several scoring constraint tests.

### Refactor

- Add internal test support modules per crate (`#[cfg(test)] mod test_support`) with:
  - canonical descriptor builders,
  - common toy domain structs,
  - helper constructors for directors.
- Migrate suites to shared fixtures; keep scenario-specific assertions local.

### LOC savings estimate

- Conservative first-pass reduction:
- **Estimated net savings: ~180–280 LOC**.

## Aggregate savings

### Conservative low estimate
- Priority 1: 1,660
- Priority 2: 400
- Priority 3: 420
- Priority 4: 275
- Priority 5: 165
- Priority 6: 180

**Total conservative estimate: ~3,100 LOC saved**.

### Higher-confidence upper estimate
- Priority 1: 1,660
- Priority 2: 470
- Priority 3: 520
- Priority 4: 385
- Priority 5: 250
- Priority 6: 280

**Total upper estimate: ~3,560 LOC saved**.

## Recommended execution order

1. **P1 first** (duplicate tests): immediate, low-risk, largest return.
2. **P2 + P3 together**: unify stream DSL repetition in one architectural pass.
3. **P4 next**: apply same generation strategy to higher-arity incremental constraints.
4. **P5**: selector kernels once test duplication is removed and benchmark baselines are stable.
5. **P6 continuously**: migrate fixtures opportunistically during touch-ups.

## Risk controls

- Preserve zero-erasure boundaries and current ownership model.
- Validate with `make test` after each priority slice.
- For P2–P5, benchmark hot paths before/after each refactor slice to ensure no runtime regression.
- Avoid compatibility shims; migrate all call sites directly per repository policy.
