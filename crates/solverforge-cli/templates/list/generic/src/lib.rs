/* {{project_name}} — list variable constraint optimizer built with SolverForge

   Structure:
     domain/      — Item (problem fact), Container (planning entity), Plan (solution)
     constraints/ — Scoring rules
     solver/      — Engine, service, termination config
     api/         — HTTP API (axum)
     data/        — Demo data / data loading */

pub mod api;
pub mod constraints;
pub mod data;
pub mod domain;
pub mod solver;
