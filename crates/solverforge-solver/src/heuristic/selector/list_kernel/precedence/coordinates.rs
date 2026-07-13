//! Critical-block coordinate arithmetic and route-cycle filtering.

use smallvec::{smallvec, SmallVec};

use crate::heuristic::r#move::MAX_LIST_PERMUTE_WINDOW_SIZE;
use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

pub(super) const CRITICAL_PERMUTE_MAX_WINDOW_SIZE: usize = 5;
pub(super) const CRITICAL_RUIN_MAX_SIZE: usize = 5;
pub(super) const CRITICAL_SUBLIST_MAX_SIZE: usize = 3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CriticalBlock {
    pub(super) entity: usize,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) route_len: usize,
}

impl CriticalBlock {
    pub(super) const fn new(entity: usize, start: usize, end: usize, route_len: usize) -> Self {
        Self {
            entity,
            start,
            end,
            route_len,
        }
    }

    pub(super) fn len(self) -> usize {
        self.end - self.start + 1
    }

    pub(super) fn change_move_count(self) -> usize {
        self.len() * self.route_len.saturating_sub(1)
    }

    pub(super) fn adjacent_change_move_count(self) -> usize {
        self.len().saturating_sub(1)
    }

    fn boundary_change_move_count(self) -> usize {
        count_boundary_change_moves(self)
    }

    pub(super) fn permute_move_count(self) -> usize {
        count_permute_moves_for_len(self.len(), CRITICAL_PERMUTE_MAX_WINDOW_SIZE)
    }

    pub(super) fn swap_move_count(self) -> usize {
        self.len().saturating_mul(self.len().saturating_sub(1)) / 2
    }

    pub(super) fn reverse_move_count(self) -> usize {
        self.len().saturating_mul(self.len().saturating_sub(1)) / 2
    }

    pub(super) fn adjacent_sublist_swap_move_count(self) -> usize {
        count_adjacent_sublist_swap_moves_for_len(self.len(), CRITICAL_SUBLIST_MAX_SIZE)
    }

    pub(super) fn ruin_move_count(self) -> usize {
        if self.len() < 2 {
            0
        } else {
            self.len() - self.len().min(CRITICAL_RUIN_MAX_SIZE) + 1
        }
    }

    pub(super) fn sublist_change_move_count(self) -> usize {
        count_sublist_change_moves_for_len(self.len(), self.route_len, CRITICAL_SUBLIST_MAX_SIZE)
    }

    pub(super) fn move_count(self) -> usize {
        self.change_move_count()
            + self.swap_move_count()
            + self.reverse_move_count()
            + self.adjacent_sublist_swap_move_count()
            + self.ruin_move_count()
            + self.sublist_change_move_count()
            + self.permute_move_count()
    }
}

pub(super) fn tiered_precedence_move_index(
    block: CriticalBlock,
    offset: usize,
    context: MoveStreamContext,
    salt: u64,
) -> usize {
    let adjacent_count = block.adjacent_change_move_count();
    if offset < adjacent_count {
        return ordered_index(
            offset,
            adjacent_count,
            context,
            salt ^ 0xAD1A_CE17_0000_0001,
        );
    }
    let boundary_count = block.boundary_change_move_count();
    if offset < adjacent_count + boundary_count {
        return adjacent_count
            + ordered_index(
                offset - adjacent_count,
                boundary_count,
                context,
                salt ^ 0xAD1A_CE17_0000_0002,
            );
    }
    let remaining_count = block.move_count() - adjacent_count - boundary_count;
    adjacent_count
        + boundary_count
        + ordered_index(
            offset - adjacent_count - boundary_count,
            remaining_count,
            context,
            salt ^ 0xAD1A_CE17_0000_0003,
        )
}

pub(super) fn non_adjacent_change(block: CriticalBlock, offset: usize) -> (usize, usize) {
    let boundary_count = block.boundary_change_move_count();
    if offset < boundary_count {
        return boundary_change(block, offset)
            .expect("critical block boundary change offset should map to a valid move");
    }
    if let Some(change) = interior_change(block, offset - boundary_count) {
        return change;
    }
    panic!("critical block non-adjacent change offset should map to a valid move")
}

pub(super) fn critical_swap(block: CriticalBlock, mut offset: usize) -> (usize, usize) {
    for first_offset in 0..block.len() {
        for second_offset in first_offset + 1..block.len() {
            if offset == 0 {
                return (block.start + first_offset, block.start + second_offset);
            }
            offset -= 1;
        }
    }
    panic!("critical block swap offset should map to a valid move")
}

pub(super) fn critical_reverse(block: CriticalBlock, mut offset: usize) -> (usize, usize) {
    for start_offset in 0..block.len() {
        for end_offset in start_offset + 1..block.len() {
            if offset == 0 {
                return (block.start + start_offset, block.start + end_offset + 1);
            }
            offset -= 1;
        }
    }
    panic!("critical block reverse offset should map to a valid move")
}

