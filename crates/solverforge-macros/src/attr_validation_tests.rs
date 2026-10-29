use quote::{quote, ToTokens};

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
    assert_eq!(
        path_text(&flags.constraints_path),
        Some("constraints".to_string())
    );
    assert_eq!(path_text(&flags.config_path), Some("config".to_string()));
    assert_eq!(flags.solver_toml_path.as_deref(), Some("solver.toml"));
    assert_eq!(
        path_text(&flags.search_path),
        Some("search :: search".to_string())
    );
    assert_eq!(
        path_text(&flags.conflict_repairs_path),
        Some("repairs".to_string())
    );
    assert_eq!(
        path_text(&flags.scalar_groups_path),
        Some("groups".to_string())
    );
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
fn parse_solution_flags_rejects_every_invalid_rust_path() {
    let cases = [
        quote! { search = "search::" },
        quote! { constraints = "crate::constraints::create constraints" },
        quote! { constraints = "" },
        quote! { config = "crate::my config" },
        quote! { conflict_repairs = "repairs()" },
        quote! { scalar_groups = "groups-fn" },
    ];
    for (args, name) in cases.into_iter().zip([
        "search",
        "constraints",
        "constraints",
        "config",
        "conflict_repairs",
        "scalar_groups",
    ]) {
        let error = parse_solution_flags(args).expect_err("invalid path strings should fail");
        assert_eq!(
            error.to_string(),
            format!("planning_solution argument `{name}` must be a valid Rust path string")
        );
    }
}

fn path_text(path: &Option<syn::Path>) -> Option<String> {
    path.as_ref().map(|path| path.to_token_stream().to_string())
}

#[test]
fn parse_serde_flag_rejects_unknown_top_level_arguments() {
    let error = parse_serde_flag(quote! { debug }, "planning_entity")
        .expect_err("unknown planning_entity arguments should fail");

    assert!(error
        .to_string()
        .contains("unsupported planning_entity argument `debug`"));
}
