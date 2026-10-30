//! Zero-erasure move selectors for zero-erasure move generation.
//!
//! Selectors now expose cursor-owned storage plus borrowable candidates.
//! The solver evaluates candidates by reference and only takes ownership of the
//! selected move once the forager commits to an index.

use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

#[cfg(test)]
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::r#move::{Move, SequentialCompositeMoveRef};

#[cfg(test)]
use super::entity::{EntitySelector, FromSolutionEntitySelector};
#[cfg(test)]
use super::value_selector::{StaticValueSelector, ValueSelector};

include!("move_selector/borrowed.rs");
include!("move_selector/unit_resource.rs");
include!("move_selector/arena.rs");
include!("move_selector/iter.rs");
#[cfg(test)]
include!("move_selector/change.rs");
