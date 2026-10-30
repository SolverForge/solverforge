use solverforge_config::SolverConfig;

use super::super::selector_values::validate_selector_values;
use super::*;

fn parse(toml: &str) -> SolverConfig {
    SolverConfig::from_toml_str(toml).unwrap_or_else(|error| panic!("{error}\n{toml}"))
}

/// One local-search phase whose `move_selector` is the given selector body.
fn local_search(selector: &str) -> SolverConfig {
    parse(&format!(
        "[[phases]]\ntype = \"construction_heuristic\"\n\n\
         [[phases]]\ntype = \"local_search\"\n\n\
         [phases.move_selector]\n{selector}\n"
    ))
}

fn construction(fields: &str) -> SolverConfig {
    parse(&format!(
        "[[phases]]\ntype = \"construction_heuristic\"\n{fields}\n"
    ))
}

fn assert_invalid(config: &SolverConfig, path: &str) {
    let error = validate_selector_values(config).expect_err(path);
    assert_eq!(error.path, path, "{error}");
    assert!(
        matches!(error.kind, RuntimeCompileErrorKind::InvalidValue { .. }),
        "{path}: {error}"
    );
}

fn assert_valid(config: &SolverConfig, label: &str) {
    if let Err(error) = validate_selector_values(config) {
        panic!("{label}: valid value rejected at {}: {error}", error.path);
    }
}

const SELECTOR: &str = "phases[1].local_search.move_selector";

