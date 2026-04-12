from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from .model import (
    BoolExpr,
    CallExpr,
    CompareExpr,
    ConstraintDef,
    ConstExpr,
    EntityDef,
    Expr,
    ImpactSpec,
    JoinSpec,
    ModelDef,
    RefExpr,
)


@dataclass(frozen=True)
class GeneratedRustProject:
    crate_dir: Path
    cargo_toml: Path
    lib_rs: Path


_SCORE_MAP = {
    "soft": "SoftScore",
    "hard_soft": "HardSoftScore",
    "hard_medium_soft": "HardMediumSoftScore",
    "hard_soft_decimal": "HardSoftDecimalScore",
    "bendable": "BendableScore<1, 1>",
}


def generate_rust_module(model: ModelDef) -> str:
    score_ty = _score_type(model.solution.score_kind)
    lines: list[str] = []
    lines.extend(
        [
            "use solverforge::prelude::*;",
            "use solverforge::stream::{joiner::*, ConstraintFactory};",
            "",
        ]
    )

    lines.extend(_emit_facts(model.facts))
    lines.extend(_emit_entities(model.entities))
    lines.extend(_emit_solution(model))

    lines.append(f"use {model.solution.name}ConstraintStreams;")
    lines.append("")
    lines.append(
        f"pub fn define_constraints() -> impl ConstraintSet<{model.solution.name}, {score_ty}> {{"
    )

    for idx, constraint in enumerate(model.constraints.constraints):
        c_name = f"c_{idx}"
        lines.extend(_emit_constraint_builder(model, constraint, c_name, score_ty))

    tuple_items = ", ".join([f"c_{i}" for i in range(len(model.constraints.constraints))])
    lines.append(f"    ({tuple_items})")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def write_rust_project(model: ModelDef, out_dir: Path, crate_name: str = "solverforge_py_model") -> GeneratedRustProject:
    crate_dir = out_dir / crate_name
    src_dir = crate_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    cargo_toml = crate_dir / "Cargo.toml"
    lib_rs = src_dir / "lib.rs"

    cargo_toml.write_text(
        "\n".join(
            [
                "[package]",
                f'name = "{crate_name}"',
                'version = "0.1.0"',
                'edition = "2021"',
                "",
                "[dependencies]",
                'solverforge = { version = "0.8", features = ["console"] }',
                "",
            ]
        )
    )
    lib_rs.write_text(generate_rust_module(model))

    return GeneratedRustProject(crate_dir=crate_dir, cargo_toml=cargo_toml, lib_rs=lib_rs)


def _emit_facts(facts: Iterable) -> list[str]:
    lines: list[str] = []
    for fact in facts:
        lines.append("#[problem_fact]")
        lines.append(f"pub struct {fact.name} {{")
        for field in fact.fields:
            rust_ty = _map_type(field.type_ref)
            lines.append(f"    pub {field.name}: {rust_ty},")
        lines.append("}")
        lines.append("")
    return lines


def _emit_entities(entities: Iterable[EntityDef]) -> list[str]:
    lines: list[str] = []
    for entity in entities:
        variable_names = {v.name for v in entity.variables}

        lines.append("#[planning_entity]")
        lines.append(f"pub struct {entity.name} {{")
        for field in entity.fields:
            if field.name == entity.planning_id_field:
                lines.append("    #[planning_id]")
            if field.name in variable_names:
                lines.append("    #[planning_variable]")
            rust_ty = _map_type(field.type_ref)
            lines.append(f"    pub {field.name}: {rust_ty},")
        lines.append("}")
        lines.append("")
    return lines


def _emit_solution(model: ModelDef) -> list[str]:
    score_ty = _score_type(model.solution.score_kind)
    lines: list[str] = ["#[planning_solution]", f"pub struct {model.solution.name} {{"]

    for field_name, fact_name in model.solution.fact_collections.items():
        lines.append("    #[problem_fact_collection]")
        lines.append(f"    pub {field_name}: Vec<{fact_name}>,")

    for field_name, entity_name in model.solution.entity_collections.items():
        lines.append("    #[planning_entity_collection]")
        lines.append(f"    pub {field_name}: Vec<{entity_name}>,")

    lines.append("    #[planning_score]")
    lines.append(f"    pub score: Option<{score_ty}>,")
    lines.append("}")
    lines.append("")
    return lines


