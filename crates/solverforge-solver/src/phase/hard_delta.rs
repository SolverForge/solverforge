use solverforge_core::score::{Score, ScoreLevel};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HardScoreDelta {
    Improving,
    Neutral,
    Worse,
}

pub(crate) fn hard_score_delta<ScoreT>(
    previous: ScoreT,
    candidate: ScoreT,
) -> Option<HardScoreDelta>
where
    ScoreT: Score,
{
    let mut saw_hard_level = false;
    for level in 0..ScoreT::levels_count() {
        if ScoreT::level_label(level) != ScoreLevel::Hard {
            continue;
        }
        saw_hard_level = true;
        match candidate
            .level_number(level)
            .cmp(&previous.level_number(level))
        {
            Ordering::Greater => return Some(HardScoreDelta::Improving),
            Ordering::Less => return Some(HardScoreDelta::Worse),
            Ordering::Equal => {}
        }
    }

    saw_hard_level.then_some(HardScoreDelta::Neutral)
}
