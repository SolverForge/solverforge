"""Solver module - compatibility layer for Timefold-style imports."""

from enum import Enum

from solverforge._solverforge import (
    SolverFactory,
    Solver,
    SolveHandle,
    SolveResponse,
    SolutionManager as _SolutionManager,
    ScoreExplanation,
    ConstraintMatch,
    Indictment,
    ScoreDto,
)


class SolverStatus(Enum):
    """Solver status enum matching Timefold's SolverStatus.

    NOT_SOLVING: No active solve operation.
    SOLVING_SCHEDULED: Solve has been submitted but not yet started.
    SOLVING_ACTIVE: Actively solving the problem.
    """

    NOT_SOLVING = "NOT_SOLVING"
    SOLVING_SCHEDULED = "SOLVING_SCHEDULED"
    SOLVING_ACTIVE = "SOLVING_ACTIVE"


class SolverManager:
    """High-level solver manager providing Timefold-compatible API.

    Manages solver lifecycle and provides synchronous solving capabilities.
    """

    @staticmethod
    def create(solver_config, service_url=None):
        """Create a SolverManager from config.

        Args:
            solver_config: SolverConfig with solution/entity classes and constraints
            service_url: Optional URL for the solver service

        Returns:
            Configured SolverManager instance
        """
        return SolverManager(solver_config, service_url)

    def __init__(self, solver_config, service_url=None):
        self._config = solver_config
        self._service_url = service_url
        self._active_solves = {}

        # Extract constraint provider from score_director_factory_config
        constraint_provider = None
        if solver_config.score_director_factory_config:
            constraint_provider = (
                solver_config.score_director_factory_config.constraint_provider
            )

        # Get the native config wrapper
        inner_config = (
            solver_config._inner if hasattr(solver_config, "_inner") else solver_config
        )

        if not constraint_provider:
            raise ValueError(
                "SolverConfig must include score_director_factory_config with constraint_provider"
            )

        self._factory = SolverFactory.create(
            inner_config, constraint_provider, service_url
        )

    def solve(self, problem_id, problem, final_best_solution_consumer=None):
        """Solve a problem synchronously.

        Args:
            problem_id: Unique identifier for this solve operation
            problem: The planning problem instance to solve
            final_best_solution_consumer: Optional callback invoked with final solution

        Returns:
            SolveResponse containing the best solution and score
        """
        solver = self._factory.build_solver()
        response = solver.solve(problem)

        if final_best_solution_consumer and response.solution:
            final_best_solution_consumer(response.solution)

        return response

    def solve_and_listen(self, problem_id, problem, listener=None):
        """Solve with progress listener.

        Currently equivalent to solve() - streaming updates planned for future release.

        Args:
            problem_id: Unique identifier for this solve operation
            problem: The planning problem instance to solve
            listener: Callback for solution updates (called once with final solution)

        Returns:
            SolveResponse containing the best solution and score
        """
        return self.solve(problem_id, problem, listener)

    def close(self):
        """Release solver manager resources."""
        self._active_solves.clear()


class SolutionManager:
    """Solution manager for score calculation and constraint analysis.

    Wraps the native SolutionManager to provide Timefold-compatible API.
    """

    @staticmethod
    def create(solver_factory_or_manager):
        """Create a SolutionManager from a SolverFactory or SolverManager.

        Args:
            solver_factory_or_manager: SolverFactory or SolverManager instance

        Returns:
            Configured SolutionManager instance
        """
        return SolutionManager(solver_factory_or_manager)

    def __init__(self, solver_factory_or_manager):
        if isinstance(solver_factory_or_manager, SolverManager):
            self._factory = solver_factory_or_manager._factory
        else:
            self._factory = solver_factory_or_manager

        # Create native SolutionManager from factory
        self._inner = _SolutionManager.create(self._factory)

    def update(self, solution):
        """Calculate and update the score for a solution.

        Args:
            solution: Planning solution to score

        Returns:
            ScoreDto with the calculated score
        """
        return self._inner.update(solution)

    def analyze(self, solution):
        """Analyze constraint matches for a solution.

        Args:
            solution: Planning solution to analyze

        Returns:
            ScoreExplanation with constraint match details
        """
        return self._inner.analyze(solution)

    def explain(self, solution):
        """Get score explanation for a solution.

        Args:
            solution: Planning solution to explain

        Returns:
            ScoreExplanation with detailed constraint breakdown
        """
        return self._inner.explain(solution)


__all__ = [
    "SolverStatus",
    "SolverFactory",
    "Solver",
    "SolveHandle",
    "SolveResponse",
    "SolverManager",
    "SolutionManager",
    "ScoreExplanation",
    "ConstraintMatch",
    "Indictment",
    "ScoreDto",
]
