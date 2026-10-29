// Strict parsing: unknown keys are errors, and everything the serializer
// emits parses again under the strict model.

use super::*;

fn toml_error(toml: &str) -> String {
    match SolverConfig::from_toml_str(toml) {
        Err(ConfigError::Toml(error)) => error.to_string(),
        Err(other) => panic!("expected a TOML error, got {other:?}"),
        Ok(config) => panic!("expected an error, parsed {}", config.canonical_toml()),
    }
}

#[test]
fn misspelled_keys_are_rejected_with_the_key_named() {
    let cases = [
        ("[termination]\nsecnds_spent_limit = 5\n", "secnds_spent_limit"),
        ("[[phase]]\ntype = \"local_search\"\n", "phase"),
        (
            "[[phases]]\ntype = \"construction_heuristic\"\nconstruction_heuristic_typ = \"cheapest_insertion\"\n",
            "construction_heuristic_typ",
        ),
        (
            "[[phases]]\ntype = \"local_search\"\n[phases.acceptor]\ntype = \"tabu_search\"\nentity_tabu_sise = 7\n",
            "entity_tabu_sise",
        ),
        (
            "[[phases]]\ntype = \"local_search\"\n[phases.move_selector]\ntype = \"change_move_selector\"\nvariable_nme = \"row_idx\"\n",
            "variable_nme",
        ),
        ("[score_director]\nconstraint_match_enable = true\n", "constraint_match_enable"),
        ("[candidate_trace]\nmax_entries = 4\nmax_entrys = 5\n", "max_entrys"),
        (
            "[[phases]]\ntype = \"partitioned_search\"\n[[phases.child_phases]]\ntype = \"custom\"\nname = \"a\"\nnam = \"b\"\n",
            "nam",
        ),
    ];
    for (toml, key) in cases {
        let message = toml_error(toml);
        assert!(
            message.contains(&format!("unknown field `{key}`")),
            "{key}: {message}"
        );
    }
}

#[test]
fn payload_free_variants_reject_extra_keys() {
    let acceptor =
        "[[phases]]\ntype = \"local_search\"\n[phases.acceptor]\ntype = \"hill_climbing\"\n";
    let config = SolverConfig::from_toml_str(acceptor).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    assert!(matches!(
        local_search.acceptor,
        Some(AcceptorConfig::HillClimbing {})
    ));
    let message = toml_error(&format!("{acceptor}late_acceptance_size = 3\n"));
    assert!(
        message.contains("unknown field `late_acceptance_size`"),
        "{message}"
    );

    for forager in [
        "best_score",
        "first_accepted",
        "first_best_score_improving",
        "first_last_step_score_improving",
    ] {
        let toml = format!(
            "[[phases]]\ntype = \"local_search\"\n[phases.forager]\ntype = \"{forager}\"\n"
        );
        let config = SolverConfig::from_toml_str(&toml).unwrap();
        let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
            panic!("phase should be local_search");
        };
        assert!(local_search.forager.is_some(), "{forager}");
        let message = toml_error(&format!("{toml}limit = 3\n"));
        assert!(
            message.contains("unknown field `limit`"),
            "{forager}: {message}"
        );
    }
}

#[test]
fn flattened_variable_targets_still_parse() {
    let toml = r#"
        [[phases]]
        type = "construction_heuristic"
        entity_class = "Shift"
        variable_name = "employee"

        [[phases]]
        type = "local_search"
        [phases.move_selector]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee"
    "#;
    let config = SolverConfig::from_toml_str(toml).unwrap();
    let expected = VariableTargetConfig {
        entity_class: Some("Shift".to_string()),
        variable_name: Some("employee".to_string()),
    };
    let PhaseConfig::ConstructionHeuristic(construction) = &config.phases[0] else {
        panic!("phase should be construction_heuristic");
    };
    assert_eq!(construction.target, expected);
    let PhaseConfig::LocalSearch(local_search) = &config.phases[1] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::ChangeMoveSelector(change)) = &local_search.move_selector else {
        panic!("selector should be change_move_selector");
    };
    assert_eq!(change.target, expected);
}

const TARGET: &str = "entity_class = \"Plan\"\nvariable_name = \"slot\"\n";

/// Every leaf selector, with its flattened target where it has one.
fn leaf_selectors() -> Vec<String> {
    let targeted = [
        "change_move_selector",
        "swap_move_selector",
        "nearby_change_move_selector",
        "nearby_swap_move_selector",
        "pillar_change_move_selector",
        "pillar_swap_move_selector",
        "ruin_recreate_move_selector",
        "list_change_move_selector",
        "nearby_list_change_move_selector",
        "list_swap_move_selector",
        "list_permute_move_selector",
        "list_precedence_move_selector",
        "nearby_list_swap_move_selector",
        "sublist_change_move_selector",
        "sublist_swap_move_selector",
        "list_reverse_move_selector",
        "k_opt_move_selector",
        "list_ruin_move_selector",
    ];
    let mut selectors: Vec<String> = targeted
        .iter()
        .map(|kind| {
            let extra = match *kind {
                kind if kind.starts_with("nearby_") => "max_nearby = 4\n",
                "pillar_change_move_selector" | "pillar_swap_move_selector" => {
                    "minimum_sub_pillar_size = 1\nmaximum_sub_pillar_size = 3\n"
                }
                "ruin_recreate_move_selector" => {
                    "min_ruin_count = 1\nmax_ruin_count = 2\nrecreate_heuristic_type = \"first_fit\"\n"
                }
                "list_ruin_move_selector" => "min_ruin_count = 1\nmax_ruin_count = 2\n",
                "list_permute_move_selector" => "min_window_size = 2\nmax_window_size = 3\n",
                "sublist_change_move_selector" | "sublist_swap_move_selector" => {
                    "min_sublist_size = 1\nmax_sublist_size = 3\n"
                }
                "k_opt_move_selector" => "k = 2\nmin_segment_len = 1\nmax_nearby = 4\n",
                _ => "",
            };
            format!("type = \"{kind}\"\n{extra}{TARGET}")
        })
        .collect();
    selectors.push("type = \"grouped_scalar_move_selector\"\ngroup_name = \"pairs\"\n".into());
    selectors.push("type = \"conflict_repair_move_selector\"\nconstraints = [\"c\"]\n".into());
    selectors
        .push("type = \"compound_conflict_repair_move_selector\"\nconstraints = [\"c\"]\n".into());
    selectors
}

