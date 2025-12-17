"""SolverForge - Constraint solver for Python.

This package provides Python bindings for the SolverForge constraint solver,
offering a 1:1 compatible API with Timefold's Python bindings.

Example:
    >>> from solverforge import (
    ...     planning_entity, planning_solution, constraint_provider,
    ...     PlanningId, PlanningVariable, HardSoftScore,
    ... )
"""

from solverforge._solverforge import (
    __version__,
    # Annotation marker classes
    PlanningId,
    PlanningVariable,
    PlanningListVariable,
    PlanningScore,
    ValueRangeProvider,
    ProblemFactProperty,
    ProblemFactCollectionProperty,
    PlanningEntityProperty,
    PlanningEntityCollectionProperty,
    PlanningPin,
    InverseRelationShadowVariable,
    # Score types
    SimpleScore,
    HardSoftScore,
    HardMediumSoftScore,
    # Decorators
    planning_entity,
    planning_solution,
    get_domain_class,
    build_domain_model,
    DomainClass,
    DomainModel,
    constraint_provider,
    ConstraintProvider,
    # Solver runtime
    TerminationConfig,
    DiminishedReturnsConfig,
    EnvironmentMode,
    MoveThreadCount,
    SolverConfig,
    SolveHandle,
    SolveState,
    ScoreDto,
    SolveStatus,
    SolveResponse,
    SolverFactory,
    Solver,
    # Constraint streams
    ConstraintFactory,
    UniConstraintStream,
    BiConstraintStream,
    TriConstraintStream,
    UniConstraintBuilder,
    BiConstraintBuilder,
    TriConstraintBuilder,
    Constraint,
    # Joiners
    Joiner,
    Joiners,
    # Collectors
    Collector,
    ConstraintCollectors,
)

__all__ = [
    "__version__",
    # Annotation marker classes
    "PlanningId",
    "PlanningVariable",
    "PlanningListVariable",
    "PlanningScore",
    "ValueRangeProvider",
    "ProblemFactProperty",
    "ProblemFactCollectionProperty",
    "PlanningEntityProperty",
    "PlanningEntityCollectionProperty",
    "PlanningPin",
    "InverseRelationShadowVariable",
    # Score types
    "SimpleScore",
    "HardSoftScore",
    "HardMediumSoftScore",
    # Decorators
    "planning_entity",
    "planning_solution",
    "get_domain_class",
    "build_domain_model",
    "DomainClass",
    "DomainModel",
    "constraint_provider",
    "ConstraintProvider",
    # Solver runtime
    "TerminationConfig",
    "DiminishedReturnsConfig",
    "EnvironmentMode",
    "MoveThreadCount",
    "SolverConfig",
    "SolveHandle",
    "SolveState",
    "ScoreDto",
    "SolveStatus",
    "SolveResponse",
    "SolverFactory",
    "Solver",
    # Constraint streams
    "ConstraintFactory",
    "UniConstraintStream",
    "BiConstraintStream",
    "TriConstraintStream",
    "UniConstraintBuilder",
    "BiConstraintBuilder",
    "TriConstraintBuilder",
    "Constraint",
    # Joiners
    "Joiner",
    "Joiners",
    # Collectors
    "Collector",
    "ConstraintCollectors",
]