/// `(selector body, invalid field override, valid field override, field)`.
/// The body holds every required field; overrides are appended last so
/// TOML takes them as the selector's only value for the field.
const LEAF_CASES: &[(&str, &str, &str, &str)] = &[
    ("type = \"change_move_selector\"", "value_candidate_limit = 0", "value_candidate_limit = 1", "value_candidate_limit"),
    ("type = \"nearby_change_move_selector\"", "max_nearby = 0", "max_nearby = 1", "max_nearby"),
    ("type = \"nearby_change_move_selector\"\nmax_nearby = 4", "value_candidate_limit = 0", "value_candidate_limit = 3", "value_candidate_limit"),
    ("type = \"nearby_swap_move_selector\"", "max_nearby = 0", "max_nearby = 1", "max_nearby"),
    ("type = \"pillar_change_move_selector\"\nmaximum_sub_pillar_size = 2", "minimum_sub_pillar_size = 5", "minimum_sub_pillar_size = 1", "minimum_sub_pillar_size"),
    ("type = \"pillar_change_move_selector\"\nmaximum_sub_pillar_size = 3", "minimum_sub_pillar_size = 0", "minimum_sub_pillar_size = 2", "minimum_sub_pillar_size"),
    ("type = \"pillar_change_move_selector\"\nminimum_sub_pillar_size = 2", "maximum_sub_pillar_size = 0", "maximum_sub_pillar_size = 2", "maximum_sub_pillar_size"),
    ("type = \"pillar_change_move_selector\"\nminimum_sub_pillar_size = 0\nmaximum_sub_pillar_size = 0", "value_candidate_limit = 0", "value_candidate_limit = 2", "value_candidate_limit"),
    ("type = \"pillar_swap_move_selector\"\nmaximum_sub_pillar_size = 2", "minimum_sub_pillar_size = 5", "minimum_sub_pillar_size = 2", "minimum_sub_pillar_size"),
    ("type = \"pillar_swap_move_selector\"\nminimum_sub_pillar_size = 1", "maximum_sub_pillar_size = 0", "maximum_sub_pillar_size = 4", "maximum_sub_pillar_size"),
    ("type = \"ruin_recreate_move_selector\"\nmax_ruin_count = 3\nrecreate_heuristic_type = \"first_fit\"", "min_ruin_count = 0", "min_ruin_count = 1", "min_ruin_count"),
    ("type = \"ruin_recreate_move_selector\"\nmax_ruin_count = 2\nrecreate_heuristic_type = \"first_fit\"", "min_ruin_count = 3", "min_ruin_count = 2", "min_ruin_count"),
    ("type = \"ruin_recreate_move_selector\"\nmin_ruin_count = 1\nmax_ruin_count = 2\nrecreate_heuristic_type = \"first_fit\"", "moves_per_step = 0", "moves_per_step = 1", "moves_per_step"),
    ("type = \"ruin_recreate_move_selector\"\nmin_ruin_count = 1\nmax_ruin_count = 2\nrecreate_heuristic_type = \"first_fit\"", "value_candidate_limit = 0", "value_candidate_limit = 1", "value_candidate_limit"),
    ("type = \"grouped_scalar_move_selector\"\ngroup_name = \"g\"", "value_candidate_limit = 0", "value_candidate_limit = 1", "value_candidate_limit"),
    ("type = \"grouped_scalar_move_selector\"\ngroup_name = \"g\"", "max_moves_per_step = 0", "max_moves_per_step = 1", "max_moves_per_step"),
    ("type = \"nearby_list_change_move_selector\"", "max_nearby = 0", "max_nearby = 1", "max_nearby"),
    ("type = \"nearby_list_swap_move_selector\"", "max_nearby = 0", "max_nearby = 1", "max_nearby"),
    ("type = \"list_permute_move_selector\"\nmax_window_size = 4", "min_window_size = 1", "min_window_size = 2", "min_window_size"),
    ("type = \"list_permute_move_selector\"\nmax_window_size = 3", "min_window_size = 4", "min_window_size = 3", "min_window_size"),
    ("type = \"list_permute_move_selector\"\nmin_window_size = 2", "max_window_size = 9", "max_window_size = 8", "max_window_size"),
    ("type = \"sublist_change_move_selector\"\nmax_sublist_size = 3", "min_sublist_size = 0", "min_sublist_size = 1", "min_sublist_size"),
    ("type = \"sublist_change_move_selector\"\nmax_sublist_size = 2", "min_sublist_size = 3", "min_sublist_size = 2", "min_sublist_size"),
    ("type = \"sublist_swap_move_selector\"\nmax_sublist_size = 3", "min_sublist_size = 0", "min_sublist_size = 1", "min_sublist_size"),
    ("type = \"sublist_swap_move_selector\"\nmax_sublist_size = 1", "min_sublist_size = 2", "min_sublist_size = 1", "min_sublist_size"),
    ("type = \"k_opt_move_selector\"\nmin_segment_len = 1\nmax_nearby = 0", "k = 1", "k = 2", "k"),
    ("type = \"k_opt_move_selector\"\nmin_segment_len = 1\nmax_nearby = 0", "k = 6", "k = 5", "k"),
    ("type = \"k_opt_move_selector\"\nk = 3\nmax_nearby = 4", "min_segment_len = 0", "min_segment_len = 1", "min_segment_len"),
    ("type = \"list_ruin_move_selector\"\nmax_ruin_count = 3", "min_ruin_count = 0", "min_ruin_count = 1", "min_ruin_count"),
    ("type = \"list_ruin_move_selector\"\nmax_ruin_count = 1", "min_ruin_count = 2", "min_ruin_count = 1", "min_ruin_count"),
    ("type = \"list_ruin_move_selector\"\nmin_ruin_count = 1\nmax_ruin_count = 2", "moves_per_step = 0", "moves_per_step = 1", "moves_per_step"),
    ("type = \"list_ruin_move_selector\"\nmin_ruin_count = 1\nmax_ruin_count = 2", "max_source_list_len = 0", "max_source_list_len = 1", "max_source_list_len"),
    ("type = \"conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_matches_per_step = 0", "max_matches_per_step = 1", "max_matches_per_step"),
    ("type = \"conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_repairs_per_match = 0", "max_repairs_per_match = 1", "max_repairs_per_match"),
    ("type = \"conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_moves_per_step = 0", "max_moves_per_step = 1", "max_moves_per_step"),
    ("type = \"compound_conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_matches_per_step = 0", "max_matches_per_step = 1", "max_matches_per_step"),
    ("type = \"compound_conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_repairs_per_match = 0", "max_repairs_per_match = 1", "max_repairs_per_match"),
    ("type = \"compound_conflict_repair_move_selector\"\nconstraints = [\"c\"]", "max_moves_per_step = 0", "max_moves_per_step = 1", "max_moves_per_step"),
];

