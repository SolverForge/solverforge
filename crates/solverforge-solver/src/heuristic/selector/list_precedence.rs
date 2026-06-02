/* List precedence critical-path selector.

Generates existing list moves around critical same-list arcs in precedence
makespan models. The selector consumes plain list-slot hooks for node duration
and fixed successors; it does not depend on a problem-specific model type.
*/

use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{
    ListChangeMove, ListMoveUnion, ListMultiSwapMove, ListPermuteMove, ListReverseMove,
    ListRuinMove, ListSwapMove, SublistChangeMove, SublistSwapMove, MAX_LIST_PERMUTE_WINDOW_SIZE,
};

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::precedence_route::{PrecedenceRouteGraph, PrecedenceRouteHooks};

const CRITICAL_PERMUTE_MAX_WINDOW_SIZE: usize = 5;
const CRITICAL_RUIN_MAX_SIZE: usize = 5;
const CRITICAL_SUBLIST_MAX_SIZE: usize = 3;

#[derive(Clone, Copy, Debug)]
struct CriticalBlock {
    entity: usize,
    start: usize,
    end: usize,
    route_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdjacentSwap {
    entity: usize,
    position: usize,
}

impl AdjacentSwap {
    fn as_tuple(self) -> (usize, usize, usize) {
        (self.entity, self.position, self.position + 1)
    }
}

impl CriticalBlock {
    fn len(self) -> usize {
        self.end - self.start + 1
    }

    fn change_move_count(self) -> usize {
        self.len() * self.route_len.saturating_sub(1)
    }

    fn adjacent_change_move_count(self) -> usize {
        self.len().saturating_sub(1)
    }

    fn boundary_change_move_count(self) -> usize {
        count_boundary_change_moves(self)
    }

    fn permute_move_count(self) -> usize {
        count_permute_moves_for_len(self.len(), CRITICAL_PERMUTE_MAX_WINDOW_SIZE)
    }

    fn swap_move_count(self) -> usize {
        self.len().saturating_mul(self.len().saturating_sub(1)) / 2
    }

    fn reverse_move_count(self) -> usize {
        self.len().saturating_mul(self.len().saturating_sub(1)) / 2
    }

    fn adjacent_sublist_swap_move_count(self) -> usize {
        count_adjacent_sublist_swap_moves_for_len(self.len(), CRITICAL_SUBLIST_MAX_SIZE)
    }

    fn ruin_move_count(self) -> usize {
        if self.len() < 2 {
            0
        } else {
            let window_len = self.len().min(CRITICAL_RUIN_MAX_SIZE);
            self.len() - window_len + 1
        }
    }

    fn sublist_change_move_count(self) -> usize {
        count_sublist_change_moves_for_len(self.len(), self.route_len, CRITICAL_SUBLIST_MAX_SIZE)
    }

