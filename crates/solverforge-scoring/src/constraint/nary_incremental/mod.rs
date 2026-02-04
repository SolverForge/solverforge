//! Macro-generated N-ary incremental constraints for self-join evaluation.
//!
//! This module provides the `impl_incremental_nary_constraint!` macro that generates
//! fully monomorphized incremental constraint implementations for bi/tri/quad/penta arities.
//!
//! Zero-erasure: all closures are concrete generic types, no trait objects, no Arc.

#[macro_use]
mod bi;
#[macro_use]
mod penta;
#[macro_use]
mod quad;
#[macro_use]
mod tri;

pub use bi::impl_incremental_bi_constraint;
pub use penta::impl_incremental_penta_constraint;
pub use quad::impl_incremental_quad_constraint;
pub use tri::impl_incremental_tri_constraint;

/// Generates an incremental N-ary constraint struct and implementations.
///
/// This macro produces:
/// - The constraint struct with all fields
/// - Constructor `new()`
/// - Private helper methods `compute_score()`, `insert_entity()`, `retract_entity()`
/// - Full `IncrementalConstraint<S, Sc>` trait implementation
/// - `Debug` implementation
///
/// # Usage
///
/// ```text
/// impl_incremental_nary_constraint!(bi, IncrementalBiConstraint);
/// impl_incremental_nary_constraint!(tri, IncrementalTriConstraint);
/// impl_incremental_nary_constraint!(quad, IncrementalQuadConstraint);
/// impl_incremental_nary_constraint!(penta, IncrementalPentaConstraint);
/// ```
#[macro_export]
macro_rules! impl_incremental_nary_constraint {
    (bi, $struct_name:ident) => {
        $crate::impl_incremental_bi_constraint!($struct_name);
    };
    (tri, $struct_name:ident) => {
        $crate::impl_incremental_tri_constraint!($struct_name);
    };
    (quad, $struct_name:ident) => {
        $crate::impl_incremental_quad_constraint!($struct_name);
    };
    (penta, $struct_name:ident) => {
        $crate::impl_incremental_penta_constraint!($struct_name);
    };
}

pub use impl_incremental_nary_constraint;

// Generate the N-ary constraint types
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;

impl_incremental_nary_constraint!(bi, IncrementalBiConstraint);
impl_incremental_nary_constraint!(tri, IncrementalTriConstraint);
impl_incremental_nary_constraint!(quad, IncrementalQuadConstraint);
impl_incremental_nary_constraint!(penta, IncrementalPentaConstraint);
