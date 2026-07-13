//! Shared list-access protocol for typed and dynamic planning-list slots.
//!
//! Canonical list kernels import this declaration-only module. Physical
//! typed/dynamic dispatch lives in sibling modules; route and savings stay
//! separate semantic bundles.

mod dynamic;
mod route;
mod static_access;
mod types;

pub(crate) use route::{RouteAccess, RouteSequenceAccess, SavingsAccess};
pub(crate) use types::{ListAccess, ListAccessCapability, ListAccessError};
