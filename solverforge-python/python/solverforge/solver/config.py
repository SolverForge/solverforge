"""Configuration classes - compatibility layer for Timefold-style imports."""

from solverforge._solverforge import (
    SolverConfig,
    TerminationConfig,
    DiminishedReturnsConfig,
    EnvironmentMode,
    MoveThreadCount,
)

from datetime import timedelta


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

    def __repr__(self) -> str:
        return f"Duration({self._timedelta})"


class ScoreDirectorFactoryConfig:
    """Score director factory configuration.

    This is a compatibility stub - constraint providers are registered
    directly with SolverConfig in SolverForge.
    """

    def __init__(self, constraint_provider_class=None):
        self.constraint_provider_class = constraint_provider_class

    def with_constraint_provider_class(self, cls):
        """Set the constraint provider class."""
        self.constraint_provider_class = cls
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
