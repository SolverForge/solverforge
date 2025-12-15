//! Derive macros for SolverForge domain types.
//!
//! This crate provides derive macros for implementing `PlanningEntity` and
//! `PlanningSolution` traits from `solverforge-core`.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_derive::{PlanningEntity, PlanningSolution};
//!
//! #[derive(PlanningEntity, Clone)]
//! pub struct Lesson {
//!     #[planning_id]
//!     pub id: String,
//!     pub subject: String,
//!     #[planning_variable(value_range_provider = "rooms")]
//!     pub room: Option<Room>,
//! }
//!
//! #[derive(PlanningSolution, Clone)]
//! #[constraint_provider = "define_constraints"]
//! pub struct Timetable {
//!     #[problem_fact_collection]
//!     #[value_range_provider(id = "rooms")]
//!     pub rooms: Vec<Room>,
//!     #[planning_entity_collection]
//!     pub lessons: Vec<Lesson>,
//!     #[planning_score]
//!     pub score: Option<HardSoftScore>,
//! }
//! ```

use proc_macro::TokenStream;

mod entity;
mod solution;

/// Derive macro for implementing the `PlanningEntity` trait.
///
/// # Attributes
///
/// ## Field Attributes
///
/// - `#[planning_id]` - Marks the field as the unique identifier for this entity.
///   Required for every planning entity.
///
/// - `#[planning_variable(value_range_provider = "...")]` - Marks the field as a
///   planning variable. The solver will assign values from the specified value
///   range provider.
///
/// - `#[planning_variable(value_range_provider = "...", allows_unassigned = true)]` -
///   Allows the variable to remain unassigned (null/None).
///
/// # Example
///
/// ```ignore
/// #[derive(PlanningEntity, Clone)]
/// pub struct Lesson {
///     #[planning_id]
///     pub id: String,
///
///     pub subject: String,
///     pub teacher: String,
///
///     #[planning_variable(value_range_provider = "timeslots")]
///     pub timeslot: Option<Timeslot>,
///
///     #[planning_variable(value_range_provider = "rooms", allows_unassigned = true)]
///     pub room: Option<Room>,
/// }
/// ```
#[proc_macro_derive(
    PlanningEntity,
    attributes(planning_id, planning_variable, planning_list_variable)
)]
pub fn derive_planning_entity(input: TokenStream) -> TokenStream {
    entity::derive_planning_entity_impl(input)
}

/// Derive macro for implementing the `PlanningSolution` trait.
///
/// # Attributes
///
/// ## Struct Attributes
///
/// - `#[constraint_provider = "function_name"]` - Specifies the function that
///   provides constraints for this solution. The function must have signature
///   `fn(ConstraintFactory) -> Vec<Constraint>`.
///
/// ## Field Attributes
///
/// - `#[problem_fact_collection]` - Marks a collection field as containing
///   immutable problem facts.
///
/// - `#[problem_fact]` - Marks a single field as a problem fact.
///
/// - `#[planning_entity_collection]` - Marks a collection field as containing
///   planning entities that will be modified during solving.
///
/// - `#[planning_entity]` - Marks a single field as a planning entity.
///
/// - `#[value_range_provider(id = "...")]` - Marks a field as providing values
///   for planning variables with the matching `value_range_provider` reference.
///
/// - `#[planning_score]` - Marks the field that will hold the solution's score.
///
/// # Example
///
/// ```ignore
/// #[derive(PlanningSolution, Clone)]
/// #[constraint_provider = "define_constraints"]
/// pub struct Timetable {
///     #[problem_fact_collection]
///     #[value_range_provider(id = "timeslots")]
///     pub timeslots: Vec<Timeslot>,
///
///     #[problem_fact_collection]
///     #[value_range_provider(id = "rooms")]
///     pub rooms: Vec<Room>,
///
///     #[planning_entity_collection]
///     pub lessons: Vec<Lesson>,
///
///     #[planning_score]
///     pub score: Option<HardSoftScore>,
/// }
/// ```
#[proc_macro_derive(
    PlanningSolution,
    attributes(
        constraint_provider,
        problem_fact_collection,
        problem_fact,
        planning_entity_collection,
        planning_entity,
        value_range_provider,
        planning_score
    )
)]
pub fn derive_planning_solution(input: TokenStream) -> TokenStream {
    solution::derive_planning_solution_impl(input)
}
