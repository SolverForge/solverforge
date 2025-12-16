# SolverForge

A Rust-based constraint solver library that bridges language bindings to the Timefold JVM via WebAssembly and HTTP.

## Project Scope

SolverForge enables constraint satisfaction and optimization problems to be defined in any language (Python, JavaScript, etc.) and solved using the Timefold solver engine. Instead of requiring JNI or native bindings, SolverForge:

1. **Generates WASM modules** containing domain object accessors and constraint predicates
2. **Communicates via HTTP** with an embedded Java service running Timefold
3. **Serializes solutions as JSON** for language-agnostic integration

### Goals

- **Language-agnostic**: Core library in Rust with bindings for Python, JavaScript, etc.
- **No JNI complexity**: Pure HTTP/JSON interface to Timefold
- **WASM-based constraints**: Constraint predicates compiled to WebAssembly for execution in the JVM
- **Timefold compatibility**: Full access to Timefold's constraint streams, moves, and solving algorithms

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Language Bindings                                  │
│                     (Python, JavaScript, etc.)                               │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          solverforge-core (Rust)                             │
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
│                      timefold-wasm-service (Java)                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Chicory    │  │   Dynamic    │  │  Timefold    │  │    Host      │    │
│  │ WASM Runtime │  │ Class Gen    │  │   Solver     │  │  Functions   │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Workspace Structure

```
solverforge/
├── Cargo.toml                 # Workspace root
├── solverforge-core/          # Core library (Rust)
│   └── src/
│       ├── analysis/          # Score explanation & constraint matching
│       ├── constraints/       # Constraint streams (forEach, filter, join, etc.)
│       ├── domain/            # Domain model (classes, fields, annotations)
│       ├── score/             # Score types (Simple, HardSoft, Bendable)
│       ├── solver/            # Solver configuration & HTTP client
│       └── wasm/              # WASM module generation
├── solverforge-python/        # Python bindings (PyO3)
│   └── src/
│       ├── annotations.rs     # @planning_entity, @planning_solution, etc.
│       ├── collectors.rs      # ConstraintCollectors (count, sum, etc.)
│       ├── decorators.rs      # Python decorators for domain classes
│       ├── joiners.rs         # Joiners (equal, lessThan, overlapping, etc.)
│       ├── lambda_analyzer.rs # Python lambda → WASM function analysis
│       ├── score.rs           # HardSoftScore, SimpleScore, etc.
│       ├── solver.rs          # SolverFactory, Solver, SolverConfig
│       └── stream.rs          # ConstraintFactory, Uni/Bi/TriConstraintStream
├── solverforge-service/       # JVM lifecycle management (Rust)
│   └── src/
│       └── service.rs         # EmbeddedService - starts/stops Java process
└── timefold-wasm-service/     # Java Quarkus service (submodule)
    └── src/main/java/ai/timefold/wasm/service/
        ├── SolverResource.java              # HTTP endpoints (/solve, /analyze)
        ├── HostFunctionProvider.java        # WASM host functions
        └── classgen/                        # Dynamic bytecode generation
            ├── DomainObjectClassGenerator   # Domain class generation
            └── ConstraintProviderClassGenerator # Constraint stream generation
```

## Implementation Details

### WASM Memory Layout

Domain objects are stored in WASM linear memory with proper alignment:

- **32-bit types** (int, float, pointers): 4-byte alignment, 4-byte size
- **64-bit types** (long, double, LocalDate, LocalDateTime): 8-byte alignment, 8-byte size
- **Field offsets**: Calculated with alignment padding to match Rust's `LayoutCalculator`

Example `Shift` layout:
```
Field            Offset  Size  Type
-----------------------------------
id               0       4     String (pointer)
employee         4       4     Employee (pointer)
location         8       4     String (pointer)
[padding]        12      4     (align for LocalDateTime)
start            16      8     LocalDateTime (i64)
end              24      8     LocalDateTime (i64)
requiredSkill    32      4     String (pointer)
-----------------------------------
Total size: 40 bytes (aligned to 8-byte boundary)
```

**Critical**: Both Rust (WASM generation) and Java (JSON parsing/serialization) must use identical alignment rules, or field reads will access garbage memory.

## How It Works

### 1. Define Domain Model

Define planning entities, solutions, and constraints in the host language:

```python
# Example: Employee Scheduling (conceptual)
class Shift:
    employee: Employee  # @PlanningVariable

class Schedule:
    employees: list[Employee]  # @ProblemFactCollection, @ValueRangeProvider
    shifts: list[Shift]        # @PlanningEntityCollection
    score: HardSoftScore       # @PlanningScore
```

