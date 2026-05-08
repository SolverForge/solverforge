# SolverForge

<div align="center">

[![CI](https://github.com/solverforge/solverforge-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/solverforge/solverforge-rs/actions/workflows/ci.yml)
[![Release](https://github.com/solverforge/solverforge-rs/actions/workflows/release.yml/badge.svg)](https://github.com/solverforge/solverforge-rs/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solverforge.svg)](https://crates.io/crates/solverforge)
[![Documentation](https://docs.rs/solverforge/badge.svg)](https://docs.rs/solverforge)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/solverforge.svg)](https://crates.io/crates/solverforge)

</div>

> **Used in Production**
>
> "Working like a charm, A+" — *Dr. Fawaz Halwani, Pathologist, The Ottawa Hospital*

A Rust planning engine for problems where you need to assign scarce resources
without breaking hard rules.

Use SolverForge when the answer depends on many coupled decisions: who works
which shift, which vehicle visits which stop, which task runs on which machine,
or which resource should be reserved for which job. You describe the facts, the
planning variables, and the constraints. SolverForge searches for a feasible,
high-scoring solution while preserving concrete Rust types through the runtime.

Most new projects should start with `solverforge-cli`. This repository contains
the runtime crates, examples, and API references that the generated application
uses after the scaffold exists.

## Get Started

```bash
cargo install solverforge-cli
solverforge new my-scheduler
cd my-scheduler
solverforge generate fact resource --field category:String --field load:i32
solverforge generate entity task --field label:String --field priority:i32
solverforge generate variable resource_idx --entity Task --kind scalar --range resources --allows-unassigned
solverforge server
```

Open http://localhost:7860 to see your solver in action.

That sequence creates a complete local SolverForge application, adds a resource
fact, adds a task planning entity, gives each task an optional scalar assignment,
and starts the generated web server.

The CLI is intentionally neutral. `solverforge new <name>` gives you the shell;
`solverforge generate ...` adds facts, entities, variables, constraints, and
demo data as your domain becomes clearer. Generated applications can mix scalar
planning variables with independent planning lists, and they use the same
`solverforge` facade crate documented here.

## Extend the Scaffold

- [Extend the domain](docs/extend-domain.md) when you need more entities,
  facts, variables, scoring, or mixed scalar/list modeling inside a generated
  app.
- [Extend the solver](docs/extend-solver.md) when you need custom phases,
  selectors, termination, or solver configuration beyond the scaffold defaults.

## Documentation Map

- New users should start with this README and the complete packages under
  [`examples/`](examples/).
- Generated app work starts in the standalone
  [`solverforge-cli`](https://github.com/solverforge/solverforge-cli)
  repository, then continues against the runtime crates in this workspace.
- `docs/extend-domain.md` and `docs/extend-solver.md` cover scaffold extension
  workflows.
- `docs/lifecycle-pause-resume-contract.md` defines the retained lifecycle contract, including exact pause/resume semantics, snapshot identity, and terminal-state cleanup rules.
- `docs/naming-charter.md` is the canonical naming contract for scalar/list terminology and selector-family cleanup.
- `docs/naming-contract-audit.md` records the current neutral selector and extractor naming model.
- `crates/*/WIREFRAME.md` files are the canonical public API maps for each crate.
- `AGENTS.md` defines repository-level engineering and documentation expectations for coding agents.

## Zero-Erasure Architecture

SolverForge preserves concrete types through the entire solver pipeline:

- **No trait objects** (`Box<dyn Trait>`, `Arc<dyn Trait>`)
- **No runtime dispatch** - all generics resolved at compile time
- **No hidden allocations** - moves, scores, and constraints are stack-allocated
- **Deterministic neighborhood order** - canonical list, nearby-list, and sublist selector enumeration keeps seeded local search reproducible
- **Predictable performance** - no GC pauses, no vtable lookups

This enables aggressive compiler optimizations and cache-friendly data layouts.

Current public naming follows neutral Rust contracts rather than helper-role prefixes. The object-safe descriptor boundary is still intentional, but the concrete adapter and selector surface are now documented as `EntityCollectionExtractor`, `ValueSelector`, and `MoveSelector`. The historical rename and rationale are captured in [docs/naming-contract-audit.md](docs/naming-contract-audit.md).

## Features

- **Score Types**: SoftScore, HardSoftScore, HardMediumSoftScore, BendableScore, HardSoftDecimalScore
- **ConstraintStream API**: Declarative constraints with fluent builders, model-owned collection sources, single-source and cross-join projected scoring rows, existence checks, joins, grouping, `consecutive_runs`, and balance/complemented streams
- **SERIO Engine**: Scoring Engine for Real-time Incremental Optimization
- **Solver Phases**:
  - Generic Construction Heuristics (`FirstFit`, `CheapestInsertion`) over one mixed scalar/list runtime plan when matching list work is present, plus descriptor construction routing for scalar-only targets and specialized list phases (`ListRoundRobin`, `ListCheapestInsertion`, `ListRegretInsertion`, `ListClarkeWright`, `ListKOpt`)
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing, Diversified Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded via rayon)
  - VND (Variable Neighborhood Descent)
- **Move System**: Zero-allocation moves with cursor-scoped ownership and selected-winner materialization
  - Scalar: ChangeMove, SwapMove, PillarChangeMove, PillarSwapMove, RuinMove
  - List: ListChangeMove, ListSwapMove, SublistChangeMove, SublistSwapMove, KOptMove, ListRuinMove
  - Scalar ruin-recreate, composite moves, cartesian composition, and nearby selection for scalar and list neighborhoods
- **SolverManager API**: Retained job / snapshot / checkpoint lifecycle with exact pause/resume, lifecycle-complete events, snapshot retrieval, snapshot-bound analysis, and telemetry
- **Model Macros**: `planning_model!`, `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Configuration**: TOML/YAML support with builder API, bounded scalar candidate limits, grouped scalar move selectors, conflict-repair selectors, selector telemetry, and level-aware simulated annealing configuration
- **Console Output**: Colorful tracing-based progress display with solve telemetry

Grouped scoring weights receive both the group key and collected result:

```rust
ConstraintFactory::<Schedule, HardSoftScore>::new()
    .for_each(Schedule::shifts())
    .filter(|shift: &Shift| shift.nurse_idx.is_some())
    .group_by(
        |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
        consecutive_runs(|shift: &Shift| shift.day),
    )
    .penalize_with(|_nurse_idx: &usize, runs: &Runs| {
        let excess_days = runs
            .runs()
            .iter()
            .map(|run| run.point_count().saturating_sub(2) as i64)
            .sum();
        HardSoftScore::of_soft(excess_days)
    })
    .named("Long work streaks");
```

## Installation

If you are building directly against the runtime crates instead of starting from
`solverforge-cli`, add the facade crate to your `Cargo.toml`:

```toml
[dependencies]
solverforge = { version = "0.12.0", features = ["console"] }
```

When `move_selector` is omitted from local search or VND, the canonical runtime
uses explicit streaming defaults instead of broad exhaustive neighborhoods:

- scalar-only models default to `ChangeMoveSelector` plus `SwapMoveSelector`
- list-only models default to `NearbyListChangeMoveSelector(20)`,
  `NearbyListSwapMoveSelector(20)`, and `ListReverseMoveSelector`
- mixed models concatenate the list defaults first, then the scalar defaults

Runtime telemetry now preserves exact counts and `Duration`s through the whole
pipeline. Retained status/events expose generated, evaluated, and accepted move
counts together with generation and evaluation durations; human-facing
`moves/s` remains a display-only derived value.

Neighborhood selector cleanup is benchmark-gated in the runtime crates. Shared
support code backs exact sizing and reusable bookkeeping, while the
move-enumeration hot loops for list and sublist neighborhoods stay explicit and
monomorphized.

### Feature Flags

| Feature | Description |
|---------|-------------|
| `console` | Colorful console output with progress tracking |
| `verbose-logging` | DEBUG-level progress updates (1/sec during local search) |
| `decimal` | Decimal score support via `rust_decimal` |
| `serde` | Serialization support for domain types |

## Quick Start

The README quickstart is the checked-in
[`minimal-shift-scheduling`](examples/minimal-shift-scheduling/) package. It is
small enough to read in one sitting, but it is still a real SolverForge model:
`make examples` builds it, and the command below compiles and runs it from the
workspace.

```bash
cargo run -p minimal-shift-scheduling
```

A successful run prints a feasible nurse assignment:

```text
score: 0hard/0soft
day 0 slot 0 -> Amina
day 0 slot 1 -> Bruno
day 1 slot 0 -> Bruno
day 1 slot 1 -> Chiara
day 2 slot 0 -> Chiara
day 2 slot 1 -> Amina
day 3 slot 0 -> Amina
day 3 slot 1 -> Bruno
day 4 slot 0 -> Bruno
day 4 slot 1 -> Chiara
day 5 slot 0 -> Chiara
day 5 slot 1 -> Amina
```

Read the quickstart files in this order:

- [`src/domain/mod.rs`](examples/minimal-shift-scheduling/src/domain/mod.rs)
  declares the model with `solverforge::planning_model!`.
- [`src/domain/nurse.rs`](examples/minimal-shift-scheduling/src/domain/nurse.rs)
  defines the problem facts.
- [`src/domain/shift.rs`](examples/minimal-shift-scheduling/src/domain/shift.rs)
  defines the planning entities and the `nurse_idx` planning variable.
- [`src/domain/schedule.rs`](examples/minimal-shift-scheduling/src/domain/schedule.rs)
  owns the solution, constraints, `solver.toml` link, and coverage group hooks.
- [`solver.toml`](examples/minimal-shift-scheduling/solver.toml) configures
  coverage-first construction and local search.
- [`src/main.rs`](examples/minimal-shift-scheduling/src/main.rs) builds a small
  schedule, starts `SolverManager`, consumes solver events, and prints the
  completed assignment.

The important boundary is that SolverForge model metadata is owned by the model,
not by detached helper imports. The domain manifest lists normal Rust modules;
the `#[planning_solution]` macro generates collection source functions such as
`Schedule::shifts()`, and constraints use those functions through
`ConstraintFactory::for_each(...)`.

For a new application, start with `solverforge-cli` so the file layout,
`solver.toml`, model manifest, constraints, and frontend handoff are generated
together. Use the runtime crates directly when you want to embed SolverForge in
an existing Rust service or build lower-level integrations.

## Console Output

With `features = ["console"]`, SolverForge displays colorful progress:

The solve-start line is shape-aware: list models show `elements`, and scalar
models show average `candidates`.

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.12.0 - Zero-Erasure Constraint Solver

  0.000s ▶ Solving │ 14 entities │ 5 candidates │ scale 9.799 x 10^0
  0.001s ▶ Construction Heuristic started
  0.002s ◀ Construction Heuristic ended │ 1ms │ 14 steps │ 14,000/s │ 0hard/-50soft
  0.002s ▶ Local Search started │ 0hard/-50soft
  1.002s ⚡    12,456 steps │      445,000/s │ -2hard/8soft
  2.003s ⚡    24,891 steps │      448,000/s │ 0hard/12soft
 30.001s ◀ Local Search ended │ 30.00s │ 104,864 steps │ 456,000/s │ 0hard/15soft
 30.001s ■ Solving complete │ 0hard/15soft │ FEASIBLE

╔══════════════════════════════════════════════════════════╗
║                 FEASIBLE SOLUTION FOUND                  ║
╠══════════════════════════════════════════════════════════╣
║  Final Score:                            0hard/15soft    ║
║  Moves Generated:                            104,864     ║
║  Steps:                                      104,864     ║
║  Generation Time:                              1.24s     ║
║  Evaluation Time:                             28.76s     ║
║  Moves/s:                                    456,000     ║
║  Moves Evaluated:                            104,864     ║
║  Moves Accepted:                              12,456     ║
║  Score Calcs:                                104,864     ║
║  Acceptance:                                  11.9%      ║
╚══════════════════════════════════════════════════════════╝
```

### Log Levels

| Level | Content | When |
|-------|---------|------|
| INFO | Lifecycle events (solve/phase start/end) | Default |
| DEBUG | Progress updates (1/sec with speed and score) | `verbose-logging` feature |
| TRACE | Individual move evaluations | `RUST_LOG=solverforge_solver=trace` |

## Architecture

![SERIO incremental scoring](assets/SERIO.jpg)

```
┌─────────────────────────────────────────────────────────────────┐
│                         solverforge                             │
│                    (facade + re-exports)                        │
└─────────────────────────────────────────────────────────────────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌──────────────┬──────────────┬──────────────┬──────────────┐
│solverforge-  │solverforge-  │solverforge-  │solverforge-  │
│   solver     │   scoring    │   config     │   console    │
│              │              │              │              │
│ • Phases     │ • Constraint │ • TOML       │ • Banner     │
│ • Moves      │   Streams    │ • Builders   │ • Tracing    │
│ • Selectors  │ • Score      │              │ • Progress   │
│ • Acceptors  │   Directors  │              │              │
│ • Termination│ • SERIO      │              │              │
│ • Manager    │   Engine     │              │              │
│ • Telemetry  │              │              │              │
└──────────────┴──────────────┴──────────────┴──────────────┘
        │              │
        └──────┬───────┘
               ▼
        ┌──────────────────────────────┐
        │       solverforge-core       │
        │                              │
        │ • Score types                │
        │ • Domain traits              │
        │ • Descriptors                │
        │ • Variable system            │
        └──────────────────────────────┘
                       │
                       ▼
        ┌──────────────────────────────┐
        │      solverforge-macros      │
        │                              │
        │ • planning_model!           │
        │ • #[planning_solution]       │
        │ • #[planning_entity]         │
        │ • #[problem_fact]            │
        └──────────────────────────────┘
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `solverforge` | Main facade with prelude and re-exports |
| `solverforge-core` | Core types: scores, domain traits, descriptors |
| `solverforge-solver` | Solver engine: phases, moves, termination, SolverManager, telemetry |
| `solverforge-scoring` | ConstraintStream API, SERIO incremental scoring |
| `solverforge-config` | Configuration via TOML and builder API |
| `solverforge-console` | Tracing-based console output with banner and progress display |
| `solverforge-macros` | Procedural macros for domain model |
| `solverforge-cvrp` | CVRP domain helpers: `VrpSolution`, `ProblemData`, distance meters, feasibility functions |
| `solverforge-test` | Shared test fixtures for workspace crates |

## Score Types

```rust
use solverforge::prelude::*;

// Single-level score
let score = SoftScore::of(-5);

// Two-level score (hard + soft)
let score = HardSoftScore::of(-2, 100);
assert!(!score.is_feasible());  // Hard score < 0

// Three-level score
let score = HardMediumSoftScore::of(0, -50, 200);

// Decimal precision
let score = HardSoftDecimalScore::of_scaled(0, -12_345_000);

// N-level configurable
let score = BendableScore::<2, 2>::of([0, -1], [-50, -100]);
```

## Termination

Configure via `solver.toml`:

```toml
[termination]
seconds_spent_limit = 30
unimproved_seconds_spent_limit = 5
step_count_limit = 10000
```

Or programmatically:

```rust
let config = SolverConfig::load("solver.toml").unwrap_or_default();
```

For macro-generated retained solves, the solution module listed by
`planning_model!` can use `config = "..."` to decorate the loaded `solver.toml`
config instead of replacing it:

```rust
#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_solution"
)]
pub struct Schedule {
    // ...
}

fn solver_config_for_solution(solution: &Schedule, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}
```

## SolverManager API

The `SolverManager` owns the retained runtime lifecycle for each job. The public contract uses neutral `job`, `snapshot`, and `checkpoint` terminology throughout the API. `pause()` settles at a runtime-owned safe boundary and `resume()` continues from the exact in-process checkpoint rather than restarting from the best solution. Built-in search phases now poll retained-runtime control during large neighborhood generation and evaluation, so config time limits, `pause()`, and `cancel()` unwind promptly without app-side watchdogs. Declare a `static` instance so it satisfies the `'static` lifetime requirement:

```rust
use solverforge::{SolverLifecycleState, SolverManager, SolverStatus, Solvable};

static MANAGER: SolverManager<MySchedule> = SolverManager::new();

let (job_id, mut events) = MANAGER.solve(problem).unwrap();

let status: SolverStatus<_> = MANAGER.get_status(job_id).unwrap();
assert_eq!(status.lifecycle_state, SolverLifecycleState::Solving);

MANAGER.pause(job_id).unwrap();
MANAGER.resume(job_id).unwrap();
MANAGER.cancel(job_id).unwrap();

let snapshot = MANAGER.get_snapshot(job_id, None).unwrap();
let analysis = MANAGER.analyze_snapshot(job_id, Some(snapshot.snapshot_revision)).unwrap();
MANAGER.delete(job_id).unwrap();
```

Lifecycle events carry `job_id`, monotonic `event_sequence`, `snapshot_revision`, telemetry, and authoritative lifecycle state. Progress metadata reflects the current runtime state, including `PauseRequested` while a pause is settling. After `pause()` is accepted, the stream delivers `PauseRequested` before any later worker-side event already published in `PauseRequested` state. Snapshot analysis is always bound to a retained `snapshot_revision`, whether the job is still solving, pause-requested, paused, or already terminal, and analysis availability must never be treated as proof that a job has completed. `delete` is reserved for cleanup of terminal jobs only: it removes the retained job from the public API immediately, and the underlying slot becomes reusable once the worker has fully exited.

## Score Analysis

Analyze solutions without solving:

```rust
use solverforge::analyze;

let analysis = analyze(&solution);

println!("Score: {}", analysis.score);
for constraint in &analysis.constraints {
    println!("  {}: {}", constraint.name, constraint.score);
}
```

## Examples

Root workspace examples live under [`examples/`](examples/) as complete solver
packages:

```bash
make examples
cargo run -p scalar-graph-coloring
cargo run -p minimal-shift-scheduling
cargo run -p list-tsp
cargo run -p mixed-job-shop
cargo run -p nqueens
```

`make examples` builds every checked-in example package. Run an individual
`cargo run -p ...` command when you want to inspect one model's behavior.

`minimal-shift-scheduling` is the compact public solver path: it uses
`planning_model!`, generated collection sources, `solver.toml`,
`CoverageGroup`, `consecutive_runs`, and `SolverManager`.

For project scaffolding and end-to-end application templates, use the standalone [`solverforge-cli`](https://github.com/solverforge/solverforge-cli) repository: `cargo install solverforge-cli`, then `solverforge new ...`.

## Performance

SolverForge leverages Rust's zero-cost abstractions:

- **Zero-Erasure Moves**: Values stored inline, no boxing, arena-based ownership (never cloned)
- **RPITIT Selectors**: Return-position impl Trait eliminates `Box<dyn Iterator>` from all selectors
- **Incremental Scoring**: SERIO propagates only changed constraints
- **No GC**: Predictable latency without garbage collection
- **Cache-friendly**: Contiguous memory layouts for hot paths
- **No vtable dispatch**: Monomorphized score directors, deciders, and bounders

Typical throughput: 300k-1M moves/second depending on constraint complexity for scheduling; 2.5M+ moves/second on VRP

## Status

**Current workspace version:** 0.12.0

The current checked-in workspace exposes:

- the `solverforge` facade crate for application code;
- `solverforge-cli` as the recommended first project path;
- model-owned `planning_model!` domains with generated collection sources;
- scalar, list, grouped-scalar, coverage, conflict-repair, ruin-recreate,
  cartesian, and nearby neighborhoods;
- retained `SolverManager` jobs with progress events, exact pause/resume,
  snapshots, checkpoint status, telemetry, and snapshot-bound analysis;
- TOML/YAML solver configuration plus programmatic overrides;
- optional console rendering through the `console` feature; and
- five workspace examples validated by `make examples`.

Release history belongs in [CHANGELOG.md](CHANGELOG.md). Maintainer release
steps live in [RELEASE.md](RELEASE.md).

## Release Operations

The workspace release checklist, publish order, and crate stability matrix live
in [RELEASE.md](RELEASE.md). Version bumps are explicit release decisions, and
`CHANGELOG.md` remains managed by the `commit-and-tag-version` workflow rather
than hand-maintained in feature branches.

## Minimum Rust Version

Rust 1.95 or later.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Please open an issue or pull request.
