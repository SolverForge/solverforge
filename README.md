# SolverForge

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
  - Construction Heuristic (First Fit, Best Fit)
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded)
  - VND (Variable Neighborhood Descent)
- **SolverManager/SolutionManager API**: Channel-based async solving with score analysis
- **Move System**: Zero-allocation typed moves
- **Derive Macros**: `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Configuration**: TOML support with builder API
- **Console Output**: Colorful tracing-based progress display

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
                   v0.5.0 - Zero-Erasure Constraint Solver

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
│   solver     │   scoring    │   config     │  benchmark   │
│              │              │              │              │
│ • Phases     │ • Constraint │ • TOML       │ • Runner     │
│ • Moves      │   Streams    │ • Builders   │ • Statistics │
│ • Selectors  │ • Score      │              │ • Reports    │
│ • Acceptors  │   Directors  │              │              │
│ • Termination│ • SERIO      │              │              │
│ • Manager    │   Engine     │              │              │
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
| `solverforge-solver` | Solver engine: phases, moves, termination, SolverManager |
| `solverforge-scoring` | ConstraintStream API, SERIO incremental scoring |
| `solverforge-config` | Configuration via TOML and builder API |
| `solverforge-macros` | Procedural macros for domain model |
| `solverforge-benchmark` | Benchmarking framework |

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

- **Typed Moves**: Values stored inline, no boxing
- **Incremental Scoring**: SERIO propagates only changed constraints
- **No GC**: Predictable latency without garbage collection
- **Cache-friendly**: Contiguous memory layouts for hot paths

Typical throughput: 100k-500k moves/second depending on constraint complexity.

## Status

**Current Version**: 0.5.0 (pre-release, on `release/0.5.0` branch)

### What's New in 0.5.0

**Breaking Changes:**
- **Solution-aware filter traits**: Uni-stream filters can now optionally access the solution using `filter_with_solution()`, enabling access to shadow variables and computed solution state. The standard `filter()` method remains unchanged for simple predicates. Bi/Tri/Quad/Penta stream filters (after joins) continue to receive only the entity tuples without the solution reference.

**Improvements:**
- Added `filter_with_solution()` for uni-streams to access shadow variables
- Refactored incremental constraint internals using macro-based codegen
- Improved code organization with extracted test utilities
- Enhanced clippy compliance and eliminated unnecessary clones
- Better structured logging with trace-level move evaluation

### Component Status

| Component | Status |
|-----------|--------|
| Score types | Complete |
| Domain model macros | Complete |
| ConstraintStream API | Complete |
| SERIO incremental scoring | Complete |
| Construction heuristics | Complete |
| Local search | Complete |
| Exhaustive search | Complete |
| Partitioned search | Complete |
| VND | Complete |
| Move system | Complete |
| Termination | Complete |
| SolverManager | Complete |
| SolutionManager | Complete |
| Console output | Complete |
| Benchmarking | Complete |

## Minimum Rust Version

Rust 1.80 or later.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Please open an issue or pull request.

## Acknowledgments

Inspired by [Timefold](https://timefold.ai/) (formerly OptaPlanner).
