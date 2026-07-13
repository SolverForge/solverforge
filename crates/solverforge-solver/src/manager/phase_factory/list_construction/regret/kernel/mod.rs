//! Canonical source-indexed regret-insertion construction.
//!
//! Public static facades and compiled runtime slots provide only this access
//! protocol.  Candidate ordering, score trials, precedence handling,
//! owner-restricted work bounds, interruption points, and trace transitions
//! live in the modules below exactly once.

use crate::list_placement::OwnerRestriction;

mod evaluation;
mod execute;
mod fallback;
mod precedence;

pub(crate) use super::super::ScoredListConstructionAccess as RegretAccess;
pub(crate) use execute::run_regret;

pub(super) const OWNER_RESTRICTED_REGRET_TRIAL_BUDGET: usize = 16_384;
pub(super) const OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET: usize = 8_000_000;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum RegretValue<Sc> {
    Finite(Sc),
    Forced,
}

pub(super) enum RegretEvaluation<T> {
    Complete(Option<T>),
    Interrupted,
}

impl<Sc: Copy> Copy for RegretValue<Sc> {}

impl<Sc: Copy> Clone for RegretValue<Sc> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Sc: Ord> PartialOrd for RegretValue<Sc> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Sc: Ord> Ord for RegretValue<Sc> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Forced, Self::Forced) => std::cmp::Ordering::Equal,
            (Self::Forced, Self::Finite(_)) => std::cmp::Ordering::Greater,
            (Self::Finite(_), Self::Forced) => std::cmp::Ordering::Less,
            (Self::Finite(left), Self::Finite(right)) => left.cmp(right),
        }
    }
}

pub(super) fn regret_choice_is_better<Sc: Copy + Ord>(
    regret: RegretValue<Sc>,
    score: Sc,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
) -> bool {
    regret > best_regret || (regret == best_regret && score > best_score)
}

pub(super) fn regret_choice_is_better_with_downstream<Sc: Copy + Ord>(
    regret: RegretValue<Sc>,
    score: Sc,
    downstream: usize,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
    best_downstream: usize,
) -> bool {
    regret_choice_is_better(regret, score, best_regret, best_score)
        || (regret == best_regret && score == best_score && downstream > best_downstream)
}

pub(super) fn precedence_frontier_choice_is_better<Sc: Copy + Ord>(
    downstream: usize,
    regret: RegretValue<Sc>,
    score: Sc,
    best_downstream: usize,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
) -> bool {
    downstream > best_downstream
        || (downstream == best_downstream
            && regret_choice_is_better(regret, score, best_regret, best_score))
}

pub(super) fn source_position_by_index<E>(
    source_count: usize,
    entries: &[crate::builder::context::SourceElement<E>],
) -> Option<Vec<Option<usize>>> {
    let mut positions = vec![None; source_count];
    for (position, entry) in entries.iter().enumerate() {
        let slot = positions.get_mut(entry.source_index)?;
        if slot.replace(position).is_some() {
            return None;
        }
    }
    Some(positions)
}

pub(super) fn candidate_entities(
    restriction: OwnerRestriction,
    entity_count: usize,
) -> impl Iterator<Item = usize> {
    match restriction {
        OwnerRestriction::Unrestricted => 0..entity_count,
        OwnerRestriction::Fixed(owner) if owner < entity_count => owner..owner + 1,
        OwnerRestriction::Fixed(_) | OwnerRestriction::Invalid => 0..0,
    }
}
