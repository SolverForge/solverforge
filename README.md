# SolverForge

<div align="center">

[![CI](https://github.com/solverforge/solverforge/actions/workflows/ci.yml/badge.svg)](https://github.com/solverforge/solverforge/actions/workflows/ci.yml)
[![Release](https://github.com/solverforge/solverforge/actions/workflows/release.yml/badge.svg)](https://github.com/solverforge/solverforge/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solverforge.svg)](https://crates.io/crates/solverforge)
[![Documentation](https://docs.rs/solverforge/badge.svg)](https://docs.rs/solverforge)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/solverforge.svg)](https://crates.io/crates/solverforge)

</div>

> **Used in Production**
>
> "Working like a charm, A+" — *Dr. Fawaz Halwani, Pathologist, The Ottawa Hospital*

A zero-erasure constraint solver in Rust.

SolverForge optimizes planning and scheduling problems using metaheuristic algorithms. It combines a declarative constraint API, retained projected scoring rows, bounded scalar neighborhoods, and incremental scoring to solve complex real-world problems like employee scheduling, vehicle routing, and resource allocation.

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

Start new projects with the standalone [`solverforge-cli`](https://github.com/solverforge/solverforge-cli) repository. It scaffolds complete SolverForge applications and sample data, while this repository provides the runtime crates you extend once the scaffold exists.

The current CLI scaffolds a neutral shell via `solverforge new <name>`. You shape that shell afterward with `solverforge generate ...`, adding facts, entities, variables, constraints, and generated data as the domain becomes concrete. Generated applications can mix scalar planning variables with multiple independent planning lists, and the emitted code targets the same retained-runtime facade documented in this repository.
The generated runtime now builds one `ModelContext` for every planning model. Scalar runtime metadata is ordered by descriptor index and variable name, with a compact generated index reserved for getter/setter dispatch, so module declaration order is not part of the user contract. Generic `FirstFit` and `CheapestInsertion` use the canonical construction engine when matching list work is present, while pure scalar construction uses the descriptor-scalar boundary. Canonical local search runs over the typed `ModelContext`; descriptor-scalar selectors remain an explicit descriptor engine. Specialized list heuristics such as round-robin, regret insertion, Clarke-Wright, and list K-opt remain explicit opt-in phases.
Scalar variables declared with `allows_unassigned = true` keep optional-assignment semantics in that runtime: stock construction can keep `None` when it is the best legal baseline, revision-advancing mutations reopen those completed optional slots for reconsideration, and stock local search can both assign and unassign.
Scalar construction heuristics that sort entities or values declare those capabilities explicitly on `#[planning_variable]`: use `construction_entity_order_key = "fn_name"` for entity-priority ordering and `construction_value_order_key = "fn_name"` for weakest/strongest-fit and queue-style value ordering. Those hooks are evaluated against the live working solution at each construction step, not cached once at phase start, and they never reorder local-search scalar candidates.
Generated applications and normal `solverforge` facade usage keep the same syntax. The recent construction unification only changes advanced direct `solverforge-solver` runtime assembly APIs.

## Extend the Scaffold

- [Extend the solver](docs/extend-solver.md) when you need custom constraints, phases, selectors, termination, or solver configuration beyond the default scaffold.
- [Extend the domain](docs/extend-domain.md) when you need more entities, facts, variables, scoring, or mixed scalar/list modeling inside the generated app.

## Documentation Map

- `README.md` is the user-facing entry point for the workspace and generated-project integration model.
- `docs/extend-solver.md` and `docs/extend-domain.md` cover scaffold extension workflows.
- `docs/lifecycle-pause-resume-contract.md` defines the retained lifecycle contract, including exact pause/resume semantics, snapshot identity, and terminal-state cleanup rules.
- `docs/naming-charter.md` is the canonical naming contract for scalar/list terminology and selector-family cleanup.
- `docs/typed-contract-audit.md` records the current neutral selector and extractor naming model, including the `EntityCollectionExtractor`, `ValueSelector`, and `MoveSelector` surface adopted in `0.7.0`.
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

Current public naming follows neutral Rust contracts rather than `Typed*` prefixes. The object-safe descriptor boundary is still intentional, but the concrete adapter and selector surface are now documented as `EntityCollectionExtractor`, `ValueSelector`, and `MoveSelector`. The historical rename and rationale are captured in [docs/typed-contract-audit.md](docs/typed-contract-audit.md).

## Features

- **Score Types**: SoftScore, HardSoftScore, HardMediumSoftScore, BendableScore, HardSoftDecimalScore
- **ConstraintStream API**: Declarative constraints with fluent builders, source-aware generated streams, single-source and cross-join projected scoring rows, existence checks, joins, grouping, and balance/complemented streams
- **SERIO Engine**: Scoring Engine for Real-time Incremental Optimization
- **Solver Phases**:
  - Generic Construction Heuristics (`FirstFit`, `CheapestInsertion`) over one mixed scalar/list `ModelContext` when matching list work is present, plus descriptor-scalar construction routing for pure scalar targets and specialized list phases (`ListRoundRobin`, `ListCheapestInsertion`, `ListRegretInsertion`, `ListClarkeWright`, `ListKOpt`)
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing, Diversified Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded via rayon)
  - VND (Variable Neighborhood Descent)
- **Move System**: Zero-allocation typed moves with cursor-scoped ownership and selected-winner materialization
  - Scalar: ChangeMove, SwapMove, PillarChangeMove, PillarSwapMove, RuinMove
  - List: ListChangeMove, ListSwapMove, SublistChangeMove, SublistSwapMove, KOptMove, ListRuinMove
  - Scalar ruin-recreate, composite moves, cartesian composition, and nearby selection for scalar and list neighborhoods
- **SolverManager API**: Retained job / snapshot / checkpoint lifecycle with exact pause/resume, lifecycle-complete events, snapshot retrieval, snapshot-bound analysis, and telemetry
- **Model Macros**: `planning_model!`, `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Configuration**: TOML/YAML support with builder API, bounded scalar candidate limits, grouped scalar move selectors, conflict-repair selectors, selector telemetry, and level-aware simulated annealing configuration
- **Console Output**: Colorful tracing-based progress display with solve telemetry

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge = { version = "0.11.1", features = ["console"] }
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

## Release Operations

The workspace release checklist, publish order, and crate stability matrix live in [RELEASE.md](RELEASE.md). Version bumps are explicit release decisions, and `CHANGELOG.md` remains managed by the `commit-and-tag-version` workflow rather than hand-maintained in feature branches.

## Quick Start

### 1. Define Your Domain Model

```rust
// src/domain/mod.rs
solverforge::planning_model! {
    root = "src/domain";

    mod employee;
    mod shift;
    mod schedule;

    pub use employee::Employee;
    pub use shift::Shift;
    pub use schedule::Schedule;
}

// src/domain/employee.rs
use solverforge::prelude::*;

#[problem_fact]
pub struct Employee {
    #[planning_id]
    pub id: usize,
    pub name: String,
    pub skills: Vec<String>,
}

// src/domain/shift.rs
use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: usize,
    pub required_skill: String,
    pub start: i64,
    pub end: i64,
    #[planning_variable]
    pub employee: Option<usize>,
}

// src/domain/schedule.rs
use solverforge::prelude::*;

use super::{Employee, Shift};

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

`planning_model!` is the canonical domain manifest. It preserves normal
separate Rust files while making model metadata deterministic and owned by the
model instead of by proc-macro expansion order.

Public Rust aliases are accepted at the manifest boundary, including
`type Alias = Type;` and `pub use module::Type as Alias;`. Solver configuration
targets still use canonical descriptor type names such as `Task.worker`, not
alias names from collection fields.

Nearby scalar neighborhoods are model-provided, not inferred. If a solver policy
uses `nearby_change_move_selector` or `nearby_swap_move_selector`, declare the
matching candidate hook on the variable with
`#[planning_variable(nearby_value_candidates = "...")]` and/or
`#[planning_variable(nearby_entity_candidates = "...")]`. Distance meters
(`nearby_value_distance_meter` and `nearby_entity_distance_meter`) may rank or
filter those bounded candidates, but they are not candidate-discovery hooks.

Scalar value neighborhoods can also be bounded with
`candidate_values = "fn_name"` on the planning variable plus
`value_candidate_limit` in construction, change, nearby-change, pillar-change,
or ruin-recreate selector config. `cheapest_insertion` for scalar construction
and scalar ruin-recreate requires one of those bounded candidate sources.

Scalar construction ordering is model-provided too. If a construction phase uses
`first_fit_decreasing`, `weakest_fit*`, `strongest_fit*`,
`allocate_entity_from_queue`, or `allocate_to_value_from_queue`, declare the
matching `construction_entity_order_key = "..."` and/or
`construction_value_order_key = "..."` hook on that scalar variable. SolverForge
re-evaluates those hooks on the current working solution at every construction
step, so queue-style and weakest/strongest-fit heuristics track the live model
state instead of a phase-start snapshot. Local-search scalar change,
pillar-change, and ruin/recreate selectors keep canonical bounded candidate
order; they do not consume construction order keys.

### 2. Define Constraints

The `#[planning_solution]` macro generates a `ScheduleConstraintStreams` trait with typed accessors for each collection field, so `factory.shifts()` replaces manual `for_each` extractors:

```rust
use solverforge::{ConstraintSet, HardSoftScore};
use crate::domain::ScheduleConstraintStreams; // generated by #[planning_solution]
use solverforge::stream::{joiner::*, ConstraintFactory};

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let required_skill = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .shifts()
        .join((
            ConstraintFactory::<Schedule, HardSoftScore>::new().employees(),
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

Projected scoring rows can come from either one source row or one retained
joined pair. They are useful when the constraint shape is easier to express as
a scoring-only row rather than either source object directly:

```rust
struct AssignedShift {
    shift_id: usize,
    employee_id: usize,
    start: i64,
    end: i64,
}

let assigned_overlaps = ConstraintFactory::<Schedule, HardSoftScore>::new()
    .shifts()
    .join((
        ConstraintFactory::<Schedule, HardSoftScore>::new().employees(),
        equal_bi(|shift: &Shift| shift.employee, |emp: &Employee| Some(emp.id)),
    ))
    .project(|shift: &Shift, employee: &Employee| AssignedShift {
        shift_id: shift.id,
        employee_id: employee.id,
        start: shift.start,
        end: shift.end,
    })
    .join(equal(|row: &AssignedShift| row.employee_id))
    .filter(|a: &AssignedShift, b: &AssignedShift| {
        a.shift_id != b.shift_id && a.start < b.end && b.start < a.end
    })
    .penalize_hard_with(|_a: &AssignedShift, _b: &AssignedShift| {
        HardSoftScore::of_hard(1)
    })
    .named("Assigned overlap");
```

Projected rows are retained scoring rows. They are not planning entities,
problem facts, value ranges, or move-selector targets.

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

The solve-start line is shape-aware: list models show `elements`, and scalar
models show average `candidates`.

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.11.1 - Zero-Erasure Constraint Solver

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

**Current Version**: 0.11.1

### What's New in 0.11.1

- **Facade configuration exports are complete**: solver configuration controls such as `AcceptorConfig`, `PhaseConfig`, `MoveSelectorConfig`, `ForagerConfig`, `SolverConfigOverride`, and related enums are available directly from the `solverforge` facade crate, matching the documented single-dependency workflow.
- **Recording score directors are available from the facade**: `RecordingDirector` is re-exported beside `Director` and `ScoreDirector` for extension code that needs trial-move rollback without depending on the scoring crate directly.

### What's New in 0.11.0

- **Joined projected scoring rows use the existing `.project(...)` verb**: keyed cross joins can now project retained scoring rows directly with `.project(|left, right| Row { ... })`, while single-source `Projection` types keep the existing bounded multi-row path.
- **Projected scoring paths no longer require cloned rows or keys**: projected outputs, projected self-join keys, and grouped collector values can be non-`Clone`, and retained projected state avoids row/key clones in scoring hot paths.
- **Constraint identity is borrowed from the owning constraint**: metadata and analysis views now preserve package-qualified `ConstraintRef` identity without cloning it, so public reporting types carry borrowed lifetimes instead of owned constraint references.

### What's New in 0.10.0

- **Projected scoring rows keep coordinate-stable self-join order**: retained projected joins reuse sparse row storage without letting storage slots define pair orientation, so multi-output projections with order-sensitive filters stay incrementally consistent with full evaluation.
- **Constraint metadata identity is package-aware**: scoring metadata deduplicates by full `ConstraintRef`, package-qualified constraints with the same short name stay distinct, and conflict-repair selectors resolve configured keys against that exact identity.
- **Grouped scalar construction and search are explicit**: named `ScalarGroupContext` providers emit atomic `CompoundScalarMove` candidates for coupled nullable-scalar decisions, with separate construction and local-search limits.
- **Hard-improvement gates are shared across compound moves**: grouped scalar, conflict-repair, cartesian, local-search, and VND paths now enforce the same hard-score improvement requirement when configured.
- **Score level access is allocation-free by contract**: custom `Score` types implement `level_number()` as the required per-level accessor, and `to_level_numbers()` is the derived vector view for callers that need owned level data.
- **Level-aware simulated annealing is production-ready**: simulated annealing uses per-score-level temperatures, hard-regression policy, calibration, and cooling into hill-climbing behavior for `HardSoftScore` and other multi-level scores.

### What's New in 0.9.2

- **Existence scoring now indexes exact `usize` keys internally**: direct and flattened `if_exists` / `if_not_exists` constraints keep the same public stream API, while exact `usize` join keys use dense vector bookkeeping and all other key types retain hashed storage.
- **Local-search phase starts include the current score in console output**: solver telemetry emits the score on `phase_start`, and `solverforge-console` renders it when present so phase transitions preserve score context.

### What's New in 0.9.0

- **Scalar is now the canonical public name for non-list planning variables**: runtime metadata, macro-generated helpers, solve-shape output, and the coordinated docs surface now use `scalar` terminology consistently.
- **`planning_model!` is the canonical domain manifest**: `src/domain/mod.rs`
  lists normal Rust modules and exports, and the macro derives deterministic
  model-owned metadata for scalar, list, and mixed models.
- **Scalar runtime assembly is descriptor-addressed**: generated scalar helpers keep a compact `variable_index` for getter/setter dispatch, while runtime hook attachment and ordering use descriptor index plus variable name, so module declaration order is not a modeling contract.
- **Scalar nearby selectors are bounded model-declared capabilities**: `#[planning_variable]` supports `candidate_values`, `nearby_value_candidates`, and `nearby_entity_candidates`; distance meters rank or filter those bounded candidates and are rejected as standalone discovery mechanisms.
- **Scalar construction ordering is model-declared too**: `#[planning_variable]` now supports `construction_entity_order_key` and `construction_value_order_key`, and scalar-only construction heuristics validate those hooks before phase build. These hooks are construction-only and do not change local-search selector order.
- **Construction routing is capability-driven**: scalar-only heuristics route through the descriptor-scalar engine, list-only heuristics validate the existing list hook surface before build, and generic `FirstFit` / `CheapestInsertion` stay on the mixed engine when matching list work is present.
- **Move selectors are cursor-based**: `open_cursor()` now yields stable candidate indices plus borrowable candidates, cartesian neighborhoods stay preview-safe and cursor-native, and ownership materializes only for the selected winner. Convenience owned-stream helpers such as `iter_moves()` and `append_moves()` are not a cartesian-safe contract.
- **Large modules stay split by behavior**: solver, descriptor-scalar, runtime, construction, and macro-generated support code keep implementation and test chunks in adjacent subsystem files so each Rust source file stays below the 500 LOC maintenance boundary.
- **Scalar solve startup telemetry now reports candidates instead of descriptor slots**: runtime logging estimates the average candidate count per scalar slot from range providers and countable ranges, and the console labels scalar solve startup scale as `candidates`.

### What's New in 0.8.12

- **Optional `FirstFit` now respects `None` as a real baseline**: optional scalar construction keeps `None` unless an assignment is strictly better, matching `CheapestInsertion` semantics while preserving `FirstFit`'s eager search order.
- **Accepted-count local search now retains the best accepted candidates**: the accepted-count forager `limit` caps the retained accepted moves for final selection and no longer acts as an implicit early-exit threshold.
- **Construction/runtime cleanup**: the canonical generic construction engine now lives under `phase/construction/engine.rs`, pure-scalar construction uses the descriptor-scalar construction boundary, and round-robin list construction uses a single shared implementation for runtime and builder assembly.

### What's New in 0.8.11

- **Limited neighborhoods**: `limited_neighborhood` now carries move caps at the neighborhood level instead of exposing a selector decorator wrapper.
- **Selector ownership is cursor-scoped**: selectors now keep candidate storage on the cursor side so search phases can evaluate borrowable candidates and materialize owned moves only when a forager commits to one.

### What's New in 0.8.8

- **Streaming-first default neighborhoods**: omitting `move_selector` now resolves to explicit streaming defaults instead of broad exhaustive search. Scalar models default to change plus swap; list models default to nearby change, nearby swap, and list reverse; mixed models concatenate the list defaults before the scalar defaults.
- **Exact retained telemetry**: retained status/events now preserve generated/evaluated/accepted move counts and generation/evaluation `Duration`s through the solver pipeline. Human-facing `moves/s` remains an edge-derived display metric only.

### What's New in 0.8.6

- Fixed `ListRuinMove` undo bookkeeping for repeated same-entity reinsertion patterns so ruin-and-recreate neighborhoods restore list state exactly under interacting insertion positions.

### What's New in 0.8.1

- **Emerald build banner**: the root `Makefile` banner now uses the emerald truecolor accent so local build and validation commands match the current branded console presentation.

### What's New in 0.8.0

- **Retained runtime lifecycle contract**: `SolverManager` now models a retained job lifecycle around exact in-process checkpoints. `pause()` and `resume()` operate on runtime-owned checkpoints instead of restart-from-best semantics.
- **Neutral lifecycle terminology**: public docs and APIs now speak in terms of jobs, snapshots, and checkpoints rather than schedule-specific runtime terms.
- **Lifecycle-complete event stream**: retained jobs now emit `Progress`, `BestSolution`, `PauseRequested`, `Paused`, `Resumed`, `Completed`, `Cancelled`, and `Failed` with authoritative lifecycle metadata and monotonic `event_sequence` / `snapshot_revision`.
- **Snapshot-bound analysis across retained states**: `analyze_snapshot()` is revision-specific and remains available for retained snapshots while a job is active or terminal. Analysis is informational, not a terminal-state signal.
- **Breaking runtime entrypoint**: manual retained-runtime implementations now use `Solvable::solve(self, runtime: SolverRuntime<Self>)`, and `SolverManager::solve()` returns `(job_id, receiver)` so consumers can coordinate lifecycle state and snapshot analysis explicitly.

### What's New in 0.7.0

- Release notes are managed in `CHANGELOG.md` by commit-and-tag workflow.

- **Modern CLI templates**: The standalone CLI introduced first-class application scaffolds around the retained `SolverManager` + `Solvable` + `solver.toml` API. The current CLI has since consolidated those starters behind the neutral `solverforge new ...` shell plus `solverforge generate ...` domain shaping. No manual solver loops, no sub-crate imports — only the `solverforge` facade crate.
- **Generated domain accessors**: `#[planning_solution]` generates a `{Name}ConstraintStreams` trait with typed `.field_name()` methods on `ConstraintFactory` — e.g., `factory.shifts()` instead of `factory.for_each(|s| &s.shifts)`
- **Ergonomic extractors**: `CollectionExtract<S>` trait accepts both `|s| s.field.as_slice()` and `|s| &s.field` (via `vec(|s| &s.field)`) — no forced `.as_slice()` at every call site
- **Generated `.unassigned()` filter**: entities with `Option` planning variables get a `{Entity}UnassignedFilter` trait — e.g., `factory.shifts().unassigned()` filters to unassigned entities
- **Projected scoring rows**: generated accessors support `.project(...)` with named bounded projection types, creating scoring-only rows without materialized facts.
- **Convenience scoring**: `penalize_hard()`, `penalize_soft()`, `reward_hard()`, `reward_soft()` on all stream types
- **Single `.join(target)`**: one join method dispatching on argument type — `equal(|a| key)` for self-join, `(extractor_b, equal_bi(ka, kb))` for keyed cross-join, `(other_stream, |a, b| pred)` for predicate join
- **`.named("name")`**: sole finalization method on all builders (replaces `as_constraint`)
- **Score trait**: `one_hard()`, `one_soft()`, `one_medium()` default methods
- **Joiners**: `equal`, `equal_bi`, `less_than`, `less_than_or_equal`, `greater_than`, `greater_than_or_equal`, `overlapping`, `filtering`, with `.and()` composition
- **Conditional existence**: `if_exists(...)`, `if_not_exists(...)` over generated/source-aware collection targets, including flattened collection existence for nested list membership

### What's New in 0.5.15

- `solverforge-cvrp` wired into the facade: `solverforge::cvrp::VrpSolution`, `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`, and all CVRP free functions now accessible from the main crate
- Fixed circular dependency: `solverforge-cvrp` now depends on `solverforge-solver` directly instead of the facade

### What's New in 0.5.14

- Added `ListKOptPhase`, `solverforge-cvrp` library, and fixed doctest signatures

### What's New in 0.5.7

- API cleanup: ~1500-1900 LOC removed across scoring and solver crates
- Consolidated tri/quad/penta n-ary constraints and arity stream macros into shared macro files
- Deleted `ShadowAwareScoreDirector`, `ScoreDirectorFactory` (dead wrappers)
- Trimmed `ScoreDirector` trait: removed `variable_name` param, `before/after_entity_changed`, `trigger_variable_listeners`, `get_entity`; deleted dead pinning infrastructure
- Eliminated `Box<dyn Acceptor<S>>` via `AnyAcceptor<S>` enum in `AcceptorBuilder`
- Removed `run_solver_with_channel`; collapsed `scalar.rs` solve overloads
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
- **ScalarMoveUnion**: Monomorphized union of ChangeMove + SwapMove with `UnionMoveSelector` for mixed move neighborhoods
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
- Eliminated Vec clones in KOptMove, SublistChangeMove, and SublistSwapMove hot paths
- Fixed 6 hot-path regressions in local search and SA acceptor
- Score macros (`impl_score_ops!`, `impl_score_scale!`, `impl_score_parse!`) reduce codegen

**Fixes:**
- Construction heuristic and local search producing 0 steps (entity_count wiring)
- Overflow panics in IntegerRange, ValueRangeDef, and date evaluation
- Correct `Acceptor::is_accepted` signature (`&mut self`)

### What's New in 0.5.1

- Removed the solution-aware filter helper in favor of shadow variables on entities

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
| Construction heuristics (scalar, grouped-scalar, list, and mixed routing) | Complete |
| Local search acceptors and foragers | Complete |
| Exhaustive search | Complete |
| Partitioned search | Complete |
| VND | Complete |
| Move system (scalar, list, grouped scalar, conflict repair, ruin-recreate, cartesian, and composite families) | Complete |
| Nearby selection | Complete |
| Ruin-and-recreate (LNS) | Complete |
| Selector decorators and cursor-native composition | Complete |
| Termination | Complete |
| SolverManager | Complete |
| Score Analysis (`analyze()`) | Complete |
| Solve telemetry | Complete |
| Console output | Complete |

## Minimum Rust Version

Rust 1.95 or later.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Please open an issue or pull request.