def _emit_constraint_builder(model: ModelDef, constraint: ConstraintDef, c_name: str, score_ty: str) -> list[str]:
    solution_name = model.solution.name
    source_entity_name = model.solution.entity_collections[constraint.source.collection_field]

    lines = [
        f"    let {c_name} = ConstraintFactory::<{solution_name}, {score_ty}>::new()",
        f"        .{constraint.source.collection_field}()",
    ]

    current_aliases: list[str] = ["left"]
    current_entity_names: list[str] = [source_entity_name]

    for join in constraint.joins:
        right_entity = None
        if join.right_collection_field:
            right_entity = model.solution.entity_collections.get(join.right_collection_field)
            if right_entity is None:
                right_entity = model.solution.fact_collections[join.right_collection_field]

        if join.kind == "self_equal":
            left_ty = current_entity_names[0]
            left_expr = _rust_expr(join.left_key or RefExpr("left", ""), {"left": "left"})
            lines.append(f"        .join(equal(|left: &{left_ty}| {left_expr}))")
            current_aliases.append("right")
            current_entity_names.append(left_ty)

        elif join.kind == "cross_keyed":
            if not join.right_collection_field or not join.left_key or not join.right_key:
                raise ValueError("cross_keyed join requires right_collection_field, left_key, right_key")

            left_ty = current_entity_names[0]
            right_ty = right_entity
            left_key_expr = _rust_expr(join.left_key, {"left": "left"})
            right_key_expr = _rust_expr(join.right_key, {"right": "right"})
            lines.extend(
                [
                    "        .join((",
                    f"            |s: &{solution_name}| &s.{join.right_collection_field},",
                    "            equal_bi(",
                    f"                |left: &{left_ty}| {left_key_expr},",
                    f"                |right: &{right_ty}| {right_key_expr},",
                    "            ),",
                    "        ))",
                ]
            )
            current_aliases.append("right")
            current_entity_names.append(right_ty)

        elif join.kind == "cross_predicate":
            if not join.right_collection_field or not join.predicate:
                raise ValueError("cross_predicate join requires right_collection_field and predicate")
            left_ty = current_entity_names[0]
            right_ty = right_entity
            pred = _rust_expr(join.predicate, {"left": "left", "right": "right"})
            lines.extend(
                [
                    "        .join((",
                    f"            |s: &{solution_name}| &s.{join.right_collection_field},",
                    f"            |left: &{left_ty}, right: &{right_ty}| {pred},",
                    "        ))",
                ]
            )
            current_aliases.append("right")
            current_entity_names.append(right_ty)

    for f in constraint.filters:
        args = []
        alias_map: dict[str, str] = {}
        for idx, alias in enumerate(current_aliases):
            var_name = alias if idx == 0 else f"{alias}{idx}"
            ty = current_entity_names[idx]
            args.append(f"{var_name}: &{ty}")
            alias_map[alias] = var_name

        pred = _rust_expr(f.predicate, alias_map)
        args_sig = ", ".join(args)
        lines.append(f"        .filter(|{args_sig}| {pred})")

    lines.append(f"        .{_impact_method(constraint.impact)}({constraint.impact.weight})")
    lines.append(f"        .named(\"{constraint.name}\");")
    lines.append("")
    return lines


def _impact_method(impact: ImpactSpec) -> str:
    prefix = "penalize" if impact.impact == "penalize" else "reward"
    return f"{prefix}_{impact.level}"


def _rust_expr(expr: Expr, alias_map: dict[str, str]) -> str:
    if isinstance(expr, ConstExpr):
        return _const_to_rust(expr.value)

    if isinstance(expr, RefExpr):
        base = alias_map.get(expr.stream_alias, expr.stream_alias)
        if expr.field_path:
            return f"{base}.{expr.field_path}"
        return base

    if isinstance(expr, CompareExpr):
        left = _rust_expr(expr.left, alias_map)
        right = _rust_expr(expr.right, alias_map)
        return f"({left} {expr.op} {right})"

    if isinstance(expr, BoolExpr):
        if expr.op == "not":
            return f"(!{_rust_expr(expr.args[0], alias_map)})"
        joiner = " && " if expr.op == "and" else " || "
        return f"({joiner.join(_rust_expr(a, alias_map) for a in expr.args)})"

    if isinstance(expr, CallExpr):
        args = [_rust_expr(a, alias_map) for a in expr.args]
        if expr.fn == "contains":
            if len(args) != 2:
                raise ValueError("contains expects 2 args")
            return f"{args[0]}.contains(&{args[1]})"
        if expr.fn == "len":
            if len(args) != 1:
                raise ValueError("len expects 1 arg")
            return f"{args[0]}.len()"
        if expr.fn == "overlaps":
            if len(args) != 4:
                raise ValueError("overlaps expects 4 args: a_start,a_end,b_start,b_end")
            return f"(({args[0]} < {args[3]}) && ({args[2]} < {args[1]}))"
        raise ValueError(f"Unsupported call: {expr.fn}")

    raise TypeError(f"Unsupported expression type: {type(expr).__name__}")


def _const_to_rust(value: object) -> str:
    if value is None:
        return "None"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, str):
        return f'"{value}".to_string()'
    return repr(value)


def _score_type(score_kind: str) -> str:
    try:
        return _SCORE_MAP[score_kind]
    except KeyError as e:
        raise ValueError(f"Unsupported score kind '{score_kind}'.") from e


def _map_type(type_ref: str) -> str:
    cleaned = type_ref.strip()

    if cleaned.startswith("Option[") and cleaned.endswith("]"):
        inner = cleaned[len("Option[") : -1]
        return f"Option<{_map_type(inner)}>"

    if cleaned.startswith("Vec[") and cleaned.endswith("]"):
        inner = cleaned[len("Vec[") : -1]
        return f"Vec<{_map_type(inner)}>"

    primitives = {
        "str": "String",
        "string": "String",
        "i64": "i64",
        "int": "i64",
        "f64": "f64",
        "float": "f64",
        "bool": "bool",
    }
    return primitives.get(cleaned, cleaned)
