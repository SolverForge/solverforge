# solverforge-derive

Procedural macros for SolverForge domain modeling.

## Overview

This crate provides derive macros for implementing planning domain types:

- `#[derive(PlanningEntity)]` - Mark structs as planning entities
- `#[derive(PlanningSolution)]` - Mark structs as planning solutions

## Usage

```toml
[dependencies]
solverforge-core = "0.2"
solverforge-derive = "0.2"
```

## Planning Entity

```rust
use solverforge_derive::PlanningEntity;

#[derive(PlanningEntity, Clone)]
pub struct Lesson {
    #[planning_id]
    pub id: String,

    pub subject: String,

    #[planning_variable(value_range_provider = "timeslots")]
    pub timeslot: Option<Timeslot>,

    #[planning_variable(value_range_provider = "rooms", allows_unassigned = true)]
    pub room: Option<Room>,
}
```

### Entity Attributes

| Attribute | Description |
|-----------|-------------|
| `#[planning_id]` | Unique identifier field (required) |
| `#[planning_variable(value_range_provider = "...")]` | Field assigned by solver |
| `#[planning_variable(..., allows_unassigned = true)]` | Variable can remain unassigned |
| `#[planning_list_variable(value_range_provider = "...")]` | List field assigned by solver |

### Shadow Variable Attributes

Shadow variables are automatically updated when their source variable changes.

```rust
#[derive(PlanningEntity, Clone)]
pub struct Visit {
    #[planning_id]
    pub id: String,

    // Tracks which vehicle this visit belongs to (inverse of Vehicle.visits)
    #[inverse_relation_shadow(source = "visits")]
    pub vehicle: Option<String>,

    // Tracks position in the vehicle's visit list
    #[index_shadow(source = "visits")]
    pub index: Option<i32>,

    // Tracks the previous visit in the list
    #[previous_element_shadow(source = "visits")]
    pub previous_visit: Option<String>,

    // Tracks the next visit in the list
    #[next_element_shadow(source = "visits")]
    pub next_visit: Option<String>,
}

#[derive(PlanningEntity, Clone)]
pub struct ChainedEntity {
    #[planning_id]
    pub id: String,

    // For chained planning variables: tracks the anchor
    #[anchor_shadow(source = "previous")]
    pub anchor: Option<String>,
}
```

| Attribute | Description |
|-----------|-------------|
| `#[inverse_relation_shadow(source = "...")]` | Back-reference to list containing this entity |
| `#[index_shadow(source = "...")]` | Position in source list (0-indexed) |
| `#[previous_element_shadow(source = "...")]` | Previous element in source list |
| `#[next_element_shadow(source = "...")]` | Next element in source list |
| `#[anchor_shadow(source = "...")]` | First element in chained variable chain |

### Entity Comparators

Control entity ordering for move selection:

```rust
#[derive(PlanningEntity, Clone)]
#[difficulty_comparator = "compare_lesson_difficulty"]
pub struct Lesson {
    #[planning_id]
    pub id: String,
    // ...
}

fn compare_lesson_difficulty(a: &Lesson, b: &Lesson) -> std::cmp::Ordering {
    a.constraint_count.cmp(&b.constraint_count)
}
```

| Attribute | Description |
|-----------|-------------|
| `#[difficulty_comparator = "..."]` | Order entities by difficulty for planning |
| `#[strength_comparator = "..."]` | Order values by strength |

## Planning Solution

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

### Solution Attributes

| Attribute | Description |
|-----------|-------------|
| `#[constraint_provider = "..."]` | Constraint function name (struct-level) |
| `#[problem_fact_collection]` | Immutable fact collection |
| `#[problem_fact]` | Single immutable fact |
| `#[planning_entity_collection]` | Entity collection modified by solver |
| `#[planning_entity]` | Single entity modified by solver |
| `#[value_range_provider(id = "...")]` | Provides values for planning variables |
| `#[planning_score]` | Score field |

## Vehicle Routing Example

Complete example with list variables and shadows:

```rust
#[derive(PlanningEntity, Clone)]
pub struct Vehicle {
    #[planning_id]
    pub id: String,

    pub depot: Location,

    #[planning_list_variable(value_range_provider = "visits")]
    pub visits: Vec<String>,  // Visit IDs
}

#[derive(PlanningEntity, Clone)]
pub struct Visit {
    #[planning_id]
    pub id: String,

    pub location: Location,
    pub demand: i32,

    #[inverse_relation_shadow(source = "visits")]
    pub vehicle: Option<String>,

    #[index_shadow(source = "visits")]
    pub index: Option<i32>,

    #[previous_element_shadow(source = "visits")]
    pub previous: Option<String>,

    #[next_element_shadow(source = "visits")]
    pub next: Option<String>,
}

#[derive(PlanningSolution, Clone)]
#[constraint_provider = "vehicle_routing_constraints"]
pub struct VehicleRoutingPlan {
    #[problem_fact_collection]
    #[value_range_provider(id = "visits")]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub vehicles: Vec<Vehicle>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

## License

Apache-2.0
