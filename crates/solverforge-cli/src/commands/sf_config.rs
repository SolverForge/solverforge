// JSON-based sf-config.json reader/writer for UI wiring.
// Reads/modifies/writes static/sf-config.json.
// If the file doesn't exist, operations return Ok(()) silently.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CliResult;

#[derive(Debug, Deserialize, Serialize)]
struct EntityEntry {
    name: String,
    label: String,
    plural: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SfConfig {
    title: String,
    subtitle: String,
    #[serde(default)]
    constraints: Vec<String>,
    #[serde(default)]
    entities: Vec<EntityEntry>,
    #[serde(default)]
    facts: Vec<EntityEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    view: Option<Value>,
}

const CONFIG_PATH: &str = "static/sf-config.json";

fn load() -> Option<SfConfig> {
    let path = Path::new(CONFIG_PATH);
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save(config: &SfConfig) -> CliResult {
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        crate::error::CliError::general(format!("sf-config serialize error: {}", e))
    })?;
    fs::write(CONFIG_PATH, json).map_err(|e| crate::error::CliError::IoError {
        context: "failed to write sf-config.json".to_string(),
        source: e,
    })?;
    Ok(())
}

pub fn add_constraint(name: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    if !config.constraints.contains(&name.to_string()) {
        config.constraints.push(name.to_string());
        save(&config)?;
    }
    Ok(())
}

pub fn remove_constraint(name: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    config.constraints.retain(|c| c != name);
    save(&config)
}

pub fn add_entity(name: &str, label: &str, plural: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    if !config.entities.iter().any(|e| e.name == name) {
        config.entities.push(EntityEntry {
            name: name.to_string(),
            label: label.to_string(),
            plural: plural.to_string(),
        });
        save(&config)?;
    }
    Ok(())
}

pub fn remove_entity(name: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    config.entities.retain(|e| e.name != name);
    save(&config)
}

pub fn add_fact(name: &str, label: &str, plural: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    if !config.facts.iter().any(|f| f.name == name) {
        config.facts.push(EntityEntry {
            name: name.to_string(),
            label: label.to_string(),
            plural: plural.to_string(),
        });
        save(&config)?;
    }
    Ok(())
}

pub fn remove_fact(name: &str) -> CliResult {
    let Some(mut config) = load() else {
        return Ok(());
    };
    config.facts.retain(|f| f.name != name);
    save(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    #[test]
    fn preserves_arbitrary_view_shape_when_updating_constraints() {
        let guard = test_support::lock_cwd();

        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let original_dir = std::env::current_dir().expect("failed to get current dir");

        std::fs::create_dir_all(tmp.path().join("static")).expect("failed to create static dir");
        std::fs::write(
            tmp.path().join("static").join("sf-config.json"),
            r#"{
  "title": "demo",
  "subtitle": "demo",
  "constraints": ["all_assigned"],
  "view": {
    "type": "assignment_board",
    "columns": [{"id": "todo"}],
    "meta": {"accent": "red"}
  }
}"#,
        )
        .expect("failed to write sf-config");

        std::env::set_current_dir(tmp.path()).expect("failed to enter temp dir");
        add_constraint("capacity_limit").expect("add_constraint should succeed");
        let saved = std::fs::read_to_string(tmp.path().join("static").join("sf-config.json"))
            .expect("failed to read saved sf-config");
        std::env::set_current_dir(original_dir).expect("failed to restore current dir");
        drop(guard);

        assert!(saved.contains("\"capacity_limit\""));
        assert!(saved.contains("\"assignment_board\""));
        assert!(saved.contains("\"columns\""));
        assert!(saved.contains("\"accent\": \"red\""));
    }
}
