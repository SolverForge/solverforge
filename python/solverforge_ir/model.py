from __future__ import annotations

import ast
import inspect
import textwrap
from dataclasses import asdict, dataclass, field
from typing import Any, Callable, Literal, Optional, Union

ScoreKind = Literal["soft", "hard_soft", "hard_medium_soft", "bendable", "hard_soft_decimal"]
VarKind = Literal["basic", "list"]
JoinKind = Literal["self_equal", "cross_keyed", "cross_predicate"]
Impact = Literal["penalize", "reward"]
Level = Literal["hard", "medium", "soft"]


@dataclass(frozen=True)
class FieldDef:
    name: str
    type_ref: str
    nullable: bool = False


@dataclass(frozen=True)
class FactDef:
    name: str
    fields: list[FieldDef]


@dataclass(frozen=True)
class VariableDef:
    name: str
    value_type: str
    kind: VarKind
    pinned_field: Optional[str] = None


@dataclass(frozen=True)
class EntityDef:
    name: str
    planning_id_field: str
    fields: list[FieldDef]
    variables: list[VariableDef] = field(default_factory=list)


@dataclass(frozen=True)
class SolutionDef:
    name: str
    score_kind: ScoreKind
    entity_collections: dict[str, str]
    fact_collections: dict[str, str]


@dataclass(frozen=True)
class RefExpr:
    stream_alias: str
    field_path: str


@dataclass(frozen=True)
class ConstExpr:
    value: Any


@dataclass(frozen=True)
class CallExpr:
    fn: str
    args: list["Expr"]


@dataclass(frozen=True)
class CompareExpr:
    op: Literal["==", "!=", "<", "<=", ">", ">="]
    left: "Expr"
    right: "Expr"


@dataclass(frozen=True)
class BoolExpr:
    op: Literal["and", "or", "not"]
    args: list["Expr"]


Expr = Union[RefExpr, ConstExpr, CallExpr, CompareExpr, BoolExpr]


@dataclass(frozen=True)
class Source:
    collection_field: str


@dataclass(frozen=True)
class JoinSpec:
    kind: JoinKind
    right_collection_field: Optional[str] = None
    left_key: Optional[Expr] = None
    right_key: Optional[Expr] = None
    predicate: Optional[Expr] = None


@dataclass(frozen=True)
class FilterSpec:
    predicate: Expr


@dataclass(frozen=True)
class ImpactSpec:
    impact: Impact
    level: Level
    weight: int = 1


@dataclass(frozen=True)
class ConstraintDef:
    name: str
    source: Source
    joins: list[JoinSpec] = field(default_factory=list)
    filters: list[FilterSpec] = field(default_factory=list)
    impact: ImpactSpec = field(default_factory=lambda: ImpactSpec("penalize", "hard", 1))


@dataclass(frozen=True)
class ConstraintSetDef:
    constraints: list[ConstraintDef]


@dataclass(frozen=True)
class TerminationDef:
    time_limit_ms: Optional[int] = None
    step_count_limit: Optional[int] = None
    unimproved_time_limit_ms: Optional[int] = None
    unimproved_step_limit: Optional[int] = None


@dataclass(frozen=True)
class SolverDef:
    termination: TerminationDef


@dataclass(frozen=True)
class ModelDef:
    facts: list[FactDef]
    entities: list[EntityDef]
    solution: SolutionDef
    constraints: ConstraintSetDef
    solver: SolverDef

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


_ALLOWED_CALLS = {
    "contains",
    "overlaps",
    "len",
}


class LambdaLoweringError(ValueError):
    pass


class ModelValidationError(ValueError):
    pass


def lambda_to_expr(fn: Callable[..., Any], aliases: list[str]) -> Expr:
    source = textwrap.dedent(inspect.getsource(fn))
    tree = ast.parse(source)
    expr_node: ast.AST | None = None

    if isinstance(tree.body[0], ast.Expr) and isinstance(tree.body[0].value, ast.Lambda):
        lam = tree.body[0].value
        expr_node = lam.body
    elif isinstance(tree.body[0], ast.Assign) and isinstance(tree.body[0].value, ast.Lambda):
        lam = tree.body[0].value
        expr_node = lam.body
    elif isinstance(tree.body[0], ast.FunctionDef):
        fn_def = tree.body[0]
        returns = [n for n in fn_def.body if isinstance(n, ast.Return)]
        if len(returns) != 1:
            raise LambdaLoweringError("Function must have exactly one return statement.")
        expr_node = returns[0].value
    else:
        raise LambdaLoweringError("Only lambda or single-return function expressions are supported.")

    return _lower_ast_expr(expr_node, aliases)


