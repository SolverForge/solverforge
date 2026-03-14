/* {{project_name}} — standard variable constraint optimizer built with SolverForge

   Structure:
     domain/      — Resource (problem fact), Task (planning entity), Plan (solution)
     constraints/ — Scoring rules
     solver/      — Engine, service, termination config
     api/         — HTTP API (axum)
     data/        — Demo data / data loading */

pub mod api;
pub mod constraints;
pub mod data;
pub mod domain;
pub mod solver;
