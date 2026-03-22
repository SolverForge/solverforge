use std::fs;
use std::path::Path;

use crate::commands::generate_constraint::parse_domain;
use crate::commands::generate_domain::{find_file_for_type, snake_to_pascal};
use crate::error::{CliError, CliResult};
use crate::output;

fn confirm_destroy(kind: &str, name: &str, skip_confirm: bool) -> CliResult<bool> {
    if skip_confirm {
        return Ok(true);
    }

    let prompt = format!("Remove {} '{}'?", kind, name);
    let confirmed = dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| CliError::general(format!("prompt failed: {}", e)))?;

    Ok(confirmed)
}

pub fn run_solution(skip_confirm: bool) -> CliResult {
    let domain = parse_domain().ok_or(CliError::NotInProject {
        missing: "src/domain/ (no planning solution found)",
    })?;

    if !confirm_destroy("solution", &domain.solution_type, skip_confirm)? {
        output::print_skip(&format!("solution {}", domain.solution_type));
        return Ok(());
    }

    let domain_dir = Path::new("src/domain");
    let solution_file = find_file_for_type(domain_dir, &domain.solution_type)?;

    let file_name = solution_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CliError::general("invalid solution file name"))?
        .to_string();

    fs::remove_file(&solution_file).map_err(|e| CliError::IoError {
        context: format!("failed to delete {}", solution_file.display()),
        source: e,
    })?;

    remove_from_domain_mod(&file_name)?;

    output::print_remove(&format!("src/domain/{}.rs", file_name));
    output::print_update("src/domain/mod.rs");
    Ok(())
}

pub fn run_entity(name: &str, skip_confirm: bool) -> CliResult {
    let domain = parse_domain().ok_or(CliError::NotInProject {
        missing: "src/domain/",
    })?;

    let snake = name.to_lowercase().replace('-', "_");
    let pascal = snake_to_pascal(&snake);

    let entity = domain
        .entities
        .iter()
        .find(|e| e.item_type == pascal)
        .ok_or_else(|| CliError::ResourceNotFound {
            kind: "entity",
            name: name.to_string(),
        })?;

    if !confirm_destroy("entity", &pascal, skip_confirm)? {
        output::print_skip(&format!("entity {}", pascal));
        return Ok(());
    }

    let domain_dir = Path::new("src/domain");
    let file_path = find_file_for_type(domain_dir, &pascal).or_else(|_| {
        let path = domain_dir.join(format!("{}.rs", snake));
        if path.exists() {
            Ok(path)
        } else {
            Err(CliError::ResourceNotFound {
                kind: "entity file",
                name: pascal.clone(),
            })
        }
    })?;

    let file_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CliError::general("invalid entity file name"))?
        .to_string();

    fs::remove_file(&file_path).map_err(|e| CliError::IoError {
        context: format!("failed to delete {}", file_path.display()),
        source: e,
    })?;

    remove_from_domain_mod(&file_name)?;
    unwire_collection_from_solution(&entity.field_name, &entity.item_type, &domain.solution_type)?;
    crate::commands::sf_config::remove_entity(&snake)?;

    output::print_remove(&format!("src/domain/{}.rs", file_name));
    output::print_update("src/domain/mod.rs");
    Ok(())
}

pub fn run_fact(name: &str, skip_confirm: bool) -> CliResult {
    let domain = parse_domain().ok_or(CliError::NotInProject {
        missing: "src/domain/",
    })?;

    let snake = name.to_lowercase().replace('-', "_");
    let pascal = snake_to_pascal(&snake);

    let fact = domain
        .facts
        .iter()
        .find(|f| f.item_type == pascal)
        .ok_or_else(|| CliError::ResourceNotFound {
            kind: "fact",
            name: name.to_string(),
        })?;

    if !confirm_destroy("fact", &pascal, skip_confirm)? {
        output::print_skip(&format!("fact {}", pascal));
        return Ok(());
    }

    let domain_dir = Path::new("src/domain");
    let file_path = find_file_for_type(domain_dir, &pascal).or_else(|_| {
        let path = domain_dir.join(format!("{}.rs", snake));
        if path.exists() {
            Ok(path)
        } else {
            Err(CliError::ResourceNotFound {
                kind: "fact file",
                name: pascal.clone(),
            })
        }
    })?;

    let file_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CliError::general("invalid fact file name"))?
        .to_string();

    fs::remove_file(&file_path).map_err(|e| CliError::IoError {
        context: format!("failed to delete {}", file_path.display()),
        source: e,
    })?;

    remove_from_domain_mod(&file_name)?;
    unwire_collection_from_solution(&fact.field_name, &fact.item_type, &domain.solution_type)?;
    crate::commands::sf_config::remove_fact(&snake)?;

    output::print_remove(&format!("src/domain/{}.rs", file_name));
    output::print_update("src/domain/mod.rs");
    Ok(())
}

