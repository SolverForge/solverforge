# SolverForge

SolverForge is a high-performance heuristic constraint programming framework and solver written in Rust. 

SolverForge optimizes planning and scheduling problems using metaheuristic algorithms. It combines a declarative constraint API with efficient incremental scoring to solve complex real-world problems like employee scheduling, vehicle routing, and resource allocation.

## Features

- **Score Types**: SimpleScore, HardSoftScore, HardMediumSoftScore, BendableScore
- **ConstraintStream API**: Declarative constraint definition
- **Incremental Scoring**: SERIO engine (Scoring Engine for Real-time Incremental Optimization)
- **Solver Phases**:
  - Construction Heuristic (First Fit, Best Fit)
  - Local Search (Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance)
  - Exhaustive Search (Branch and Bound with DFS/BFS/Score-First)
  - Partitioned Search (multi-threaded)
- **SolverManager API**: Ergonomic builder pattern for solver configuration
- **Phase Factories**: Auto-configuration of phases from solution metadata
- **Move System**: Zero-allocation typed moves with arena allocation
- **Derive Macros**: `#[planning_solution]`, `#[planning_entity]`, `#[value_range_provider]`
- **Configuration**: TOML/YAML support with builder API
- **Variable Types**: Genuine, shadow, list, and chained variables
 
## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge = "0.4"
```

Or for specific crates:

```toml
[dependencies]
solverforge-core = "0.4"      # Core types and traits
solverforge-solver = "0.4"    # Solver engine, phases, moves, SolverManager
solverforge-scoring = "0.4"   # ConstraintStream API, SERIO incremental scoring
solverforge-macros = "0.4"    # Derive macros
solverforge-config = "0.4"    # Configuration (TOML/YAML)
solverforge-benchmark = "0.4" # Benchmarking framework
```

## Quick Start

### 1. Define Your Domain Model

Use derive macros for ergonomic domain modeling:

```rust
use chrono::NaiveDateTime;
use solverforge::prelude::*;

/// An employee who can be assigned to shifts.
#[problem_fact]
pub struct Employee {
    pub index: usize,
    pub name: String,
    pub skills: HashSet<String>,
}

/// A shift that needs to be staffed.
#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub required_skill: String,
    #[planning_variable(allows_unassigned = true)]
    pub employee_idx: Option<usize>,  // Solver assigns this
}

/// The employee scheduling solution.
#[planning_solution]
pub struct EmployeeSchedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,
    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

### 2. Define Constraints (Fluent ConstraintStream API)

```rust
use solverforge::prelude::*;
use solverforge::stream::{ConstraintFactory, joiner::equal_bi};

fn define_constraints() -> impl ConstraintSet<EmployeeSchedule, HardSoftScore> {
    let factory = ConstraintFactory::<EmployeeSchedule, HardSoftScore>::new();

    // HARD: Employee must have the required skill
    let required_skill = factory
        .clone()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .filter(|shift: &Shift, emp: &Employee| {
            !emp.skills.contains(&shift.required_skill)
        })
        .penalize(HardSoftScore::ONE_HARD)
        .as_constraint("Required skill");

    // HARD: No overlapping shifts for same employee
    let no_overlap = factory
        .clone()
        .for_each_unique_pair(
            |s: &EmployeeSchedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| shift.employee_idx),
        )
        .filter(|a: &Shift, b: &Shift| {
            a.employee_idx.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize(HardSoftScore::ONE_HARD)
        .as_constraint("Overlapping shift");

    // SOFT: Balance shift assignments across employees
    let balanced = factory
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .balance(|shift: &Shift| shift.employee_idx)
        .penalize(HardSoftScore::ONE_SOFT)
        .as_constraint("Balance assignments");

    // Combine constraints into a tuple (zero-erasure, fully monomorphized)
    (required_skill, no_overlap, balanced)
}
```

### 3. Configure and Run the Solver

```rust
use solverforge::{SolverManager, LocalSearchType};
use std::time::Duration;

fn main() {
    let schedule = EmployeeSchedule::new(employees, shifts);

    // Build solver with fluent API
    let manager = SolverManager::<EmployeeSchedule>::builder(define_constraints())
        .with_construction_heuristic()
        .with_local_search(LocalSearchType::TabuSearch)
        .with_time_limit(Duration::from_secs(30))
        .build()
        .expect("Failed to build solver");

    // Solve
    let solution = manager.solve(schedule);
    println!("Score: {:?}", solution.score);
}
```

## Architecture

![SERIO Scoring Engine](assets/SERIO.jpg)

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
│ • Phases     │ • Constraint │ • TOML/YAML  │ • Runner     │
│ • Moves      │   Streams    │ • Builders   │ • Statistics │
│ • Selectors  │ • Score      │              │ • Reports    │
│ • Foragers   │   Directors  │              │              │
│ • Acceptors  │ • SERIO      │              │              │
│ • Termination│   Engine     │              │              │
│ • Manager    │              │              │              │
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
        │ • #[derive(PlanningSolution)]│
        │ • #[derive(PlanningEntity)]  │
        │ • #[derive(ProblemFact)]     │
        └──────────────────────────────┘
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `solverforge` | Main facade with prelude and re-exports |
| `solverforge-core` | Core types: scores, domain traits, descriptors, variable system |
| `solverforge-solver` | Solver engine: phases, moves, selectors, termination, SolverManager |
| `solverforge-scoring` | ConstraintStream API, SERIO incremental scoring, score directors |
| `solverforge-config` | Configuration via TOML/YAML and builder API |
| `solverforge-macros` | Procedural derive macros for domain model |
| `solverforge-benchmark` | Benchmarking framework for solver configurations |