/// Fields a leaf body must also carry when an override does not set them.
fn completed(body: &str, field_override: &str) -> String {
    let mut lines = vec![body.to_string()];
    let defaults: &[(&str, &[&str])] = &[
        ("nearby_change_move_selector", &["max_nearby = 4"]),
        ("nearby_swap_move_selector", &["max_nearby = 4"]),
        ("nearby_list_change_move_selector", &["max_nearby = 4"]),
        ("nearby_list_swap_move_selector", &["max_nearby = 4"]),
        (
            "pillar_change_move_selector",
            &["minimum_sub_pillar_size = 0", "maximum_sub_pillar_size = 0"],
        ),
        (
            "pillar_swap_move_selector",
            &["minimum_sub_pillar_size = 0", "maximum_sub_pillar_size = 0"],
        ),
        (
            "ruin_recreate_move_selector",
            &["min_ruin_count = 1", "max_ruin_count = 2"],
        ),
        (
            "list_permute_move_selector",
            &["min_window_size = 2", "max_window_size = 3"],
        ),
        (
            "sublist_change_move_selector",
            &["min_sublist_size = 1", "max_sublist_size = 3"],
        ),
        (
            "sublist_swap_move_selector",
            &["min_sublist_size = 1", "max_sublist_size = 3"],
        ),
        (
            "k_opt_move_selector",
            &["k = 3", "min_segment_len = 1", "max_nearby = 0"],
        ),
        (
            "list_ruin_move_selector",
            &["min_ruin_count = 1", "max_ruin_count = 2"],
        ),
    ];
    let selector_type = body.lines().next().unwrap_or_default();
    for (name, fields) in defaults {
        if selector_type != format!("type = \"{name}\"") {
            continue;
        }
        for field in *fields {
            let key = field.split(" =").next().unwrap_or_default();
            let set = |text: &str| {
                text.lines()
                    .any(|line| line.starts_with(&format!("{key} =")))
            };
            if !set(body) && !set(field_override) {
                lines.push((*field).to_string());
            }
        }
    }
    lines.push(field_override.to_string());
    lines.join("\n")
}

#[test]
fn every_numeric_selector_field_rejects_its_invalid_value_with_the_key_path() {
    for (body, invalid, valid, field) in LEAF_CASES {
        assert_invalid(
            &local_search(&completed(body, invalid)),
            &format!("{SELECTOR}.{field}"),
        );
        assert_valid(&local_search(&completed(body, valid)), valid);
    }
}

#[test]
fn documented_zero_meanings_stay_valid() {
    for body in [
        "type = \"pillar_change_move_selector\"\nminimum_sub_pillar_size = 0\nmaximum_sub_pillar_size = 0",
        "type = \"pillar_swap_move_selector\"\nminimum_sub_pillar_size = 0\nmaximum_sub_pillar_size = 0",
        "type = \"k_opt_move_selector\"\nk = 3\nmin_segment_len = 1\nmax_nearby = 0",
    ] {
        assert_valid(&local_search(body), body);
    }
}

