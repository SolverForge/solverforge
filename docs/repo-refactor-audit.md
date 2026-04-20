# Repository Refactor Audit: Repetition Reduction Plan

Date: 2026-04-20

## Scope and method

I ran a structural duplication audit across the Rust workspace focused on:

1. near-identical test modules,
2. arity-specialized code generation hand-written across multiple files,
3. repeated builder/selector patterns that can be collapsed via typed helpers.

Sampling and metrics used:

- pairwise similarity over normalized non-comment lines,
- family-level LOC totals (non-empty, non-comment),
- conservative savings estimate that assumes we keep typed/monomorphized hot paths.

The goal is **deleting repeated source**, not adding abstraction that harms runtime characteristics.

---

## Highest-yield refactor opportunities

## 1) Remove duplicated test suites in `solverforge-solver` move selectors

### Evidence

The following pairs are nearly identical:

- `heuristic/move/list_change_tests.rs` vs `heuristic/move/tests/list_change.rs` (~0.94 similarity),
- `heuristic/move/sublist_change_tests.rs` vs `heuristic/move/tests/sublist_change.rs` (~0.94 similarity),
- `heuristic/move/sublist_swap_tests.rs` vs `heuristic/move/tests/sublist_swap.rs` (~0.95 similarity).

Family size today: **1202 LOC** across 6 files.

### Proposal

Pick one canonical organization for move tests and remove the parallel duplicate tree:

- keep `heuristic/move/tests/*.rs` and delete the `*_tests.rs` duplicates, or
- keep `*_tests.rs` and delete `heuristic/move/tests/*.rs`.

Then centralize shared fixtures in one helper module (`tests/common.rs`) so additional move tests stop copying setup code.

### Estimated savings

- Direct duplicate deletion: **~592 LOC** (minimum-side deletion from the 3 strongest duplicate pairs).
- Fixture extraction and de-dup inside kept tests: **~80 LOC**.

**Subtotal: ~672 LOC saved.**

---

## 2) Remove duplicated test suites in `solverforge-solver` selectors

### Evidence

Strong duplicated pairs:

- `heuristic/selector/pillar_tests.rs` vs `heuristic/selector/tests/pillar.rs` (~0.95 similarity),
- `heuristic/selector/k_opt/tests.rs` vs `heuristic/selector/tests/k_opt.rs` (~0.94 similarity).

Plus partial duplication:

- `heuristic/selector/move_selector_tests.rs` vs `heuristic/selector/tests/move_selector.rs` (~0.61 similarity).

Family size today: **1138 LOC** across 6 files.

### Proposal

- Collapse to a single selector test tree (prefer `heuristic/selector/tests/*`).
- Split reusable cases into table-driven helper functions to avoid same assertions repeated per selector type.
- Keep only one integration-style `move_selector` suite and derive specific variants from shared harness inputs.

### Estimated savings

- Direct file-level duplicate removal: **~386 LOC**.
- Partial merge in `move_selector*` test logic: **~120 LOC**.

**Subtotal: ~506 LOC saved.**

---

## 3) Replace hand-expanded n-ary stream macro files with one arity template

### Evidence

`solverforge-scoring/src/stream/arity_stream_macros/nary_stream/` has 4 large files (`bi`, `tri`, `quad`, `penta`) with heavy structural repetition.

Family size today: **1243 LOC** across 4 files.

### Proposal

Introduce a single internal meta-macro for arity patterns and keep per-arity files as tiny declarative invocations only (or fold into one file with declarative entries).

Constraints to preserve:

- zero-erasure hot path,
- concrete type propagation,
- no `dyn`/boxing in stream execution path.

### Estimated savings

- compress repeated boilerplate while retaining typed expansions: **~640 LOC**.

**Subtotal: ~640 LOC saved.**

---

## 4) Collapse higher-arity incremental constraints into generic scaffolding

### Evidence

`solverforge-scoring/src/constraint/nary_incremental/higher_arity/{tri,quad,penta}.rs` are strongly similar (0.63–0.68).

Family size today: **990 LOC** across 3 files.

### Proposal

Create one typed scaffolding layer for shared incremental lifecycle steps and keep arity-specific type bindings as thin wrappers.

Good fit:

- shared tuple update bookkeeping,
- repeated insert/retract dispatch structure,
- repeated collector wiring.

### Estimated savings

- factor shared mechanics once, preserve arity entrypoints: **~430 LOC**.

**Subtotal: ~430 LOC saved.**

---

## 5) Normalize repeated count-based termination implementations

### Evidence

`termination/move_count.rs` and `termination/score_calculation_count.rs` share substantial shape (similarity ~0.61) and follow the same state/check lifecycle structure.

### Proposal

Create a small generic counter-termination primitive with typed labels/config wrappers for each public termination mode.

### Estimated savings

**~90 LOC saved.**

---

## Aggregate LOC savings

Conservative estimate from the five tracks above:

- Move test duplication cleanup: **~672 LOC**
- Selector test duplication cleanup: **~506 LOC**
- N-ary stream macro consolidation: **~640 LOC**
- Higher-arity incremental scaffolding: **~430 LOC**
- Termination counter normalization: **~90 LOC**

## **Total estimated savings: ~2338 LOC**

A realistic delivery range is **~2.1k to ~2.6k LOC** depending on how aggressively we unify test harnesses.

---

## Implementation sequence (recommended)

1. **Test de-dup first** (tracks 1 and 2)
   - low product risk,
   - immediate LOC reduction,
   - faster CI compile/test feedback afterward.

2. **N-ary stream macro consolidation** (track 3)
   - highest maintenance burden reduction,
   - can be validated with existing stream tests.

3. **Higher-arity incremental scaffolding** (track 4)
   - perform after stream macro cleanup to avoid overlapping churn.

4. **Termination counter normalization** (track 5)
   - quick cleanup pass at end.

---

## Guardrails for the refactor

To keep behavior and performance stable while doing ground-up cleanup:

- preserve zero-erasure boundaries and concrete typing in solver/scoring hot paths,
- do not introduce trait-object indirection in frequently executed move/score loops,
- verify with `make test` and targeted benchmark tests around scoring/director modules,
- keep public API unchanged unless a follow-up API review is explicitly approved.