## Score Types

```rust
use solverforge::prelude::*;

// Single-level score (constraint violations count)
let score = SimpleScore::of(-5);

// Two-level score (hard constraints + soft optimization)
let score = HardSoftScore::of(-2, -100);  // 2 hard violations, 100 soft penalty
assert!(!score.is_feasible());  // Hard score < 0

// Three-level score
let score = HardMediumSoftScore::of(0, -50, -200);

// N-level configurable score
let score = BendableScore::new(vec![0, -1], vec![-50, -100]);
```

## Solver Phases

### Construction Heuristic

Builds an initial solution by assigning values to uninitialized variables:

```rust
use solverforge::{ConstructionPhaseFactory, SolverPhaseFactory};

// First Fit: Accept first valid assignment
let factory = ConstructionPhaseFactory::first_fit(|| create_placer());
let phase = factory.create_phase();

// Best Fit: Evaluate all, pick best score
let factory = ConstructionPhaseFactory::best_fit(|| create_placer());
let phase = factory.create_phase();
```

### Local Search

Improves solution through iterative moves:

```rust
use solverforge::{LocalSearchPhaseFactory, SolverPhaseFactory};

// Hill Climbing: Only accept improvements
let factory = LocalSearchPhaseFactory::hill_climbing(|| create_move_selector())
    .with_step_limit(1000);
let phase = factory.create_phase();

// Tabu Search: Avoid recently visited states
let factory = LocalSearchPhaseFactory::tabu_search(|| create_move_selector(), 10);

// Simulated Annealing: Accept worse moves with decreasing probability
let factory = LocalSearchPhaseFactory::simulated_annealing(
    || create_move_selector(), 1.0, 0.995
);

// Late Acceptance: Compare against score from N steps ago
let factory = LocalSearchPhaseFactory::late_acceptance(|| create_move_selector(), 100);
```

### Exhaustive Search

Systematically explores solution space with pruning:

```rust
let phase = ExhaustiveSearchPhase::new(
    decider,
    ExplorationOrder::DepthFirst,  // or BreadthFirst, ScoreFirst
    bounder,
);
```

## Termination Conditions

```rust
use solverforge::{
    TimeTermination, StepCountTermination, BestScoreTermination,
    UnimprovedStepCountTermination, OrCompositeTermination,
};

// Stop after 30 seconds
let term = TimeTermination::new(Duration::from_secs(30));

// Stop after 1000 steps
let term = StepCountTermination::new(1000);

// Stop when reaching target score
let term = BestScoreTermination::new(SimpleScore::ZERO);

// Stop if no improvement for 100 steps
let term = UnimprovedStepCountTermination::new(100);

// Combine: stop when ANY condition is met
let term = OrCompositeTermination::new(vec![
    Box::new(TimeTermination::new(Duration::from_secs(60))),
    Box::new(BestScoreTermination::new(SimpleScore::ZERO)),
]);
```

## Move Types

```rust
use solverforge::{ChangeMove, SwapMove};

// ChangeMove: Assign new value to entity's variable
let mv = ChangeMove::<Solution, i32>::new(entity_idx, "row", new_value);

// SwapMove: Exchange values between two entities
let mv = SwapMove::<Solution, i32>::new(entity1_idx, entity2_idx, "row");
```

## Configuration

### Builder API

```rust
use solverforge::SolverConfig;

let config = SolverConfig::new()
    .with_environment_mode(EnvironmentMode::Reproducible)
    .with_termination_seconds(30)
    .with_construction_heuristic(ConstructionHeuristicConfig::default())
    .with_local_search(LocalSearchConfig {
        acceptor: AcceptorConfig::HillClimbing,
        forager: ForagerConfig::default(),
        termination: Some(TerminationConfig {
            step_count_limit: Some(1000),
            ..Default::default()
        }),
    });
```

## Performance

SolverForge leverages Rust's zero-cost abstractions:

- **Typed Moves**: `ChangeMove<S, V>` stores values inline (no boxing)
- **Arena Allocation**: `MoveArena<M>` provides O(1) per-step cleanup
- **Incremental Scoring**: SERIO engine propagates only changed constraints
- **No GC Pauses**: Predictable latency without garbage collection

## Examples

See the [`examples/`](examples/) directory:

- **Employee Scheduling**: Real-world workforce scheduling with skills, availability, and shift constraints

Run examples:

```bash
cargo run -p employee-scheduling
```

## Status

SolverForge is feature-complete for a production constraint solver:

| Component | Status |
|-----------|--------|
| Score types | Complete |
| Domain model | Complete |
| ConstraintStream API | Complete |
| SERIO incremental scoring | Complete |
| Construction heuristics | Complete |
| Local search | Complete |
| Exhaustive search | Complete |
| Partitioned search | Complete |
| VND (Variable Neighborhood Descent) | Complete |
| Move system | Complete (zero-erasure) |
| Termination | Complete |
| Configuration | Complete |
| Benchmarking | Complete |

### Roadmap

- Multi-threaded move evaluation
- Constraint strength system
- Constraint match analysis/explanation

## Minimum Rust Version

Rust 1.80 or later.

## License

Licensed under Apache License 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please open an issue or submit a pull request.

## Acknowledgments

SolverForge is inspired by [Timefold](https://timefold.ai/) (formerly OptaPlanner), the leading open-source constraint solver for Java. We thank the Timefold team for their excellent documentation and design patterns.
