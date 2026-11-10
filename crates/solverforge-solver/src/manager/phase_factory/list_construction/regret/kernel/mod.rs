//! Canonical source-indexed regret-insertion construction.
//!
//! Public static facades and compiled runtime slots provide only this access
//! protocol.  Candidate ordering, score trials, precedence handling,
//! owner-restricted work bounds, interruption points, and trace transitions
//! live in the modules below exactly once.

mod budgets;
mod choice;
mod evaluation;
mod execute;
mod fallback;
mod precedence;

pub(crate) use super::super::ScoredListConstructionAccess as RegretAccess;
pub(crate) use execute::run_regret;

use budgets::{OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET, OWNER_RESTRICTED_REGRET_TRIAL_BUDGET};
use choice::{
    candidate_entities, precedence_frontier_choice_is_better,
    regret_choice_is_better_with_downstream, source_position_by_index, RegretEvaluation,
    RegretValue,
};
