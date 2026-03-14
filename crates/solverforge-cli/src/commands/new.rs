use include_dir::{include_dir, Dir};
use std::path::Path;
use std::process::Command;

use crate::error::{is_rust_keyword, CliError, CliResult};
use crate::output;
use crate::template;

static BASIC_GENERIC_TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/basic/generic");

static EMPLOYEE_SCHEDULING_TEMPLATE: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/templates/basic/employee-scheduling");

static VEHICLE_ROUTING_TEMPLATE: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/templates/list/vehicle-routing");

const AVAILABLE_TEMPLATES: &str = "
  Standard Variable (each entity holds one value):
    --basic                         — generic standard-variable skeleton
    --basic=employee-scheduling     — assign employees to shifts

  List Variable (each entity owns an ordered sequence):
    --list                          — generic list-variable skeleton  (coming soon)
    --list=vehicle-routing          — capacitated vehicle routing (CVRP)";

pub fn run(
    name: &str,
    template: Template,
    skip_git: bool,
    skip_readme: bool,
    quiet: bool,
) -> CliResult {
    let crate_name = to_crate_name(name);

    // Validate project name
    validate_project_name(name, &crate_name)?;

    match template {
        Template::Basic => scaffold(
            name,
            &crate_name,
            &BASIC_GENERIC_TEMPLATE,
            "basic",
            skip_git,
            skip_readme,
            quiet,
        ),
        Template::BasicEmployeeScheduling => scaffold(
            name,
            &crate_name,
            &EMPLOYEE_SCHEDULING_TEMPLATE,
            "basic/employee-scheduling",
            skip_git,
            skip_readme,
            quiet,
        ),
        Template::List => Err(CliError::with_hint(
            "the generic list-variable skeleton is not yet available",
            format!("Available templates:{AVAILABLE_TEMPLATES}"),
        )),
        Template::ListVehicleRouting => scaffold(
            name,
            &crate_name,
            &VEHICLE_ROUTING_TEMPLATE,
            "list/vehicle-routing",
            skip_git,
            skip_readme,
            quiet,
        ),
    }
}

fn validate_project_name(name: &str, crate_name: &str) -> CliResult {
    if name.is_empty() {
        return Err(CliError::InvalidProjectName {
            name: name.to_string(),
            reason: "name cannot be empty",
        });
    }

    // Must start with a letter
    if !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(CliError::InvalidProjectName {
            name: name.to_string(),
            reason: "must start with a letter",
        });
    }

    // Only valid chars
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(CliError::InvalidProjectName {
            name: name.to_string(),
            reason: "may only contain letters, digits, hyphens, and underscores",
        });
    }

    // Check Rust keyword
    if is_rust_keyword(crate_name) {
        return Err(CliError::ReservedKeyword {
            name: crate_name.to_string(),
        });
    }

    Ok(())
}

fn scaffold(
    project_name: &str,
    crate_name: &str,
    template_dir: &Dir,
    label: &str,
    skip_git: bool,
    skip_readme: bool,
    quiet: bool,
) -> CliResult {
    let start = std::time::Instant::now();
    let dest = Path::new(project_name);
    if dest.exists() {
        return Err(CliError::DirectoryExists {
            name: project_name.to_string(),
        });
    }

    output::print_heading(&format!("Creating {} project '{}'", label, project_name));

    let vars: &[(&str, &str)] = &[
        ("project_name", project_name),
        ("crate_name", crate_name),
        ("solverforge_version", env!("CARGO_PKG_VERSION")),
    ];

    template::render(template_dir, dest, vars)?;

    // Write .gitignore
    let gitignore_content = "/target\n**/*.rs.bk\nCargo.lock\n";
    std::fs::write(dest.join(".gitignore"), gitignore_content).map_err(|e| CliError::IoError {
        context: "failed to write .gitignore".to_string(),
        source: e,
    })?;
    output::print_create(".gitignore");

    if !skip_readme {
        // Write README.md
        let readme = generate_readme(project_name, crate_name, label);
        std::fs::write(dest.join("README.md"), readme).map_err(|e| CliError::IoError {
            context: "failed to write README.md".to_string(),
            source: e,
        })?;
        output::print_create("README.md");
    }

    // Print file listing
    print_file_tree(dest, dest)?;

    if !skip_git {
        // git init
        let git_ok = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dest)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if git_ok {
            // Initial commit
            let add_ok = Command::new("git")
                .args(["add", "."])
                .current_dir(dest)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

            if add_ok {
                let _ = Command::new("git")
                    .args([
                        "commit",
                        "--quiet",
                        "-m",
                        "Initial commit from solverforge new",
                    ])
                    .current_dir(dest)
                    .status();
            }

            output::print_invoke("git init");
        }
    }

    println!();
    output::print_success(&format!(
        "  Project created in {} ({})",
        project_name,
        output::format_elapsed(start)
    ));
    println!();

    // Template-specific guidance
    print_template_guidance(project_name, label);

    // Optional cargo check prompt (skipped in quiet mode)
    if !quiet {
        run_cargo_check_prompt(dest)?;
    }

    Ok(())
}

