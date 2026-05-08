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

The `#[planning_solution]` macro generates source functions on the solution type
for each collection field. Use those functions with `ConstraintFactory::for_each(...)`
so constraints stay tied to the model-owned field instead of to a generated trait
import:

```rust
use solverforge::prelude::*;
use solverforge::stream::{joiner::*, ConstraintFactory};

use crate::domain::{Employee, Schedule, Shift};

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let unassigned = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .unassigned()
        .penalize_hard()
        .named("Unassigned shift");

    let required_skill = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .join((
            ConstraintFactory::<Schedule, HardSoftScore>::new()
                .for_each(Schedule::employees()),
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
        .for_each(Schedule::shifts())
        .join(equal(|shift: &Shift| shift.employee))
        .filter(|a: &Shift, b: &Shift| {
            a.employee.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize_hard()
        .named("No overlap");

    (unassigned, required_skill, no_overlap)
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
    .for_each(Schedule::shifts())
    .join((
        ConstraintFactory::<Schedule, HardSoftScore>::new().for_each(Schedule::employees()),
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
