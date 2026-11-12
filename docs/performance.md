# Performance and zero-erasure design

SolverForge keeps the native solver and scoring pipeline concrete. Solver,
selector, move, provider and scoring generics resolve at compile time, so the
compiler can inline across the hot path and lay data out contiguously. There is
no garbage collector, so latency is predictable.

## What stays concrete

- **Moves.** Values stay concrete and inline. Selector cursors own discovered
  candidates; the selected move leaves its cursor by value.
- **Selectors.** GAT-based concrete cursors expose stable candidate IDs and
  borrowable moves, with no `Box<dyn Iterator>` in selector hot paths.
- **Scoring.** SERIO propagates only changed constraints, and retained scoring
  state holds no trait-object allocations.
- **Runtime carriers.** Score directors, deciders, bounders, move carriers and
  selector carriers are monomorphized. Retained forager state is explicit.
- **Order.** Canonical list, nearby-list and sublist enumeration is
  deterministic, so seeded local search is reproducible.

## Where erasure is intentional

A few boundaries use dynamic dispatch on purpose. They sit at integration
edges, not in the move-evaluation loop:

- dynamic scalar and list access and metadata, for host-language models;
- configured candidate metrics for `Sorted` and `Probabilistic` selector
  ordering;
- host-language compound providers. Native Rust `ScalarCandidateProvider` and
  `RepairProvider` functions stay concrete function pointers and carry typed
  `ScalarEdit` values straight into the shared cursor normalizer;
- the object-safe `EntityExtractor`, for runtime-erased `SolutionDescriptor`
  access only. Generated runtime and scoring-stream paths keep their concrete
  collection and selector types; the public concrete adapter and selector APIs
  are `EntityCollectionExtractor` and `MoveSelector`;
- scorer-agnostic `&dyn Director` callbacks, real-time problem changes,
  analysis payloads and preserved panic payloads.

`AGENTS.md` lists every allowed boundary and is the rule contributors follow.

Typed row operators have their own ownership contract, described in
[Extending typed relational scoring](extend-scoring.md). Equality conditions
use indexed candidates; arbitrary predicates scan the opposite input. The
recursive operator tree supports owned non-Clone projection values on either
join input through a fresh full-evaluation owner. It does not yet replace every
public fluent stream family.

## Parallel move evaluation

Local search can evaluate candidates on several cores within one solve:

```toml
move_thread_count = { count = 4 }
```

Python configs through solverforge-py use `{"move_thread_count": {"count": 4}}`.
Each step's trials then run on up to four evaluators while one coordinator
keeps the same acceptance order, so results are reproducible at fixed work. The
count is bounded by the solve's Rayon threads and by 32, the size of one
evaluation window.

The default, `auto`, uses up to four evaluators and decides per step, from
measured costs, whether parallel windows beat sequential evaluation. The result
is the same either way.

## Measuring

Absolute throughput depends on the model, constraints, neighborhood, hardware
and termination policy. Use the generated and evaluated throughput a run emits
to understand that run, and the controlled benchmark gate below to decide
whether a change regressed.

### The zero-regression gate

`make pre-release` is portable across supported development hosts. Release
qualification additionally requires `make bench-zero-regression` on a
controlled Linux host. A host that cannot run it has not completed release
qualification.

The gate needs `taskset`, `/usr/bin/time`, `valgrind`, readable `/sys` CPU
topology, the selected `BENCH_CPU` (CPU 10 by default) and passwordless `sudo`.
It uses `sudo` to give that CPU and its hyperthread siblings to the measurement
alone through an isolated cgroup cpuset partition. `perf` counters are
collected when permitted, each trial's counters averaged over `PERF_REPEATS`
runs (45 by default).

How it measures:

1. It builds an independently linked binary for each case. Both sides share
   one package name, so identical sources produce identical symbol mangling
   and code layout, and the report records per-case `.text` SHA-256
   fingerprints for both binaries.
2. It alternates paired trials.
3. It verifies full-enumeration candidate count and order separately from
   first-fit work.
4. It requires exact equality for the deterministic allocation counters.
5. It grades wall-time and hardware instruction medians against the
   `BENCH_TOLERANCE_PERCENT` median regression budget (5% by default).
6. It grades cache and branch behavior on Cachegrind's simulated instructions,
   L1 misses, last-level misses and branch mispredictions against the same
   budget. The simulation repeats to within a count on a shared machine; the
   hardware cache and branch counters stay in the report for diagnosis only.

The default cases compile a `SolverConfig` through the same runtime compilation
a configured solve runs, then enumerate the compiled runtime's own cursors
against an empty constraint set:

- the local-search composition cursor over the compiled list and scalar
  neighborhood leaves: list change and swap, nearby change and swap, sublist
  change and swap, k-opt, union, Cartesian, scalar change and scalar
  ruin/recreate;
- the scalar construction placer;
- a first-fit construction solve through the compiled runtime phase runner.

The opt-in `BENCH_CASES="scoring scoring_weights_one scoring_weights_mixed"`
cases measure incremental scoring churn through uni, join, shared grouped and
balance constraints, with and without runtime constraint weights. Both sides of
the weighted cases are built with the weight API, so a baseline without it is
refused rather than compared unweighted.

Neighborhood selector cleanup is gated by this benchmark. Shared support code
handles exact sizing and reusable bookkeeping, while the move-enumeration hot
loops for list and sublist neighborhoods stay explicit and monomorphized.
