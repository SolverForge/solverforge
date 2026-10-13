//! Canonical union child scheduling shared by ordinary and indexed cursors.
use crate::heuristic::selector::move_selector::{CandidateId, MoveStreamContext};
use solverforge_config::UnionSelectionOrder;

/// The one canonical scheduler for a vector selector union.
///
/// It owns only child-selection state. Candidate storage and ownership remain
/// with the caller's cursors, which lets both ordinary and resource-aware
/// composition cursors use the exact same sequential, round-robin, rotating,
/// and stratified ordering rules.
pub(crate) struct UnionScheduler {
    current_cursor: usize,
    selection_order: UnionSelectionOrder,
    exhausted: Vec<bool>,
    live_cursor_count: usize,
    cursor_offset: usize,
    cursor_stride: usize,
    context: MoveStreamContext,
    random_draw: u64,
    weights: Vec<u64>,
    weighted_current: Vec<i128>,
    total_live_weight: u64,
}

impl UnionScheduler {
    pub(crate) fn new(
        cursor_count: usize,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        assert_eq!(
            weights.len(),
            cursor_count,
            "union weight count must match child count"
        );
        assert!(
            matches!(
                selection_order,
                UnionSelectionOrder::Random | UnionSelectionOrder::StratifiedRandom
            ) || weights.iter().all(|weight| *weight == 1),
            "union weights require random or stratified_random selection order"
        );
        let exhausted = weights
            .iter()
            .map(|weight| *weight == 0)
            .collect::<Vec<_>>();
        let live_cursor_count = exhausted.iter().filter(|exhausted| !**exhausted).count();
        let total_live_weight = weights.iter().sum();
        let cursor_offset = match selection_order {
            UnionSelectionOrder::RotatingRoundRobin | UnionSelectionOrder::StratifiedRandom => {
                context.random_index(cursor_count, 0xA11C_E5E1_EC70_0001)
            }
            UnionSelectionOrder::Sequential
            | UnionSelectionOrder::RoundRobin
            | UnionSelectionOrder::Random => 0,
        };
        let cursor_stride = match selection_order {
            UnionSelectionOrder::StratifiedRandom => {
                context.random_stride(cursor_count, 0xA11C_E5E1_EC70_0002)
            }
            UnionSelectionOrder::Sequential
            | UnionSelectionOrder::RoundRobin
            | UnionSelectionOrder::RotatingRoundRobin
            | UnionSelectionOrder::Random => 1,
        };
        let current_cursor = match selection_order {
            UnionSelectionOrder::StratifiedRandom => 0,
            _ => cursor_offset,
        };
        Self {
            current_cursor,
            selection_order,
            exhausted,
            live_cursor_count,
            cursor_offset,
            cursor_stride,
            context,
            random_draw: 0,
            weighted_current: vec![0; cursor_count],
            weights,
            total_live_weight,
        }
    }

    /// Selects the next child candidate through the union's canonical order.
    ///
    /// `next_child` is invoked only for a branch the schedule reaches. This
    /// is the reachability boundary used by resource-aware provider cursors:
    /// an unreached union branch cannot pull a callback merely because the
    /// outer union was opened.
    pub(crate) fn next_child(
        &mut self,
        cursor_count: usize,
        mut next_child: impl FnMut(usize) -> Option<CandidateId>,
    ) -> Option<(usize, CandidateId)> {
        match self.selection_order {
            UnionSelectionOrder::Sequential => {
                while self.current_cursor < cursor_count {
                    let cursor_index = self.current_cursor;
                    if let Some(candidate) = next_child(cursor_index) {
                        return Some((cursor_index, candidate));
                    }
                    self.current_cursor += 1;
                }
                None
            }
            UnionSelectionOrder::RoundRobin | UnionSelectionOrder::RotatingRoundRobin => {
                if self.live_cursor_count == 0 {
                    return None;
                }
                while self.live_cursor_count > 0 {
                    let cursor_index = self.current_cursor % cursor_count;
                    self.current_cursor = (self.current_cursor + 1) % cursor_count;
                    if self.exhausted[cursor_index] {
                        continue;
                    }
                    if let Some(candidate) = next_child(cursor_index) {
                        return Some((cursor_index, candidate));
                    }
                    self.exhausted[cursor_index] = true;
                    self.live_cursor_count -= 1;
                }
                None
            }
            UnionSelectionOrder::Random => {
                while self.live_cursor_count > 0 {
                    let draw = self
                        .context
                        .random_seed(0xA11C_E5E1_EC70_1000_u64.wrapping_add(self.random_draw))
                        % self.total_live_weight;
                    self.random_draw = self.random_draw.wrapping_add(1);
                    let mut cumulative = 0;
                    let cursor_index = self
                        .weights
                        .iter()
                        .enumerate()
                        .find_map(|(index, weight)| {
                            if self.exhausted[index] {
                                return None;
                            }
                            cumulative += *weight;
                            (draw < cumulative).then_some(index)
                        })
                        .expect("random union live weights must select one child");
                    if let Some(candidate) = next_child(cursor_index) {
                        return Some((cursor_index, candidate));
                    }
                    self.exhausted[cursor_index] = true;
                    self.live_cursor_count -= 1;
                    self.total_live_weight -= self.weights[cursor_index];
                }
                None
            }
            UnionSelectionOrder::StratifiedRandom => {
                while self.live_cursor_count > 0 {
                    let mut selected = None;
                    let mut selected_weight = i128::MIN;
                    for position in 0..cursor_count {
                        let cursor_index =
                            (self.cursor_offset + position * self.cursor_stride) % cursor_count;
                        if self.exhausted[cursor_index] {
                            continue;
                        }
                        self.weighted_current[cursor_index] +=
                            i128::from(self.weights[cursor_index]);
                        if self.weighted_current[cursor_index] > selected_weight {
                            selected = Some(cursor_index);
                            selected_weight = self.weighted_current[cursor_index];
                        }
                    }
                    let cursor_index = selected
                        .expect("stratified union live-child count must match exhaustion state");
                    self.weighted_current[cursor_index] -= i128::from(self.total_live_weight);
                    if let Some(candidate) = next_child(cursor_index) {
                        return Some((cursor_index, candidate));
                    }
                    self.exhausted[cursor_index] = true;
                    self.live_cursor_count -= 1;
                    self.total_live_weight -= self.weights[cursor_index];
                }
                None
            }
        }
    }
}
