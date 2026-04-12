import tempfile
import unittest
from pathlib import Path

from solverforge_ir.codegen import generate_rust_module, write_rust_project
from solverforge_ir.model import (
    CallExpr,
    CompareExpr,
    ConstraintDef,
    ConstraintSetDef,
    ConstExpr,
    EntityDef,
    FactDef,
    FieldDef,
    FilterSpec,
    ImpactSpec,
    JoinSpec,
    ModelDef,
    RefExpr,
    SolutionDef,
    SolverDef,
    Source,
    TerminationDef,
    VariableDef,
)


def _schedule_model() -> ModelDef:
    return ModelDef(
        facts=[
            FactDef(name="Employee", fields=[FieldDef("id", "i64"), FieldDef("skills", "Vec[str]")])
        ],
        entities=[
            EntityDef(
                name="Shift",
                planning_id_field="id",
                fields=[
                    FieldDef("id", "i64"),
                    FieldDef("employee_id", "Option[i64]", nullable=True),
                    FieldDef("required_skill", "str"),
                ],
                variables=[VariableDef(name="employee_id", value_type="Option[i64]", kind="basic")],
            )
        ],
        solution=SolutionDef(
            name="Schedule",
            score_kind="hard_soft",
            entity_collections={"shifts": "Shift"},
            fact_collections={"employees": "Employee"},
        ),
        constraints=ConstraintSetDef(
            constraints=[
                ConstraintDef(
                    name="required_skill",
                    source=Source("shifts"),
                    joins=[
                        JoinSpec(
                            kind="cross_keyed",
                            right_collection_field="employees",
                            left_key=RefExpr("left", "employee_id"),
                            right_key=RefExpr("right", "id"),
                        )
                    ],
                    filters=[
                        FilterSpec(
                            predicate=CallExpr(
                                "contains",
                                [RefExpr("right", "skills"), RefExpr("left", "required_skill")],
                            )
                        )
                    ],
                    impact=ImpactSpec("penalize", "hard", 1),
                ),
                ConstraintDef(
                    name="assigned",
                    source=Source("shifts"),
                    filters=[
                        FilterSpec(
                            predicate=CompareExpr(
                                "!=",
                                left=RefExpr("left", "employee_id"),
                                right=ConstExpr(None),
                            )
                        )
                    ],
                    impact=ImpactSpec("reward", "soft", 1),
                ),
            ]
        ),
        solver=SolverDef(termination=TerminationDef(step_count_limit=1000)),
    )


class TestCodegen(unittest.TestCase):
    def test_generate_rust_module_contains_core_builders(self) -> None:
        text = generate_rust_module(_schedule_model())

        self.assertIn("#[planning_solution]", text)
        self.assertIn("ConstraintFactory::<Schedule, HardSoftScore>::new()", text)
        self.assertIn('.named("required_skill")', text)
        self.assertIn('.reward_soft(1)', text)

    def test_write_project_emits_cargo_and_lib(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = write_rust_project(_schedule_model(), Path(tmp), crate_name="demo_py_model")
            self.assertTrue(out.cargo_toml.exists())
            self.assertTrue(out.lib_rs.exists())
            self.assertIn('name = "demo_py_model"', out.cargo_toml.read_text())


if __name__ == "__main__":
    unittest.main()
