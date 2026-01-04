//! Vehicle Routing Quickstart for SolverForge
//!
//! Solves vehicle routing problems with time windows, capacity constraints,
//! and travel time minimization using Late Acceptance local search.
//!
//! # Domain Model
//!
//! - [`Location`](domain::Location): Geographic point with haversine distance
//! - [`Visit`](domain::Visit): Customer to visit with time window and demand
//! - [`Vehicle`](domain::Vehicle): Delivery vehicle with capacity and route
//! - [`VehicleRoutePlan`](domain::VehicleRoutePlan): Complete planning solution
//!
//! # Constraints
//!
//! - **Vehicle capacity** (hard): Total demand must not exceed vehicle capacity
//! - **Time windows** (hard): Service must finish before max end time
//! - **Travel time** (soft): Minimize total driving time

pub mod api;
pub mod console;
pub mod constraints;
pub mod demo_data;
pub mod domain;
pub mod geometry;
pub mod routing;
pub mod solver;
