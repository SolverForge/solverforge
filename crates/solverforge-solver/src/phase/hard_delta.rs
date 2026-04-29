use solverforge_core::score::{Score, ScoreLevel};

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
    let hard_index = (0..ScoreT::levels_count())
        .find(|index| ScoreT::level_label(*index) == ScoreLevel::Hard)?;
    let previous_hard = previous.level_number(hard_index);
    let candidate_hard = candidate.level_number(hard_index);
    Some(match candidate_hard.cmp(&previous_hard) {
        std::cmp::Ordering::Greater => HardScoreDelta::Improving,
        std::cmp::Ordering::Equal => HardScoreDelta::Neutral,
        std::cmp::Ordering::Less => HardScoreDelta::Worse,
    })
}