    fn move_count(self) -> usize {
        self.change_move_count()
            + self.swap_move_count()
            + self.reverse_move_count()
            + self.adjacent_sublist_swap_move_count()
            + self.ruin_move_count()
            + self.sublist_change_move_count()
            + self.permute_move_count()
    }
}

pub struct ListPrecedenceMoveSelector<S, V, ES> {
    entity_selector: ES,
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    node_duration: fn(&S, V) -> usize,
    fixed_successors: fn(&S, V, &mut Vec<V>),
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    ruin_remove: fn(&mut S, usize, usize) -> V,
    ruin_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

pub struct ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListMoveUnion<S, V>>,
    blocks: Vec<CriticalBlock>,
    route_graph: PrecedenceRouteGraph,
    context: MoveStreamContext,
    block_idx: usize,
    move_idx: usize,
    multi_swap_idx: usize,
    multi_swap_count: usize,
    multi_ruin_idx: usize,
    multi_ruin_count: usize,
    critical_swaps: Vec<AdjacentSwap>,
    support_swaps: Vec<AdjacentSwap>,
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    fixed_successors: fn(&S, V, &mut Vec<V>),
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    ruin_remove: fn(&mut S, usize, usize) -> V,
    ruin_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        blocks: Vec<CriticalBlock>,
        route_graph: PrecedenceRouteGraph,
        context: MoveStreamContext,
        element_count: fn(&S) -> usize,
        index_to_element: fn(&S, usize) -> V,
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let critical_swaps = critical_adjacent_swaps(&blocks);
        let support_swaps = support_adjacent_swaps(&blocks, &route_graph);
        let multi_swap_count = multi_support_swap_count(&critical_swaps, &support_swaps);
        let multi_ruin_count = multi_critical_ruin_count(&blocks);
        Self {
            store: CandidateStore::new(),
            blocks,
            route_graph,
            context,
            block_idx: 0,
            move_idx: 0,
            multi_swap_idx: 0,
            multi_swap_count,
            multi_ruin_idx: 0,
            multi_ruin_count,
            critical_swaps,
            support_swaps,
            element_count,
            index_to_element,
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            list_set,
            list_reverse,
            ruin_remove,
            ruin_insert,
            element_owner_fn,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
        }
    }

    fn push_move(&mut self, block: CriticalBlock, move_idx: usize) -> CandidateId {
        let adjacent_count = block.adjacent_change_move_count();
        if move_idx < adjacent_count {
            let source = block.start + move_idx;
            let mov = ListMoveUnion::ListChange(ListChangeMove::new(
                block.entity,
                source,
                block.entity,
                source + 2,
                self.list_len,
                self.list_get,
                self.list_remove,
                self.list_insert,
                self.variable_name,
                self.descriptor_index,
            ));
            return self.store.push(mov);
        }

        let change_count = block.change_move_count();
        if move_idx < change_count {
            let (source, dest) = non_adjacent_change(block, move_idx - adjacent_count);
            let mov = ListMoveUnion::ListChange(ListChangeMove::new(
                block.entity,
                source,
                block.entity,
                dest,
                self.list_len,
                self.list_get,
                self.list_remove,
                self.list_insert,
                self.variable_name,
                self.descriptor_index,
            ));
            return self.store.push(mov);
        }

        let swap_count = block.swap_move_count();
        if move_idx < change_count + swap_count {
            let (first, second) = critical_swap(block, move_idx - change_count);
            let mov = ListMoveUnion::ListSwap(ListSwapMove::new(
                block.entity,
                first,
                block.entity,
                second,
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            ));
            return self.store.push(mov);
        }

        let reverse_count = block.reverse_move_count();
        if move_idx < change_count + swap_count + reverse_count {
            let (start, end) = critical_reverse(block, move_idx - change_count - swap_count);
            let mov = ListMoveUnion::ListReverse(ListReverseMove::new(
                block.entity,
                start,
                end,
                self.list_len,
                self.list_get,
                self.list_reverse,
                self.variable_name,
                self.descriptor_index,
            ));
            return self.store.push(mov);
        }

        let adjacent_sublist_swap_count = block.adjacent_sublist_swap_move_count();
        if move_idx < change_count + swap_count + reverse_count + adjacent_sublist_swap_count {
            let (first_start, first_end, second_start, second_end) = critical_adjacent_sublist_swap(
                block,
                move_idx - change_count - swap_count - reverse_count,
            );
            let mov = ListMoveUnion::SublistSwap(SublistSwapMove::new(
                block.entity,
                first_start,
                first_end,
                block.entity,
                second_start,
                second_end,
                self.list_len,
                self.list_get,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            ));
            return self.store.push(mov);
        }

        let ruin_count = block.ruin_move_count();
        if move_idx
            < change_count + swap_count + reverse_count + adjacent_sublist_swap_count + ruin_count
        {
            let indices = critical_ruin_indices(
                block,
                move_idx - change_count - swap_count - reverse_count - adjacent_sublist_swap_count,
            );
            let mov = ListMoveUnion::ListRuin(
                ListRuinMove::new(
                    block.entity,
                    &indices,
                    self.entity_count,
                    self.list_len,
                    self.list_get,
                    self.ruin_remove,
                    self.ruin_insert,
                    self.variable_name,
                    self.descriptor_index,
                )
                .with_element_owner_fn(self.element_owner_fn)
                .with_precedence_hooks(
                    Some(self.element_count),
                    Some(self.index_to_element),
                    Some(self.fixed_successors),
                ),
            );
            return self.store.push(mov);
        }

        let sublist_change_count = block.sublist_change_move_count();
        if move_idx
            < change_count
                + swap_count
                + reverse_count
                + adjacent_sublist_swap_count
                + ruin_count
                + sublist_change_count
        {
            let (source_start, size, dest) = critical_sublist_change(
                block.start,
                block.len(),
                block.route_len,
                move_idx
                    - change_count
                    - swap_count
                    - reverse_count
                    - adjacent_sublist_swap_count
                    - ruin_count,
            );
            let source_start = block.start + source_start;
            return self
                .store
                .push(ListMoveUnion::SublistChange(SublistChangeMove::new(
                    block.entity,
                    source_start,
                    source_start + size,
                    block.entity,
                    dest,
                    self.list_len,
                    self.list_get,
                    self.sublist_remove,
                    self.sublist_insert,
                    self.variable_name,
                    self.descriptor_index,
                )));
        }

        let permute_idx = move_idx
            - change_count
            - swap_count
            - reverse_count
            - adjacent_sublist_swap_count
            - ruin_count
            - sublist_change_count;
        let (start_offset, size, permutation) = critical_permutation(block.len(), permute_idx);
        let start = block.start + start_offset;
        self.store
            .push(ListMoveUnion::ListPermute(ListPermuteMove::new(
                block.entity,
                start,
                start + size,
                permutation,
                self.list_len,
                self.list_get,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            )))
    }

