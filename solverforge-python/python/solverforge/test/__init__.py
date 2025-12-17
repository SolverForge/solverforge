"""
ConstraintVerifier for testing constraint providers.

Provides a fluent API for testing constraints defined by @constraint_provider functions.

Examples
--------
>>> from solverforge.test import ConstraintVerifier
>>> from domain import Lesson, Timetable, define_constraints
>>>
>>> verifier = ConstraintVerifier.build(
...     define_constraints,
...     Timetable,
...     Lesson,
... )
>>> timeslot = Timeslot(...)
>>> (verifier.verify_that(lambda cf: cf.for_each(Lesson)...)
...          .given(Lesson('Amy', Room('A'), timeslot),
...                 Lesson('Amy', Room('B'), timeslot))
...          .penalizes_by(1))
"""

from typing import Callable, Generic, List, Optional, Type, TypeVar, Union

from solverforge._solverforge import (
    SolverFactory,
    SolutionManager as _SolutionManager,
)
from solverforge.solver.config import (
    SolverConfig,
    ScoreDirectorFactoryConfig,
    TerminationConfig,
)


Solution_ = TypeVar("Solution_")


class ConstraintVerifier(Generic[Solution_]):
    """Entry point for testing constraints defined by @constraint_provider.

    Use build() to create a verifier, then verify_that() to test specific constraints.
    """

    def __init__(
        self,
        constraint_provider: Callable,
        solution_class: Type[Solution_],
        entity_classes: List[Type],
        solver_factory: Optional[SolverFactory] = None,
    ):
        self._constraint_provider = constraint_provider
        self._solution_class = solution_class
        self._entity_classes = entity_classes
        self._solver_factory = solver_factory

    @staticmethod
    def build(
        constraint_provider: Callable,
        planning_solution_class: Type[Solution_],
        *entity_classes: Type,
    ) -> "ConstraintVerifier[Solution_]":
        """Build a ConstraintVerifier for testing constraints.

        Args:
            constraint_provider: The @constraint_provider decorated function
            planning_solution_class: The @planning_solution decorated class
            entity_classes: The @planning_entity decorated classes

        Returns:
            ConstraintVerifier instance for testing
        """
        entity_list = list(entity_classes)

        # Create solver config with minimal termination
        solver_config = SolverConfig(
            solution_class=planning_solution_class,
            entity_class_list=entity_list,
            score_director_factory_config=ScoreDirectorFactoryConfig(
                constraint_provider_function=constraint_provider,
            ),
            termination_config=TerminationConfig(
                move_count_limit=0,  # Don't actually solve, just score
            ),
        )

        # Create factory
        factory = SolverFactory.create(solver_config._inner, constraint_provider, None)

        return ConstraintVerifier(
            constraint_provider=constraint_provider,
            solution_class=planning_solution_class,
            entity_classes=entity_list,
            solver_factory=factory,
        )

    @staticmethod
    def create(solver_config: SolverConfig) -> "ConstraintVerifier":
        """Create a ConstraintVerifier from a SolverConfig.

        Args:
            solver_config: Fully configured SolverConfig

        Returns:
            ConstraintVerifier instance for testing
        """
        constraint_provider = None
        if solver_config.score_director_factory_config:
            constraint_provider = (
                solver_config.score_director_factory_config.constraint_provider
            )

        if not constraint_provider:
            raise ValueError("SolverConfig must have a constraint_provider")

        factory = SolverFactory.create(solver_config._inner, constraint_provider, None)

        return ConstraintVerifier(
            constraint_provider=constraint_provider,
            solution_class=solver_config.solution_class,
            entity_classes=solver_config.entity_class_list or [],
            solver_factory=factory,
        )

    def verify_that(
        self,
        constraint_function: Optional[Callable] = None,
    ) -> Union[
        "SingleConstraintVerification[Solution_]",
        "MultiConstraintVerification[Solution_]",
    ]:
        """Verify a specific constraint or all constraints.

        Args:
            constraint_function: Optional - a function that takes ConstraintFactory
                and returns the constraint to test. If not provided, verifies
                all constraints.

        Returns:
            SingleConstraintVerification if constraint_function provided,
            MultiConstraintVerification otherwise
        """
        if constraint_function is None:
            return MultiConstraintVerification(
                constraint_provider=self._constraint_provider,
                solution_class=self._solution_class,
                entity_classes=self._entity_classes,
                solver_factory=self._solver_factory,
                target_constraint=None,
            )
        else:
            return SingleConstraintVerification(
                constraint_provider=self._constraint_provider,
                solution_class=self._solution_class,
                entity_classes=self._entity_classes,
                solver_factory=self._solver_factory,
                target_constraint=constraint_function,
            )