#[test]
fn limited_neighborhood_and_nested_selectors_report_their_child_paths() {
    assert_invalid(
        &local_search("type = \"limited_neighborhood\"\nselected_count_limit = 0\n\n[phases.move_selector.selector]\ntype = \"list_change_move_selector\""),
        &format!("{SELECTOR}.selected_count_limit"),
    );
    assert_valid(
        &local_search("type = \"limited_neighborhood\"\nselected_count_limit = 1\n\n[phases.move_selector.selector]\ntype = \"list_change_move_selector\""),
        "selected_count_limit = 1",
    );
    assert_invalid(
        &local_search("type = \"limited_neighborhood\"\nselected_count_limit = 4\n\n[phases.move_selector.selector]\ntype = \"change_move_selector\"\nvalue_candidate_limit = 0"),
        &format!("{SELECTOR}.selector.value_candidate_limit"),
    );
    assert_invalid(
        &local_search("type = \"union_move_selector\"\n\n[[phases.move_selector.selectors]]\ntype = \"list_change_move_selector\"\n\n[[phases.move_selector.selectors]]\ntype = \"sublist_change_move_selector\"\nmin_sublist_size = 4\nmax_sublist_size = 2"),
        &format!("{SELECTOR}.selectors[1].min_sublist_size"),
    );
    assert_invalid(
        &local_search("type = \"cartesian_product_move_selector\"\n\n[[phases.move_selector.selectors]]\ntype = \"nearby_list_change_move_selector\"\nmax_nearby = 0\n\n[[phases.move_selector.selectors]]\ntype = \"list_swap_move_selector\""),
        &format!("{SELECTOR}.selectors[0].max_nearby"),
    );
    assert_invalid(
        &parse("[[phases]]\ntype = \"local_search\"\nlocal_search_type = \"variable_neighborhood_descent\"\n\n[[phases.neighborhoods]]\ntype = \"list_change_move_selector\"\n\n[[phases.neighborhoods]]\ntype = \"list_ruin_move_selector\"\nmin_ruin_count = 1\nmax_ruin_count = 2\nmoves_per_step = 0"),
        "phases[0].local_search.neighborhoods[1].moves_per_step",
    );
}

#[test]
fn construction_values_reject_their_invalid_value_with_the_key_path() {
    let cases = [
        (
            "value_candidate_limit = 0",
            "value_candidate_limit = 1",
            "value_candidate_limit",
        ),
        (
            "group_candidate_limit = 0",
            "group_candidate_limit = 1",
            "group_candidate_limit",
        ),
        ("k = 3", "k = 2", "k"),
    ];
    for (invalid, valid, field) in cases {
        assert_invalid(
            &construction(invalid),
            &format!("phases[0].construction.{field}"),
        );
        assert_valid(&construction(valid), valid);
    }
    assert_valid(&construction(""), "defaults");
}

#[test]
fn partitioned_child_phase_values_are_validated() {
    let config = parse(
        "[[phases]]\ntype = \"partitioned_search\"\npartitioner = \"p\"\n\n\
         [[phases.child_phases]]\ntype = \"construction_heuristic\"\n\n\
         [[phases.child_phases]]\ntype = \"local_search\"\n\n\
         [phases.child_phases.move_selector]\ntype = \"pillar_swap_move_selector\"\n\
         minimum_sub_pillar_size = 5\nmaximum_sub_pillar_size = 2\n",
    );
    assert_invalid(
        &config,
        "phases[0].partitioned_search.child_phases[1].local_search.move_selector.minimum_sub_pillar_size",
    );
    assert_invalid(
        &parse("[[phases]]\ntype = \"partitioned_search\"\npartitioner = \"p\"\n\n[[phases.child_phases]]\ntype = \"construction_heuristic\"\nvalue_candidate_limit = 0\n"),
        "phases[0].partitioned_search.child_phases[0].construction.value_candidate_limit",
    );
}

#[test]
fn runtime_graph_compilation_rejects_selector_values_before_any_phase() {
    let config =
        local_search("type = \"list_ruin_move_selector\"\nmin_ruin_count = 3\nmax_ruin_count = 2");
    let context = SearchContext::new(descriptor(), model(PrecedenceShape::Absent), None)
        .expect("search context should bind");
    let error = match compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    ) {
        Ok(_) => panic!("min_ruin_count > max_ruin_count must fail compilation"),
        Err(error) => error,
    };
    assert_eq!(error.path, format!("{SELECTOR}.min_ruin_count"));
    assert!(
        matches!(error.kind, RuntimeCompileErrorKind::InvalidValue { .. }),
        "{error}"
    );
}
