//! Incremental bi-constraint for self-join evaluation.
//!
//! Zero-erasure: all closures are concrete generic types, fully monomorphized.
//! Uses key-based indexing for O(k) lookups instead of O(n) iteration.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;

crate::impl_incremental_nary_constraint!(bi, IncrementalBiConstraint);
