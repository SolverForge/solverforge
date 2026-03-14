use crate::commands::generate_constraint;
use crate::commands::generate_domain;
use crate::error::CliResult;
use crate::output;

// Orchestrates `generate entity`, `generate constraint`, and optionally a paired twin entity
// in a single command invocation.
//
// Example:
//   solverforge generate scaffold shift employee_idx:usize --entity --constraint no_overlap --pair
//
// Arguments:
//   name        — primary entity name in snake_case
//   fields      — "name:Type" field specs for the entity (same syntax as `generate entity --field`)
//   with_entity — also run entity generation for `name`
//   constraint  — also generate a constraint with this name
//   pair        — also generate a paired twin entity named `<name>_pair`
#[allow(clippy::too_many_arguments)]
pub fn run(
    name: &str,
    fields: &[String],
    with_entity: bool,
    constraint: Option<&str>,
    pair: bool,
    force: bool,
    pretend: bool,
) -> CliResult {
    output::print_heading(&format!("Scaffolding '{}'", name));
    println!();

    // Parse "name:Type" fields into (field_name, field_type) for the planning variable.
    // The first field is used as the planning variable; remaining are extra fields.
    let (planning_variable, extra_fields) = split_planning_variable(fields);

    // Generate primary entity.
    if with_entity {
        output::print_status("scaffold", &format!("entity '{}'", name));
        generate_domain::run_entity(
            name,
            planning_variable.as_deref(),
            &extra_fields,
            force,
            pretend,
        )?;
    }

    // Generate paired twin entity.
    if pair {
        let pair_name = format!("{}_pair", name);
        output::print_status("scaffold", &format!("paired entity '{}'", pair_name));
        generate_domain::run_entity(
            &pair_name,
            planning_variable.as_deref(),
            &extra_fields,
            force,
            pretend,
        )?;
    }

    // Generate constraint.
    if let Some(constraint_name) = constraint {
        output::print_status("scaffold", &format!("constraint '{}'", constraint_name));
        // Use pair pattern when --pair was given, otherwise unary.
        let use_pair = pair;
        generate_constraint::run(
            constraint_name,
            false,     // soft
            !use_pair, // unary
            use_pair,  // pair
            false,     // join
            false,     // balance
            false,     // reward
            force,
            pretend,
        )?;
    }

    println!();
    if !output::is_quiet() {
        output::print_success(&format!("  Scaffold for '{}' complete", name));
        println!();
    }

    Ok(())
}

// Splits the flat field list into a planning variable (first field name only) and remaining extra fields.
// Fields are in "name:Type" format.
fn split_planning_variable(fields: &[String]) -> (Option<String>, Vec<String>) {
    if fields.is_empty() {
        return (None, vec![]);
    }

    let first = &fields[0];
    let planning_variable = first.split(':').next().map(|s| s.trim().to_string());
    let extra = fields[1..].to_vec();
    (planning_variable, extra)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ```
    /// // split_planning_variable extracts the first field name as planning variable.
    /// ```
    #[test]
    fn test_split_no_fields() {
        let (pv, extra) = split_planning_variable(&[]);
        assert!(pv.is_none());
        assert!(extra.is_empty());
    }

    #[test]
    fn test_split_one_field() {
        let fields = vec!["employee_idx:usize".to_string()];
        let (pv, extra) = split_planning_variable(&fields);
        assert_eq!(pv.as_deref(), Some("employee_idx"));
        assert!(extra.is_empty());
    }

    #[test]
    fn test_split_multiple_fields() {
        let fields = vec!["employee_idx:usize".to_string(), "start:String".to_string()];
        let (pv, extra) = split_planning_variable(&fields);
        assert_eq!(pv.as_deref(), Some("employee_idx"));
        assert_eq!(extra, vec!["start:String"]);
    }
}
