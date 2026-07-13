//! Critical-path analysis for the canonical precedence cursor.

use std::collections::VecDeque;
use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::selector::entity::EntitySelector;
use crate::heuristic::selector::list_support::collect_selected_entities;
use crate::heuristic::selector::precedence_route::{PrecedenceRouteGraph, PrecedenceRouteHooks};

use super::coordinates::CriticalBlock;

#[derive(Default)]
pub(crate) struct CriticalAnalysis {
    pub(crate) blocks: Vec<CriticalBlock>,
    pub(crate) route_graph: PrecedenceRouteGraph,
}

/// Builds immutable critical blocks once at cursor opening.
#[allow(clippy::too_many_arguments)]
pub(crate) fn critical_analysis<S, V, D, ES>(
    score_director: &D,
    entity_selector: &ES,
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    node_duration: fn(&S, V) -> usize,
    list_len: fn(&S, usize) -> usize,
    route_hooks: PrecedenceRouteHooks<S, V>,
) -> CriticalAnalysis
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: Director<S>,
    ES: EntitySelector<S>,
{
    let solution = score_director.working_solution();
    let node_count = element_count(solution);
    if node_count == 0 {
        return CriticalAnalysis::default();
    }
    let elements = (0..node_count)
        .map(|index| index_to_element(solution, index))
        .collect::<Vec<_>>();
    let durations = elements
        .iter()
        .map(|element| usize_to_i64(node_duration(solution, element.clone())))
        .collect::<Vec<_>>();
    let route_graph = route_hooks.build_graph_with_elements(solution, &elements);
    let selected = collect_selected_entities(entity_selector, score_director, list_len);
    critical_analysis_from_graph(&durations, &selected.entities, route_graph)
}

pub(crate) fn critical_analysis_from_graph(
    durations: &[i64],
    selected_entities: &[usize],
    route_graph: PrecedenceRouteGraph,
) -> CriticalAnalysis {
    let Some(summary) = graph_summary(
        durations,
        route_graph.successors(),
        route_graph.predecessors(),
    ) else {
        return CriticalAnalysis {
            blocks: Vec::new(),
            route_graph,
        };
    };

    let mut blocks = Vec::new();
    for &entity in selected_entities {
        let Some(nodes) = route_graph.route(entity) else {
            continue;
        };
        let mut position = 0;
        while position < nodes.len() {
            let starts_arc = position + 1 < nodes.len()
                && is_critical_arc(
                    nodes[position],
                    nodes[position + 1],
                    durations,
                    route_graph.successors(),
                    &summary,
                );
            if !starts_arc {
                if is_critical_node(nodes[position], &summary) {
                    blocks.push(CriticalBlock::new(entity, position, position, nodes.len()));
                }
                position += 1;
                continue;
            }

            let start = position;
            position += 1;
            while position + 1 < nodes.len()
                && is_critical_arc(
                    nodes[position],
                    nodes[position + 1],
                    durations,
                    route_graph.successors(),
                    &summary,
                )
            {
                position += 1;
            }
            blocks.push(CriticalBlock::new(entity, start, position, nodes.len()));
            position += 1;
        }
    }
    CriticalAnalysis {
        blocks,
        route_graph,
    }
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
    let mut earliest = vec![0_i64; node_count];
    let mut ready = VecDeque::new();
    for (node, &degree) in indegree.iter().enumerate() {
        if degree == 0 {
            ready.push_back(node);
        }
    }

    let mut topological = Vec::with_capacity(node_count);
    while let Some(node) = ready.pop_front() {
        topological.push(node);
        let finish = earliest[node].saturating_add(durations[node]);
        for &successor in &successors[node] {
            earliest[successor] = earliest[successor].max(finish);
            indegree[successor] -= 1;
            if indegree[successor] == 0 {
                ready.push_back(successor);
            }
        }
    }
    if topological.len() != node_count {
        return None;
    }

    let makespan = topological
        .iter()
        .map(|&node| earliest[node].saturating_add(durations[node]))
        .max()
        .unwrap_or(0);
    let mut latest = vec![i64::MAX; node_count];
    for &node in topological.iter().rev() {
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

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
