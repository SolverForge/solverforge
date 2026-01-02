//! Test utilities for the SolverForge workspace.
//!
//! This crate provides shared fixtures and proptest strategies used across
//! unit tests, integration tests, and doctests in the workspace.
//!
//! # Organization
//!
//! - [`fixtures`]: Reusable domain models (Lesson, Timetable, Employee, etc.)
//! - [`strategies`]: proptest strategies for property-based testing

pub mod fixtures;
pub mod strategies;
