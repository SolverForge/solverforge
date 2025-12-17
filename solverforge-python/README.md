# SolverForge

Python bindings for the SolverForge constraint solver.

## Installation

```bash
pip install solverforge
```

## Quick Start

```python
from solverforge import (
    planning_entity,
    planning_solution,
    PlanningId,
    PlanningVariable,
    HardSoftScore,
)

@planning_entity
class Lesson:
    id: str
    subject: str
    timeslot: str | None = None
    room: str | None = None

@planning_solution
class Timetable:
    timeslots: list[str]
    rooms: list[str]
    lessons: list[Lesson]
    score: HardSoftScore | None = None
```

## Features

- Declarative domain modeling with decorators
- Constraint streams for defining optimization rules
- Multiple score types (Simple, HardSoft, HardMediumSoft)
- Automatic WASM constraint compilation

## Requirements

- Python 3.10+
- Java 24+ (for solver service)

## Documentation

- [User Guide](https://docs.solverforge.org)
- [API Reference](https://docs.solverforge.org/python)
- [Examples](https://github.com/solverforge/solverforge/tree/main/examples)

## License

Apache-2.0
