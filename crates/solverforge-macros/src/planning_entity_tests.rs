use crate::planning_entity::expand_derive;
use syn::parse_quote;

#[test]
fn golden_entity_expansion_includes_descriptor_and_planning_id() {
    let input = parse_quote! {
        struct Task {
            #[planning_id]
            id: String,
            #[planning_variable(allows_unassigned = true, value_range_provider = "workers")]
            worker_idx: Option<usize>,
            #[planning_list_variable(element_collection = "all_tasks")]
            chain: Vec<usize>,
        }
    };

    let expanded = expand_derive(input)
        .expect("entity expansion should succeed")
        .to_string();

    assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningEntity for Task"));
    assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningId for Task"));
    assert!(expanded.contains("with_allows_unassigned (true)"));
    assert!(expanded.contains("with_value_range (\"workers\")"));
    assert!(expanded.contains("with_id_field (stringify ! (id))"));
    assert!(expanded.contains("pub fn entity_descriptor"));
    assert!(expanded.contains("pub const __SOLVERFORGE_LIST_VARIABLE_COUNT : usize = 1"));
    assert!(expanded
        .contains("const __SOLVERFORGE_LIST_ELEMENT_COLLECTION : & 'static str = \"all_tasks\""));
    assert!(expanded.contains("pub (crate) fn __solverforge_get_worker_idx_scalar"));
    assert!(expanded.contains("pub (crate) fn __solverforge_set_worker_idx_scalar"));
    assert!(expanded.contains("const HAS_LIST_VARIABLE : bool = true"));
    assert!(expanded.contains("LIST_ELEMENT_SOURCE"));
    assert!(expanded.contains("fn __solverforge_list_metadata < Solution >"));
    assert!(expanded.contains("UnassignedEntity < __SolverForgeSolution > for Task"));
    assert!(expanded.contains("fn is_unassigned"));
}

#[test]
fn entity_descriptor_preserves_mixed_variable_declaration_order() {
    let input = parse_quote! {
        struct Route {
            #[planning_id]
            id: String,
            #[planning_list_variable(element_collection = "all_tasks")]
            chain: Vec<usize>,
            #[planning_variable(allows_unassigned = true, value_range_provider = "workers")]
            worker_idx: Option<usize>,
            #[planning_variable(allows_unassigned = true, value_range_provider = "workers")]
            backup_idx: Option<usize>,
        }
    };

    let expanded = expand_derive(input)
        .expect("entity expansion should succeed")
        .to_string();

    let chain_pos = expanded
        .find("VariableDescriptor :: list (\"chain\")")
        .expect("list variable descriptor should exist");
    let worker_pos = expanded
        .find("VariableDescriptor :: genuine (\"worker_idx\")")
        .expect("worker variable descriptor should exist");
    let backup_pos = expanded
        .find("VariableDescriptor :: genuine (\"backup_idx\")")
        .expect("backup variable descriptor should exist");

    assert!(chain_pos < worker_pos);
    assert!(worker_pos < backup_pos);

    assert!(expanded.contains("fn __solverforge_scalar_variable_count"));
    assert!(expanded.contains("fn __solverforge_scalar_get_by_index"));
    assert!(expanded.contains("fn __solverforge_scalar_set_by_index"));

    let name_helper_pos = expanded
        .find("fn __solverforge_scalar_variable_name_by_index")
        .expect("indexed scalar name helper should exist");
    let name_helper_end = expanded[name_helper_pos..]
        .find("fn __solverforge_scalar_allows_unassigned_by_index")
        .map(|offset| name_helper_pos + offset)
        .expect("next scalar helper should exist");
    let name_helper = &expanded[name_helper_pos..name_helper_end];
    let worker_name_pos = name_helper
        .find("\"worker_idx\"")
        .expect("worker scalar index arm should exist");
    let backup_name_pos = name_helper
        .find("\"backup_idx\"")
        .expect("backup scalar index arm should exist");
    assert!(worker_name_pos < backup_name_pos);
}

#[test]
fn chained_option_usize_does_not_enter_scalar_helper_indexing() {
    let input = parse_quote! {
        struct Task {
            #[planning_id]
            id: String,
            #[planning_variable(chained = true, value_range_provider = "tasks")]
            previous: Option<usize>,
            #[planning_variable(allows_unassigned = true, value_range_provider = "workers")]
            worker: Option<usize>,
        }
    };

    let expanded = expand_derive(input)
        .expect("entity expansion should succeed")
        .to_string();

    assert!(expanded.contains("VariableDescriptor :: chained (\"previous\")"));
    assert!(expanded.contains("VariableDescriptor :: genuine (\"worker\")"));
    assert!(!expanded.contains("__solverforge_get_previous_scalar"));
    assert!(!expanded.contains("__solverforge_set_previous_scalar"));
    assert!(expanded.contains("__solverforge_get_worker_scalar"));
    assert!(expanded.contains("__solverforge_set_worker_scalar"));
    assert!(expanded.contains(
        "pub (crate) const fn __solverforge_scalar_variable_count () -> usize { 1usize }"
    ));

    let name_helper_pos = expanded
        .find("fn __solverforge_scalar_variable_name_by_index")
        .expect("indexed scalar name helper should exist");
    let name_helper_end = expanded[name_helper_pos..]
        .find("fn __solverforge_scalar_allows_unassigned_by_index")
        .map(|offset| name_helper_pos + offset)
        .expect("next scalar helper should exist");
    let name_helper = &expanded[name_helper_pos..name_helper_end];
    assert!(name_helper.contains("0usize => :: core :: option :: Option :: Some (\"worker\")"));
    assert!(!name_helper.contains("\"previous\""));
}