### 2. Generate WASM Module

SolverForge generates a WASM module containing:
- **Memory layout** for domain objects
- **Field accessors** (getters/setters)
- **Constraint predicates** (filters, joiners)
- **List operations** (for collections)

### 3. Build Solve Request

```rust
let request = SolveRequest::new(
    domain,           // IndexMap of domain classes
    constraints,      // IndexMap of constraint streams
    wasm_base64,      // Base64-encoded WASM module
    "alloc",          // Memory allocator function
    "dealloc",        // Memory deallocator function
    list_accessor,    // List operation functions
    problem_json,     // JSON-serialized problem instance
)
.with_termination(TerminationConfig::new().with_seconds_spent_limit(30));
```

### 4. Solve via HTTP

```
POST /solve
Content-Type: application/json

{
  "domain": { "Shift": {...}, "Employee": {...}, "Schedule": {...} },
  "constraints": { "roomConflict": [...], "teacherConflict": [...] },
  "wasm": "AGFzbQEAAAA...",
  "problem": "{\"employees\": [...], \"shifts\": [...]}",
  "termination": { "secondsSpentLimit": 30 }
}
```

### 5. Java Service Processing

1. **Parse WASM** → Chicory runtime loads and compiles the module
2. **Generate Classes** → Dynamic bytecode for domain objects and constraints
3. **Execute Solver** → Timefold evaluates constraints via WASM calls
4. **Return Solution** → JSON-serialized solution with score and stats

## Python Bindings

SolverForge provides Python bindings compatible with Timefold's Python API:

### Domain Model

```python
from typing import Annotated, Optional, List
from solverforge import (
    planning_entity, planning_solution,
    PlanningId, PlanningVariable, PlanningScore,
    ValueRangeProvider, ProblemFactCollectionProperty,
    PlanningEntityCollectionProperty, HardSoftScore,
)

@planning_entity
class Lesson:
    id: Annotated[str, PlanningId]
    subject: str
    teacher: str
    timeslot: Annotated[Optional['Timeslot'], PlanningVariable(value_range_provider_refs=['timeslots'])]
    room: Annotated[Optional['Room'], PlanningVariable(value_range_provider_refs=['rooms'])]

@planning_solution
class Timetable:
    timeslots: Annotated[List[Timeslot], ProblemFactCollectionProperty, ValueRangeProvider(id='timeslots')]
    rooms: Annotated[List[Room], ProblemFactCollectionProperty, ValueRangeProvider(id='rooms')]
    lessons: Annotated[List[Lesson], PlanningEntityCollectionProperty]
    score: Annotated[Optional[HardSoftScore], PlanningScore]
```

### Constraint Streams

```python
from solverforge import (
    constraint_provider, ConstraintFactory,
    Joiners, ConstraintCollectors, HardSoftScore,
)

@constraint_provider
def define_constraints(factory: ConstraintFactory):
    return [
        # Hard: No two lessons in the same room at the same time
        factory.for_each_unique_pair(Lesson, Joiners.equal(lambda l: l.timeslot))
            .filter(lambda a, b: a.room == b.room)
            .penalize(HardSoftScore.ONE_HARD)
            .as_constraint("Room conflict"),

        # Hard: A teacher can only teach one lesson at a time
        factory.for_each_unique_pair(Lesson, Joiners.equal(lambda l: l.timeslot))
            .filter(lambda a, b: a.teacher == b.teacher)
            .penalize(HardSoftScore.ONE_HARD)
            .as_constraint("Teacher conflict"),

        # Soft: Prefer consecutive lessons for the same teacher
        factory.for_each(Lesson)
            .group_by(lambda l: l.teacher, ConstraintCollectors.count())
            .filter(lambda teacher, count: count > 3)
            .penalize(HardSoftScore.ONE_SOFT)
            .as_constraint("Teacher workload"),
    ]
```

### Solving

```python
from solverforge import SolverFactory, SolverConfig, TerminationConfig

config = (SolverConfig()
    .with_solution_class(Timetable)
    .with_entity_classes([Lesson])
    .with_termination(TerminationConfig().with_seconds_spent_limit(30)))

solver = SolverFactory.create(config, define_constraints).build()
solution = solver.solve(problem)

print(f"Score: {solution.score}")
```

### Available Components

