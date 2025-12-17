# solverforge-derive

Procedural macros for SolverForge domain modeling.

## Overview

This crate provides derive macros for implementing planning domain types:

- `#[derive(PlanningEntity)]` - Mark structs as planning entities
- `#[derive(PlanningSolution)]` - Mark structs as planning solutions

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge-core = "0.1"
solverforge-derive = "0.1"
```

### Planning Entity

```rust
use solverforge_derive::PlanningEntity;

#[derive(PlanningEntity, Clone)]
pub struct Lesson {
    #[planning_id]
    pub id: String,

    pub subject: String,

    #[planning_variable(value_range_provider = "timeslots")]
    pub timeslot: Option<Timeslot>,

    #[planning_variable(value_range_provider = "rooms")]
    pub room: Option<Room>,
}
```

#### Entity Attributes

| Attribute | Description |
|-----------|-------------|
| `#[planning_id]` | Unique identifier field (required) |
| `#[planning_variable(...)]` | Field assigned by solver |
| `#[planning_list_variable(...)]` | List field assigned by solver |

### Planning Solution

```rust
use solverforge_derive::PlanningSolution;

#[derive(PlanningSolution, Clone)]
#[constraint_provider = "define_constraints"]
pub struct Timetable {
    #[problem_fact_collection]
    #[value_range_provider(id = "timeslots")]
    pub timeslots: Vec<Timeslot>,

    #[problem_fact_collection]
    #[value_range_provider(id = "rooms")]
    pub rooms: Vec<Room>,

    #[planning_entity_collection]
    pub lessons: Vec<Lesson>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

#### Solution Attributes

| Attribute | Description |
|-----------|-------------|
| `#[constraint_provider = "..."]` | Constraint function name (struct-level) |
| `#[problem_fact_collection]` | Immutable fact collection |
| `#[problem_fact]` | Single immutable fact |
| `#[planning_entity_collection]` | Entity collection modified by solver |
| `#[planning_entity]` | Single entity modified by solver |
| `#[value_range_provider(id = "...")]` | Provides values for planning variables |
| `#[planning_score]` | Score field |

## Documentation

- [API Reference](https://docs.solverforge.org/solverforge-derive)
- [User Guide](https://solverforge.org/docs)

## License

Apache-2.0
