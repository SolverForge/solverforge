use std::fs;
use std::path::Path;

use crate::error::{CliError, CliResult};
use crate::output;

const CONFIG_PATH: &str = "solver.toml";

// Print the contents of solver.toml.
pub fn run_show() -> CliResult {
    let path = Path::new(CONFIG_PATH);
    if !path.exists() {
        return Err(CliError::NotInProject {
            missing: "solver.toml",
        });
    }

    let content = fs::read_to_string(path).map_err(|e| CliError::IoError {
        context: "failed to read solver.toml".to_string(),
        source: e,
    })?;

    output::print_status("config", CONFIG_PATH);
    println!();
    println!("{}", content.trim_end());
    println!();

    Ok(())
}

// Set a dotted key path in solver.toml to the given value.
pub fn run_set(key: &str, value: &str) -> CliResult {
    let path = Path::new(CONFIG_PATH);
    if !path.exists() {
        return Err(CliError::NotInProject {
            missing: "solver.toml",
        });
    }

    let content = fs::read_to_string(path).map_err(|e| CliError::IoError {
        context: "failed to read solver.toml".to_string(),
        source: e,
    })?;

    let mut doc: toml::Value = content.parse().map_err(|e: toml::de::Error| {
        CliError::general(format!("failed to parse solver.toml: {}", e))
    })?;

    set_toml_key(&mut doc, key, value)?;

    let new_content = toml::to_string_pretty(&doc)
        .map_err(|e| CliError::general(format!("failed to serialize solver.toml: {}", e)))?;

    fs::write(path, &new_content).map_err(|e| CliError::IoError {
        context: "failed to write solver.toml".to_string(),
        source: e,
    })?;

    output::print_update(CONFIG_PATH);
    Ok(())
}

// Navigate and set a dotted key path (e.g. "termination.time_spent_seconds") in a toml::Value.
fn set_toml_key(doc: &mut toml::Value, key: &str, value: &str) -> CliResult {
    let parts: Vec<&str> = key.splitn(2, '.').collect();

    if parts.len() == 1 {
        let leaf = parts[0];
        let table = doc
            .as_table_mut()
            .ok_or_else(|| CliError::general("solver.toml root is not a TOML table"))?;
        table.insert(leaf.to_string(), parse_toml_value(value));
        return Ok(());
    }

    let section = parts[0];
    let rest = parts[1];

    let table = doc
        .as_table_mut()
        .ok_or_else(|| CliError::general("solver.toml root is not a TOML table"))?;

    let child = table
        .entry(section.to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    set_toml_key(child, rest, value)
}

// Parse a string into a toml::Value: try integer, then float, then bool, then string.
fn parse_toml_value(s: &str) -> toml::Value {
    if let Ok(i) = s.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return toml::Value::Float(f);
    }
    match s {
        "true" => return toml::Value::Boolean(true),
        "false" => return toml::Value::Boolean(false),
        _ => {}
    }
    toml::Value::String(s.to_string())
}
