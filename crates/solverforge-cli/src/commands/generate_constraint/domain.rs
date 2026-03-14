use std::fs;
use std::path::Path;

type CollectionPair = (Vec<(String, String)>, Vec<(String, String)>);

#[derive(Debug)]
pub(crate) struct EntityInfo {
    pub field_name: String,
    pub item_type: String,
    pub planning_vars: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct FactInfo {
    pub field_name: String,
    pub item_type: String,
}

#[derive(Debug)]
pub(crate) struct DomainModel {
    pub solution_type: String,
    pub score_type: String,
    pub entities: Vec<EntityInfo>,
    pub facts: Vec<FactInfo>,
}

/// Parses `src/domain/*.rs` to extract planning solution, entities, facts, and variables.
/// Returns `None` if `src/domain/` doesn't exist or yields no useful information.
pub(crate) fn parse_domain() -> Option<DomainModel> {
    let domain_dir = Path::new("src/domain");
    if !domain_dir.exists() {
        return None;
    }

    let entries = fs::read_dir(domain_dir).ok()?;
    let mut all_src = String::new();
    let mut file_contents: Vec<(String, String)> = Vec::new(); // (type_name, src)

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(src) = fs::read_to_string(&path) {
            // Collect struct name from file for cross-file entity var lookup
            if let Some(struct_name) = find_struct_name(&src) {
                file_contents.push((struct_name, src.clone()));
            }
            all_src.push_str(&src);
            all_src.push('\n');
        }
    }

    if all_src.is_empty() {
        return None;
    }

    // Find solution type (struct with #[planning_solution])
    let solution_type = find_annotated_struct(&all_src, "planning_solution")?;

    // Find score type from the solution struct's score field
    let score_type =
        find_score_type(&all_src, &solution_type).unwrap_or_else(|| "HardSoftScore".to_string());

    // Find entity collections and fact collections in solution struct
    let (entities_raw, facts_raw) = find_collections(&all_src, &solution_type);

    // For each entity, find its planning variables by looking up its struct definition
    let entities: Vec<EntityInfo> = entities_raw
        .into_iter()
        .map(|(field_name, item_type)| {
            let planning_vars = find_planning_vars_for_type(&file_contents, &item_type);
            EntityInfo {
                field_name,
                item_type,
                planning_vars,
            }
        })
        .collect();

    let facts: Vec<FactInfo> = facts_raw
        .into_iter()
        .map(|(field_name, item_type)| FactInfo {
            field_name,
            item_type,
        })
        .collect();

    if entities.is_empty() && facts.is_empty() {
        return None;
    }

    Some(DomainModel {
        solution_type,
        score_type,
        entities,
        facts,
    })
}