    fn push_multi_ruin_move(
        &mut self,
        sources: SmallVec<[(usize, SmallVec<[usize; 8]>); 4]>,
    ) -> CandidateId {
        let mov = ListMoveUnion::ListRuin(
            ListRuinMove::new_multi_source(
                &sources,
                self.entity_count,
                self.list_len,
                self.list_get,
                self.ruin_remove,
                self.ruin_insert,
                self.variable_name,
                self.descriptor_index,
            )
            .with_element_owner_fn(self.element_owner_fn)
            .with_precedence_hooks(
                Some(self.element_count),
                Some(self.index_to_element),
                Some(self.fixed_successors),
            ),
        );
        self.store.push(mov)
    }

    fn push_multi_swap_move(&mut self, swaps: &[(usize, usize, usize)]) -> CandidateId {
        let mov = ListMoveUnion::ListMultiSwap(
            ListMultiSwapMove::new(
                swaps,
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            )
            .with_require_score_improvement(true),
        );
        self.store.push(mov)
    }
}

impl<S, V> MoveCursor<S, ListMoveUnion<S, V>> for ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.multi_swap_idx < self.multi_swap_count {
                let move_idx = ordered_index(
                    self.multi_swap_idx,
                    self.multi_swap_count,
                    self.context,
                    0xC917_1EAF_5EED_0004 ^ self.descriptor_index as u64,
                );
                self.multi_swap_idx += 1;
                let swaps =
                    multi_support_swaps(&self.critical_swaps, &self.support_swaps, move_idx);
                if self
                    .route_graph
                    .multi_intra_list_swaps_introduce_cycle(&swaps)
                {
                    continue;
                }
                return Some(self.push_multi_swap_move(&swaps));
            }
            if self.multi_ruin_idx < self.multi_ruin_count {
                let move_idx = ordered_index(
                    self.multi_ruin_idx,
                    self.multi_ruin_count,
                    self.context,
                    0xC917_1EAF_5EED_0003 ^ self.descriptor_index as u64,
                );
                self.multi_ruin_idx += 1;
                let sources = multi_critical_ruin_sources(&self.blocks, move_idx);
                return Some(self.push_multi_ruin_move(sources));
            }
            if self.block_idx >= self.blocks.len() {
                return None;
            }
            let block_index = ordered_index(
                self.block_idx,
                self.blocks.len(),
                self.context,
                0xC917_1EAF_5EED_0001 ^ self.descriptor_index as u64,
            );
            let block = self.blocks[block_index];
            let move_count = block.move_count();
            if self.move_idx < move_count {
                let move_idx = tiered_precedence_move_index(
                    block,
                    self.move_idx,
                    self.context,
                    0xC917_1EAF_5EED_0002
                        ^ self.descriptor_index as u64
                        ^ block.entity as u64
                        ^ ((block.start as u64) << 16)
                        ^ ((block.end as u64) << 32),
                );
                self.move_idx += 1;
                if move_introduces_route_cycle(block, move_idx, &self.route_graph) {
                    continue;
                }
                return Some(self.push_move(block, move_idx));
            }
            self.block_idx += 1;
            self.move_idx = 0;
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListMoveUnion<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListMoveUnion<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListMoveUnion<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListPrecedenceMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListPrecedenceMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, ES> ListPrecedenceMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        element_count: fn(&S) -> usize,
        index_to_element: fn(&S, usize) -> V,
        node_duration: fn(&S, V) -> usize,
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            element_count,
            index_to_element,
            node_duration,
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            list_set,
            list_reverse,
            ruin_remove,
            ruin_insert,
            element_owner_fn,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveUnion<S, V>> for ListPrecedenceMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListPrecedenceMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let analysis = self.critical_analysis(score_director);
        ListPrecedenceMoveCursor::new(
            analysis.blocks,
            analysis.route_graph,
            _context,
            self.element_count,
            self.index_to_element,
            self.fixed_successors,
            self.entity_count,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.list_set,
            self.list_reverse,
            self.ruin_remove,
            self.ruin_insert,
            self.element_owner_fn,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let analysis = self.critical_analysis(score_director);
        let single_block_count: usize = analysis
            .blocks
            .iter()
            .copied()
            .map(|block| filtered_move_count(block, &analysis.route_graph))
            .sum();
        single_block_count
            + filtered_multi_support_swap_count(&analysis.blocks, &analysis.route_graph)
            + multi_critical_ruin_count(&analysis.blocks)
    }
}