pub(super) fn critical_adjacent_sublist_swap(
    block: CriticalBlock,
    mut offset: usize,
) -> (usize, usize, usize, usize) {
    let max_size = CRITICAL_SUBLIST_MAX_SIZE.min(block.len());
    for start_offset in 0..block.len() {
        for first_size in 1..=max_size {
            let second_start_offset = start_offset + first_size;
            if second_start_offset >= block.len() {
                break;
            }
            for second_size in 1..=max_size {
                if first_size == 1 && second_size == 1 {
                    continue;
                }
                let second_end_offset = second_start_offset + second_size;
                if second_end_offset > block.len() {
                    continue;
                }
                if offset == 0 {
                    return (
                        block.start + start_offset,
                        block.start + second_start_offset,
                        block.start + second_start_offset,
                        block.start + second_end_offset,
                    );
                }
                offset -= 1;
            }
        }
    }
    panic!("critical block adjacent sublist-swap offset should map to a valid move")
}

pub(super) fn critical_ruin_indices(block: CriticalBlock, offset: usize) -> SmallVec<[usize; 8]> {
    let window_len = block.len().min(CRITICAL_RUIN_MAX_SIZE);
    let max_start = block.len() - window_len;
    assert!(
        offset <= max_start,
        "critical block ruin offset should map to a valid move"
    );
    (0..window_len)
        .map(|index| block.start + offset + index)
        .collect()
}

pub(super) fn critical_sublist_change(
    block_start: usize,
    block_len: usize,
    route_len: usize,
    mut offset: usize,
) -> (usize, usize, usize) {
    let max_size = CRITICAL_SUBLIST_MAX_SIZE.min(block_len).min(route_len);
    for size in 2..=max_size {
        for source_start in 0..=block_len - size {
            for destination in 0..=route_len - size {
                if destination == block_start + source_start {
                    continue;
                }
                if offset == 0 {
                    return (source_start, size, destination);
                }
                offset -= 1;
            }
        }
    }
    panic!("critical sublist-change offset should map to a valid move")
}

pub(super) fn critical_permutation(
    block_len: usize,
    mut offset: usize,
) -> (
    usize,
    usize,
    SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
) {
    let max_window = CRITICAL_PERMUTE_MAX_WINDOW_SIZE
        .min(MAX_LIST_PERMUTE_WINDOW_SIZE)
        .min(block_len);
    for start in 0..block_len {
        let max_valid = max_window.min(block_len - start);
        for size in 2..=max_valid {
            let count = factorial(size).saturating_sub(1);
            if offset < count {
                return (start, size, nth_permutation(size, offset + 1));
            }
            offset -= count;
        }
    }
    panic!("critical permutation offset should map to a valid window")
}

pub(crate) fn filtered_move_count(
    block: CriticalBlock,
    route_graph: &PrecedenceRouteGraph,
) -> usize {
    (0..block.move_count())
        .filter(|&move_index| !move_introduces_route_cycle(block, move_index, route_graph))
        .count()
}

pub(super) fn move_introduces_route_cycle(
    block: CriticalBlock,
    move_index: usize,
    route_graph: &PrecedenceRouteGraph,
) -> bool {
    let Some(route) = route_graph.route(block.entity) else {
        return false;
    };
    if route.len() != block.route_len {
        return false;
    }
    let change_count = block.change_move_count();
    if move_index < change_count {
        let (source, destination) = if move_index < block.adjacent_change_move_count() {
            (block.start + move_index, block.start + move_index + 2)
        } else {
            non_adjacent_change(block, move_index - block.adjacent_change_move_count())
        };
        return route_graph.intra_list_change_introduces_cycle(block.entity, source, destination);
    }
    let swap_count = block.swap_move_count();
    if move_index < change_count + swap_count {
        let (first, second) = critical_swap(block, move_index - change_count);
        return route_graph.intra_list_swap_introduces_cycle(block.entity, first, second);
    }
    let reverse_count = block.reverse_move_count();
    if move_index < change_count + swap_count + reverse_count {
        let (start, end) = critical_reverse(block, move_index - change_count - swap_count);
        return route_graph.intra_list_reverse_introduces_cycle(block.entity, start, end);
    }
    let sublist_swap_count = block.adjacent_sublist_swap_move_count();
    if move_index < change_count + swap_count + reverse_count + sublist_swap_count {
        let (first_start, first_end, second_start, second_end) = critical_adjacent_sublist_swap(
            block,
            move_index - change_count - swap_count - reverse_count,
        );
        return route_graph.intra_sublist_swap_introduces_cycle(
            block.entity,
            first_start,
            first_end,
            second_start,
            second_end,
        );
    }
    let ruin_count = block.ruin_move_count();
    if move_index < change_count + swap_count + reverse_count + sublist_swap_count + ruin_count {
        return false;
    }
    let sublist_change_count = block.sublist_change_move_count();
    if move_index
        < change_count
            + swap_count
            + reverse_count
            + sublist_swap_count
            + ruin_count
            + sublist_change_count
    {
        let (source_start, size, destination) = critical_sublist_change(
            block.start,
            block.len(),
            block.route_len,
            move_index
                - change_count
                - swap_count
                - reverse_count
                - sublist_swap_count
                - ruin_count,
        );
        return route_graph.intra_sublist_change_introduces_cycle(
            block.entity,
            block.start + source_start,
            block.start + source_start + size,
            destination,
        );
    }
    let (start_offset, size, permutation) = critical_permutation(
        block.len(),
        move_index
            - change_count
            - swap_count
            - reverse_count
            - sublist_swap_count
            - ruin_count
            - sublist_change_count,
    );
    let start = block.start + start_offset;
    route_graph.intra_list_permutation_introduces_cycle(
        block.entity,
        start,
        start + size,
        &permutation,
    )
}