fn find_struct_name(src: &str) -> Option<String> {
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("pub struct ") || t.starts_with("struct ") {
            let after = t.trim_start_matches("pub ").trim_start_matches("struct ");
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

pub(crate) fn find_annotated_struct(src: &str, attr: &str) -> Option<String> {
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.contains(&format!("#[{}]", attr)) || t.contains(&format!("#[{}(", attr)) {
            // Look ahead for struct definition
            for next_line in lines.iter().take(lines.len().min(i + 4)).skip(i + 1) {
                let next = next_line.trim();
                if next.starts_with("pub struct ") || next.starts_with("struct ") {
                    let after = next
                        .trim_start_matches("pub ")
                        .trim_start_matches("struct ");
                    let name: String = after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn find_score_type(src: &str, solution_type: &str) -> Option<String> {
    // Look for `score: Option<SomeScore>` or `score: SomeScore` in the solution struct
    let lines: Vec<&str> = src.lines().collect();
    let mut in_solution_struct = false;
    let mut brace_depth = 0i32;

    for line in &lines {
        let t = line.trim();
        if !in_solution_struct {
            if t.contains(&format!("struct {}", solution_type)) {
                in_solution_struct = true;
                brace_depth += t.chars().filter(|&c| c == '{').count() as i32;
                brace_depth -= t.chars().filter(|&c| c == '}').count() as i32;
            }
            continue;
        }
        brace_depth += t.chars().filter(|&c| c == '{').count() as i32;
        brace_depth -= t.chars().filter(|&c| c == '}').count() as i32;
        if brace_depth <= 0 {
            break;
        }
        // Look for score field
        if t.contains("score") && t.contains(':') {
            for score in &[
                "HardSoftDecimalScore",
                "HardMediumSoftScore",
                "HardSoftScore",
                "BendableScore",
                "SimpleScore",
            ] {
                if t.contains(score) {
                    return Some(score.to_string());
                }
            }
        }
    }
    None
}

/// Returns (entity_collections, fact_collections) as Vec<(field_name, item_type)>
fn find_collections(src: &str, solution_type: &str) -> CollectionPair {
    let lines: Vec<&str> = src.lines().collect();
    let mut in_solution_struct = false;
    let mut brace_depth = 0i32;
    let mut entities = Vec::new();
    let mut facts = Vec::new();
    let mut next_annotation: Option<&str> = None;

    for line in &lines {
        let t = line.trim();

        if !in_solution_struct {
            if t.contains(&format!("struct {}", solution_type)) {
                in_solution_struct = true;
            }
            continue;
        }

        brace_depth += t.chars().filter(|&c| c == '{').count() as i32;
        brace_depth -= t.chars().filter(|&c| c == '}').count() as i32;
        if brace_depth <= 0 && in_solution_struct && t.contains('}') {
            break;
        }

        if t.contains("#[planning_entity_collection]")
            || t.contains("#[planning_entity_collection(")
        {
            next_annotation = Some("entity");
        } else if t.contains("#[problem_fact_collection]")
            || t.contains("#[problem_fact_collection(")
        {
            next_annotation = Some("fact");
        } else if let Some(ann) = next_annotation.take() {
            if let Some((field, item)) = parse_vec_field(t) {
                match ann {
                    "entity" => entities.push((field, item)),
                    "fact" => facts.push((field, item)),
                    _ => {}
                }
            }
        } else {
            next_annotation = None;
        }
    }

    (entities, facts)
}

/// Parses `field_name: Vec<ItemType>` or `field_name: Vec<ItemType>,`
pub(crate) fn parse_vec_field(line: &str) -> Option<(String, String)> {
    let t = line.trim().trim_end_matches(',');
    // Look for "pub field: Vec<Type>" or "field: Vec<Type>"
    let t = t.trim_start_matches("pub ");
    if let Some(colon) = t.find(':') {
        let field = t[..colon].trim().to_string();
        let type_part = t[colon + 1..].trim();
        if let Some(inner) = extract_vec_inner(type_part) {
            return Some((field, inner));
        }
        // Also handle `[Type]` or `&[Type]` styles
        if type_part.starts_with("&[") || type_part.starts_with('[') {
            let inner = type_part
                .trim_start_matches('&')
                .trim_start_matches('[')
                .trim_end_matches(']');
            if !inner.is_empty() {
                return Some((field, inner.to_string()));
            }
        }
    }
    None
}

fn extract_vec_inner(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with("Vec<") && s.ends_with('>') {
        Some(s[4..s.len() - 1].to_string())
    } else if s.starts_with("Option<Vec<") && s.ends_with(">>") {
        Some(s[11..s.len() - 2].to_string())
    } else {
        None
    }
}

fn find_planning_vars_for_type(file_contents: &[(String, String)], type_name: &str) -> Vec<String> {
    for (struct_name, src) in file_contents {
        if struct_name == type_name {
            return find_planning_vars_in_src(src);
        }
    }
    Vec::new()
}

fn find_planning_vars_in_src(src: &str) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut vars = Vec::new();
    let mut next_is_var = false;

    for line in &lines {
        let t = line.trim();
        if t.contains("#[planning_variable]") || t.contains("#[planning_variable(") {
            next_is_var = true;
        } else if next_is_var {
            next_is_var = false;
            // Extract field name
            let t = t.trim_start_matches("pub ").trim_end_matches(',');
            if let Some(colon) = t.find(':') {
                let field = t[..colon].trim().to_string();
                if !field.is_empty() {
                    vars.push(field);
                }
            }
        }
    }
    vars
}