fn local_search(acceptor: &str, forager: &str, selector: &str) -> String {
    format!(
        "[[phases]]\ntype = \"local_search\"\n[phases.acceptor]\n{acceptor}[phases.forager]\n{forager}[phases.move_selector]\n{selector}"
    )
}

#[test]
fn canonical_toml_of_every_variant_reparses_under_the_strict_model() {
    let mut toml = String::from(
        "environment_mode = \"reproducible\"\nrandom_seed = 7\nmove_thread_count = { count = 2 }\n\
         [termination]\nseconds_spent_limit = 1\nminutes_spent_limit = 1\nbest_score_limit = \"0hard/0soft\"\n\
         step_count_limit = 5\nunimproved_step_count_limit = 3\nunimproved_seconds_spent_limit = 1\n\
         [score_director]\nconstraint_provider = \"p\"\nconstraint_match_enabled = true\n\
         [candidate_trace]\nmax_entries = 8\n\
         [[phases]]\ntype = \"construction_heuristic\"\nconstruction_heuristic_type = \"cheapest_insertion\"\n",
    );
    toml.push_str(TARGET);

    let acceptors = [
        "type = \"hill_climbing\"\n",
        "type = \"step_counting_hill_climbing\"\nstep_count_limit = 4\n",
        "type = \"tabu_search\"\nentity_tabu_size = 2\nvalue_tabu_size = 2\nmove_tabu_size = 2\nundo_move_tabu_size = 2\naspiration_enabled = true\n",
        "type = \"simulated_annealing\"\nlevel_temperatures = [1.0, 2.0]\ndecay_rate = 0.5\nhill_climbing_temperature = 0.1\n",
        "type = \"simulated_annealing\"\n[phases.acceptor.calibration]\nsample_size = 4\ntarget_acceptance_probability = 0.5\nfallback_temperature = 1.0\n",
        "type = \"late_acceptance\"\nlate_acceptance_size = 4\n",
        "type = \"diversified_late_acceptance\"\nlate_acceptance_size = 4\n",
        "type = \"great_deluge\"\nwater_level_increase_ratio = 0.1\n",
    ];
    let foragers = [
        "type = \"accepted_count\"\nlimit = 4\n",
        "type = \"best_score\"\n",
        "type = \"first_accepted\"\n",
        "type = \"first_best_score_improving\"\n",
        "type = \"first_last_step_score_improving\"\n",
    ];
    let leaves = leaf_selectors();
    for (index, leaf) in leaves.iter().enumerate() {
        toml.push_str(&local_search(
            acceptors[index % acceptors.len()],
            foragers[index % foragers.len()],
            leaf,
        ));
    }
    for (acceptor, forager) in acceptors.iter().zip(foragers.iter().cycle()) {
        toml.push_str(&local_search(acceptor, forager, &leaves[0]));
    }

    let config = SolverConfig::from_toml_str(&toml).unwrap();
    let mut phases = config.phases.clone();
    let mut composite = LocalSearchConfig::default();
    let leaf_selectors: Vec<MoveSelectorConfig> = config
        .phases
        .iter()
        .filter_map(|phase| match phase {
            PhaseConfig::LocalSearch(local_search) => local_search.move_selector.clone(),
            _ => None,
        })
        .take(leaves.len())
        .collect();
    assert_eq!(leaf_selectors.len(), leaves.len());
    composite.move_selector = Some(MoveSelectorConfig::UnionMoveSelector(
        UnionMoveSelectorConfig {
            selectors: vec![
                MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
                    require_hard_improvement: true,
                    selectors: leaf_selectors.clone(),
                }),
                MoveSelectorConfig::LimitedNeighborhood(LimitedNeighborhoodConfig {
                    selected_count_limit: 3,
                    selector: Box::new(leaf_selectors[0].clone()),
                }),
            ],
            weights: vec![1, 2],
            ..UnionMoveSelectorConfig::default()
        },
    ));
    phases.push(PhaseConfig::LocalSearch(composite));
    phases.push(PhaseConfig::PartitionedSearch(PartitionedSearchConfig {
        partitioner: Some("rows".to_string()),
        log_progress: true,
        child_phases: config.phases.clone(),
        termination: config.termination.clone(),
        ..PartitionedSearchConfig::default()
    }));
    phases.push(PhaseConfig::Custom(CustomPhaseConfig {
        name: "custom".to_string(),
    }));
    let full = SolverConfig { phases, ..config };

    let canonical = full.canonical_toml();
    let reparsed = SolverConfig::from_toml_str(&canonical)
        .unwrap_or_else(|error| panic!("canonical TOML must reparse: {error}\n{canonical}"));
    assert_eq!(reparsed.canonical_toml(), canonical);
}
