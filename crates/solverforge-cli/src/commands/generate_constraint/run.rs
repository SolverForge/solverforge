use std::fs;
use std::path::Path;

use super::domain::parse_domain;
use super::mod_rewriter::{extract_types, rewrite_mod};
use super::skeleton::generate_skeleton;
use super::utils::{snake_to_title, validate_name};
use super::wizard::resolve_pattern_and_hardness;
use crate::error::{CliError, CliResult};
use crate::output;

// Print lines present in `after` but not in `before`, prefixed with '+', when --verbose.
fn print_diff_verbose(before: &str, after: &str) {
    if !output::is_verbose() {
        return;
    }
    let before_lines: Vec<&str> = before.lines().collect();
    for line in after.lines() {
        if !before_lines.contains(&line) {
            println!("+ {}", line);
        }
    }
}

/// Runs `solverforge generate constraint <name> [pattern flags] [--hard|--soft]`.
#[allow(clippy::too_many_arguments)]
pub fn run(
    name: &str,
    soft: bool,
    unary: bool,
    pair: bool,
    join: bool,
    balance: bool,
    reward: bool,
    force: bool,
    pretend: bool,
) -> CliResult {
    validate_name(name)?;

    let constraints_dir = Path::new("src/constraints");
    let mod_path = constraints_dir.join("mod.rs");
    let new_file = constraints_dir.join(format!("{}.rs", name));

    if !constraints_dir.exists() {
        return Err(CliError::NotInProject {
            missing: "src/constraints/",
        });
    }
    if !mod_path.exists() {
        return Err(CliError::NotInProject {
            missing: "src/constraints/mod.rs",
        });
    }
    if new_file.exists() && !force {
        return Err(CliError::ResourceExists {
            kind: "constraint",
            name: name.to_string(),
        });
    }

    let mod_src = fs::read_to_string(&mod_path).map_err(|e| CliError::IoError {
        context: "failed to read src/constraints/mod.rs".to_string(),
        source: e,
    })?;

    // Parse domain model; fall back gracefully
    let domain = parse_domain();

    // Resolve solution/score types: prefer domain parser, fall back to mod.rs extraction
    let (solution_type, score_type) = if let Some(ref d) = domain {
        (d.solution_type.clone(), d.score_type.clone())
    } else {
        extract_types(&mod_src)
    };

    let constraint_name = snake_to_title(name);

    // Determine pattern + hardness
    let (pattern, is_soft) =
        resolve_pattern_and_hardness(soft, unary, pair, join, balance, reward, &domain)?;

    // Generate the new constraint file
    let skeleton = generate_skeleton(
        name,
        pattern,
        is_soft,
        &solution_type,
        &score_type,
        &constraint_name,
        domain.as_ref(),
    );

    if pretend {
        println!("Would create src/constraints/{}.rs", name);
        println!("Would update src/constraints/mod.rs");
        return Ok(());
    }

    fs::write(&new_file, &skeleton).map_err(|e| CliError::IoError {
        context: format!("failed to write {}", new_file.display()),
        source: e,
    })?;

    // Rewrite mod.rs
    let new_mod = rewrite_mod(&mod_src, name);
    fs::write(&mod_path, &new_mod).map_err(|e| CliError::IoError {
        context: "failed to write src/constraints/mod.rs".to_string(),
        source: e,
    })?;

    crate::commands::sf_config::add_constraint(name)?;

    output::print_create(&format!("src/constraints/{}.rs", name));
    print_diff_verbose("", &skeleton);
    output::print_update("src/constraints/mod.rs");
    print_diff_verbose(&mod_src, &new_mod);
    println!();
    if !output::is_quiet() {
        println!("  Next steps:");
        println!("    1. Open src/constraints/{}.rs", name);
        println!("    2. Replace the TODO placeholders with your domain logic");
        println!("    solverforge server  # test your constraint");
        println!();
    }

    Ok(())
}
