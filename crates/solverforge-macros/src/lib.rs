//! Procedural macros for SolverForge domain models.
//!
//! Use them through the `solverforge` facade, which re-exports
//! `planning_model!`, `#[planning_solution]`, `#[planning_entity]`,
//! `#[problem_fact]` and `#[solverforge_constraints]`; the generated code
//! refers to `::solverforge` paths.

#![warn(missing_docs)]

use proc_macro::TokenStream;

mod attr_parse;
mod attr_validation;
mod constraints;
mod entrypoints;
mod planning_entity;
mod planning_model;
mod planning_solution;
mod problem_fact;

/// Derives planning-entity metadata for a struct: its `#[planning_id]`,
/// scalar and list planning variables, and shadow variables.
#[proc_macro_attribute]
pub fn planning_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::planning_entity_attr(attr, item)
}

/// Derives the planning solution: entity and fact collections, the score
/// field, the runtime model and the generated `Solvable` solve entry point.
#[proc_macro_attribute]
pub fn planning_solution(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::planning_solution_attr(attr, item)
}

/// Derives problem-fact metadata for an immutable input struct.
#[proc_macro_attribute]
pub fn problem_fact(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::problem_fact_attr(attr, item)
}

/// Compiles a constraint-provider function's constraint streams into one
/// shared, incremental constraint set.
#[proc_macro_attribute]
pub fn solverforge_constraints(attr: TokenStream, item: TokenStream) -> TokenStream {
    entrypoints::solverforge_constraints_attr(attr, item)
}

/// Declares a domain's model manifest: the solution, entity and fact modules
/// that make up one planning model.
#[proc_macro]
pub fn planning_model(input: TokenStream) -> TokenStream {
    entrypoints::planning_model_macro(input)
}

#[doc(hidden)]
#[proc_macro_derive(
    PlanningEntityImpl,
    attributes(
        planning_id,
        planning_variable,
        planning_list_variable,
        planning_pin,
        inverse_relation_shadow_variable,
        index_shadow_variable,
        previous_element_shadow_variable,
        next_element_shadow_variable,
        cascading_update_shadow_variable
    )
)]
pub fn derive_planning_entity(input: TokenStream) -> TokenStream {
    entrypoints::derive_planning_entity(input)
}

#[doc(hidden)]
#[proc_macro_derive(
    PlanningSolutionImpl,
    attributes(
        planning_entity_collection,
        planning_list_element_collection,
        problem_fact_collection,
        planning_score,
        constraint_weights,
        value_range_provider,
        shadow_variable_updates,
        solverforge_constraints_path,
        solverforge_config_path,
        solverforge_solver_toml_path,
        solverforge_search_path,
        solverforge_conflict_repairs_path,
        solverforge_scalar_groups_path
    )
)]
pub fn derive_planning_solution(input: TokenStream) -> TokenStream {
    entrypoints::derive_planning_solution(input)
}

#[doc(hidden)]
#[proc_macro_derive(ProblemFactImpl, attributes(planning_id))]
pub fn derive_problem_fact(input: TokenStream) -> TokenStream {
    entrypoints::derive_problem_fact(input)
}