class SingleConstraintVerification(Generic[Solution_]):
    """Verification for a single constraint."""

    def __init__(
        self,
        constraint_provider: Callable,
        solution_class: Type[Solution_],
        entity_classes: List[Type],
        solver_factory: SolverFactory,
        target_constraint: Callable,
    ):
        self._constraint_provider = constraint_provider
        self._solution_class = solution_class
        self._entity_classes = entity_classes
        self._solver_factory = solver_factory
        self._target_constraint = target_constraint

    def given(self, *facts) -> "SingleConstraintAssertion":
        """Set the facts for this verification.

        Args:
            facts: Entity instances to test the constraint against

        Returns:
            SingleConstraintAssertion for making assertions
        """
        return SingleConstraintAssertion(
            constraint_provider=self._constraint_provider,
            solution_class=self._solution_class,
            entity_classes=self._entity_classes,
            solver_factory=self._solver_factory,
            target_constraint=self._target_constraint,
            facts=list(facts),
            solution=None,
        )

    def given_solution(self, solution: Solution_) -> "SingleConstraintAssertion":
        """Set a full solution for this verification.

        Args:
            solution: Complete planning solution to test

        Returns:
            SingleConstraintAssertion for making assertions
        """
        return SingleConstraintAssertion(
            constraint_provider=self._constraint_provider,
            solution_class=self._solution_class,
            entity_classes=self._entity_classes,
            solver_factory=self._solver_factory,
            target_constraint=self._target_constraint,
            facts=None,
            solution=solution,
        )