impl<S, V, ES> ListPrecedenceMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn critical_analysis<D: Director<S>>(&self, score_director: &D) -> CriticalAnalysis {
        let solution = score_director.working_solution();
        let node_count = (self.element_count)(solution);
        if node_count == 0 {
            return CriticalAnalysis::default();
        }

        let elements = (0..node_count)
            .map(|index| (self.index_to_element)(solution, index))
            .collect::<Vec<_>>();
        let durations = elements
            .iter()
            .map(|element| usize_to_i64((self.node_duration)(solution, element.clone())))
            .collect::<Vec<_>>();
        let route_hooks = PrecedenceRouteHooks::new(
            self.element_count,
            self.index_to_element,
            self.fixed_successors,
            self.entity_count,
            self.list_len,
            self.list_get,
        );
        let route_graph = route_hooks.build_graph_with_elements(solution, &elements);

        let Some(summary) = graph_summary(
            &durations,
            route_graph.successors(),
            route_graph.predecessors(),
        ) else {
            return CriticalAnalysis {
                blocks: Vec::new(),
                route_graph,
            };
        };
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        let mut blocks = Vec::new();
        for &entity in &selected.entities {
            let Some(nodes) = route_graph.route(entity) else {
                continue;
            };
            let mut pos = 0;
            while pos < nodes.len() {
                let starts_critical_arc = pos + 1 < nodes.len()
                    && is_critical_arc(
                        nodes[pos],
                        nodes[pos + 1],
                        &durations,
                        route_graph.successors(),
                        &summary,
                    );
                if !starts_critical_arc {
                    if is_critical_node(nodes[pos], &summary) {
                        blocks.push(CriticalBlock {
                            entity,
                            start: pos,
                            end: pos,
                            route_len: nodes.len(),
                        });
                    }
                    pos += 1;
                    continue;
                }

                let start = pos;
                pos += 1;
                while pos + 1 < nodes.len()
                    && is_critical_arc(
                        nodes[pos],
                        nodes[pos + 1],
                        &durations,
                        route_graph.successors(),
                        &summary,
                    )
                {
                    pos += 1;
                }
                blocks.push(CriticalBlock {
                    entity,
                    start,
                    end: pos,
                    route_len: nodes.len(),
                });
                pos += 1;
            }
        }
        CriticalAnalysis {
            blocks,
            route_graph,
        }
    }
}

