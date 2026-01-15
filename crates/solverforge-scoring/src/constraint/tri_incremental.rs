//! Zero-erasure incremental tri-constraint for self-join triple evaluation.
//!
//! All function types are concrete generics - no trait objects, no Arc.
//! Uses key-based indexing: entities are grouped by join key for O(k) lookups.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;

crate::impl_incremental_nary_constraint!(tri, IncrementalTriConstraint);
