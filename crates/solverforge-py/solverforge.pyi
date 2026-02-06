"""Type stubs for the solverforge native module."""

from typing import Any, Optional


class Solver:
    """Synchronous constraint solver.

    Define entity classes, value ranges, constraints, and solve.
    """

    def __init__(self) -> None: ...

    def entity_class(self, name: str, fields: list[tuple[str, str] | tuple[str, str, dict[str, Any]]]) -> None:
        """Define an entity class.

        Fields are tuples of (name, type) or (name, type, options).
        Types: "int", "float", "str", "bool", "ref", "list".
        Options: {"planning_variable": True, "value_range": "range_name"}.
        """
        ...

    def value_range(self, name: str, values: list[Any]) -> None:
        """Define a value range with explicit values."""
        ...

    def int_range(self, name: str, start: int, end: int) -> None:
        """Define an integer range [start, end)."""
        ...

    def add_entities(self, class_name: str, data: list[dict[str, Any]]) -> None:
        """Add entities to a class. Each dict maps field names to values."""
        ...

    def constraint(self, name: str, weight: str) -> "ConstraintBuilder":
        """Start defining a constraint.

        Weight format: "1hard", "2soft", or "1hard/2soft".
        """
        ...

    def add_constraint(self, builder: "ConstraintBuilder") -> None:
        """Add a completed constraint to the solver."""
        ...

    def solve(self, time_limit_seconds: int = 30) -> "PySolveResult":
        """Solve the problem within the given time limit."""
        ...


class SolverManager:
    """Asynchronous constraint solver manager.

    Same configuration API as Solver, but solves asynchronously.
    """

    def __init__(self) -> None: ...

    def entity_class(self, name: str, fields: list[tuple[str, str] | tuple[str, str, dict[str, Any]]]) -> None:
        """Define an entity class."""
        ...

    def int_range(self, name: str, start: int, end: int) -> None:
        """Define an integer range [start, end)."""
        ...

    def add_entities(self, class_name: str, data: list[dict[str, Any]]) -> None:
        """Add entities to a class."""
        ...

    def constraint(self, name: str, weight: str) -> "ConstraintBuilder":
        """Start defining a constraint."""
        ...

    def add_constraint(self, builder: "ConstraintBuilder") -> None:
        """Add a completed constraint."""
        ...

    def solve_async(self, time_limit_seconds: int = 30) -> None:
        """Start solving asynchronously."""
        ...

    def get_status(self) -> str:
        """Get current solve status: "NOT_SOLVING", "SOLVING", or "TERMINATED"."""
        ...

    def get_best_solution(self) -> Optional["PySolveResult"]:
        """Get the best solution found so far, or None if not available."""
        ...

    def terminate(self) -> None:
        """Request termination of the solve."""
        ...

    def is_terminating(self) -> bool:
        """Check if termination was requested."""
        ...


class ConstraintBuilder:
    """Fluent constraint definition builder.

    Chain operations to define a constraint pipeline:
    for_each -> join/filter/distinct_pair -> penalize/reward
    """

    def for_each(self, class_name: str, solver: Solver) -> "ConstraintBuilder":
        """Iterate over all entities of a class."""
        ...

    def for_each_idx(self, class_idx: int) -> "ConstraintBuilder":
        """Iterate over all entities of a class by index."""
        ...

    def join(self, class_name: str, *conditions: str, solver: Solver) -> "ConstraintBuilder":
        """Join with another class using condition expressions.

        Conditions are strings like "A.field == B.field".
        """
        ...

    def join_idx(self, class_idx: int, condition: str) -> "ConstraintBuilder":
        """Join with another class by index."""
        ...

    def filter(self, predicate: str) -> "ConstraintBuilder":
        """Filter tuples using a predicate expression.

        Expression examples: "A.start < B.end", "A.value != B.value".
        """
        ...

    def distinct_pair(self) -> "ConstraintBuilder":
        """Filter to distinct pairs (A < B to avoid duplicates)."""
        ...

    def penalize(self) -> "ConstraintBuilder":
        """Penalize matching tuples with the constraint weight."""
        ...

    def reward(self) -> "ConstraintBuilder":
        """Reward matching tuples with the constraint weight."""
        ...


class PySolveResult:
    """Result of solving a constraint problem."""

    score: str
    """Score as string, e.g. "0hard/0soft"."""

    hard_score: int
    """Hard score component."""

    soft_score: int
    """Soft score component."""

    is_feasible: bool
    """Whether the solution is feasible (hard_score >= 0)."""

    duration_ms: int
    """Solve duration in milliseconds."""

    steps: int
    """Number of steps taken."""

    moves_evaluated: int
    """Number of moves evaluated."""

    def get_entities(self, class_name: str) -> list[dict[str, Any]]:
        """Get entities of a specific class.

        Returns a list of dicts, each mapping field names to values,
        plus an "id" key for the entity ID.
        """
        ...
