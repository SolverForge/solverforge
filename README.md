# SolverForge

A Rust constraint solver library that bridges to Timefold JVM via WebAssembly and HTTP.

## Installation

```bash
cargo add solverforge
```

## Overview

SolverForge enables constraint satisfaction and optimization problems to be defined in Rust and solved using the Timefold solver engine. Instead of requiring JNI or native bindings, SolverForge:

1. **Generates WASM modules** containing domain object accessors and constraint predicates
2. **Communicates via HTTP** with an embedded Java service running Timefold
3. **Serializes solutions as JSON** for language-agnostic integration

### Goals

- **Rust-first**: Core library and API in Rust
- **No JNI complexity**: Pure HTTP/JSON interface to Timefold
- **WASM-based constraints**: Constraint predicates compiled to WebAssembly for execution in the JVM
- **Timefold compatibility**: Full access to Timefold's constraint streams, moves, and solving algorithms
- **Near-native performance**: ~80-100k moves/second

## Quick Start

### 1. Define Domain Model

```rust
use solverforge::prelude::*;

// Problem fact: Employee with skills
#[derive(Clone)]
struct Employee {
    name: String,
    skills: Vec<String>,
}

// Planning entity: Shift with employee assignment
#[derive(PlanningEntity, Clone)]
struct Shift {
    #[planning_id]
    id: String,

    #[planning_variable(value_range_provider = "employees")]
    employee: Option<Employee>,

    required_skill: String,
}

// Planning solution: Schedule
#[derive(PlanningSolution, Clone)]
struct Schedule {
    #[problem_fact_collection]
    #[value_range_provider(id = "employees")]
    employees: Vec<Employee>,

    #[planning_entity_collection]
    shifts: Vec<Shift>,

    #[planning_score]
    score: Option<HardSoftScore>,
}
```

### 2. Define Constraints

```rust
use solverforge::{Constraint, ConstraintFactory, HardSoftScore};

fn define_constraints(factory: ConstraintFactory) -> Vec<Constraint> {
    vec![
        // Hard: Employee must have the required skill
        factory.for_each::<Shift>()
            .filter(|shift| {
                shift.employee.as_ref().map_or(false, |emp| {
                    !emp.skills.contains(&shift.required_skill)
                })
            })
            .penalize(HardSoftScore::ONE_HARD)
            .as_constraint("Required skill"),

        // Soft: Prefer balanced workload
        factory.for_each::<Shift>()
            .group_by(|shift| shift.employee.clone(), count())
            .penalize(HardSoftScore::ONE_SOFT, |_emp, count| count * count)
            .as_constraint("Balanced workload"),
    ]
}
```

### 3. Solve

```rust
use solverforge::{SolverFactory, SolverConfig, TerminationConfig};

let config = SolverConfig::new()
    .with_solution_class::<Schedule>()
    .with_entity_classes::<Shift>()
    .with_termination(
        TerminationConfig::new().with_seconds_spent_limit(30)
    );

let solver = SolverFactory::create(config, define_constraints).build();
let solution = solver.solve(problem)?;

println!("Score: {:?}", solution.score);
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          solverforge (Rust)                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Domain     │  │ Constraints  │  │    WASM      │  │    HTTP      │    │
│  │   Model      │  │   Streams    │  │   Builder    │  │   Client     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                              HTTP/JSON
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      solverforge-wasm-service (Java)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Chicory    │  │   Dynamic    │  │  Timefold    │  │    Host      │    │
│  │ WASM Runtime │  │ Class Gen    │  │   Solver     │  │  Functions   │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

The embedded solver service starts automatically when needed.

### Workspace Structure

```
solverforge/
├── solverforge/               # Main crate with prelude
├── solverforge-core/          # Core library
├── solverforge-derive/        # Derive macros
├── solverforge-service/       # JVM lifecycle management
├── solverforge-python/        # Python bindings (PyO3)
└── solverforge-wasm-service/  # Java Quarkus service
```

## API Reference

### Derive Macros

**`#[derive(PlanningEntity)]`** - Marks a struct as a planning entity

Field attributes:
- `#[planning_id]` - Unique identifier (required)
- `#[planning_variable(value_range_provider = "...")]` - Solver-assigned field
- `#[planning_variable(..., allows_unassigned = true)]` - Can remain unassigned
- `#[planning_list_variable(value_range_provider = "...")]` - List variable