def _lower_ast_expr(node: ast.AST, aliases: list[str]) -> Expr:
    if isinstance(node, ast.Constant):
        return ConstExpr(node.value)

    if isinstance(node, ast.Name):
        if node.id in aliases:
            return RefExpr(stream_alias=node.id, field_path="")
        raise LambdaLoweringError(f"Unknown name '{node.id}'.")

    if isinstance(node, ast.Attribute):
        parts: list[str] = []
        cur = node
        while isinstance(cur, ast.Attribute):
            parts.append(cur.attr)
            cur = cur.value
        if isinstance(cur, ast.Name) and cur.id in aliases:
            return RefExpr(stream_alias=cur.id, field_path=".".join(reversed(parts)))
        raise LambdaLoweringError("Attribute access must start from a known stream alias.")

    if isinstance(node, ast.Compare):
        if len(node.ops) != 1 or len(node.comparators) != 1:
            raise LambdaLoweringError("Only single comparisons are supported.")
        op_map = {
            ast.Eq: "==",
            ast.NotEq: "!=",
            ast.Lt: "<",
            ast.LtE: "<=",
            ast.Gt: ">",
            ast.GtE: ">=",
        }
        for k, v in op_map.items():
            if isinstance(node.ops[0], k):
                return CompareExpr(
                    op=v,
                    left=_lower_ast_expr(node.left, aliases),
                    right=_lower_ast_expr(node.comparators[0], aliases),
                )
        raise LambdaLoweringError("Unsupported comparison operator.")

    if isinstance(node, ast.BoolOp):
        if isinstance(node.op, ast.And):
            return BoolExpr(op="and", args=[_lower_ast_expr(v, aliases) for v in node.values])
        if isinstance(node.op, ast.Or):
            return BoolExpr(op="or", args=[_lower_ast_expr(v, aliases) for v in node.values])
        raise LambdaLoweringError("Unsupported boolean operator.")

    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
        return BoolExpr(op="not", args=[_lower_ast_expr(node.operand, aliases)])

    if isinstance(node, ast.Call):
        fn_name = _call_name(node.func)
        if fn_name not in _ALLOWED_CALLS:
            raise LambdaLoweringError(f"Call '{fn_name}' is not in the allowed call whitelist.")
        args = [_lower_ast_expr(a, aliases) for a in node.args]
        return CallExpr(fn=fn_name, args=args)

    raise LambdaLoweringError(f"Unsupported syntax node: {type(node).__name__}")


def _call_name(func_node: ast.AST) -> str:
    if isinstance(func_node, ast.Name):
        return func_node.id
    if isinstance(func_node, ast.Attribute):
        return func_node.attr
    raise LambdaLoweringError("Unsupported callable expression.")


def validate_model(model: ModelDef) -> None:
    entity_names = {e.name for e in model.entities}
    fact_names = {f.name for f in model.facts}

    if len(entity_names) != len(model.entities):
        raise ModelValidationError("Duplicate entity names are not allowed.")
    if len(fact_names) != len(model.facts):
        raise ModelValidationError("Duplicate fact names are not allowed.")

    for field_name, entity_name in model.solution.entity_collections.items():
        if entity_name not in entity_names:
            raise ModelValidationError(
                f"Solution entity collection '{field_name}' references unknown entity '{entity_name}'."
            )

    for field_name, fact_name in model.solution.fact_collections.items():
        if fact_name not in fact_names:
            raise ModelValidationError(
                f"Solution fact collection '{field_name}' references unknown fact '{fact_name}'."
            )

    all_collections = set(model.solution.entity_collections) | set(model.solution.fact_collections)

    for constraint in model.constraints.constraints:
        if constraint.source.collection_field not in all_collections:
            raise ModelValidationError(
                f"Constraint '{constraint.name}' source references unknown collection "
                f"'{constraint.source.collection_field}'."
            )
        if not constraint.name:
            raise ModelValidationError("Constraint name must not be empty.")

        for join in constraint.joins:
            if join.kind in {"cross_keyed", "cross_predicate"} and not join.right_collection_field:
                raise ModelValidationError(
                    f"Constraint '{constraint.name}' has join '{join.kind}' without right collection."
                )
            if join.right_collection_field and join.right_collection_field not in all_collections:
                raise ModelValidationError(
                    f"Constraint '{constraint.name}' join references unknown collection "
                    f"'{join.right_collection_field}'."
                )
            if join.kind == "cross_keyed" and (join.left_key is None or join.right_key is None):
                raise ModelValidationError(
                    f"Constraint '{constraint.name}' keyed join requires left_key and right_key."
                )
            if join.kind == "cross_predicate" and join.predicate is None:
                raise ModelValidationError(
                    f"Constraint '{constraint.name}' predicate join requires predicate expression."
                )