fn count_boundary_change_moves(block: CriticalBlock) -> usize {
    boundary_change_offsets(block)
        .into_iter()
        .map(|source_offset| {
            let source = block.start + source_offset;
            (0..=block.route_len)
                .filter(|&destination| {
                    is_valid_non_adjacent_dest(source, source_offset, destination, block.len())
                })
                .count()
        })
        .sum()
}

fn boundary_change_offsets(block: CriticalBlock) -> SmallVec<[usize; 2]> {
    if block.len() == 0 {
        return SmallVec::new();
    }
    let mut offsets = smallvec![0];
    let last = block.len() - 1;
    if last != 0 {
        offsets.push(last);
    }
    offsets
}

fn boundary_change(block: CriticalBlock, mut offset: usize) -> Option<(usize, usize)> {
    for source_offset in boundary_change_offsets(block) {
        let source = block.start + source_offset;
        for destination in 0..=block.route_len {
            if !is_valid_non_adjacent_dest(source, source_offset, destination, block.len()) {
                continue;
            }
            if offset == 0 {
                return Some((source, destination));
            }
            offset -= 1;
        }
    }
    None
}

fn interior_change(block: CriticalBlock, mut offset: usize) -> Option<(usize, usize)> {
    for source_offset in 0..block.len() {
        if source_offset == 0 || source_offset + 1 == block.len() {
            continue;
        }
        let source = block.start + source_offset;
        for destination in 0..=block.route_len {
            if !is_valid_non_adjacent_dest(source, source_offset, destination, block.len()) {
                continue;
            }
            if offset == 0 {
                return Some((source, destination));
            }
            offset -= 1;
        }
    }
    None
}

fn is_valid_non_adjacent_dest(
    source: usize,
    source_offset: usize,
    destination: usize,
    block_len: usize,
) -> bool {
    destination != source
        && destination != source + 1
        && !(source_offset + 1 < block_len && destination == source + 2)
}

fn count_adjacent_sublist_swap_moves_for_len(block_len: usize, max_size: usize) -> usize {
    if block_len < 3 {
        return 0;
    }
    let max_size = max_size.min(block_len);
    let mut count = 0;
    for start in 0..block_len {
        for first_size in 1..=max_size {
            let second_start = start + first_size;
            if second_start >= block_len {
                break;
            }
            for second_size in 1..=max_size {
                if first_size != 1 || second_size != 1 {
                    count += usize::from(second_start + second_size <= block_len);
                }
            }
        }
    }
    count
}

fn count_permute_moves_for_len(block_len: usize, max_window: usize) -> usize {
    if block_len < 2 {
        return 0;
    }
    let max_window = max_window.min(MAX_LIST_PERMUTE_WINDOW_SIZE).min(block_len);
    (0..block_len)
        .map(|start| {
            (2..=max_window.min(block_len - start))
                .map(|size| factorial(size).saturating_sub(1))
                .sum::<usize>()
        })
        .sum()
}

fn count_sublist_change_moves_for_len(
    block_len: usize,
    route_len: usize,
    max_size: usize,
) -> usize {
    if block_len < 2 || route_len < 2 {
        return 0;
    }
    let max_size = max_size.min(block_len).min(route_len);
    (2..=max_size)
        .map(|size| (block_len - size + 1) * route_len.saturating_sub(size))
        .sum()
}

fn factorial(value: usize) -> usize {
    (2..=value).product()
}

fn nth_permutation(len: usize, mut rank: usize) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut remaining: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> = (0..len).collect();
    let mut permutation = SmallVec::new();
    for position in 0..len {
        let suffix = len - position - 1;
        let step = factorial(suffix);
        let index = rank / step;
        rank %= step;
        permutation.push(remaining.remove(index));
    }
    permutation
}
