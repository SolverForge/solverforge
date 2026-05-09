use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::entity::EntitySelector;
use super::move_selector::MoveStreamContext;

pub(crate) struct SelectedEntities {
    pub(crate) entities: Vec<usize>,
    pub(crate) route_lens: Vec<usize>,
}

impl SelectedEntities {
    pub(crate) fn apply_stream_order(&mut self, context: MoveStreamContext, salt: u64) {
        let start = context.start_offset(self.entities.len(), salt);
        self.entities.rotate_left(start);
        self.route_lens.rotate_left(start);
    }

    pub(crate) fn total_elements(&self) -> usize {
        self.route_lens.iter().sum()
    }

    pub(crate) fn list_change_move_capacity(&self) -> usize {
        let entity_count = self.entities.len();
        let total_elements = self.total_elements();

        self.route_lens
            .iter()
            .map(|&source_len| {
                let intra_tail = source_len.saturating_sub(1);
                let intra_moves = intra_tail * intra_tail;
                let inter_destinations =
                    total_elements.saturating_sub(source_len) + entity_count.saturating_sub(1);
                intra_moves + source_len * inter_destinations
            })
            .sum()
    }

    pub(crate) fn list_swap_move_capacity(&self) -> usize {
        let intra: usize = self
            .route_lens
            .iter()
            .map(|&route_len| route_len * route_len.saturating_sub(1) / 2)
            .sum();
        let inter: usize = (0..self.route_lens.len())
            .flat_map(|left| (left + 1..self.route_lens.len()).map(move |right| (left, right)))
            .map(|(left, right)| self.route_lens[left] * self.route_lens[right])
            .sum();
        intra + inter
    }
}

pub(crate) fn ordered_index(
    offset: usize,
    len: usize,
    context: MoveStreamContext,
    salt: u64,
) -> usize {
    debug_assert!(offset < len);
    if len <= 1 {
        return 0;
    }

    let start = context.start_offset(len, salt);
    let stride = context.stride(len, salt ^ 0xD1B5_4A32_D192_ED03);
    (start + offset * stride) % len
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
