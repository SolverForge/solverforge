"""SolverForge - Constraint solver for Python.

This package provides Python bindings for the SolverForge constraint solver,
offering a 1:1 compatible API with Timefold's Python bindings.

Example:
    >>> from solverforge import (
    ...     planning_entity, planning_solution, constraint_provider,
    ...     PlanningId, PlanningVariable, HardSoftScore,
    ... )
"""

from solverforge._solverforge import __version__

__all__ = [
    "__version__",
]
