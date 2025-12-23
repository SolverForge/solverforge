# SolverForge Python

Python bindings for the SolverForge constraint solver.

## Installation

```bash
pip install solverforge
```

## Quick Start

```python
from solverforge import (
    planning_entity, planning_solution, constraint_provider,
    PlanningId, PlanningVariable, PlanningListVariable,
    HardSoftScore, SolverFactory, TerminationConfig,
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

@constraint_provider
def define_constraints(factory):
    return [
        factory.for_each(Lesson)
            .join(Lesson, equal(lambda l: l.room), equal(lambda l: l.timeslot))
            .filter(lambda l1, l2: l1.id < l2.id)
            .penalize(HardSoftScore.ONE_HARD)
            .as_constraint("Room conflict"),
    ]

# Solve
solver = SolverFactory.create(Timetable, define_constraints)
    .with_termination(TerminationConfig.with_spent_limit("PT30S"))
    .build()
solution = solver.solve(problem)
```

## Features

### Domain Modeling

```python
from solverforge import (
    planning_entity, planning_solution,
    PlanningVariable, PlanningListVariable,
    InverseRelationShadow, IndexShadow, PreviousElementShadow, NextElementShadow,
)

# Standard planning variable
@planning_entity
class Shift:
    id: str
    employee: str | None = None  # Assigned by solver

# List planning variable (vehicle routing)
@planning_entity
class Vehicle:
    id: str
    visits: list[str] = []  # List of visit IDs, assigned by solver

# Shadow variables (auto-updated when list changes)
@planning_entity
class Visit:
    id: str
    vehicle: str | None = None      # @InverseRelationShadowVariable
    index: int | None = None        # @IndexShadowVariable
    previous: str | None = None     # @PreviousElementShadowVariable
    next: str | None = None         # @NextElementShadowVariable
```

### Constraint Streams

```python
from solverforge import (
    equal, less_than, greater_than, overlapping, filtering,
    count, sum_, min_, max_, to_list, to_set,
)

@constraint_provider
def constraints(factory):
    return [
        # forEach / forEachIncludingUnassigned / forEachUniquePair
        factory.for_each(Lesson),
        factory.for_each_including_unassigned(Lesson),
        factory.for_each_unique_pair(Lesson, equal(lambda l: l.timeslot)),

        # filter
        factory.for_each(Lesson)
            .filter(lambda l: l.room is not None),

        # join with joiners
        factory.for_each(Lesson)
            .join(Room, equal(lambda l: l.room, lambda r: r.id)),

        # ifExists / ifNotExists
        factory.for_each(Lesson)
            .if_exists(Conflict, equal(lambda l: l.id, lambda c: c.lesson_id)),

        # groupBy with collectors
        factory.for_each(Lesson)
            .group_by(lambda l: l.room, count())
            .filter(lambda room, cnt: cnt > 1)
            .penalize(HardSoftScore.ONE_HARD, lambda room, cnt: cnt - 1),

        # map / expand / flattenLast
        factory.for_each(Vehicle)
            .flatten_last(lambda v: v.visits),
    ]
```

### SolverManager (Multi-Problem Solving)

```python
from solverforge import SolverManager, HttpSolverService

service = HttpSolverService("http://localhost:8080")
manager = SolverManager(Timetable, define_constraints, service)

# Solve multiple problems concurrently
manager.solve("problem-1", problem1)
manager.solve("problem-2", problem2)

# Check solutions
solution = manager.get_best_solution("problem-1")
if solution:
    print(f"Score: {solution.score}")

# Terminate
manager.terminate("problem-1")
manager.terminate_all()
```

### ConstraintVerifier (Testing)

```python
from solverforge import ConstraintVerifier

verifier = ConstraintVerifier.build(Timetable, define_constraints)

# Test specific constraint
verifier.verify_that(lambda f: f.for_each(Lesson)...)
    .given(lesson1, lesson2)
    .penalizes_by(1)

# Test entire solution
verifier.verify_that()
    .given_solution(solution)
    .scores(HardSoftScore.of(-2, -10))
```

### Score Types

```python
from solverforge import (
    SimpleScore, HardSoftScore, HardMediumSoftScore,
    BendableScore, SimpleDecimalScore, HardSoftDecimalScore,
)

score = HardSoftScore.of(-2, -15)
print(f"Hard: {score.hard_score}, Soft: {score.soft_score}")
print(f"Feasible: {score.is_feasible()}")

# Arithmetic
combined = score + HardSoftScore.of(0, -5)
```

### Solver Configuration

```python
from solverforge import (
    SolverFactory, SolverConfig, TerminationConfig,
    EnvironmentMode, MoveThreadCount,
)

solver = SolverFactory.create(Timetable, define_constraints)
    .with_termination(
        TerminationConfig()
            .with_spent_limit("PT5M")
            .with_unimproved_spent_limit("PT1M")
            .with_best_score_feasible(True)
    )
    .with_environment_mode(EnvironmentMode.REPRODUCIBLE)
    .with_random_seed(42)
    .with_move_thread_count(MoveThreadCount.AUTO)
    .build()
```

## Requirements

- Python 3.10+
- Java 21+ (for solver service, auto-started)

## License

Apache-2.0