fn run_cargo_check_prompt(dest: &Path) -> CliResult {
    use dialoguer::Confirm;

    let run_check = Confirm::new()
        .with_prompt("Run `cargo check` to verify the project compiles?")
        .default(true)
        .interact()
        .unwrap_or(false);

    if run_check {
        output::print_invoke("cargo check");
        let status = Command::new("cargo")
            .arg("check")
            .current_dir(dest)
            .status()
            .map_err(|e| CliError::IoError {
                context: "failed to run cargo check".to_string(),
                source: e,
            })?;

        if status.success() {
            output::print_success("  cargo check passed");
        } else {
            output::print_error("cargo check failed — the project may need fixes");
        }
    }

    Ok(())
}

fn print_template_guidance(project_name: &str, label: &str) {
    if output::is_quiet() {
        return;
    }

    println!("  Next steps:");
    println!("    cd {}", project_name);

    match label {
        "basic/employee-scheduling" => {
            println!("    solverforge server");
            println!();
            println!("  This template includes:");
            println!(
                "    - 7 real constraints (skill matching, no overlap, 10h gap, balance, etc.)"
            );
            println!("    - Web UI at http://localhost:7860 with timeline visualization");
            println!("    - CSV data loading (employees.csv, shifts.csv)");
            println!("    - Console output with phase timing");
        }
        "list/vehicle-routing" => {
            println!("    solverforge server");
            println!();
            println!("  This template includes:");
            println!("    - 2-phase solver (Clarke-Wright construction + late acceptance)");
            println!("    - Capacity constraint (hard) and total distance (soft)");
            println!("    - Web UI at http://localhost:7860 with route visualization");
            println!("    - Built-in CVRP demo instance");
        }
        "basic" => {
            println!("    solverforge generate solution schedule");
            println!("    solverforge generate entity task --planning-variable resource_idx");
            println!("    solverforge generate fact resource");
            println!("    solverforge generate constraint all_assigned --unary --hard");
            println!("    solverforge server");
        }
        _ => {
            println!("    solverforge server");
        }
    }

    println!();
}

fn print_file_tree(root: &Path, dir: &Path) -> CliResult {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| CliError::IoError {
            context: format!("failed to read directory {:?}", dir),
            source: e,
        })?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);

        if path.is_dir() {
            // Skip .git directory
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            print_file_tree(root, &path)?;
        } else {
            output::print_create(&relative.display().to_string());
        }
    }

    Ok(())
}

fn generate_readme(project_name: &str, _crate_name: &str, label: &str) -> String {
    let mut readme = format!("# {}\n\n", project_name);
    readme.push_str(&format!(
        "A SolverForge constraint optimization project (template: `{}`).\n\n",
        label
    ));
    readme.push_str("## Quick Start\n\n");
    readme.push_str("```bash\n");
    readme.push_str("# Start the solver server\n");
    readme.push_str("solverforge server\n\n");
    readme.push_str("# Or run directly\n");
    readme.push_str("cargo run --release\n");
    readme.push_str("```\n\n");
    readme.push_str("## Development\n\n");
    readme.push_str("```bash\n");
    readme.push_str("# Add a new constraint\n");
    readme.push_str("solverforge generate constraint my_rule --unary --hard\n\n");
    readme.push_str("# Add a domain entity\n");
    readme.push_str("solverforge generate entity worker --planning-variable shift_idx\n\n");
    readme.push_str("# Add a problem fact\n");
    readme.push_str("solverforge generate fact location\n\n");
    readme.push_str("# Remove a resource\n");
    readme.push_str("solverforge destroy constraint my_rule\n");
    readme.push_str("```\n\n");
    readme.push_str("## Project Structure\n\n");
    readme.push_str("| Directory | Purpose |\n");
    readme.push_str("|-----------|--------|\n");
    readme.push_str("| `src/domain/` | Planning entities, facts, and solution struct |\n");
    readme.push_str("| `src/constraints/` | Constraint definitions (scored by the solver) |\n");
    readme.push_str("| `src/solver/` | Solver service and configuration |\n");
    readme.push_str("| `src/api/` | HTTP routes and DTOs |\n");
    readme.push_str("| `src/data/` | Data loading and generation |\n");
    readme.push_str("| `solver.toml` | Solver configuration (termination, phases) |\n");
    readme
}

pub enum Template {
    Basic,
    BasicEmployeeScheduling,
    List,
    ListVehicleRouting,
}

impl Template {
    pub fn parse(basic: bool, list: bool, specialization: Option<&str>) -> CliResult<Self> {
        match (basic, list, specialization) {
            (true, false, None) => Ok(Template::Basic),
            (true, false, Some("employee-scheduling")) => Ok(Template::BasicEmployeeScheduling),
            (false, true, None) => Ok(Template::List),
            (false, true, Some("vehicle-routing")) => Ok(Template::ListVehicleRouting),
            (false, false, None) => Err(CliError::with_hint(
                "specify a template flag",
                format!("Available templates:{AVAILABLE_TEMPLATES}"),
            )),
            (true, true, _) => Err(CliError::general(
                "--basic and --list are mutually exclusive",
            )),
            (_, _, Some(s)) => Err(CliError::with_hint(
                format!("unknown specialization: '{}'", s),
                format!("Available templates:{AVAILABLE_TEMPLATES}"),
            )),
        }
    }
}

/// Converts a project name to a valid Rust crate name (underscores, lowercase).
fn to_crate_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}
