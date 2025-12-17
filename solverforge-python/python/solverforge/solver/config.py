"""Configuration classes - compatibility layer for Timefold-style imports."""

from solverforge._solverforge import (
    SolverConfig as _SolverConfig,
    TerminationConfig as _TerminationConfig,
    DiminishedReturnsConfig,
    EnvironmentMode,
    MoveThreadCount,
)

from datetime import timedelta
from typing import List, Optional, Union


class Duration:
    """Duration helper for Timefold compatibility.

    Wraps Python timedelta for solver configuration.
    """

    def __init__(self, seconds: int = 0, minutes: int = 0, hours: int = 0):
        self._timedelta = timedelta(seconds=seconds, minutes=minutes, hours=hours)

    @staticmethod
    def ofSeconds(seconds: int) -> "Duration":
        """Create duration from seconds."""
        return Duration(seconds=seconds)

    @staticmethod
    def ofMinutes(minutes: int) -> "Duration":
        """Create duration from minutes."""
        return Duration(minutes=minutes)

    @staticmethod
    def ofHours(hours: int) -> "Duration":
        """Create duration from hours."""
        return Duration(hours=hours)

    def total_seconds(self) -> float:
        """Get total seconds."""
        return self._timedelta.total_seconds()

    def to_iso8601(self) -> str:
        """Convert to ISO-8601 duration string (e.g., PT30S, PT5M)."""
        total = int(self._timedelta.total_seconds())
        if total >= 3600 and total % 3600 == 0:
            return f"PT{total // 3600}H"
        elif total >= 60 and total % 60 == 0:
            return f"PT{total // 60}M"
        else:
            return f"PT{total}S"

    def __repr__(self) -> str:
        return f"Duration({self._timedelta})"


class ScoreDirectorFactoryConfig:
    """Score director factory configuration.

    Holds the constraint provider for score calculation.
    """

    def __init__(
        self,
        constraint_provider_class=None,
        constraint_provider_function=None,
    ):
        # Accept both parameter names for compatibility
        self.constraint_provider = (
            constraint_provider_function or constraint_provider_class
        )

    def with_constraint_provider_class(self, cls):
        """Set the constraint provider class."""
        self.constraint_provider = cls
        return self


class TerminationConfig:
    """Termination configuration with Timefold-compatible constructor.

    Wraps the native TerminationConfig with keyword argument support.
    """

    def __init__(
        self,
        spent_limit: Optional[Union[Duration, str]] = None,
        unimproved_spent_limit: Optional[Union[Duration, str]] = None,
        best_score_limit: Optional[str] = None,
        best_score_feasible: Optional[bool] = None,
        step_count_limit: Optional[int] = None,
        unimproved_step_count_limit: Optional[int] = None,
        score_calculation_count_limit: Optional[int] = None,
        move_count_limit: Optional[int] = None,
    ):
        self._inner = _TerminationConfig()
        self.spent_limit = spent_limit
        self.unimproved_spent_limit = unimproved_spent_limit
        self.best_score_limit = best_score_limit
        self.best_score_feasible = best_score_feasible
        self.step_count_limit = step_count_limit
        self.unimproved_step_count_limit = unimproved_step_count_limit
        self.score_calculation_count_limit = score_calculation_count_limit
        self.move_count_limit = move_count_limit

        # Build the inner config
        if spent_limit is not None:
            limit_str = (
                spent_limit.to_iso8601()
                if isinstance(spent_limit, Duration)
                else spent_limit
            )
            self._inner = self._inner.with_spent_limit(limit_str)
        if unimproved_spent_limit is not None:
            limit_str = (
                unimproved_spent_limit.to_iso8601()
                if isinstance(unimproved_spent_limit, Duration)
                else unimproved_spent_limit
            )
            self._inner = self._inner.with_unimproved_spent_limit(limit_str)
        if best_score_limit is not None:
            self._inner = self._inner.with_best_score_limit(best_score_limit)
        if best_score_feasible is not None:
            self._inner = self._inner.with_best_score_feasible(best_score_feasible)
        if step_count_limit is not None:
            self._inner = self._inner.with_step_count_limit(step_count_limit)
        if unimproved_step_count_limit is not None:
            self._inner = self._inner.with_unimproved_step_count(
                unimproved_step_count_limit
            )
        if score_calculation_count_limit is not None:
            self._inner = self._inner.with_score_calculation_count_limit(
                score_calculation_count_limit
            )
        if move_count_limit is not None:
            self._inner = self._inner.with_move_count_limit(move_count_limit)

    def with_spent_limit(self, limit: Union[Duration, str]) -> "TerminationConfig":
        """Set the spent time limit."""
        limit_str = limit.to_iso8601() if isinstance(limit, Duration) else limit
        self._inner = self._inner.with_spent_limit(limit_str)
        self.spent_limit = limit
        return self

    def with_unimproved_spent_limit(
        self, limit: Union[Duration, str]
    ) -> "TerminationConfig":
        """Set the unimproved spent time limit."""
        limit_str = limit.to_iso8601() if isinstance(limit, Duration) else limit
        self._inner = self._inner.with_unimproved_spent_limit(limit_str)
        self.unimproved_spent_limit = limit
        return self


class SolverConfig:
    """Solver configuration with Timefold-compatible constructor.

    Wraps the native SolverConfig with keyword argument support.
    """

    def __init__(
        self,
        solution_class=None,
        entity_class_list: Optional[List] = None,
        score_director_factory_config: Optional[ScoreDirectorFactoryConfig] = None,
        termination_config: Optional[TerminationConfig] = None,
        environment_mode: Optional[EnvironmentMode] = None,
        move_thread_count: Optional[MoveThreadCount] = None,
        random_seed: Optional[int] = None,
    ):
        self._inner = _SolverConfig()
        self.solution_class = solution_class
        self.entity_class_list = entity_class_list or []
        self.score_director_factory_config = score_director_factory_config
        self.termination_config = termination_config
        self.environment_mode = environment_mode
        self.move_thread_count = move_thread_count
        self.random_seed = random_seed

        # Build the inner config
        if solution_class is not None:
            self._inner = self._inner.with_solution_class(solution_class)
        for entity_class in self.entity_class_list:
            self._inner = self._inner.with_entity_class(entity_class)
        if termination_config is not None:
            self._inner = self._inner.with_termination(termination_config._inner)
        if environment_mode is not None:
            self._inner = self._inner.with_environment_mode(environment_mode)
        if move_thread_count is not None:
            self._inner = self._inner.with_move_thread_count(move_thread_count)
        if random_seed is not None:
            self._inner = self._inner.with_random_seed(random_seed)

    def with_solution_class(self, cls) -> "SolverConfig":
        """Set the solution class."""
        self._inner = self._inner.with_solution_class(cls)
        self.solution_class = cls
        return self

    def with_entity_class(self, cls) -> "SolverConfig":
        """Add an entity class."""
        self._inner = self._inner.with_entity_class(cls)
        self.entity_class_list.append(cls)
        return self

    def with_termination(self, termination: TerminationConfig) -> "SolverConfig":
        """Set the termination configuration."""
        self._inner = self._inner.with_termination(termination._inner)
        self.termination_config = termination
        return self


__all__ = [
    "SolverConfig",
    "TerminationConfig",
    "DiminishedReturnsConfig",
    "EnvironmentMode",
    "MoveThreadCount",
    "Duration",
    "ScoreDirectorFactoryConfig",
]
