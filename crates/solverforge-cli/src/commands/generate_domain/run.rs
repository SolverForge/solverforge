use std::fs;
use std::path::Path;

use crate::commands::generate_constraint::parse_domain;
use crate::commands::generate_constraint::validate_name;
use crate::error::{CliError, CliResult};
use crate::output;

use super::generators::{generate_entity, generate_fact, generate_solution};
use super::utils::{ensure_domain_dir, find_file_for_type, snake_to_pascal, validate_score_type};
use super::wiring::{
    inject_planning_variable, replace_score_type, update_domain_mod, wire_collection_into_solution,
};

pub fn run_entity(
    name: &str,
    planning_variable: Option<&str>,
    fields: &[String],
    force: bool,
    pretend: bool,
) -> CliResult {
    validate_name(name)?;
    if let Some(var) = planning_variable {
        validate_name(var)?;
    }
    let parsed_fields = parse_fields(fields)?;

    let domain_dir = Path::new("src/domain");
    ensure_domain_dir(domain_dir)?;

    let pascal = snake_to_pascal(name);
    let file_path = domain_dir.join(format!("{}.rs", name));

    if file_path.exists() && !force {
        return Err(CliError::ResourceExists {
            kind: "entity",
            name: name.to_string(),
        });
    }

    let src = generate_entity(&pascal, planning_variable, &parsed_fields);

    if pretend {
        println!("Would create {}", file_path.display());
        println!("Would update src/domain/mod.rs");
        println!("Would wire into planning solution");
        return Ok(());
    }

    fs::write(&file_path, &src).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", file_path.display()),
        source: e,
    })?;

    update_domain_mod(name, &pascal)?;
    wire_collection_into_solution(name, &pascal, "planning_entity_collection")?;

    output::print_create(file_path.to_str().unwrap());
    print_diff_verbose("", &src);
    output::print_update("src/domain/mod.rs");
    Ok(())
}

pub fn run_fact(name: &str, fields: &[String], force: bool, pretend: bool) -> CliResult {
    validate_name(name)?;
    let parsed_fields = parse_fields(fields)?;

    let domain_dir = Path::new("src/domain");
    ensure_domain_dir(domain_dir)?;

    let pascal = snake_to_pascal(name);
    let file_path = domain_dir.join(format!("{}.rs", name));

    if file_path.exists() && !force {
        return Err(CliError::ResourceExists {
            kind: "fact",
            name: name.to_string(),
        });
    }

    let src = generate_fact(&pascal, &parsed_fields);

    if pretend {
        println!("Would create {}", file_path.display());
        println!("Would update src/domain/mod.rs");
        println!("Would wire into planning solution");
        return Ok(());
    }

    fs::write(&file_path, &src).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", file_path.display()),
        source: e,
    })?;

    update_domain_mod(name, &pascal)?;
    wire_collection_into_solution(name, &pascal, "problem_fact_collection")?;

    output::print_create(file_path.to_str().unwrap());
    print_diff_verbose("", &src);
    output::print_update("src/domain/mod.rs");
    Ok(())
}

// Parse "name:Type" field specifications.
fn parse_fields(fields: &[String]) -> CliResult<Vec<(String, String)>> {
    fields
        .iter()
        .map(|f| {
            let mut parts = f.splitn(2, ':');
            let field_name = parts.next().unwrap_or("").trim().to_string();
            let field_type = parts.next().unwrap_or("").trim().to_string();
            if field_name.is_empty() || field_type.is_empty() {
                Err(CliError::general(format!(
                    "invalid --field '{}': expected 'name:Type'",
                    f
                )))
            } else {
                Ok((field_name, field_type))
            }
        })
        .collect()
}

// Print lines of new_src prefixed with '+' when verbose.
fn print_diff_verbose(before: &str, after: &str) {
    if !output::is_verbose() {
        return;
    }
    let before_lines: Vec<&str> = if before.is_empty() {
        vec![]
    } else {
        before.lines().collect()
    };
    let after_lines: Vec<&str> = after.lines().collect();
    for line in &after_lines {
        if !before_lines.contains(line) {
            println!("+ {}", line);
        }
    }
}

