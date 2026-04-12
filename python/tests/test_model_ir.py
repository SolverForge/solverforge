import unittest

from solverforge_ir.model import (
    CompareExpr,
    ConstExpr,
    ConstraintDef,
    ConstraintSetDef,
    EntityDef,
    FactDef,
    FieldDef,
    FilterSpec,
    ImpactSpec,
    JoinSpec,
    ModelDef,
    ModelValidationError,
    SolutionDef,
    SolverDef,
    Source,
    TerminationDef,
    lambda_to_expr,
    validate_model,
)


def _valid_model() -> ModelDef:
    employee = FactDef(
        name="Employee",
        fields=[FieldDef("id", "i64"), FieldDef("skills", "Vec[str]")],
    )
    shift = EntityDef(
        name="Shift",
        planning_id_field="id",
        fields=[
            FieldDef("id", "i64"),
            FieldDef("employee_id", "Option[i64]", nullable=True),
            FieldDef("required_skill", "str"),
        ],
    )

    required_skill = ConstraintDef(
        name="required_skill",
        source=Source("shifts"),
        joins=[
            JoinSpec(
                kind="cross_keyed",
                right_collection_field="employees",
                left_key=ConstExpr("shift.employee_id"),
                right_key=ConstExpr("employee.id"),
            )
        ],
        filters=[FilterSpec(predicate=ConstExpr(True))],
        impact=ImpactSpec(impact="penalize", level="hard", weight=1),
    )

    return ModelDef(
        facts=[employee],
        entities=[shift],
        solution=SolutionDef(
            name="Schedule",
            score_kind="hard_soft",
            entity_collections={"shifts": "Shift"},
            fact_collections={"employees": "Employee"},
        ),
        constraints=ConstraintSetDef([required_skill]),
        solver=SolverDef(TerminationDef(step_count_limit=10000)),
    )


def shift_unassigned(shift):
    return shift.employee_id == None


class TestModelIr(unittest.TestCase):
    def test_lambda_lowering_comparison(self) -> None:
        expr = lambda_to_expr(shift_unassigned, aliases=["shift"])
        self.assertIsInstance(expr, CompareExpr)
        self.assertEqual(expr.op, "==")

    def test_model_validation_ok(self) -> None:
        model = _valid_model()
        validate_model(model)

    def test_model_validation_unknown_collection(self) -> None:
        model = _valid_model()
        bad = ConstraintDef(
            name="bad",
            source=Source("missing_collection"),
            impact=ImpactSpec("penalize", "hard", 1),
        )
        broken = ModelDef(
            facts=model.facts,
            entities=model.entities,
            solution=model.solution,
            constraints=ConstraintSetDef(model.constraints.constraints + [bad]),
            solver=model.solver,
        )

        with self.assertRaises(ModelValidationError):
            validate_model(broken)


if __name__ == "__main__":
    unittest.main()
