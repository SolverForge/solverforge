use crate::heuristic::r#move::ScalarMoveUnion;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use solverforge_core::domain::PlanningSolution;

include!("selector/grouped_scalar.rs");
pub(crate) mod types;