class SingleConstraintAssertion:
    """Assertion for a single constraint with given facts."""

    def __init__(
        self,
        constraint_provider: Callable,
        solution_class: Type,
        entity_classes: List[Type],
        solver_factory: SolverFactory,
        target_constraint: Callable,
        facts: Optional[List],
        solution,
    ):
        self._constraint_provider = constraint_provider
        self._solution_class = solution_class
        self._entity_classes = entity_classes
        self._solver_factory = solver_factory
        self._target_constraint = target_constraint
        self._facts = facts
        self._solution = solution
        self._matches: Optional[List] = None
        self._match_count: Optional[int] = None
        self._penalty_count: int = 0
        self._reward_count: int = 0
        self._penalty_weight: int = 0
        self._reward_weight: int = 0

    def _evaluate(self):
        """Evaluate the constraint and cache results."""
        if self._matches is not None:
            return

        # Create SolutionManager for evaluation
        solution_manager = _SolutionManager.create(self._solver_factory)

        # Get the solution to evaluate
        if self._solution is not None:
            test_solution = self._solution
        else:
            # Create a minimal solution with the given facts
            test_solution = self._create_solution_from_facts()

        # Get constraint analysis
        explanation = solution_manager.explain(test_solution)

        # Filter to target constraint if specified
        target_name = None
        if self._target_constraint:
            # Get constraint name by invoking with a dummy factory
            from solverforge._solverforge import ConstraintFactory

            dummy_factory = ConstraintFactory()
            constraint = self._target_constraint(dummy_factory)
            target_name = (
                constraint.get_name() if hasattr(constraint, "get_name") else None
            )

        # Analyze matches
        self._matches = []
        self._penalty_count = 0
        self._reward_count = 0
        self._penalty_weight = 0
        self._reward_weight = 0

        if hasattr(explanation, "constraint_matches"):
            for match in explanation.constraint_matches:
                # Filter by constraint name if target specified
                if target_name and hasattr(match, "constraint_name"):
                    if match.constraint_name != target_name:
                        continue

                self._matches.append(match)

                # Determine if penalty or reward from score impact
                if hasattr(match, "score") and match.score:
                    score_str = str(match.score)
                    # Negative impact = penalty, positive = reward
                    if "-" in score_str or "hard" in score_str.lower():
                        self._penalty_count += 1
                        # Extract weight if available
                        if hasattr(match, "weight"):
                            self._penalty_weight += abs(match.weight)
                        else:
                            self._penalty_weight += 1
                    else:
                        self._reward_count += 1
                        if hasattr(match, "weight"):
                            self._reward_weight += abs(match.weight)
                        else:
                            self._reward_weight += 1
                else:
                    # Default to penalty if unclear
                    self._penalty_count += 1
                    self._penalty_weight += 1

        self._match_count = len(self._matches)

    def _create_solution_from_facts(self):
        """Create a solution object from the given facts."""
        # Try to instantiate solution class with facts
        solution = self._solution_class()

        # Group facts by their class type
        facts_by_type = {}
        for fact in self._facts:
            fact_type = type(fact)
            if fact_type not in facts_by_type:
                facts_by_type[fact_type] = []
            facts_by_type[fact_type].append(fact)

        # Try to set facts on solution via common attribute patterns
        for entity_class in self._entity_classes:
            if entity_class in facts_by_type:
                facts_list = facts_by_type[entity_class]
                # Try common attribute names
                class_name = entity_class.__name__.lower()
                for attr_name in [
                    f"{class_name}s",
                    f"{class_name}_list",
                    f"{class_name}es",
                    class_name,
                ]:
                    if hasattr(solution, attr_name):
                        setattr(solution, attr_name, facts_list)
                        break

        return solution

    def penalizes(
        self, times: Optional[int] = None, message: Optional[str] = None
    ) -> None:
        """Assert that the constraint penalizes.

        Args:
            times: Expected number of penalties (if None, just check there is at least one)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if times is None:
            if self._penalty_count == 0:
                msg = message or "Expected at least one penalty, but got none"
                raise AssertionError(msg)
        else:
            if self._penalty_count != times:
                msg = (
                    message
                    or f"Expected {times} penalties, but got {self._penalty_count}"
                )
                raise AssertionError(msg)

    def penalizes_by(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert the total penalty weight.

        Args:
            match_weight_total: Expected total penalty weight
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._penalty_weight != match_weight_total:
            msg = (
                message
                or f"Expected penalty weight {match_weight_total}, but got {self._penalty_weight}"
            )
            raise AssertionError(msg)

    def penalizes_less_than(self, times: int, message: Optional[str] = None) -> None:
        """Assert fewer penalties than given.

        Args:
            times: Maximum number of penalties (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._penalty_count >= times:
            msg = (
                message
                or f"Expected less than {times} penalties, but got {self._penalty_count}"
            )
            raise AssertionError(msg)

    def penalizes_more_than(self, times: int, message: Optional[str] = None) -> None:
        """Assert more penalties than given.

        Args:
            times: Minimum number of penalties (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._penalty_count <= times:
            msg = (
                message
                or f"Expected more than {times} penalties, but got {self._penalty_count}"
            )
            raise AssertionError(msg)

    def penalizes_by_less_than(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert penalty weight is less than given.

        Args:
            match_weight_total: Maximum penalty weight (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._penalty_weight >= match_weight_total:
            msg = (
                message
                or f"Expected penalty weight less than {match_weight_total}, but got {self._penalty_weight}"
            )
            raise AssertionError(msg)

    def penalizes_by_more_than(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert penalty weight is more than given.

        Args:
            match_weight_total: Minimum penalty weight (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._penalty_weight <= match_weight_total:
            msg = (
                message
                or f"Expected penalty weight more than {match_weight_total}, but got {self._penalty_weight}"
            )
            raise AssertionError(msg)

    def rewards(
        self, times: Optional[int] = None, message: Optional[str] = None
    ) -> None:
        """Assert that the constraint rewards.

        Args:
            times: Expected number of rewards (if None, just check there is at least one)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if times is None:
            if self._reward_count == 0:
                msg = message or "Expected at least one reward, but got none"
                raise AssertionError(msg)
        else:
            if self._reward_count != times:
                msg = (
                    message or f"Expected {times} rewards, but got {self._reward_count}"
                )
                raise AssertionError(msg)

    def rewards_with(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert the total reward weight.

        Args:
            match_weight_total: Expected total reward weight
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._reward_weight != match_weight_total:
            msg = (
                message
                or f"Expected reward weight {match_weight_total}, but got {self._reward_weight}"
            )
            raise AssertionError(msg)

    def rewards_less_than(self, times: int, message: Optional[str] = None) -> None:
        """Assert fewer rewards than given.

        Args:
            times: Maximum number of rewards (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._reward_count >= times:
            msg = (
                message
                or f"Expected less than {times} rewards, but got {self._reward_count}"
            )
            raise AssertionError(msg)

    def rewards_more_than(self, times: int, message: Optional[str] = None) -> None:
        """Assert more rewards than given.

        Args:
            times: Minimum number of rewards (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._reward_count <= times:
            msg = (
                message
                or f"Expected more than {times} rewards, but got {self._reward_count}"
            )
            raise AssertionError(msg)

    def rewards_with_less_than(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert reward weight is less than given.

        Args:
            match_weight_total: Maximum reward weight (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._reward_weight >= match_weight_total:
            msg = (
                message
                or f"Expected reward weight less than {match_weight_total}, but got {self._reward_weight}"
            )
            raise AssertionError(msg)

    def rewards_with_more_than(
        self, match_weight_total: int, message: Optional[str] = None
    ) -> None:
        """Assert reward weight is more than given.

        Args:
            match_weight_total: Minimum reward weight (exclusive)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        if self._reward_weight <= match_weight_total:
            msg = (
                message
                or f"Expected reward weight more than {match_weight_total}, but got {self._reward_weight}"
            )
            raise AssertionError(msg)


class MultiConstraintVerification(Generic[Solution_]):
    """Verification for all constraints together."""

    def __init__(
        self,
        constraint_provider: Callable,
        solution_class: Type[Solution_],
        entity_classes: List[Type],
        solver_factory: SolverFactory,
        target_constraint: Optional[Callable],
    ):
        self._constraint_provider = constraint_provider
        self._solution_class = solution_class
        self._entity_classes = entity_classes
        self._solver_factory = solver_factory
        self._target_constraint = target_constraint

    def given(self, *facts) -> "MultiConstraintAssertion":
        """Set the facts for this verification.

        Args:
            facts: Entity instances to test constraints against

        Returns:
            MultiConstraintAssertion for making assertions
        """
        return MultiConstraintAssertion(
            constraint_provider=self._constraint_provider,
            solution_class=self._solution_class,
            entity_classes=self._entity_classes,
            solver_factory=self._solver_factory,
            facts=list(facts),
            solution=None,
        )

    def given_solution(self, solution: Solution_) -> "MultiConstraintAssertion":
        """Set a full solution for this verification.

        Args:
            solution: Complete planning solution to test

        Returns:
            MultiConstraintAssertion for making assertions
        """
        return MultiConstraintAssertion(
            constraint_provider=self._constraint_provider,
            solution_class=self._solution_class,
            entity_classes=self._entity_classes,
            solver_factory=self._solver_factory,
            facts=None,
            solution=solution,
        )


class MultiConstraintAssertion:
    """Assertion for all constraints with given facts."""

    def __init__(
        self,
        constraint_provider: Callable,
        solution_class: Type,
        entity_classes: List[Type],
        solver_factory: SolverFactory,
        facts: Optional[List],
        solution,
    ):
        self._constraint_provider = constraint_provider
        self._solution_class = solution_class
        self._entity_classes = entity_classes
        self._solver_factory = solver_factory
        self._facts = facts
        self._solution = solution
        self._score = None

    def _evaluate(self):
        """Evaluate all constraints and get total score."""
        if self._score is not None:
            return

        # Create SolutionManager for evaluation
        solution_manager = _SolutionManager.create(self._solver_factory)

        # Get the solution to evaluate
        if self._solution is not None:
            test_solution = self._solution
        else:
            # Create a minimal solution with the given facts
            test_solution = self._create_solution_from_facts()

        # Get score
        score_dto = solution_manager.update(test_solution)
        self._score = score_dto

    def _create_solution_from_facts(self):
        """Create a solution object from the given facts."""
        solution = self._solution_class()

        facts_by_type = {}
        for fact in self._facts:
            fact_type = type(fact)
            if fact_type not in facts_by_type:
                facts_by_type[fact_type] = []
            facts_by_type[fact_type].append(fact)

        for entity_class in self._entity_classes:
            if entity_class in facts_by_type:
                facts_list = facts_by_type[entity_class]
                class_name = entity_class.__name__.lower()
                for attr_name in [
                    f"{class_name}s",
                    f"{class_name}_list",
                    f"{class_name}es",
                    class_name,
                ]:
                    if hasattr(solution, attr_name):
                        setattr(solution, attr_name, facts_list)
                        break

        return solution

    def scores(self, expected_score, message: Optional[str] = None) -> None:
        """Assert the total score equals expected.

        Args:
            expected_score: Expected score (can be Score object or string)
            message: Optional message for assertion error

        Raises:
            AssertionError: If assertion fails
        """
        self._evaluate()

        # Compare scores
        actual_str = str(self._score) if self._score else "None"
        expected_str = str(expected_score)

        if actual_str != expected_str:
            msg = message or f"Expected score {expected_str}, but got {actual_str}"
            raise AssertionError(msg)


__all__ = [
    "ConstraintVerifier",
    "SingleConstraintVerification",
    "SingleConstraintAssertion",
    "MultiConstraintVerification",
    "MultiConstraintAssertion",
]