#[derive(Default)]
struct CriticalAnalysis {
    blocks: Vec<CriticalBlock>,
    route_graph: PrecedenceRouteGraph,
}

struct GraphSummary {
    earliest: Vec<i64>,
    latest: Vec<i64>,
}

fn graph_summary(
    durations: &[i64],
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
) -> Option<GraphSummary> {
    let node_count = durations.len();
    let mut indegree = predecessors.iter().map(Vec::len).collect::<Vec<_>>();
    let mut earliest = vec![0i64; node_count];
    let mut ready = VecDeque::new();
    for (node, &degree) in indegree.iter().enumerate() {
        if degree == 0 {
            ready.push_back(node);
        }
    }

    let mut topo = Vec::with_capacity(node_count);
    while let Some(node) = ready.pop_front() {
        topo.push(node);
        let finish = earliest[node].saturating_add(durations[node]);
        for &successor in &successors[node] {
            earliest[successor] = earliest[successor].max(finish);
            indegree[successor] -= 1;
            if indegree[successor] == 0 {
                ready.push_back(successor);
            }
        }
    }
    if topo.len() != node_count {
        return None;
    }

    let makespan = topo
        .iter()
        .map(|&node| earliest[node].saturating_add(durations[node]))
        .max()
        .unwrap_or(0);
    let mut latest = vec![i64::MAX; node_count];
    for &node in topo.iter().rev() {
        latest[node] = if successors[node].is_empty() {
            makespan.saturating_sub(durations[node])
        } else {
            successors[node]
                .iter()
                .map(|&successor| latest[successor].saturating_sub(durations[node]))
                .min()
                .unwrap_or_else(|| makespan.saturating_sub(durations[node]))
        };
    }

    Some(GraphSummary { earliest, latest })
}

fn is_critical_arc(
    from: usize,
    to: usize,
    durations: &[i64],
    successors: &[Vec<usize>],
    summary: &GraphSummary,
) -> bool {
    successors[from].contains(&to)
        && summary.earliest[from] == summary.latest[from]
        && summary.earliest[to] == summary.latest[to]
        && summary.earliest[from].saturating_add(durations[from]) == summary.earliest[to]
}

fn is_critical_node(node: usize, summary: &GraphSummary) -> bool {
    summary.earliest[node] == summary.latest[node]
}

