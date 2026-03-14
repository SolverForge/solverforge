use super::domain::DomainModel;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pattern {
    Unary,
    Pair,
    Join,
    Balance,
    Reward,
}

pub(crate) fn generate_skeleton(
    _name: &str,
    pattern: Pattern,
    is_soft: bool,
    solution_type: &str,
    score_type: &str,
    constraint_name: &str,
    domain: Option<&DomainModel>,
) -> String {
    let hardness_comment = if is_soft {
        "SOFT: TODO — describe what this constraint optimizes."
    } else {
        "HARD: TODO — describe what this constraint enforces."
    };

    // Pick the first entity and fact from domain (if available)
    let entity = domain.and_then(|d| d.entities.first());
    let fact = domain.and_then(|d| d.facts.first());

    let entity_field = entity.map(|e| e.field_name.as_str()).unwrap_or("entities");
    let entity_type = entity
        .map(|e| e.item_type.as_str())
        .unwrap_or(solution_type);
    let planning_var = entity
        .and_then(|e| e.planning_vars.first())
        .map(|s| s.as_str())
        .unwrap_or("value");

    let fact_field = fact.map(|f| f.field_name.as_str()).unwrap_or("facts");
    let fact_type = fact.map(|f| f.item_type.as_str()).unwrap_or("Fact");

    // Build import line(s)
    let imports = match pattern {
        Pattern::Join => {
            if fact.is_some() {
                format!(
                    "use crate::domain::{{{solution_type}, {entity_type}, {fact_type}}};\nuse solverforge::prelude::*;\nuse solverforge::stream::joiner::equal_bi;\nuse solverforge::IncrementalConstraint;",
                )
            } else {
                format!(
                    "use crate::domain::{{{solution_type}, {entity_type}}};\nuse solverforge::prelude::*;\nuse solverforge::stream::joiner::equal_bi;\nuse solverforge::IncrementalConstraint;",
                )
            }
        }
        _ => format!(
            "use crate::domain::{{{solution_type}, {entity_type}}};\nuse solverforge::prelude::*;\nuse solverforge::IncrementalConstraint;",
        ),
    };

    let penalty_expr = if is_soft {
        format!("{score_type}::ONE_SOFT")
    } else {
        format!("{score_type}::ONE_HARD")
    };

    let body = match pattern {
        Pattern::Unary => {
            let action = if is_soft {
                format!("        .reward({penalty_expr})")
            } else {
                format!("        .penalize({penalty_expr})")
            };
            format!(
                r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .filter(|e: &{entity_type}| todo!("add your condition"))
{action}
        .as_constraint("{constraint_name}")"#
            )
        }

        Pattern::Pair => format!(
            r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each_unique_pair(
            |s: &{solution_type}| s.{entity_field}.as_slice(),
            joiner::equal(|e: &{entity_type}| e.{planning_var}),
        )
        .filter(|a: &{entity_type}, b: &{entity_type}| {{
            a.{planning_var}.is_some() && todo!("add conflict condition")
        }})
        .penalize({penalty_expr})
        .as_constraint("{constraint_name}")"#
        ),

        Pattern::Join => format!(
            r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .join(
            |s: &{solution_type}| s.{fact_field}.as_slice(),
            equal_bi(
                |e: &{entity_type}| e.{planning_var},
                |f: &{fact_type}| todo!("return matching key from {fact_type}"),
            ),
        )
        .filter(|e: &{entity_type}, f: &{fact_type}| {{
            e.{planning_var}.is_some() && todo!("add mismatch condition")
        }})
        .penalize({penalty_expr})
        .as_constraint("{constraint_name}")"#
        ),

        Pattern::Balance => format!(
            r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .balance(|e: &{entity_type}| e.{planning_var})
        .penalize({score_type}::of_soft(1))
        .as_constraint("{constraint_name}")"#
        ),

        Pattern::Reward => format!(
            r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .filter(|e: &{entity_type}| todo!("add your condition"))
        .reward({penalty_expr})
        .as_constraint("{constraint_name}")"#
        ),
    };

    format!(
        "{imports}\n\n/// {hardness_comment}\npub fn constraint() -> impl IncrementalConstraint<{solution_type}, {score_type}> {{\n{body}\n}}\n"
    )
}
