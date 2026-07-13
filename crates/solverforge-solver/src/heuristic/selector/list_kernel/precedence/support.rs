//! Cross-block support swaps and multi-ruin coordinates.

use smallvec::{smallvec, SmallVec};

use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::coordinates::CriticalBlock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdjacentSwap {
    entity: usize,
    position: usize,
}

impl AdjacentSwap {
    fn as_tuple(self) -> (usize, usize, usize) {
        (self.entity, self.position, self.position + 1)
    }
}

pub(super) fn critical_adjacent_swaps(blocks: &[CriticalBlock]) -> Vec<AdjacentSwap> {
    let mut swaps = Vec::new();
    for block in blocks {
        for position in block.start..block.end {
            push_unique(
                &mut swaps,
                AdjacentSwap {
                    entity: block.entity,
                    position,
                },
            );
        }
    }
    swaps
}

pub(super) fn support_adjacent_swaps(
    blocks: &[CriticalBlock],
    route_graph: &PrecedenceRouteGraph,
) -> Vec<AdjacentSwap> {
    let mut swaps = Vec::new();
    for block in blocks {
        let Some(route) = route_graph.route(block.entity) else {
            continue;
        };
        for position in block.start..=block.end {
            let Some(&node) = route.get(position) else {
                continue;
            };
            for &successor in route_graph.fixed_successors(node) {
                push_support_adjacent_swaps(route_graph, successor, &mut swaps);
            }
            for &predecessor in route_graph.fixed_predecessors(node) {
                push_support_adjacent_swaps(route_graph, predecessor, &mut swaps);
            }
        }
    }
    swaps
}

pub(super) fn multi_support_swap_count(
    critical_swaps: &[AdjacentSwap],
    support_swaps: &[AdjacentSwap],
) -> usize {
    let mut count = 0;
    for first_index in 0..critical_swaps.len() {
        let first = critical_swaps[first_index];
        for &second in &critical_swaps[first_index + 1..] {
            if first.entity == second.entity {
                continue;
            }
            count += support_swaps
                .iter()
                .filter(|&&support| {
                    support.entity != first.entity && support.entity != second.entity
                })
                .count();
        }
    }
    count
}

pub(super) fn multi_support_swaps(
    critical_swaps: &[AdjacentSwap],
    support_swaps: &[AdjacentSwap],
    mut offset: usize,
) -> SmallVec<[(usize, usize, usize); 4]> {
    for first_index in 0..critical_swaps.len() {
        let first = critical_swaps[first_index];
        for &second in &critical_swaps[first_index + 1..] {
            if first.entity == second.entity {
                continue;
            }
            for &support in support_swaps {
                if support.entity == first.entity || support.entity == second.entity {
                    continue;
                }
                if offset == 0 {
                    return smallvec![first.as_tuple(), second.as_tuple(), support.as_tuple()];
                }
                offset -= 1;
            }
        }
    }
    SmallVec::new()
}

pub(crate) fn filtered_multi_support_swap_count(
    blocks: &[CriticalBlock],
    route_graph: &PrecedenceRouteGraph,
) -> usize {
    let critical = critical_adjacent_swaps(blocks);
    let support = support_adjacent_swaps(blocks, route_graph);
    (0..multi_support_swap_count(&critical, &support))
        .filter(|&offset| {
            !route_graph.multi_intra_list_swaps_introduce_cycle(&multi_support_swaps(
                &critical, &support, offset,
            ))
        })
        .count()
}

pub(crate) fn multi_critical_ruin_count(blocks: &[CriticalBlock]) -> usize {
    let mut count = 0;
    for first_index in 0..blocks.len() {
        let first_count = blocks[first_index].len();
        if first_count == 0 {
            continue;
        }
        for second in &blocks[first_index + 1..] {
            count += first_count * second.len();
        }
    }
    count
}

pub(super) fn multi_critical_ruin_sources(
    blocks: &[CriticalBlock],
    mut offset: usize,
) -> SmallVec<[(usize, SmallVec<[usize; 8]>); 4]> {
    for first_index in 0..blocks.len() {
        let first = blocks[first_index];
        let first_count = first.len();
        if first_count == 0 {
            continue;
        }
        for second in &blocks[first_index + 1..] {
            let second_count = second.len();
            let pair_count = first_count * second_count;
            if offset >= pair_count {
                offset -= pair_count;
                continue;
            }
            return smallvec![
                (first.entity, smallvec![first.start + offset / second_count]),
                (
                    second.entity,
                    smallvec![second.start + offset % second_count]
                )
            ];
        }
    }
    SmallVec::new()
}

fn push_support_adjacent_swaps(
    route_graph: &PrecedenceRouteGraph,
    node: usize,
    swaps: &mut Vec<AdjacentSwap>,
) {
    let Some((entity, position)) = route_graph.node_route_position(node) else {
        return;
    };
    let Some(route) = route_graph.route(entity) else {
        return;
    };
    if position > 0 {
        push_unique(
            &mut *swaps,
            AdjacentSwap {
                entity,
                position: position - 1,
            },
        );
    }
    if position + 1 < route.len() {
        push_unique(swaps, AdjacentSwap { entity, position });
    }
}

fn push_unique(swaps: &mut Vec<AdjacentSwap>, swap: AdjacentSwap) {
    if !swaps.contains(&swap) {
        swaps.push(swap);
    }
}
