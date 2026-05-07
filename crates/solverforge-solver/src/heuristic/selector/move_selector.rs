/* Zero-erasure move selectors for zero-erasure move generation.

Selectors now expose cursor-owned storage plus borrowable candidates.
The solver evaluates candidates by reference and only takes ownership of the
selected move once the forager commits to an index.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ChangeMove, Move, MoveArena, SequentialCompositeMoveRef, SwapMove};

use super::entity::{EntitySelector, FromSolutionEntitySelector};
use super::value_selector::{StaticValueSelector, ValueSelector};

mod scalar_union;

include!("move_selector/borrowed.rs");
include!("move_selector/iter.rs");
include!("move_selector/change.rs");
include!("move_selector/swap.rs");
