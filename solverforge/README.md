# SolverForge

Constraint solver library powered by Timefold.

SolverForge is a Rust-based constraint solver library that uses WASM modules and HTTP communication to solve constraint satisfaction and optimization problems.

## Installation

```bash
cargo add solverforge
```

## Quick Start

```rust
use solverforge::prelude::*;

#[derive(PlanningEntity, Clone)]
struct Shift {
    #[planning_id]
    id: i64,
    #[planning_variable(value_range_provider = "employees")]
    employee: Option<Employee>,
}

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

## Features

- `embedded` (default) - Includes embedded service that auto-manages the Java solver process

To disable the embedded service:

```bash
cargo add solverforge --no-default-features
```

## License

Apache-2.0