pub fn run_constraint(name: &str, skip_confirm: bool) -> CliResult {
    let snake = name.to_lowercase().replace('-', "_");
    let file_path = format!("src/constraints/{}.rs", snake);

    if !Path::new(&file_path).exists() {
        return Err(CliError::ResourceNotFound {
            kind: "constraint",
            name: name.to_string(),
        });
    }

    if !confirm_destroy("constraint", name, skip_confirm)? {
        output::print_skip(&format!("constraint {}", name));
        return Ok(());
    }

    fs::remove_file(&file_path).map_err(|e| CliError::IoError {
        context: format!("failed to delete {}", file_path),
        source: e,
    })?;

    remove_constraint_from_mod(&snake)?;
    crate::commands::sf_config::remove_constraint(&snake)?;

    output::print_remove(&file_path);
    output::print_update("src/constraints/mod.rs");
    Ok(())
}

fn remove_from_domain_mod(mod_name: &str) -> CliResult {
    let mod_path = Path::new("src/domain/mod.rs");
    if !mod_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(mod_path).map_err(|e| CliError::IoError {
        context: "failed to read src/domain/mod.rs".to_string(),
        source: e,
    })?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();

    for line in lines {
        if line.trim() == format!("mod {};", mod_name)
            || line.trim().starts_with(&format!("pub use {}::", mod_name))
        {
            continue;
        }
        new_lines.push(line.to_string());
    }

    let new_content = new_lines.join("\n");
    fs::write(mod_path, new_content).map_err(|e| CliError::IoError {
        context: "failed to update src/domain/mod.rs".to_string(),
        source: e,
    })?;

    Ok(())
}

fn unwire_collection_from_solution(
    field_name: &str,
    type_name: &str,
    solution_type: &str,
) -> CliResult {
    let domain_dir = Path::new("src/domain");
    let solution_file = find_file_for_type(domain_dir, solution_type)?;

    let content = fs::read_to_string(&solution_file).map_err(|e| CliError::IoError {
        context: format!("failed to read {}", solution_file.display()),
        source: e,
    })?;

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];

        if line.contains(&format!("{}: Vec<{}>", field_name, type_name)) {
            let mut start = i;
            while start > 0 && lines[start - 1].trim().starts_with('#') {
                start -= 1;
            }
            lines.drain(start..=i);
            i = start;
            continue;
        }

        if line.contains(&format!("{}: Vec::new()", field_name)) {
            lines.remove(i);
            continue;
        }

        if line.trim() == format!("use super::{};", type_name) {
            lines.remove(i);
            continue;
        }

        i += 1;
    }

    let new_content = lines.join("\n");
    fs::write(&solution_file, new_content).map_err(|e| CliError::IoError {
        context: format!("failed to update {}", solution_file.display()),
        source: e,
    })?;

    Ok(())
}

fn remove_constraint_from_mod(name: &str) -> CliResult {
    let mod_path = Path::new("src/constraints/mod.rs");
    if !mod_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(mod_path).map_err(|e| CliError::IoError {
        context: "failed to read src/constraints/mod.rs".to_string(),
        source: e,
    })?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();

    for line in lines {
        if line.trim() == format!("mod {};", name) {
            continue;
        }

        if let Some(updated_line) = remove_constraint_call_from_line(line, name) {
            if updated_line.trim().is_empty() {
                continue;
            }
            new_lines.push(updated_line);
            continue;
        }
        new_lines.push(line.to_string());
    }

    let result = new_lines.join("\n");

    fs::write(mod_path, result).map_err(|e| CliError::IoError {
        context: "failed to update src/constraints/mod.rs".to_string(),
        source: e,
    })?;

    Ok(())
}

fn remove_constraint_call_from_line(line: &str, name: &str) -> Option<String> {
    let needle = format!("{name}::constraint()");
    if !line.contains(&needle) {
        return None;
    }

    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let trimmed = line.trim();
    let had_trailing_comma = trimmed.ends_with(',');
    let without_trailing_comma = trimmed.trim_end_matches(',');
    let has_tuple_wrapper =
        without_trailing_comma.starts_with('(') && without_trailing_comma.ends_with(')');
    let inner = if has_tuple_wrapper {
        &without_trailing_comma[1..without_trailing_comma.len() - 1]
    } else {
        without_trailing_comma
    };

    let kept_parts: Vec<&str> = inner
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != needle)
        .collect();

    if kept_parts.is_empty() {
        return Some(String::new());
    }

    let mut rebuilt = if has_tuple_wrapper {
        format!("({})", kept_parts.join(", "))
    } else {
        kept_parts.join(", ")
    };

    if had_trailing_comma {
        rebuilt.push(',');
    }

    Some(format!("{indent}{rebuilt}"))
}

#[cfg(test)]
mod tests {
    use super::remove_constraint_call_from_line;

    #[test]
    fn removes_constraint_from_multiline_tuple_entry() {
        let line = "            all_assigned::constraint(),";
        let updated = remove_constraint_call_from_line(line, "all_assigned")
            .expect("line should be rewritten");
        assert!(updated.is_empty());
    }

    #[test]
    fn removes_constraint_from_flat_tuple_line() {
        let line = "    (capacity::constraint(), extra::constraint(), distance::constraint())";
        let updated =
            remove_constraint_call_from_line(line, "extra").expect("line should be rewritten");
        assert_eq!(
            updated,
            "    (capacity::constraint(), distance::constraint())"
        );
    }
}
