"""Score types and constraint system - compatibility layer for Timefold-style imports."""

from solverforge._solverforge import (
    # Score types
    SimpleScore,
    HardSoftScore,
    HardMediumSoftScore,
    # Constraint system
    ConstraintFactory,
    UniConstraintStream,
    BiConstraintStream,
    TriConstraintStream,
    UniConstraintBuilder,
    BiConstraintBuilder,
    TriConstraintBuilder,
    Constraint,
    # Decorator
    constraint_provider,
    ConstraintProvider,
    # Joiners
    Joiner,
    Joiners,
    # Collectors
    Collector,
    ConstraintCollectors,
)

__all__ = [
    # Score types
    "SimpleScore",
    "HardSoftScore",
    "HardMediumSoftScore",
    # Constraint system
    "ConstraintFactory",
    "UniConstraintStream",
    "BiConstraintStream",
    "TriConstraintStream",
    "UniConstraintBuilder",
    "BiConstraintBuilder",
    "TriConstraintBuilder",
    "Constraint",
    # Decorator
    "constraint_provider",
    "ConstraintProvider",
    # Joiners
    "Joiner",
    "Joiners",
    # Collectors
    "Collector",
    "ConstraintCollectors",
]