pub fn run_solution(name: &str, score: &str) -> CliResult {
    validate_name(name)?;
    validate_score_type(score)?;

    let domain_dir = Path::new("src/domain");
    ensure_domain_dir(domain_dir)?;

    // Check if any solution already exists
    if let Some(domain) = parse_domain() {
        if is_default_scaffold()? {
            remove_default_scaffold()?;
        } else {
            return Err(CliError::with_hint(
                format!(
                    "a planning solution '{}' already exists",
                    domain.solution_type
                ),
                "use `solverforge destroy solution` then `solverforge generate solution` to replace it",
            ));
        }
    }

    let pascal = snake_to_pascal(name);
    let file_path = domain_dir.join(format!("{}.rs", name));
    if file_path.exists() {
        return Err(CliError::ResourceExists {
            kind: "solution file",
            name: file_path.display().to_string(),
        });
    }

    let src = generate_solution(&pascal, score);
    fs::write(&file_path, src).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", file_path.display()),
        source: e,
    })?;

    update_domain_mod(name, &pascal)?;

    output::print_create(file_path.to_str().unwrap());
    output::print_update("src/domain/mod.rs");
    Ok(())
}

pub fn run_variable(field: &str, entity: &str) -> CliResult {
    validate_name(field)?;

    let domain_dir = Path::new("src/domain");
    if !domain_dir.exists() {
        return Err(CliError::NotInProject {
            missing: "src/domain/",
        });
    }

    let entity_file = find_file_for_type(domain_dir, entity)?;

    let src = fs::read_to_string(&entity_file).map_err(|e| CliError::IoError {
        context: format!("failed to read {}", entity_file.display()),
        source: e,
    })?;

    let new_src = inject_planning_variable(&src, entity, field)?;
    fs::write(&entity_file, new_src).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", entity_file.display()),
        source: e,
    })?;

    output::print_update(entity_file.to_str().unwrap());
    Ok(())
}

pub fn run_score(score_type: &str) -> CliResult {
    validate_score_type(score_type)?;

    let domain_dir = Path::new("src/domain");
    if !domain_dir.exists() {
        return Err(CliError::NotInProject {
            missing: "src/domain/",
        });
    }

    let domain = parse_domain().ok_or(CliError::NotInProject {
        missing: "src/domain/ (no planning solution found)",
    })?;
    let solution_file = find_file_for_type(domain_dir, &domain.solution_type)?;

    let src = fs::read_to_string(&solution_file).map_err(|e| CliError::IoError {
        context: format!("failed to read {}", solution_file.display()),
        source: e,
    })?;

    let new_src = replace_score_type(&src, &domain.score_type, score_type)?;
    fs::write(&solution_file, new_src).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", solution_file.display()),
        source: e,
    })?;

    output::print_update(solution_file.to_str().unwrap());
    Ok(())
}

fn is_default_scaffold() -> CliResult<bool> {
    let plan_path = Path::new("src/domain/plan.rs");
    if !plan_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(plan_path).map_err(|e| CliError::IoError {
        context: "failed to read plan.rs".to_string(),
        source: e,
    })?;

    Ok(content.contains("Rename this to something domain-specific"))
}

fn remove_default_scaffold() -> CliResult {
    let domain_files = ["plan.rs", "task.rs", "resource.rs"];
    for file in &domain_files {
        let path = Path::new("src/domain").join(file);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| CliError::IoError {
                context: format!("failed to remove {}", file),
                source: e,
            })?;
        }
    }

    let domain_mod = Path::new("src/domain/mod.rs");
    if domain_mod.exists() {
        fs::write(domain_mod, "// Domain module\n").map_err(|e| CliError::IoError {
            context: "failed to clear domain/mod.rs".to_string(),
            source: e,
        })?;
    }

    let all_assigned = Path::new("src/constraints/all_assigned.rs");
    if all_assigned.exists() {
        fs::remove_file(all_assigned).map_err(|e| CliError::IoError {
            context: "failed to remove all_assigned.rs".to_string(),
            source: e,
        })?;
    }

    let constraints_mod = Path::new("src/constraints/mod.rs");
    if constraints_mod.exists() {
        let content = fs::read_to_string(constraints_mod).map_err(|e| CliError::IoError {
            context: "failed to read constraints/mod.rs".to_string(),
            source: e,
        })?;

        let lines: Vec<&str> = content
            .lines()
            .filter(|line| !line.contains("all_assigned"))
            .collect();

        let new_content = lines.join("\n");
        fs::write(constraints_mod, new_content).map_err(|e| CliError::IoError {
            context: "failed to update constraints/mod.rs".to_string(),
            source: e,
        })?;
    }

    let data_mod = Path::new("src/data/mod.rs");
    if data_mod.exists() {
        let stub_content = "// Data loading module\n\npub fn load() -> Result<(), Box<dyn std::error::Error>> {\n    todo!(\"Implement data loading\")\n}\n";
        fs::write(data_mod, stub_content).map_err(|e| CliError::IoError {
            context: "failed to stub data/mod.rs".to_string(),
            source: e,
        })?;
    }

    output::print_remove("default scaffold");
    Ok(())
}
