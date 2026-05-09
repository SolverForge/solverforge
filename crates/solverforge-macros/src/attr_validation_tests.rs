use quote::quote;

use super::{parse_serde_flag, parse_solution_flags};

#[test]
fn parse_solution_flags_accepts_current_arguments() {
    let flags = parse_solution_flags(quote! {
        serde,
        constraints = "constraints",
        config = "config",
        solver_toml = "solver.toml",
        search = "search::search",
        conflict_repairs = "repairs",
        scalar_groups = "groups"
    })
    .expect("current planning_solution arguments should parse");

    assert!(flags.has_serde);
    assert_eq!(flags.constraints_path.as_deref(), Some("constraints"));
    assert_eq!(flags.config_path.as_deref(), Some("config"));
    assert_eq!(flags.solver_toml_path.as_deref(), Some("solver.toml"));
    assert_eq!(flags.search_path.as_deref(), Some("search::search"));
    assert_eq!(flags.conflict_repairs_path.as_deref(), Some("repairs"));
    assert_eq!(flags.scalar_groups_path.as_deref(), Some("groups"));
}

#[test]
fn parse_solution_flags_rejects_unknown_arguments() {
    let error = parse_solution_flags(quote! { conflict_repair_providers = "repairs" })
        .expect_err("unknown planning_solution arguments should fail");

    assert!(error
        .to_string()
        .contains("unsupported planning_solution argument `conflict_repair_providers`"));
}

#[test]
fn parse_solution_flags_rejects_malformed_values() {
    let error = parse_solution_flags(quote! { conflict_repairs = repairs })
        .expect_err("non-string planning_solution values should fail");

    assert!(error
        .to_string()
        .contains("planning_solution argument `conflict_repairs` must be a string literal"));
}

#[test]
fn parse_solution_flags_rejects_invalid_search_path() {
    let error = parse_solution_flags(quote! { search = "search::" })
        .expect_err("invalid search path strings should fail");

    assert!(error
        .to_string()
        .contains("planning_solution argument `search` must be a valid Rust path string"));
}

#[test]
fn parse_serde_flag_rejects_unknown_top_level_arguments() {
    let error = parse_serde_flag(quote! { debug }, "planning_entity")
        .expect_err("unknown planning_entity arguments should fail");

    assert!(error
        .to_string()
        .contains("unsupported planning_entity argument `debug`"));
}