fn tiered_precedence_move_index(
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

fn is_valid_non_adjacent_dest(
    source: usize,
    source_offset: usize,
    dest: usize,
    block_len: usize,
) -> bool {
    if dest == source || dest == source + 1 {
        return false;
    }
    if source_offset + 1 < block_len && dest == source + 2 {
        return false;
    }
    true
}

fn count_boundary_change_moves(block: CriticalBlock) -> usize {
    boundary_change_offsets(block)
        .into_iter()
        .map(|source_offset| {
            let source = block.start + source_offset;
            (0..=block.route_len)
                .filter(|&dest| {
                    is_valid_non_adjacent_dest(source, source_offset, dest, block.len())
                })
                .count()
        })
        .sum()
}

fn boundary_change_offsets(block: CriticalBlock) -> SmallVec<[usize; 2]> {
    if block.len() == 0 {
        return SmallVec::new();
    }
    let mut offsets = SmallVec::new();
    offsets.push(0);
    let last = block.len() - 1;
    if last != 0 {
        offsets.push(last);
    }
    offsets
}

fn boundary_change(block: CriticalBlock, mut offset: usize) -> Option<(usize, usize)> {
    for source_offset in boundary_change_offsets(block) {
        let source = block.start + source_offset;
        for dest in 0..=block.route_len {
            if !is_valid_non_adjacent_dest(source, source_offset, dest, block.len()) {
                continue;
            }
            if offset == 0 {
                return Some((source, dest));
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
        for dest in 0..=block.route_len {
            if !is_valid_non_adjacent_dest(source, source_offset, dest, block.len()) {
                continue;
            }
            if offset == 0 {
                return Some((source, dest));
            }
            offset -= 1;
        }
    }
    None
}

fn non_adjacent_change(block: CriticalBlock, offset: usize) -> (usize, usize) {
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

fn critical_swap(block: CriticalBlock, mut offset: usize) -> (usize, usize) {
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

fn critical_reverse(block: CriticalBlock, mut offset: usize) -> (usize, usize) {
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

fn count_adjacent_sublist_swap_moves_for_len(block_len: usize, max_sublist_size: usize) -> usize {
    if block_len < 3 {
        return 0;
    }
    let max_sublist_size = max_sublist_size.min(block_len);
    let mut count = 0usize;
    for start in 0..block_len {
        for first_size in 1..=max_sublist_size {
            let second_start = start + first_size;
            if second_start >= block_len {
                break;
            }
            for second_size in 1..=max_sublist_size {
                if first_size == 1 && second_size == 1 {
                    continue;
                }
                if second_start + second_size <= block_len {
                    count += 1;
                }
            }
        }
    }
    count
}

fn critical_adjacent_sublist_swap(
    block: CriticalBlock,
    mut offset: usize,
) -> (usize, usize, usize, usize) {
    let max_sublist_size = CRITICAL_SUBLIST_MAX_SIZE.min(block.len());
    for start_offset in 0..block.len() {
        for first_size in 1..=max_sublist_size {
            let second_start_offset = start_offset + first_size;
            if second_start_offset >= block.len() {
                break;
            }
            for second_size in 1..=max_sublist_size {
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

fn critical_ruin_indices(block: CriticalBlock, offset: usize) -> SmallVec<[usize; 8]> {
    let window_len = block.len().min(CRITICAL_RUIN_MAX_SIZE);
    let max_start = block.len() - window_len;
    assert!(
        offset <= max_start,
        "critical block ruin offset should map to a valid move"
    );
    let start_offset = offset;
    (0..window_len)
        .map(|idx| block.start + start_offset + idx)
        .collect()
}

fn critical_adjacent_swaps(blocks: &[CriticalBlock]) -> Vec<AdjacentSwap> {
    let mut swaps = Vec::new();
    for block in blocks {
        for position in block.start..block.end {
            push_unique_adjacent_swap(
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

fn support_adjacent_swaps(
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
        push_unique_adjacent_swap(
            swaps,
            AdjacentSwap {
                entity,
                position: position - 1,
            },
        );
    }
    if position + 1 < route.len() {
        push_unique_adjacent_swap(swaps, AdjacentSwap { entity, position });
    }
}

fn push_unique_adjacent_swap(swaps: &mut Vec<AdjacentSwap>, swap: AdjacentSwap) {
    if !swaps.contains(&swap) {
        swaps.push(swap);
    }
}

fn multi_support_swap_count(
    critical_swaps: &[AdjacentSwap],
    support_swaps: &[AdjacentSwap],
) -> usize {
    let mut count = 0usize;
    for first_idx in 0..critical_swaps.len() {
        let first = critical_swaps[first_idx];
        for &second in &critical_swaps[first_idx + 1..] {
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

fn multi_support_swaps(
    critical_swaps: &[AdjacentSwap],
    support_swaps: &[AdjacentSwap],
    mut offset: usize,
) -> SmallVec<[(usize, usize, usize); 4]> {
    for first_idx in 0..critical_swaps.len() {
        let first = critical_swaps[first_idx];
        for &second in &critical_swaps[first_idx + 1..] {
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

fn filtered_multi_support_swap_count(
    blocks: &[CriticalBlock],
    route_graph: &PrecedenceRouteGraph,
) -> usize {
    let critical_swaps = critical_adjacent_swaps(blocks);
    let support_swaps = support_adjacent_swaps(blocks, route_graph);
    let count = multi_support_swap_count(&critical_swaps, &support_swaps);
    (0..count)
        .filter(|&offset| {
            let swaps = multi_support_swaps(&critical_swaps, &support_swaps, offset);
            !route_graph.multi_intra_list_swaps_introduce_cycle(&swaps)
        })
        .count()
}

fn multi_critical_ruin_count(blocks: &[CriticalBlock]) -> usize {
    let mut count = 0usize;
    for first_idx in 0..blocks.len() {
        let first_count = blocks[first_idx].len();
        if first_count == 0 {
            continue;
        }
        for second in &blocks[first_idx + 1..] {
            count += first_count * second.len();
        }
    }
    count
}

fn multi_critical_ruin_sources(
    blocks: &[CriticalBlock],
    mut offset: usize,
) -> SmallVec<[(usize, SmallVec<[usize; 8]>); 4]> {
    for first_idx in 0..blocks.len() {
        let first = blocks[first_idx];
        let first_count = first.len();
        if first_count == 0 {
            continue;
        }
        for second in &blocks[first_idx + 1..] {
            let second_count = second.len();
            let pair_count = first_count * second_count;
            if offset >= pair_count {
                offset -= pair_count;
                continue;
            }
            let first_offset = offset / second_count;
            let second_offset = offset % second_count;
            return smallvec![
                (first.entity, smallvec![first.start + first_offset]),
                (second.entity, smallvec![second.start + second_offset])
            ];
        }
    }
    SmallVec::new()
}

fn count_permute_moves_for_len(block_len: usize, max_window_size: usize) -> usize {
    if block_len < 2 {
        return 0;
    }
    let max_window_size = max_window_size
        .min(MAX_LIST_PERMUTE_WINDOW_SIZE)
        .min(block_len);
    let mut count = 0;
    for start in 0..block_len {
        let max_valid = max_window_size.min(block_len - start);
        for size in 2..=max_valid {
            count += factorial(size).saturating_sub(1);
        }
    }
    count
}

fn count_sublist_change_moves_for_len(
    block_len: usize,
    route_len: usize,
    max_sublist_size: usize,
) -> usize {
    if block_len < 2 || route_len < 2 {
        return 0;
    }
    let max_sublist_size = max_sublist_size.min(block_len).min(route_len);
    let mut count = 0usize;
    for size in 2..=max_sublist_size {
        let source_count = block_len - size + 1;
        let dest_count = route_len.saturating_sub(size);
        count += source_count * dest_count;
    }
    count
}

fn critical_sublist_change(
    block_start: usize,
    block_len: usize,
    route_len: usize,
    mut offset: usize,
) -> (usize, usize, usize) {
    let max_sublist_size = CRITICAL_SUBLIST_MAX_SIZE.min(block_len).min(route_len);
    for size in 2..=max_sublist_size {
        for source_start in 0..=block_len - size {
            for dest in 0..=route_len - size {
                if dest == block_start + source_start {
                    continue;
                }
                if offset == 0 {
                    return (source_start, size, dest);
                }
                offset -= 1;
            }
        }
    }
    panic!("critical sublist-change offset should map to a valid move")
}

fn critical_permutation(
    block_len: usize,
    mut offset: usize,
) -> (
    usize,
    usize,
    SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
) {
    let max_window_size = CRITICAL_PERMUTE_MAX_WINDOW_SIZE
        .min(MAX_LIST_PERMUTE_WINDOW_SIZE)
        .min(block_len);
    for start in 0..block_len {
        let max_valid = max_window_size.min(block_len - start);
        for size in 2..=max_valid {
            let permutation_count = factorial(size).saturating_sub(1);
            if offset < permutation_count {
                return (start, size, nth_permutation(size, offset + 1));
            }
            offset -= permutation_count;
        }
    }
    panic!("critical permutation offset should map to a valid window")
}

fn filtered_move_count(block: CriticalBlock, route_graph: &PrecedenceRouteGraph) -> usize {
    (0..block.move_count())
        .filter(|&move_idx| !move_introduces_route_cycle(block, move_idx, route_graph))
        .count()
}

fn move_introduces_route_cycle(
    block: CriticalBlock,
    move_idx: usize,
    route_graph: &PrecedenceRouteGraph,
) -> bool {
    let Some(route) = route_graph.route(block.entity) else {
        return false;
    };
    if route.len() != block.route_len {
        return false;
    }
    let change_count = block.change_move_count();
    if move_idx < change_count {
        let (source, dest) = if move_idx < block.adjacent_change_move_count() {
            (block.start + move_idx, block.start + move_idx + 2)
        } else {
            non_adjacent_change(block, move_idx - block.adjacent_change_move_count())
        };
        return route_graph.intra_list_change_introduces_cycle(block.entity, source, dest);
    }

    let swap_count = block.swap_move_count();
    if move_idx < change_count + swap_count {
        let (first, second) = critical_swap(block, move_idx - change_count);
        return route_graph.intra_list_swap_introduces_cycle(block.entity, first, second);
    }

    let reverse_count = block.reverse_move_count();
    if move_idx < change_count + swap_count + reverse_count {
        let (start, end) = critical_reverse(block, move_idx - change_count - swap_count);
        return route_graph.intra_list_reverse_introduces_cycle(block.entity, start, end);
    }

    let adjacent_sublist_swap_count = block.adjacent_sublist_swap_move_count();
    if move_idx < change_count + swap_count + reverse_count + adjacent_sublist_swap_count {
        let (first_start, first_end, second_start, second_end) = critical_adjacent_sublist_swap(
            block,
            move_idx - change_count - swap_count - reverse_count,
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
    if move_idx
        < change_count + swap_count + reverse_count + adjacent_sublist_swap_count + ruin_count
    {
        return false;
    }

    let sublist_change_count = block.sublist_change_move_count();
    if move_idx
        < change_count
            + swap_count
            + reverse_count
            + adjacent_sublist_swap_count
            + ruin_count
            + sublist_change_count
    {
        let (source_start, size, dest) = critical_sublist_change(
            block.start,
            block.len(),
            block.route_len,
            move_idx
                - change_count
                - swap_count
                - reverse_count
                - adjacent_sublist_swap_count
                - ruin_count,
        );
        return route_graph.intra_sublist_change_introduces_cycle(
            block.entity,
            block.start + source_start,
            block.start + source_start + size,
            dest,
        );
    }

    let (start_offset, size, permutation) = critical_permutation(
        block.len(),
        move_idx
            - change_count
            - swap_count
            - reverse_count
            - adjacent_sublist_swap_count
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

fn factorial(value: usize) -> usize {
    (2..=value).product()
}

fn nth_permutation(len: usize, mut rank: usize) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut remaining: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> = (0..len).collect();
    let mut permutation = smallvec![];
    for pos in 0..len {
        let suffix = len - pos - 1;
        let step = factorial(suffix);
        let index = if step == 0 { 0 } else { rank / step };
        rank %= step.max(1);
        permutation.push(remaining.remove(index));
    }
    permutation
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
