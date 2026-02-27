# SolverForge

<div align="center">

[![CI](https://github.com/solverforge/solverforge-rs/workflows/CI/badge.svg)](https://github.com/solverforge/solverforge-rs/actions/workflows/ci.yml)
[![Release](https://github.com/solverforge/solverforge-rs/workflows/Release/badge.svg)](https://github.com/solverforge/solverforge-rs/actions/workflows/release.yml)
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

## Zero-Erasure Architecture

SolverForge preserves concrete types through the entire solver pipeline:

- **No trait objects** (`Box<dyn Trait>`, `Arc<dyn Trait>`)
- **No runtime dispatch** - all generics resolved at compile time
- **No hidden allocations** - moves, scores, and constraints are stack-allocated
- **Predictable performance** - no GC pauses, no vtable lookups

This enables aggressive compiler optimizations and cache-friendly data layouts.

## Features

- **Score Types**: SimpleScore, HardSoftScore, HardMediumSoftScore, BendableScore, HardSoftDecimalScore
- **ConstraintStream API**: Declarative constraint definition with fluent builders
- **SERIO Engine**: Scoring Engine for Real-time Incremental Optimization
- **Solver Phases**:
  - Construction Heuristic (FirstFit, BestFit, FirstFeasible, WeakestFit, StrongestFit, CheapestInsertion, RegretInsertion)
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing, Diversified Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded via rayon)
  - VND (Variable Neighborhood Descent)
- **Move System**: Zero-allocation typed moves with arena-based ownership
  - Basic: ChangeMove, SwapMove, PillarChangeMove, PillarSwapMove, RuinMove
  - List: ListChangeMove, ListSwapMove, SubListChangeMove, SubListSwapMove, KOptMove, ListRuinMove
  - Nearby selection for list moves
- **SolverManager/SolutionManager API**: Channel-based async solving with score analysis and telemetry
- **Derive Macros**: `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Configuration**: TOML support with builder API
- **Console Output**: Colorful tracing-based progress display with solve telemetry

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge = { version = "0.5", features = ["console"] }
```

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
use solverforge::prelude::*;

#[problem_fact]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub skills: Vec<String>,
}

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,
    pub required_skill: String,
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

```rust
use solverforge::stream::{ConstraintFactory, joiner};

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let factory = ConstraintFactory::<Schedule, HardSoftScore>::new();

    let required_skill = factory
        .clone()
        .for_each(|s: &Schedule| s.shifts.as_slice())
        .join(
            |s: &Schedule| s.employees.as_slice(),
            joiner::equal_bi(
                |shift: &Shift| shift.employee_id,
                |emp: &Employee| Some(emp.id),
            ),
        )
        .filter(|shift: &Shift, emp: &Employee| {
            !emp.skills.contains(&shift.required_skill)
        })
        .penalize(HardSoftScore::ONE_HARD)
        .as_constraint("Required skill");

    let no_overlap = factory
        .for_each_unique_pair(
            |s: &Schedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| shift.employee_id),
        )
        .filter(|a: &Shift, b: &Shift| {
            a.employee_id.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize(HardSoftScore::ONE_HARD)
        .as_constraint("No overlap");

    (required_skill, no_overlap)
}
```

### 3. Solve

```rust
use solverforge::{SolverManager, Solvable};

fn main() {
    let schedule = Schedule::new(employees, shifts);

    // Channel-based solving with progress updates
    let (job_id, receiver) = SolverManager::global().solve(schedule);

    // Receive best solutions as they're found
    while let Ok((solution, score)) = receiver.recv() {
        println!("New best: {}", score);
    }
}
```

## Console Output

With `features = ["console"]`, SolverForge displays colorful progress:

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.5.2 - Zero-Erasure Constraint Solver

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

The `SolverManager` provides async solving with channel-based solution streaming:

```rust
use solverforge::{SolverManager, SolverStatus};

// Get global solver manager instance
let manager = SolverManager::global();

// Start solving (returns immediately)
let (job_id, receiver) = manager.solve(problem);

// Check status
match manager.get_status(job_id) {
    SolverStatus::Solving => println!("Still working..."),
    SolverStatus::Terminated => println!("Done!"),
    SolverStatus::NotStarted => println!("Queued"),
}

// Terminate early if needed
manager.terminate_early(job_id);

// Receive solutions as they improve
while let Ok((solution, score)) = receiver.recv() {
    // Process each improving solution
}
```

## SolutionManager API

Analyze solutions without solving:

```rust
use solverforge::SolutionManager;

let manager = SolutionManager::<Schedule>::new();
let analysis = manager.analyze(&solution);

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

For comprehensive examples including employee scheduling and vehicle routing, see [SolverForge Quickstarts](https://github.com/solverforge/solverforge-quickstarts).

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

**Current Version**: 0.5.2

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
- Channel-based SolverManager/SolutionManager API
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
| SolutionManager | Complete |
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
