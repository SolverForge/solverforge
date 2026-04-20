use crate::planning_solution::expand_derive;
use syn::parse_quote;

#[test]
fn golden_solution_expansion_emits_constraint_streams_and_descriptor() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        struct Plan {
            #[problem_fact_collection]
            workers: Vec<Worker>,
            #[planning_entity_collection]
            tasks: Vec<Task>,
            #[planning_score]
            score: Option<HardSoftScore>,
        }
    };

    let expanded = expand_derive(input)
        .expect("solution expansion should succeed")
        .to_string();

    assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningSolution for Plan"));
    assert!(expanded.contains("pub trait PlanConstraintStreams"));
    assert!(expanded
        .contains("pub fn descriptor () -> :: solverforge :: __internal :: SolutionDescriptor"));
    assert!(expanded.contains("create_constraints"));
}

#[test]
fn golden_solution_expansion_loads_solver_config_before_config_callback() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        #[solverforge_config_path = "crate::config::for_solution"]
        struct Plan {
            #[planning_entity_collection]
            tasks: Vec<Task>,
            #[planning_score]
            score: Option<HardSoftScore>,
        }
    };

    let expanded = expand_derive(input)
        .expect("solution expansion should succeed")
        .to_string();

    assert!(expanded
        .contains("let base_config = :: solverforge :: __internal :: load_solver_config ()"));
    assert!(
        expanded.contains("let config = crate :: config :: for_solution (& self , base_config)")
    );
    assert!(expanded.contains("run_solver_with_config"));
}

#[test]
fn golden_solution_expansion_embeds_explicit_solver_toml_source() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        #[solverforge_solver_toml_path = "fixtures/solver.toml"]
        struct Plan {
            #[planning_entity_collection]
            tasks: Vec<Task>,
            #[planning_score]
            score: Option<HardSoftScore>,
        }
    };

    let expanded = expand_derive(input)
        .expect("solution expansion should succeed")
        .to_string();

    assert!(expanded.contains("include_str ! (\"fixtures/solver.toml\")"));
    assert!(expanded.contains("OnceLock < :: solverforge :: SolverConfig >"));
    assert!(expanded.contains("run_solver_with_config"));
    assert!(!expanded.contains("load_solver_config ()"));
}

#[test]
fn golden_solution_expansion_binds_owner_specific_list_helpers() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        struct Plan {
            #[planning_entity_collection]
            routes: Vec<Route>,
            #[planning_entity_collection]
            shifts: Vec<Shift>,
            #[problem_fact_collection]
            route_tasks: Vec<RouteTask>,
            #[problem_fact_collection]
            shift_tasks: Vec<ShiftTask>,
            #[planning_score]
            score: Option<HardSoftScore>,
        }
    };

    let expanded = expand_derive(input)
        .expect("solution expansion should succeed")
        .to_string();

    assert!(expanded.contains("fn __solverforge_list_insert_routes"));
    assert!(expanded.contains("fn __solverforge_list_insert_shifts"));
    assert!(expanded.contains("fn __solverforge_index_to_element_routes"));
    assert!(expanded.contains("fn __solverforge_index_to_element_shifts"));
    assert!(expanded.contains("Self :: __solverforge_list_insert_routes"));
    assert!(expanded.contains("Self :: __solverforge_list_insert_shifts"));
    assert!(expanded.contains("Self :: __solverforge_index_to_element_routes"));
    assert!(expanded.contains("Self :: __solverforge_index_to_element_shifts"));
    assert!(expanded.contains("Self :: __solverforge_total_list_entities"));
    assert!(expanded.contains("Self :: __solverforge_total_list_elements"));
}
