use crate::planning_solution::expand_derive;
use syn::parse_quote;

#[test]
fn golden_solution_expansion_emits_model_sources_and_descriptor() {
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
    assert!(expanded
        .contains("pub fn workers () -> impl :: solverforge :: stream :: CollectionExtract < Self , Item = Worker >"));
    assert!(expanded
        .contains("pub fn tasks () -> impl :: solverforge :: stream :: CollectionExtract < Self , Item = Task >"));
    assert!(expanded.contains("ChangeSource :: Static"));
    assert!(expanded.contains("ChangeSource :: Descriptor (0)"));
    assert!(expanded
        .contains("pub fn descriptor () -> :: solverforge :: __internal :: SolutionDescriptor"));
    assert!(expanded.contains("pub fn __solverforge_entity_tasks"));
    assert!(expanded.contains("pub fn __solverforge_collection_tasks"));
    assert!(expanded.contains("create_constraints"));
    assert!(expanded.contains("pub trait PlanConstraintStreams"));
    assert!(expanded.contains(
        "impl < Sc : :: solverforge :: Score + 'static > PlanConstraintStreams < Sc > for :: solverforge :: stream :: ConstraintFactory < Plan , Sc >"
    ));
    assert!(expanded
        .contains("fn workers (self) -> :: solverforge :: __internal :: UniConstraintStream"));
    assert!(expanded.contains(":: solverforge :: stream :: ConstraintFactory :: for_each"));
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
fn golden_solution_expansion_binds_typed_custom_search_path() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        #[solverforge_search_path = "crate::search::search"]
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

    assert!(expanded.contains("crate :: search :: search"));
    assert!(expanded.contains("SearchContext :: new"));
    assert!(expanded.contains("build_search :: < Plan"));
    assert!(!expanded.contains("Box < dyn"));
    assert!(!expanded.contains("dyn Phase"));
    assert!(!expanded.contains("custom_phase_class"));
}

#[test]
fn golden_solution_expansion_binds_conflict_repairs_path() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        #[solverforge_conflict_repairs_path = "crate::repairs::define_repairs"]
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

    assert!(expanded.contains("crate :: repairs :: define_repairs"));
    assert!(expanded.contains("with_conflict_repairs"));
}

#[test]
fn golden_solution_expansion_binds_private_list_runtime_helpers() {
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

#[test]
fn golden_solution_expansion_sorts_runtime_variables_by_descriptor_order() {
    let input = parse_quote! {
        #[solverforge_constraints_path = "crate::constraints::create_constraints"]
        struct Plan {
            #[problem_fact_collection]
            workers: Vec<Worker>,
            #[planning_entity_collection]
            routes: Vec<Route>,
            #[problem_fact_collection]
            visits: Vec<Visit>,
            #[planning_score]
            score: Option<HardSoftScore>,
        }
    };

    let expanded = expand_derive(input)
        .expect("solution expansion should succeed")
        .to_string();

    assert!(expanded.contains("__solverforge_variables . sort_by_key"));
    assert!(expanded.contains("__solverforge_descriptor_variable_order"));
    assert!(expanded.contains("descriptor . entity_descriptors"));
    assert!(expanded.contains("variable_descriptors"));
    assert!(expanded.contains("__solverforge_scalar_variable_count"));
    assert!(expanded.contains("variable . name == __solverforge_variable_name"));
    assert!(expanded.contains("variable . usize_getter . is_some ()"));
    assert!(expanded.contains("PlanningModelSupport"));
    assert!(expanded.contains("ctx . variable_name"));
    assert!(expanded.contains(":: solverforge :: __internal :: build_phases"));
}

#[test]
fn solution_scalar_runtime_does_not_require_registered_hook_metadata() {
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
        .expect("solution expansion should succeed without prior entity macro expansion")
        .to_string();

    assert!(expanded.contains("__solverforge_scalar_variable_count"));
    assert!(expanded.contains("__solverforge_scalar_get_by_index"));
    assert!(expanded.contains("__solverforge_scalar_set_by_index"));
}

#[test]
fn solution_descriptor_delegates_scalar_hook_attachment_to_model_manifest() {
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

    assert!(expanded.contains("PlanningModelSupport"));
    assert!(expanded.contains("attach_descriptor_hooks"));
    assert!(expanded.contains("validate_model"));
}
