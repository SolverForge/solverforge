"""Solver module - compatibility layer for Timefold-style imports."""

from solverforge._solverforge import (
    SolveStatus as SolverStatus,  # Alias for compatibility
    SolverFactory,
    Solver,
    SolveHandle,
    SolveResponse,
)


# SolverManager and SolutionManager are high-level wrappers
# For now, provide stubs that users can replace
class SolverManager:
    """High-level solver manager for async solving.

    This is a compatibility stub. Use SolverFactory directly for now.
    """

    @staticmethod
    def create(solver_config):
        """Create a SolverManager from config."""
        return SolverManager(solver_config)

    def __init__(self, solver_config):
        self._config = solver_config
        self._factory = SolverFactory.create(solver_config)

    def solve(self, problem_id, problem, final_best_solution_consumer=None):
        """Solve a problem."""
        solver = self._factory.build_solver()
        return solver.solve(problem)

    def close(self):
        """Close the solver manager."""
        pass


class SolutionManager:
    """Solution manager for score analysis.

    This is a compatibility stub.
    """

    @staticmethod
    def create(solver_factory_or_manager):
        """Create a SolutionManager."""
        return SolutionManager(solver_factory_or_manager)

    def __init__(self, solver_factory_or_manager):
        self._factory = solver_factory_or_manager

    def update(self, solution):
        """Update solution scores."""
        return solution

    def analyze(self, solution):
        """Analyze solution constraints."""
        return None


__all__ = [
    "SolverStatus",
    "SolverFactory",
    "Solver",
    "SolveHandle",
    "SolveResponse",
    "SolverManager",
    "SolutionManager",
]
