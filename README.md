# SolverForge

<div align="center">

[![CI](https://github.com/solverforge/solverforge/actions/workflows/ci.yml/badge.svg)](https://github.com/solverforge/solverforge/actions/workflows/ci.yml)
[![Release](https://github.com/solverforge/solverforge/actions/workflows/release.yml/badge.svg)](https://github.com/solverforge/solverforge/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solverforge.svg)](https://crates.io/crates/solverforge)
[![Documentation](https://docs.rs/solverforge/badge.svg)](https://docs.rs/solverforge)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/solverforge.svg)](https://crates.io/crates/solverforge)

</div>

> **Used in Production**
>
> "Working like a charm, A+" — *Dr. Fawaz Halwani, Pathologist, The Ottawa Hospital*

A zero-erasure constraint solver in Rust.

SolverForge optimizes planning and scheduling problems using metaheuristic algorithms. It combines a declarative constraint API with efficient incremental scoring to solve complex real-world problems like employee scheduling, vehicle routing, and resource allocation.

## Get Started

```bash
cargo install solverforge-cli
solverforge new my-scheduler --standard
cd my-scheduler
solverforge server
```

Open http://localhost:7860 to see your solver in action.

Start new projects with the standalone [`solverforge-cli`](https://github.com/solverforge/solverforge-cli) repository. It scaffolds complete SolverForge applications and sample data, while this repository provides the runtime crates you extend once the scaffold exists.

The current templates cover standard-variable and list-heavy planning models, and the generated code targets the same unified runtime. Use `solverforge generate` to add entities, facts, and constraints interactively.

## Extend the Scaffold

- [Extend the solver](docs/extend-solver.md) when you need custom constraints, phases, selectors, termination, or solver configuration beyond the default scaffold.
- [Extend the domain](docs/extend-domain.md) when you need more entities, facts, variables, scoring, or mixed standard/list modeling inside the generated app.

## Documentation Map

- `README.md` is the user-facing entry point for the workspace and generated-project integration model.
- `docs/extend-solver.md` and `docs/extend-domain.md` cover scaffold extension workflows.
- `docs/typed-contract-audit.md` records the current neutral selector and extractor naming model, including the `EntityCollectionExtractor`, `ValueSelector`, and `MoveSelector` surface adopted in `0.7.0`.
- `crates/*/WIREFRAME.md` files are the canonical public API maps for each crate.
- `AGENTS.md` defines repository-level engineering and documentation expectations for coding agents.

## Zero-Erasure Architecture

SolverForge preserves concrete types through the entire solver pipeline:

- **No trait objects** (`Box<dyn Trait>`, `Arc<dyn Trait>`)
- **No runtime dispatch** - all generics resolved at compile time
- **No hidden allocations** - moves, scores, and constraints are stack-allocated
- **Predictable performance** - no GC pauses, no vtable lookups

This enables aggressive compiler optimizations and cache-friendly data layouts.

Current public naming follows neutral Rust contracts rather than `Typed*` prefixes. The object-safe descriptor boundary is still intentional, but the concrete adapter and selector surface are now documented as `EntityCollectionExtractor`, `ValueSelector`, and `MoveSelector`. The historical rename and rationale are captured in [docs/typed-contract-audit.md](docs/typed-contract-audit.md).

## Features

- **Score Types**: SimpleScore, HardSoftScore, HardMediumSoftScore, BendableScore, HardSoftDecimalScore
- **ConstraintStream API**: Declarative constraint definition with fluent builders
- **SERIO Engine**: Scoring Engine for Real-time Incremental Optimization
- **Solver Phases**:
  - Construction Heuristics for standard, list, and mixed planning models
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing, Diversified Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded via rayon)
  - VND (Variable Neighborhood Descent)
- **Move System**: Zero-allocation typed moves with arena-based ownership
  - Standard: ChangeMove, SwapMove, PillarChangeMove, PillarSwapMove, RuinMove
  - List: ListChangeMove, ListSwapMove, SubListChangeMove, SubListSwapMove, KOptMove, ListRuinMove
  - Nearby selection for list moves
- **SolverManager API**: Retained job / snapshot / checkpoint lifecycle with exact pause/resume, lifecycle-complete events, snapshot retrieval, snapshot-bound analysis, and telemetry
- **Derive Macros**: `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Configuration**: TOML support with builder API
- **Console Output**: Colorful tracing-based progress display with solve telemetry

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge = { version = "0.8", features = ["console"] }
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `console` | Colorful console output with progress tracking |
| `verbose-logging` | DEBUG-level progress updates (1/sec during local search) |
| `decimal` | Decimal score support via `rust_decimal` |
| `serde` | Serialization support for domain types |

## Release Operations

The workspace release checklist, publish order, and crate stability matrix live in [RELEASE.md](RELEASE.md). Version bumps are explicit release decisions, and `CHANGELOG.md` remains managed by the `commit-and-tag-version` workflow rather than hand-maintained in feature branches.

## Quick Start

### 1. Define Your Domain Model

```rust
use solverforge::prelude::*;

#[problem_fact]
pub struct Employee {
    #[planning_id]
    pub id: i64,
    pub name: String,
    pub skills: Vec<String>,
}

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,
    pub required_skill: String,
    pub start: i64,
    pub end: i64,
    #[planning_variable]
    pub employee: Option<i64>,
}

#[planning_solution]
pub struct Schedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,
    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

### 2. Define Constraints

The `#[planning_solution]` macro generates a `ScheduleConstraintStreams` trait with typed accessors for each collection field, so `factory.shifts()` replaces manual `for_each` extractors:

```rust
use solverforge::{ConstraintSet, HardSoftScore};
use ScheduleConstraintStreams; // generated by #[planning_solution]
use solverforge::stream::{joiner::*, ConstraintFactory};

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let required_skill = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .shifts()
        .join((
            |s: &Schedule| &s.employees,
            equal_bi(
                |shift: &Shift| shift.employee,
                |emp: &Employee| Some(emp.id),
            ),
        ))
        .filter(|shift: &Shift, emp: &Employee| {
            !emp.skills.contains(&shift.required_skill)
        })
        .penalize_hard()
        .named("Required skill");

    let no_overlap = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .shifts()
        .join(equal(|shift: &Shift| shift.employee))
        .filter(|a: &Shift, b: &Shift| {
            a.employee.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize_hard()
        .named("No overlap");

    (required_skill, no_overlap)
}
```

### 3. Solve

```rust
use solverforge::{SolverEvent, SolverManager, Solvable};

static MANAGER: SolverManager<Schedule> = SolverManager::new();

fn main() {
    let schedule = Schedule {
        employees,
        shifts,
        score: None,
    };

    let (job_id, mut receiver) = MANAGER.solve(schedule).unwrap();
    let mut pause_requested = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Progress { metadata } => {
                println!("job {} state {:?}", metadata.job_id, metadata.lifecycle_state);
                if !pause_requested && metadata.telemetry.step_count >= 10_000 {
                    MANAGER.pause(job_id).unwrap();
                    pause_requested = true;
                }
            }
            SolverEvent::BestSolution { metadata, .. } => {
                if let Some(snapshot_revision) = metadata.snapshot_revision {
                    let analysis = MANAGER
                        .analyze_snapshot(job_id, Some(snapshot_revision))
                        .unwrap();
                    println!(
                        "job {} snapshot {} score {}",
                        metadata.job_id,
                        snapshot_revision,
                        analysis.analysis.score
                    );
                }
            }
            SolverEvent::PauseRequested { metadata } => {
                println!("pause requested for job {}", metadata.job_id);
            }
            SolverEvent::Paused { metadata } => {
                let snapshot = MANAGER
                    .get_snapshot(job_id, metadata.snapshot_revision)
                    .unwrap();
                println!(
                    "job {} paused at snapshot {}",
                    metadata.job_id,
                    snapshot.snapshot_revision
                );
                MANAGER.resume(job_id).unwrap();
            }
            SolverEvent::Resumed { metadata } => {
                println!("job {} resumed", metadata.job_id);
            }
            SolverEvent::Completed { metadata, .. } => {
                println!("job {} completed", metadata.job_id);
                break;
            }
            SolverEvent::Cancelled { metadata } => {
                println!("job {} cancelled", metadata.job_id);
                break;
            }
            SolverEvent::Failed { metadata, error } => {
                println!("job {} failed: {}", metadata.job_id, error);
                break;
            }
        }
    }
}
```

`pause()` settles at a runtime-owned safe boundary and `resume()` continues from that exact in-process checkpoint. Snapshot analysis is always revision-bound: you analyze a retained `snapshot_revision`, never the live mutable job directly.

## Console Output

With `features = ["console"]`, SolverForge displays colorful progress:

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.8.0 - Zero-Erasure Constraint Solver

  0.000s ▶ Solving │ 14 entities │ 5 values │ scale 9.799 x 10^0
  0.001s ▶ Construction Heuristic started
  0.002s ◀ Construction Heuristic ended │ 1ms │ 14 steps │ 14,000/s │ 0hard/-50soft
  0.002s ▶ Late Acceptance started
  1.002s ⚡    12,456 steps │      445,000/s │ -2hard/8soft
  2.003s ⚡    24,891 steps │      448,000/s │ 0hard/12soft
 30.001s ◀ Late Acceptance ended │ 30.00s │ 104,864 steps │ 456,000/s │ 0hard/15soft
 30.001s ■ Solving complete │ 0hard/15soft │ FEASIBLE

╔══════════════════════════════════════════════════════════╗
║                 FEASIBLE SOLUTION FOUND                  ║
╠══════════════════════════════════════════════════════════╣
║  Final Score:                            0hard/15soft    ║
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

## Score Types

```rust
use solverforge::prelude::*;

// Single-level score
let score = SimpleScore::of(-5);

// Two-level score (hard + soft)
let score = HardSoftScore::of(-2, 100);
assert!(!score.is_feasible());  // Hard score < 0

// Three-level score
let score = HardMediumSoftScore::of(0, -50, 200);

// Decimal precision
let score = HardSoftDecimalScore::of(dec!(0), dec!(-123.45));

// N-level configurable
let score = BendableScore::new(vec![0, -1], vec![-50, -100]);
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

## SolverManager API

The `SolverManager` owns the retained runtime lifecycle for each job. The 0.8 contract uses neutral `job`, `snapshot`, and `checkpoint` terminology throughout the public API. `pause()` settles at a runtime-owned safe boundary and `resume()` continues from the exact in-process checkpoint rather than restarting from the best solution. Declare a `static` instance so it satisfies the `'static` lifetime requirement:

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

Lifecycle events carry `job_id`, monotonic `event_sequence`, `snapshot_revision`, telemetry, and authoritative lifecycle state. Progress metadata reflects the current runtime state, including `PauseRequested` while a pause is settling. Snapshot analysis is always bound to a retained `snapshot_revision`, whether the job is still solving, pause-requested, paused, or already terminal, and analysis availability must never be treated as proof that a job has completed. `delete` is reserved for cleanup of terminal jobs only: it removes the retained job from the public API immediately, and the underlying slot becomes reusable once the worker has fully exited.

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

See the [`examples/`](examples/) directory:

- **N-Queens**: Classic constraint satisfaction problem

```bash
cargo run -p nqueens
```

For project scaffolding and end-to-end application templates, use the standalone [`solverforge-cli`](https://github.com/solverforge/solverforge-cli) repository: `cargo install solverforge-cli`, then `solverforge new ...`.

## Performance

SolverForge leverages Rust's zero-cost abstractions:

- **Typed Moves**: Values stored inline, no boxing, arena-based ownership (never cloned)
- **RPITIT Selectors**: Return-position impl Trait eliminates `Box<dyn Iterator>` from all selectors
- **Incremental Scoring**: SERIO propagates only changed constraints
- **No GC**: Predictable latency without garbage collection
- **Cache-friendly**: Contiguous memory layouts for hot paths
- **No vtable dispatch**: Monomorphized score directors, deciders, and bounders

Typical throughput: 300k-1M moves/second depending on constraint complexity for scheduling; 2.5M+ moves/second on VRP

## Status

**Current Version**: 0.8.0

### What's New in 0.8.0

- **Retained runtime lifecycle contract**: `SolverManager` now models a retained job lifecycle around exact in-process checkpoints. `pause()` and `resume()` operate on runtime-owned checkpoints instead of restart-from-best semantics.
- **Neutral lifecycle terminology**: public docs and APIs now speak in terms of jobs, snapshots, and checkpoints rather than schedule-specific runtime terms.
- **Lifecycle-complete event stream**: retained jobs now emit `Progress`, `BestSolution`, `PauseRequested`, `Paused`, `Resumed`, `Completed`, `Cancelled`, and `Failed` with authoritative lifecycle metadata and monotonic `event_sequence` / `snapshot_revision`.
- **Snapshot-bound analysis across retained states**: `analyze_snapshot()` is revision-specific and remains available for retained snapshots while a job is active or terminal. Analysis is informational, not a terminal-state signal.
- **Breaking runtime entrypoint**: manual retained-runtime implementations now use `Solvable::solve(self, runtime: SolverRuntime<Self>)`, and `SolverManager::solve()` returns `(job_id, receiver)` so consumers can coordinate lifecycle state and snapshot analysis explicitly.

### What's New in 0.7.0

- Release notes are managed in `CHANGELOG.md` by commit-and-tag workflow.

- **Modern CLI templates**: The standalone CLI scaffolds standard variable and list variable projects via `solverforge new --standard ...` and `solverforge new --list ...`. The shipped templates use the config-driven retained `SolverManager` + `Solvable` + `solver.toml` API. No manual solver loops, no sub-crate imports — only the `solverforge` facade crate.
- **Generated domain accessors**: `#[planning_solution]` generates a `{Name}ConstraintStreams` trait with typed `.field_name()` methods on `ConstraintFactory` — e.g., `factory.shifts()` instead of `factory.for_each(|s| &s.shifts)`
- **Ergonomic extractors**: `CollectionExtract<S>` trait accepts both `|s| s.field.as_slice()` and `|s| &s.field` (via `vec(|s| &s.field)`) — no forced `.as_slice()` at every call site
- **Generated `.unassigned()` filter**: entities with `Option` planning variables get a `{Entity}UnassignedFilter` trait — e.g., `factory.shifts().unassigned()` filters to unassigned entities
- **Convenience scoring**: `penalize_hard()`, `penalize_soft()`, `reward_hard()`, `reward_soft()` on all stream types
- **Unified `.join(target)`**: single join method dispatching on argument type — `equal(|a| key)` for self-join, `(extractor_b, equal_bi(ka, kb))` for keyed cross-join, `(other_stream, |a, b| pred)` for predicate join
- **`.named("name")`**: sole finalization method on all builders (replaces `as_constraint`)
- **Score trait**: `one_hard()`, `one_soft()`, `one_medium()` default methods
- **Joiners**: `equal`, `equal_bi`, `less_than`, `less_than_or_equal`, `greater_than`, `greater_than_or_equal`, `overlapping`, `filtering`, with `.and()` composition
- **Conditional existence**: `if_exists_filtered()`, `if_not_exists_filtered()` with joiner-based matching

### What's New in 0.5.15

- `solverforge-cvrp` wired into the facade: `solverforge::cvrp::VrpSolution`, `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`, and all CVRP free functions now accessible from the main crate
- Fixed circular dependency: `solverforge-cvrp` now depends on `solverforge-solver` directly instead of the facade

### What's New in 0.5.14

- Added `ListKOptPhase`, `solverforge-cvrp` library, and fixed doctest signatures

### What's New in 0.5.7

- API cleanup: ~1500-1900 LOC removed across scoring and solver crates
- Consolidated tri/quad/penta n-ary constraints and arity stream macros into unified macro files
- Deleted `ShadowAwareScoreDirector`, `ScoreDirectorFactory` (dead wrappers)
- Trimmed `ScoreDirector` trait: removed `variable_name` param, `before/after_entity_changed`, `trigger_variable_listeners`, `get_entity`; deleted dead pinning infrastructure
- Eliminated `Box<dyn Acceptor<S>>` via `AnyAcceptor<S>` enum in `AcceptorBuilder`
- Removed `run_solver_with_channel`; collapsed `standard.rs` solve overloads
- Deleted dead `termination_fn` field/methods from `SolverScope`
- Added `WIREFRAME.md` canonical API references for all crates

### What's New in 0.5.6

- Fixed `GroupedUniConstraint` new-group `old_score` computation (was using `-weight(empty)` instead of `Sc::zero()`, causing phantom positive deltas)
- Fixed `UniConstraintStream::group_by()` silently dropping accumulated filters (`.filter().group_by()` now works correctly)
- Added `#[allow(too_many_arguments)]` on `GroupedUniConstraint::new` to suppress lint

### What's New in 0.5.5

- Fixed incremental scoring corruption when multiple entity classes are present — `on_insert`/`on_retract` notifications now filtered by `descriptor_index` in all constraint types (`IncrementalUniConstraint`, `GroupedUniConstraint`, all nary variants)
- `UniConstraintStream::for_descriptor(idx)` exposed in stream builder API

### What's New in 0.5.4

- Deleted dynamic/cranelift and stub dotfile artifacts (internal cleanup)

### What's New in 0.5.3

- Move streaming for never-ending selectors: local search no longer stalls when selectors produce moves lazily without exhausting

### What's New in 0.5.2

**New Features:**
- **Ruin-and-Recreate (LNS)**: `ListRuinMove` for Large Neighborhood Search on list variables
- **Nearby Selection**: Proximity-based list change/swap selectors for improved VRP solving
- **EitherMove**: Monomorphized union of ChangeMove + SwapMove with `UnionMoveSelector` for mixed move neighborhoods
- **Simulated Annealing**: Rewritten with true Boltzmann distribution
- **Telemetry**: `SolveResult` with solve statistics (moves/sec, calc/sec, acceptance rate)
- **Best Solution Callback**: `with_best_solution_callback()` on Solver for real-time progress streaming
- **DiminishedReturns Termination**: Terminate when score improvement rate falls below threshold

**Zero-Erasure Deepening:**
- Eliminated all `Box<dyn Iterator>` from selectors via RPITIT (return-position impl Trait in trait)
- Monomorphized `RecordingScoreDirector` and exhaustive search decider/bounder (no more vtable dispatch)
- Replaced `Arc<RwLock>` in MimicRecorder with `Cell` + manual refcount
- Removed `Rc` from SwapMoveSelector (eager triangular pairing)
- `PhantomData<fn() -> T>` applied across all types to prevent inherited trait bounds

**Performance:**
- Eliminated Vec clones in KOptMove, SubListChangeMove, and SubListSwapMove hot paths
- Fixed 6 hot-path regressions in local search and SA acceptor
- Score macros (`impl_score_ops!`, `impl_score_scale!`, `impl_score_parse!`) reduce codegen

**Fixes:**
- Construction heuristic and local search producing 0 steps (entity_count wiring)
- Overflow panics in IntegerRange, ValueRangeDef, and date evaluation
- Correct `Acceptor::is_accepted` signature (`&mut self`)

### What's New in 0.5.1

- Removed `filter_with_solution()` in favor of shadow variables on entities

### What's New in 0.5.0

- Zero-erasure architecture across entire solver pipeline
- ConstraintStream API with incremental SERIO scoring
- Channel-based SolverManager API with `analyze()` for score analysis
- Console output with tracing-based progress display
- Solution-aware filter traits
- Macro-based codegen for N-ary incremental constraints

### Component Status

| Component | Status |
|-----------|--------|
| Score types | Complete |
| Domain model macros | Complete |
| ConstraintStream API | Complete |
| SERIO incremental scoring | Complete |
| Construction heuristics (7 types) | Complete |
| Local search (7 acceptors, 5 foragers) | Complete |
| Exhaustive search | Complete |
| Partitioned search | Complete |
| VND | Complete |
| Move system (12 move types) | Complete |
| Nearby selection | Complete |
| Ruin-and-recreate (LNS) | Complete |
| Selector decorators (8 types) | Complete |
| Termination | Complete |
| SolverManager | Complete |
| Score Analysis (`analyze()`) | Complete |
| Solve telemetry | Complete |
| Console output | Complete |

## Minimum Rust Version

Rust 1.80 or later.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Please open an issue or pull request.

## Acknowledgments

Inspired by [Timefold](https://timefold.ai/) (formerly OptaPlanner).
