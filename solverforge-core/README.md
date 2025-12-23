# solverforge-core

Language-agnostic core library for SolverForge constraint solving.

## Overview

This crate provides the foundation for SolverForge's constraint solving capabilities:

- **Value types** - `Value`, `ObjectHandle`, `FunctionHandle`
- **Score types** - `SimpleScore`, `HardSoftScore`, `HardMediumSoftScore`, `BendableScore` (+ decimal variants)
- **Domain modeling** - `PlanningAnnotation`, `DomainModel`, `DomainClass`, `FieldDescriptor`
- **Constraint streams** - `Constraint`, `ConstraintSet`, `StreamComponent`, `Joiner`, `Collector`
- **Expression DSL** - `Expr`, `Expression`, `NamedExpression` for type-safe constraint building
- **Solver** - `SolverBuilder`, `TypedSolver`, `SolverManager`, `SolverFactory`
- **WASM generation** - `WasmModuleBuilder`, `PredicateDefinition`

## Usage

```toml
[dependencies]
solverforge-core = "0.2"
```

### Score Types

```rust
use solverforge_core::{HardSoftScore, HardMediumSoftScore, SimpleScore};

let score = HardSoftScore::of(-2, -15);
assert_eq!(score.hard_score(), -2);
assert!(score.is_feasible() == false);

let simple = SimpleScore::of(100);
let hms = HardMediumSoftScore::of(0, -5, -10);
```

### Constraint Streams

```rust
use solverforge_core::{Constraint, StreamComponent, Joiner, WasmFunction, Collector};

let constraint = Constraint::new("Room conflict")
    .with_stream(StreamComponent::for_each("Lesson"))
    .with_stream(StreamComponent::join_with_joiners(
        "Lesson",
        vec![
            Joiner::equal(WasmFunction::new("getRoom")),
            Joiner::equal(WasmFunction::new("getTimeslot")),
        ],
    ))
    .with_stream(StreamComponent::filter(WasmFunction::new("isDifferent")))
    .with_stream(StreamComponent::penalize("1hard"));
```

### Expression DSL

```rust
use solverforge_core::{Expr, NamedExpression, IntoNamedExpression, StreamComponent};
use solverforge_core::wasm::FieldAccessExt;

// Build type-safe expressions
let has_room = Expr::is_not_null(Expr::param(0).get("Lesson", "room"))
    .named_as("lesson_has_room");

// Use in stream components
let filter = StreamComponent::filter_expr(has_room);
```

### SolverManager (Multi-Problem Solving)

```rust
use solverforge_core::{SolverManager, HttpSolverService, TerminationConfig};
use std::sync::Arc;

let service = Arc::new(HttpSolverService::new("http://localhost:8080"));
let mut manager = SolverManager::<Timetable, String>::new(service)
    .with_termination(TerminationConfig::new().with_spent_limit("PT5M"));

// Solve multiple problems concurrently
manager.solve("problem-1".to_string(), problem1)?;
manager.solve("problem-2".to_string(), problem2)?;

// Check solutions
if let Some(solution) = manager.get_best_solution(&"problem-1".to_string())? {
    println!("Score: {:?}", solution.score());
}

manager.terminate_all();
```

### Shadow Variables

```rust
use solverforge_core::PlanningAnnotation;

// Available shadow variable types
let index = PlanningAnnotation::index_shadow("visits");
let next = PlanningAnnotation::next_element_shadow("visits");
let prev = PlanningAnnotation::previous_element_shadow("visits");
let anchor = PlanningAnnotation::anchor_shadow("chain");
let inverse = PlanningAnnotation::inverse_relation_shadow("vehicle", "visits");
```

## Architecture

```
solverforge-core
├── constraints/    # Constraint streams, joiners, collectors
├── domain/         # Domain model, annotations, shadow variables
├── solver/         # SolverBuilder, SolverManager, HTTP client
├── wasm/           # WASM module generation, expression compiler
├── score/          # Score types (Simple, HardSoft, etc.)
└── analysis/       # Score explanation, constraint matches
```

## License

Apache-2.0
