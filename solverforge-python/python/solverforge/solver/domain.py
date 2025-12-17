"""Domain annotations - compatibility layer for Timefold-style imports."""

from solverforge._solverforge import (
    # Decorators
    planning_entity,
    planning_solution,
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
    # Shadow variables
    InverseRelationShadowVariable,
    PreviousElementShadowVariable,
    NextElementShadowVariable,
    CascadingUpdateShadowVariable,
)

__all__ = [
    # Decorators
    "planning_entity",
    "planning_solution",
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
    # Shadow variables
    "InverseRelationShadowVariable",
    "PreviousElementShadowVariable",
    "NextElementShadowVariable",
    "CascadingUpdateShadowVariable",
]
