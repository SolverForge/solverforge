use std::fs;
use std::path::Path;

use crate::commands::generate_constraint::parse_domain;
use crate::error::{CliError, CliResult};
use crate::output;

pub fn run() -> CliResult {
    let domain_dir = Path::new("src/domain");
    if !domain_dir.exists() {
        return Err(CliError::NotInProject {
            missing: "src/domain/",
        });
    }

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Check domain model
    match parse_domain() {
        Some(domain) => {
            output::print_status("check", "domain model");

            // Verify entities have planning variables
            for entity in &domain.entities {
                if entity.planning_vars.is_empty() {
                    warnings.push(format!(
                        "Entity '{}' has no planning variables — solver cannot optimize it",
                        entity.item_type
                    ));
                }
            }

            // Check that solution exists
            if domain.solution_type.is_empty() {
                errors.push("No planning solution found".to_string());
            }
        }
        None => {
            errors.push(
                "Cannot parse domain model — ensure src/domain/ has a #[planning_solution] struct"
                    .to_string(),
            );
        }
    }

    // Check constraints directory
    let constraints_dir = Path::new("src/constraints");
    if constraints_dir.exists() {
        output::print_status("check", "constraints");

        let mod_path = constraints_dir.join("mod.rs");
        if !mod_path.exists() {
            errors.push("src/constraints/mod.rs not found".to_string());
        } else {
            // Check that constraint files listed in mod.rs actually exist
            let mod_content = fs::read_to_string(&mod_path).unwrap_or_default();
            for line in mod_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                    let mod_name = trimmed
                        .trim_start_matches("mod ")
                        .trim_end_matches(';')
                        .trim();
                    let file = constraints_dir.join(format!("{}.rs", mod_name));
                    if !file.exists() {
                        errors.push(format!(
                            "Constraint module '{}' declared in mod.rs but file not found",
                            mod_name
                        ));
                    }
                }
            }
        }
    } else {
        warnings.push("src/constraints/ directory not found".to_string());
    }

    // Check solver.toml
    output::print_status("check", "solver.toml");
    if !Path::new("solver.toml").exists() {
        warnings.push("solver.toml not found — solver will use defaults".to_string());
    }

    // Check domain/mod.rs consistency
    let domain_mod = Path::new("src/domain/mod.rs");
    if domain_mod.exists() {
        output::print_status("check", "domain/mod.rs");
        let content = fs::read_to_string(domain_mod).unwrap_or_default();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                let mod_name = trimmed
                    .trim_start_matches("mod ")
                    .trim_end_matches(';')
                    .trim();
                let file = Path::new("src/domain").join(format!("{}.rs", mod_name));
                if !file.exists() {
                    errors.push(format!(
                        "Domain module '{}' declared in mod.rs but file not found",
                        mod_name
                    ));
                }
            }
        }
    }

    println!();

    // Report
    if errors.is_empty() && warnings.is_empty() {
        output::print_success("  All checks passed.");
    } else {
        for w in &warnings {
            println!("  warning: {}", w);
        }
        for e in &errors {
            println!("  error: {}", e);
        }
        println!();

        if !errors.is_empty() {
            return Err(CliError::general(format!(
                "{} error(s), {} warning(s)",
                errors.len(),
                warnings.len()
            )));
        }

        if !warnings.is_empty() {
            println!("  {} warning(s), 0 errors", warnings.len());
        }
    }

    Ok(())
}
