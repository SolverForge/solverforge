pub(crate) fn generate_entity(
    pascal: &str,
    planning_variable: Option<&str>,
    extra_fields: &[(String, String)],
) -> String {
    // Check for a custom override in `.solverforge/templates/entity.rs.tmpl`.
    let snake = pascal_to_snake(pascal);
    let fields_repr: String = extra_fields
        .iter()
        .map(|(n, t)| format!("pub {}: {}", n, t))
        .collect::<Vec<_>>()
        .join(", ");
    let vars: &[(&str, &str)] = &[
        ("NAME", pascal),
        ("SNAKE_NAME", &snake),
        ("FIELDS", &fields_repr),
    ];
    if let Some(custom) = crate::template::load_custom("entity", vars) {
        return custom;
    }
    let var_field = if let Some(var) = planning_variable {
        format!(
            "    #[planning_variable(allows_unassigned = true)]\n    pub {}: Option<usize>,\n",
            var
        )
    } else {
        String::new()
    };

    let var_init = if let Some(var) = planning_variable {
        format!(", {}: None", var)
    } else {
        String::new()
    };

    let extra_field_defs: String = extra_fields
        .iter()
        .map(|(n, t)| format!("    pub {}: {},\n", n, t))
        .collect();

    let extra_field_params: String = extra_fields
        .iter()
        .map(|(n, t)| format!(", {}: {}", n, t))
        .collect();

    let extra_field_inits: String = extra_fields
        .iter()
        .map(|(n, _)| format!(", {}", n))
        .collect();

    let test_module = generate_entity_test(pascal, planning_variable, extra_fields);

    format!(
        r#"use serde::{{Deserialize, Serialize}};
use solverforge::prelude::*;

/// TODO — describe this entity.
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct {pascal} {{
    #[planning_id]
    pub id: String,
{var_field}{extra_field_defs}}}

impl {pascal} {{
    pub fn new(id: impl Into<String>{extra_field_params}) -> Self {{
        Self {{ id: id.into(){var_init}{extra_field_inits} }}
    }}
}}
{test_module}"#
    )
}

pub(crate) fn generate_fact(pascal: &str, extra_fields: &[(String, String)]) -> String {
    // Check for a custom override in `.solverforge/templates/fact.rs.tmpl`.
    let snake = pascal_to_snake(pascal);
    let fields_repr: String = extra_fields
        .iter()
        .map(|(n, t)| format!("pub {}: {}", n, t))
        .collect::<Vec<_>>()
        .join(", ");
    let vars: &[(&str, &str)] = &[
        ("NAME", pascal),
        ("SNAKE_NAME", &snake),
        ("FIELDS", &fields_repr),
    ];
    if let Some(custom) = crate::template::load_custom("fact", vars) {
        return custom;
    }
    let extra_field_defs: String = extra_fields
        .iter()
        .map(|(n, t)| format!("    pub {}: {},\n", n, t))
        .collect();

    let extra_field_params: String = extra_fields
        .iter()
        .map(|(n, t)| format!(", {}: {}", n, t))
        .collect();

    let extra_field_inits: String = extra_fields
        .iter()
        .map(|(n, _)| format!(", {}", n))
        .collect();

    let test_module = generate_fact_test(pascal, extra_fields);

    format!(
        r#"use serde::{{Deserialize, Serialize}};
use solverforge::prelude::*;

/// TODO — describe this fact.
#[problem_fact]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {pascal} {{
    pub index: usize,
    pub name: String,
{extra_field_defs}}}

impl {pascal} {{
    pub fn new(index: usize, name: impl Into<String>{extra_field_params}) -> Self {{
        Self {{ index, name: name.into(){extra_field_inits} }}
    }}
}}
{test_module}"#
    )
}

pub(crate) fn generate_solution(pascal: &str, score: &str) -> String {
    // Check for a custom override in `.solverforge/templates/solution.rs.tmpl`.
    let snake = pascal_to_snake(pascal);
    let vars: &[(&str, &str)] = &[("NAME", pascal), ("SNAKE_NAME", &snake), ("FIELDS", score)];
    if let Some(custom) = crate::template::load_custom("solution", vars) {
        return custom;
    }
    format!(
        r#"use serde::{{Deserialize, Serialize}};
use solverforge::prelude::*;

#[planning_solution]
#[derive(Serialize, Deserialize)]
pub struct {pascal} {{
    #[planning_score]
    pub score: Option<{score}>,
}}

impl {pascal} {{
    pub fn new() -> Self {{
        Self {{ score: None }}
    }}
}}
"#
    )
}

fn generate_entity_test(
    pascal: &str,
    planning_variable: Option<&str>,
    extra_fields: &[(String, String)],
) -> String {
    let var_assert = if let Some(var) = planning_variable {
        format!("\n        assert!(entity.{}.is_none());", var)
    } else {
        String::new()
    };

    let extra_args = extra_field_test_args(extra_fields);
    let extra_asserts: String = extra_fields
        .iter()
        .map(|(n, _)| format!("\n        let _ = &entity.{};", n))
        .collect();

    format!(
        r#"
#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{snake}_construction() {{
        let entity = {pascal}::new("test-id"{extra_args});
        assert_eq!(entity.id, "test-id");{var_assert}{extra_asserts}
    }}
}}
"#,
        snake = pascal_to_snake(pascal),
        pascal = pascal,
        extra_args = extra_args,
        var_assert = var_assert,
        extra_asserts = extra_asserts,
    )
}

fn generate_fact_test(pascal: &str, extra_fields: &[(String, String)]) -> String {
    let extra_args = extra_field_test_args(extra_fields);
    let extra_asserts: String = extra_fields
        .iter()
        .map(|(n, _)| format!("\n        let _ = &fact.{};", n))
        .collect();

    format!(
        r#"
#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{snake}_construction() {{
        let fact = {pascal}::new(0, "test"{extra_args});
        assert_eq!(fact.index, 0);
        assert_eq!(fact.name, "test");{extra_asserts}
    }}
}}
"#,
        snake = pascal_to_snake(pascal),
        pascal = pascal,
        extra_args = extra_args,
        extra_asserts = extra_asserts,
    )
}

// Produce default test arguments for extra fields based on type.
fn extra_field_test_args(extra_fields: &[(String, String)]) -> String {
    extra_fields
        .iter()
        .map(|(_, t)| match t.as_str() {
            "String" | "&str" => ", \"test\".to_string()".to_string(),
            "f32" | "f64" => ", 0.0".to_string(),
            "bool" => ", false".to_string(),
            _ if t.starts_with("Option<") => ", None".to_string(),
            _ => ", Default::default()".to_string(),
        })
        .collect()
}

// Simple pascal → snake conversion for test fn names.
fn pascal_to_snake(pascal: &str) -> String {
    let mut result = String::new();
    for (i, c) in pascal.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}
