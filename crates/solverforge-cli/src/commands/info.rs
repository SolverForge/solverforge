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

    // Read project name from Cargo.toml
    let project_name = read_project_name().unwrap_or_else(|| "<unknown>".to_string());
    output::print_heading(&format!("Project: {}", project_name));
    println!();

    // Parse domain model
    match parse_domain() {
        Some(domain) => {
            println!("  Solution:    {}", domain.solution_type);
            println!("  Score type:  {}", domain.score_type);
            println!();

            if !domain.entities.is_empty() {
                println!("  Entities:");
                for entity in &domain.entities {
                    if entity.planning_vars.is_empty() {
                        println!("    - {}", entity.item_type);
                    } else {
                        println!(
                            "    - {} (variables: {})",
                            entity.item_type,
                            entity.planning_vars.join(", ")
                        );
                    }
                }
                println!();
            }

            if !domain.facts.is_empty() {
                println!("  Facts:");
                for fact in &domain.facts {
                    println!("    - {}", fact.item_type);
                }
                println!();
            }
        }
        None => {
            println!("  No planning solution found in src/domain/");
            println!();
        }
    }

    // List constraints
    let constraints_dir = Path::new("src/constraints");
    if constraints_dir.exists() {
        let constraints = list_constraints(constraints_dir);
        if !constraints.is_empty() {
            println!("  Constraints:");
            for c in &constraints {
                println!("    - {}", c);
            }
            println!();
        }
    }

    // Show solver.toml summary
    if Path::new("solver.toml").exists() {
        println!("  Config:      solver.toml");
    }

    Ok(())
}

fn read_project_name() -> Option<String> {
    let cargo = fs::read_to_string("Cargo.toml").ok()?;
    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains('=') {
            let val = trimmed.split('=').nth(1)?.trim();
            return Some(val.trim_matches('"').to_string());
        }
    }
    None
}

fn list_constraints(dir: &Path) -> Vec<String> {
    let mut constraints = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if name != "mod" {
                    constraints.push(name.to_string());
                }
            }
        }
    }
    constraints.sort();
    constraints
}
