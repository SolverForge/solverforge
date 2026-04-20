use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::entity::EntitySelector;

pub(crate) type NearbyCandidate = (usize, usize, f64);

pub(crate) struct SelectedEntities {
    pub(crate) entities: Vec<usize>,
    pub(crate) route_lens: Vec<usize>,
}

impl SelectedEntities {
    pub(crate) fn total_elements(&self) -> usize {
        self.route_lens.iter().sum()
    }
}

pub(crate) fn collect_selected_entities<S, D, ES>(
    entity_selector: &ES,
    score_director: &D,
    list_len: fn(&S, usize) -> usize,
) -> SelectedEntities
where
    S: PlanningSolution,
    D: Director<S>,
    ES: EntitySelector<S>,
{
    let solution = score_director.working_solution();
    let entities: Vec<usize> = entity_selector
        .iter(score_director)
        .map(|reference| reference.entity_index)
        .collect();
    let route_lens = entities
        .iter()
        .map(|&entity| list_len(solution, entity))
        .collect();
    SelectedEntities {
        entities,
        route_lens,
    }
}

pub(crate) fn sort_and_limit_nearby_candidates(
    candidates: &mut Vec<NearbyCandidate>,
    max_nearby: usize,
) {
    candidates.sort_by(|left, right| {
        left.2
            .partial_cmp(&right.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(max_nearby);
}