**`#[derive(PlanningSolution)]`** - Marks a struct as the solution container

Struct attributes:
- `#[constraint_provider = "function_name"]` - Constraint function

Field attributes:
- `#[problem_fact_collection]` - Immutable input data
- `#[planning_entity_collection]` - Entities to be solved
- `#[value_range_provider(id = "...")]` - Provides values for variables
- `#[planning_score]` - Solution score field

### Constraint Streams

```rust
factory.for_each::<Entity>()           // Start stream
    .filter(|e| predicate)             // Filter entities
    .join::<Other>()                   // Join with another type
    .if_exists::<Other>()              // Filter if matching exists
    .if_not_exists::<Other>()          // Filter if no match
    .group_by(key_fn, collector)       // Group and aggregate
    .penalize(score)                   // Apply penalty
    .penalize(score, weigher)          // Weighted penalty
    .reward(score)                     // Apply reward
    .as_constraint("name")             // Name the constraint
```

### Joiners

```rust
Joiners::equal(|a| a.field, |b| b.field)
Joiners::less_than(|a| a.value, |b| b.value)
Joiners::greater_than(|a| a.value, |b| b.value)
Joiners::overlapping(|a| a.start, |a| a.end, |b| b.start, |b| b.end)
```

### Collectors

```rust
count()
count_distinct(|e| e.field)
sum(|e| e.value)
average(|e| e.value)
min(|e| e.value)
max(|e| e.value)
to_list()
to_set()
load_balance()
compose(collector1, collector2)
conditionally(filter, collector)
```

### Score Types

- `SimpleScore` - Single score level
- `HardSoftScore` - Hard constraints (must satisfy) + soft (optimize)
- `HardMediumSoftScore` - Three-level scoring
- `BendableScore` - Configurable number of levels

Each has a `Decimal` variant for precise arithmetic.

### Shadow Variables

For computed fields that update automatically:

```rust
#[derive(PlanningEntity)]
struct Visit {
    #[planning_id]
    id: i64,

    #[planning_variable(value_range_provider = "vehicles")]
    vehicle: Option<Vehicle>,

    #[inverse_relation_shadow_variable(source = "vehicle")]
    previous_visit: Option<Visit>,

    #[shadow_variable(source = "previous_visit", listener = "ArrivalTimeListener")]
    arrival_time: Option<DateTime>,
}
```

Available shadow types:
- `#[shadow_variable]` - Custom computed
- `#[inverse_relation_shadow_variable]` - Inverse reference
- `#[index_shadow_variable]` - Position in list
- `#[previous_element_shadow_variable]` - Previous in list
- `#[next_element_shadow_variable]` - Next in list
- `#[anchor_shadow_variable]` - Chain anchor
- `#[piggyback_shadow_variable]` - Follows another shadow
- `#[cascading_update_shadow_variable]` - Cascade updates

## Python Bindings

Python bindings with Timefold-compatible API (requires Python 3.13+):

```bash
pip install solverforge
```

```python
from solverforge import (
    planning_entity, planning_solution, constraint_provider,
    PlanningId, PlanningVariable, HardSoftScore,
    SolverFactory, SolverConfig,
)

@planning_entity
class Shift:
    id: Annotated[str, PlanningId]
    employee: Annotated[Optional[Employee], PlanningVariable]

@constraint_provider
def constraints(factory):
    return [
        factory.for_each(Shift)
            .filter(lambda s: ...)
            .penalize(HardSoftScore.ONE_HARD)
            .as_constraint("Constraint name"),
    ]

solver = SolverFactory.create(config, constraints).build_solver()
solution = solver.solve(problem)
```

## Performance

| Metric | SolverForge | Native Timefold |
|--------|-------------|-----------------|
| Moves/second | ~80,000-100,000 | ~100,000 |
| Constraint evaluation | WASM (Chicory) | Native JVM |
| Score calculation | Incremental | Incremental |

## Test Status

```bash
cargo build --workspace
cargo test --workspace              # Requires Java 24
make test-python                    # Python binding tests
make test-integration               # Integration tests
```

**Test Counts**: 535 core + 197 python

## Dependencies

- **Rust**: 1.75+ (edition 2021)
- **Java**: 24+ (for embedded service)
- **Maven**: 3.9+ (for building Java service)

## License

Apache-2.0
