/* {{project_name}} — list-variable vehicle routing optimizer built with SolverForge

   Structure:
     domain/      — ProblemData, Vehicle (planning entity), VrpPlan (solution)
     constraints/ — capacity (hard) + total distance (soft)
     solver/      — construction heuristic, list-variable move selectors, local search engine
     api/         — HTTP API (axum): POST /solutions, GET /solutions/{id}
     data/        — built-in demo instance */

pub mod api;
pub mod constraints;
pub mod data;
pub mod domain;
pub mod solver;