**Annotations**:
- `@planning_entity`, `@planning_solution`, `@constraint_provider`
- `PlanningId`, `PlanningVariable`, `PlanningListVariable`, `PlanningScore`
- `ValueRangeProvider`, `ProblemFactCollectionProperty`, `PlanningEntityCollectionProperty`
- `PlanningPin`, `InverseRelationShadowVariable`, `DeepPlanningClone`
- `@deep_planning_clone` decorator

**Constraint Streams**:
- `UniConstraintStream`, `BiConstraintStream`, `TriConstraintStream`
- Operations: `filter()`, `join()`, `if_exists()`, `if_not_exists()`
- Grouping: `group_by()`, `group_by_collector()`, `group_by_two_keys()`
- Scoring: `penalize()`, `reward()`, `as_constraint()`

**Joiners**:
- `Joiners.equal()`, `less_than()`, `less_than_or_equal()`
- `greater_than()`, `greater_than_or_equal()`, `overlapping()`

**Collectors**:
- `ConstraintCollectors.count()`, `count_distinct()`, `sum()`, `average()`
- `min()`, `max()`, `to_list()`, `to_set()`, `load_balance()`

**Scores**:
- `SimpleScore`, `HardSoftScore`, `HardMediumSoftScore`

## Current Status

### Working Features

- **Domain model definition** with planning annotations
- **Constraint streams**: forEach, filter, join, groupBy, complement, flattenLast, penalize, reward
- **WASM module generation** for constraint predicates with proper memory alignment
- **End-to-end solving** via HTTP with embedded Java service
- **Score types**: Simple, HardSoft, HardMediumSoft, Bendable, HardSoftBigDecimal
- **Score analysis** with constraint breakdown
- **Primitive list support**: flattenLast works with LocalDate[] and other primitive lists
- **Advanced collectors**: count, countDistinct, loadBalance
- **Python bindings** (PyO3): Full Timefold-compatible API
  - Decorators: `@planning_entity`, `@planning_solution`, `@constraint_provider`
  - Annotations: `PlanningVariable`, `PlanningScore`, `ValueRangeProvider`, etc.
  - Constraint streams: `UniConstraintStream`, `BiConstraintStream`, `TriConstraintStream`
  - Joiners: `equal`, `lessThan`, `overlapping`, etc.
  - Collectors: `count`, `sum`, `average`, `toList`, `loadBalance`, etc.
  - Lambda analysis: Python lambdas → WASM functions

### Performance Status

| Metric | Current | Target | Native Timefold |
|--------|---------|--------|-----------------|
| Moves/second | ~500 | 50,000+ | ~100,000 |

**Known Bottlenecks** (optimization plan in progress):
1. No WASM module caching - recompiled every request
2. No export function caching - string lookup per call
3. Full constraint re-evaluation - no incremental scoring
4. No join indexing - O(n*m) scans instead of O(1) lookups

### Recent Fixes

- **Memory alignment fix** (2025-12): Fixed field offset alignment mismatch between Java and Rust. Java now properly aligns 64-bit fields (long, double, LocalDate, LocalDateTime) to 8-byte boundaries, matching Rust's LayoutCalculator behavior. This resolved "out of bounds memory access" errors when using temporal types in domain models.

### Test Status

All tests passing:

```bash
# Build
cargo build --workspace

# Run all tests (requires Java 24)
cargo test --workspace

# Run Python bindings tests
make test-python

# Run specific integration test
cargo test -p solverforge-service test_employee_scheduling_solve

# Run with specific Java version
JAVA_HOME=/usr/lib64/jvm/java-24-openjdk-24 \
  cargo test -p solverforge-service --test solve_integration
```

**Test Counts**:
- solverforge-core: 478 tests
- solverforge-python: 129 tests

**Integration Tests**:
- ✅ Employee scheduling with 5 constraints (requiredSkill, noOverlappingShifts, oneShiftPerDay, atLeast10HoursBetweenTwoShifts, balanceEmployeeShiftAssignments)
- ✅ Primitive list operations (LocalDate[] with flattenLast)
- ✅ Advanced collectors (loadBalance for fair distribution)
- ✅ Weighted penalties and custom weighers
- ✅ Python domain model extraction from decorated classes
- ✅ Python constraint stream building with lambda analysis
- ✅ TriConstraintStream for 3-entity joins
- ✅ GroupBy operations with collectors

## Dependencies

- **Rust**: 1.75+ (edition 2021)
- **Java**: 24+ (for timefold-wasm-service)
- **Maven**: 3.9+ (for building Java service)
- **Python**: 3.10+ (tested on 3.10, 3.11, 3.12, 3.13)
- **maturin**: 1.8+ (for building Python wheel)

## License

Apache-2.0
