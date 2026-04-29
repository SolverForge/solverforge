// Macros for SolverForge domain models.

use proc_macro::TokenStream;

mod attr_parse;
mod entrypoints;
mod planning_entity;
mod planning_model;
mod planning_solution;
mod problem_fact;

#[proc_macro_attribute]
pub fn planning_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::planning_entity_attr(attr, item)
}

#[proc_macro_attribute]
pub fn planning_solution(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::planning_solution_attr(attr, item)
}

#[proc_macro_attribute]
pub fn problem_fact(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::problem_fact_attr(attr, item)
}

#[proc_macro]
pub fn planning_model(input: TokenStream) -> TokenStream {
    entrypoints::planning_model_macro(input)
}

#[proc_macro_derive(
    PlanningEntityImpl,
    attributes(
        planning_id,
        planning_variable,
        planning_list_variable,
        planning_pin,
        inverse_relation_shadow_variable,
        previous_element_shadow_variable,
        next_element_shadow_variable,
        cascading_update_shadow_variable
    )
)]
pub fn derive_planning_entity(input: TokenStream) -> TokenStream {
    entrypoints::derive_planning_entity(input)
}

#[proc_macro_derive(
    PlanningSolutionImpl,
    attributes(
        planning_entity_collection,
        planning_list_element_collection,
        problem_fact_collection,
        planning_score,
        value_range_provider,
        shadow_variable_updates,
        solverforge_constraints_path,
        solverforge_config_path,
        solverforge_solver_toml_path,
        solverforge_conflict_repair_providers_path,
        solverforge_scalar_groups_path
    )
)]
pub fn derive_planning_solution(input: TokenStream) -> TokenStream {
    entrypoints::derive_planning_solution(input)
}

#[proc_macro_derive(ProblemFactImpl, attributes(planning_id))]
pub fn derive_problem_fact(input: TokenStream) -> TokenStream {
    entrypoints::derive_problem_fact(input)
}
