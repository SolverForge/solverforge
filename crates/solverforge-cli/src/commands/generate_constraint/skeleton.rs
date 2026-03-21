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
            if fact.is_some() && solution_type != entity_type {
                format!(
                    "use crate::domain::{{{solution_type}, {entity_type}, {fact_type}}};\nuse solverforge::prelude::*;\nuse solverforge::stream::joiner::equal_bi;\nuse solverforge::IncrementalConstraint;",
                )
            } else if fact.is_some() {
                format!(
                    "use crate::domain::{{{solution_type}, {fact_type}}};\nuse solverforge::prelude::*;\nuse solverforge::stream::joiner::equal_bi;\nuse solverforge::IncrementalConstraint;",
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

    let (body, helpers) = match pattern {
        Pattern::Unary => {
            let action = if is_soft {
                format!("        .reward({penalty_expr})")
            } else {
                format!("        .penalize({penalty_expr})")
            };
            (
                format!(
                    r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .filter(|_e: &{entity_type}| {{
            panic!("replace placeholder condition before enabling this constraint")
        }})
{action}
        .named("{constraint_name}")"#
                ),
                String::new(),
            )
        }

        Pattern::Pair => (
            format!(
                r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .join(joiner::equal(|e: &{entity_type}| e.{planning_var}))
        .filter(|_a: &{entity_type}, _b: &{entity_type}| {{
            panic!("replace placeholder pair condition before enabling this constraint")
        }})
        .penalize({penalty_expr})
        .named("{constraint_name}")"#
            ),
            String::new(),
        ),

        Pattern::Join => (
            format!(
                r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(entity_items)
        .join((
            fact_items,
            equal_bi(
                |e: &{entity_type}| e.{planning_var},
                |_f: &{fact_type}| panic!("replace placeholder join key extractor before enabling this constraint"),
            ),
        ))
        .filter(|_e: &{entity_type}, _f: &{fact_type}| {{
            panic!("replace placeholder join condition before enabling this constraint")
        }})
        .penalize({penalty_expr})
        .named("{constraint_name}")"#
            ),
            format!(
                r#"

fn entity_items(solution: &{solution_type}) -> &[{entity_type}] {{
    solution.{entity_field}.as_slice()
}}

fn fact_items(solution: &{solution_type}) -> &[{fact_type}] {{
    solution.{fact_field}.as_slice()
}}"#
            ),
        ),

        Pattern::Balance => (
            format!(
                r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .balance(|e: &{entity_type}| e.{planning_var})
        .penalize({score_type}::of_soft(1))
        .named("{constraint_name}")"#
            ),
            String::new(),
        ),

        Pattern::Reward => (
            format!(
                r#"    ConstraintFactory::<{solution_type}, {score_type}>::new()
        .for_each(|s: &{solution_type}| s.{entity_field}.as_slice())
        .filter(|_e: &{entity_type}| {{
            panic!("replace placeholder reward condition before enabling this constraint")
        }})
        .reward({penalty_expr})
        .named("{constraint_name}")"#
            ),
            String::new(),
        ),
    };

    format!(
        "{imports}\n\n/// {hardness_comment}\npub fn constraint() -> impl IncrementalConstraint<{solution_type}, {score_type}> {{\n{body}\n}}{helpers}\n"
    )
}
