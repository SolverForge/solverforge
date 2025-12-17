"""Solver module - compatibility layer for Timefold-style imports."""

from datetime import timedelta
from enum import Enum
from threading import Thread, Lock
from typing import Callable, Optional, TypeVar, Generic

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


Solution_ = TypeVar("Solution_")
ProblemId_ = TypeVar("ProblemId_")


class SolverStatus(Enum):
    """Solver status enum matching Timefold's SolverStatus.

    NOT_SOLVING: No active solve operation.
    SOLVING_SCHEDULED: Solve has been submitted but not yet started.
    SOLVING_ACTIVE: Actively solving the problem.
    """

    NOT_SOLVING = "NOT_SOLVING"
    SOLVING_SCHEDULED = "SOLVING_SCHEDULED"
    SOLVING_ACTIVE = "SOLVING_ACTIVE"


class SolverJob(Generic[Solution_, ProblemId_]):
    """Represents a problem that has been submitted to solve on the SolverManager.

    Use get_final_best_solution() to wait for the solve to complete and get the result.
    """

    def __init__(
        self,
        problem_id: ProblemId_,
        solver: "Solver",
        problem: Solution_,
        best_solution_consumer: Optional[Callable[[Solution_], None]] = None,
        final_best_solution_consumer: Optional[Callable[[Solution_], None]] = None,
    ):
        self._problem_id = problem_id
        self._solver = solver
        self._problem = problem
        self._best_solution_consumer = best_solution_consumer
        self._final_best_solution_consumer = final_best_solution_consumer
        self._status = SolverStatus.SOLVING_SCHEDULED
        self._final_solution: Optional[Solution_] = None
        self._response: Optional[SolveResponse] = None
        self._exception: Optional[Exception] = None
        self._terminated_early = False
        self._lock = Lock()
        self._thread: Optional[Thread] = None

    def _run(self):
        """Internal method to run the solver in a thread."""
        try:
            with self._lock:
                self._status = SolverStatus.SOLVING_ACTIVE

            response = self._solver.solve(self._problem)

            with self._lock:
                self._response = response
                self._final_solution = response.solution
                self._status = SolverStatus.NOT_SOLVING

            if self._best_solution_consumer and response.solution:
                self._best_solution_consumer(response.solution)

            if self._final_best_solution_consumer and response.solution:
                self._final_best_solution_consumer(response.solution)

        except Exception as e:
            with self._lock:
                self._exception = e
                self._status = SolverStatus.NOT_SOLVING

    def _start(self):
        """Start the solver in a background thread."""
        self._thread = Thread(target=self._run, daemon=True)
        self._thread.start()

    def get_problem_id(self) -> ProblemId_:
        """Get the problem id for this solver job.

        Returns:
            The problem id.
        """
        return self._problem_id

    def get_solver_status(self) -> SolverStatus:
        """Get the current solver status.

        Returns:
            NOT_SOLVING if terminated, SOLVING_SCHEDULED if waiting,
            SOLVING_ACTIVE if actively solving.
        """
        with self._lock:
            return self._status

    def get_solving_duration(self) -> timedelta:
        """Get the duration spent solving.

        Note: Currently returns timedelta(0) as precise timing is not yet implemented.

        Returns:
            Duration spent solving.
        """
        return timedelta(0)

    def get_final_best_solution(self) -> Solution_:
        """Wait for the solver to complete and return the final best solution.

        Returns:
            The best solution found.

        Raises:
            Exception: If the solver encountered an error.
        """
        if self._thread:
            self._thread.join()

        with self._lock:
            if self._exception:
                raise self._exception
            return self._final_solution

    def terminate_early(self) -> None:
        """Terminate the solver early.

        Currently waits for the solve to complete as streaming termination
        is not yet implemented.
        """
        with self._lock:
            self._terminated_early = True

    def is_terminated_early(self) -> bool:
        """Check if terminate_early was called.

        Returns:
            True if terminate_early was called.
        """
        with self._lock:
            return self._terminated_early


class SolverManager(Generic[Solution_, ProblemId_]):
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

    def solve(
        self,
        problem_id: ProblemId_,
        problem: Solution_,
        final_best_solution_consumer: Optional[Callable[[Solution_], None]] = None,
    ) -> SolverJob[Solution_, ProblemId_]:
        """Submits a problem to solve asynchronously and returns immediately.

        The problem is solved on a background thread. Use SolverJob.get_final_best_solution()
        to wait for the result.

        Args:
            problem_id: Unique identifier for this solve operation
            problem: The planning problem instance to solve
            final_best_solution_consumer: Optional callback invoked with final solution

        Returns:
            SolverJob that can be used to get the result or terminate early
        """
        solver = self._factory.build_solver()
        job = SolverJob(
            problem_id=problem_id,
            solver=solver,
            problem=problem,
            best_solution_consumer=None,
            final_best_solution_consumer=final_best_solution_consumer,
        )
        self._active_solves[problem_id] = job
        job._start()
        return job

    def solve_and_listen(
        self,
        problem_id: ProblemId_,
        problem: Solution_,
        listener: Callable[[Solution_], None],
    ) -> SolverJob[Solution_, ProblemId_]:
        """Submits a problem to solve and calls listener for best solutions.

        The problem is solved on a background thread. The listener is called
        when new best solutions are found (currently called once with final solution).

        Args:
            problem_id: Unique identifier for this solve operation
            problem: The planning problem instance to solve
            listener: Callback for solution updates

        Returns:
            SolverJob that can be used to get the result or terminate early
        """
        solver = self._factory.build_solver()
        job = SolverJob(
            problem_id=problem_id,
            solver=solver,
            problem=problem,
            best_solution_consumer=listener,
            final_best_solution_consumer=None,
        )
        self._active_solves[problem_id] = job
        job._start()
        return job

    def get_solver_status(self, problem_id: ProblemId_) -> SolverStatus:
        """Get the status of a solver job.

        Args:
            problem_id: The problem id to check

        Returns:
            SolverStatus for the given problem, or NOT_SOLVING if not found
        """
        job = self._active_solves.get(problem_id)
        if job:
            return job.get_solver_status()
        return SolverStatus.NOT_SOLVING

    def terminate_early(self, problem_id: ProblemId_) -> None:
        """Terminate a solver job early.

        Args:
            problem_id: The problem id to terminate
        """
        job = self._active_solves.get(problem_id)
        if job:
            job.terminate_early()

    def close(self) -> None:
        """Release solver manager resources."""
        self._active_solves.clear()

    def __enter__(self) -> "SolverManager[Solution_, ProblemId_]":
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit - calls close()."""
        self.close()


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
    "SolverJob",
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
