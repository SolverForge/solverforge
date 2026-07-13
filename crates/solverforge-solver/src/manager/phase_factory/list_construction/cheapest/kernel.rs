//! Canonical source-indexed cheapest-insertion enumeration.

use std::cmp::Reverse;

use solverforge_core::domain::PlanningSolution;

use super::super::ScoredListConstructionAccess;
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_placement::OwnerRestriction;
use crate::stats::CandidateTraceSource;

/// One source-ordered scored insertion trial from the canonical kernel.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CheapestInsertionTrial {
    pub(crate) source: CandidateTraceSource,
    pub(crate) candidate_index: usize,
    pub(crate) element_source_index: usize,
    pub(crate) entity_index: usize,
    pub(crate) insertion_index: usize,
}

/// Execution boundary for the one cheapest-insertion enumeration kernel.
///
/// Live phases and detached previews can publish different lifecycle events,
/// but both receive the same frozen source order and candidate sequence.
pub(crate) trait CheapestInsertionObserver<S, A>
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
{
    type Trial: Copy;

    fn solution(&self) -> &S;
    fn should_interrupt_construction(&mut self) -> bool;
    fn evaluate_insertion(
        &mut self,
        access: &A,
        element: A::Element,
        trial: CheapestInsertionTrial,
    ) -> (Option<S::Score>, Option<Self::Trial>);
    fn discard_trial(&mut self, trial: Self::Trial);
    fn select_trial(&mut self, trial: Self::Trial);
    fn commit_insertion(
        &mut self,
        access: &A,
        element: A::Element,
        entity_index: usize,
        insertion_index: usize,
        score: S::Score,
        trial: Option<Self::Trial>,
    );
    fn finish_construction(&mut self);
    fn finish_without_work(&mut self);
}

/// Runs the one canonical cheapest-insertion candidate loop.
pub(crate) fn run_cheapest<S, A, O>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
    observer: &mut O,
) where
    S: PlanningSolution,
    S::Score: Copy + Ord,
    A: ScoredListConstructionAccess<S>,
    O: CheapestInsertionObserver<S, A>,
{
    let entity_count = access.entity_count(observer.solution());
    let mut elements =
        order_unassigned_elements(access, observer.solution(), bound_unassigned.to_vec());
    if elements.is_empty() || entity_count == 0 {
        observer.finish_without_work();
        return;
    }
    if let Some(downstream) =
        precedence_downstream(access, source_index, observer.solution(), &elements)
    {
        let mut ranked = elements.into_iter().zip(downstream).collect::<Vec<_>>();
        ranked.sort_by_key(|(_, downstream)| Reverse(*downstream));
        elements = ranked.into_iter().map(|(entry, _)| entry).collect();
    }

    'elements: for entry in elements {
        if observer.should_interrupt_construction() {
            break;
        }
        let mut best = None;
        for entity_index in
            legal_entities(access, observer.solution(), &entry.element, entity_count)
        {
            let length = access.list_len(observer.solution(), entity_index);
            for insertion_index in 0..=length {
                if observer.should_interrupt_construction() {
                    if let Some((_, _, _, Some(previous))) = best.take() {
                        observer.discard_trial(previous);
                    }
                    break 'elements;
                }
                let trial = CheapestInsertionTrial {
                    source: CandidateTraceSource::ListCheapestInsertionTrial,
                    candidate_index: insertion_index,
                    element_source_index: entry.source_index,
                    entity_index,
                    insertion_index,
                };
                let (score, trace_trial) =
                    observer.evaluate_insertion(access, entry.element.clone(), trial);
                let Some(score) = score else {
                    if let Some(trace_trial) = trace_trial {
                        observer.discard_trial(trace_trial);
                    }
                    continue;
                };
                if best
                    .as_ref()
                    .is_none_or(|(_, _, best_score, _)| score > *best_score)
                {
                    if let Some((_, _, _, Some(previous))) = best.take() {
                        observer.discard_trial(previous);
                    }
                    best = Some((entity_index, insertion_index, score, trace_trial));
                } else if let Some(trace_trial) = trace_trial {
                    observer.discard_trial(trace_trial);
                }
            }
        }
        if let Some((entity_index, insertion_index, score, trace_trial)) = best {
            if let Some(trace_trial) = trace_trial {
                observer.select_trial(trace_trial);
            }
            observer.commit_insertion(
                access,
                entry.element,
                entity_index,
                insertion_index,
                score,
                trace_trial,
            );
        }
    }
    observer.finish_construction();
}

fn order_unassigned_elements<S, A>(
    access: &A,
    solution: &S,
    mut elements: Vec<SourceElement<A::Element>>,
) -> Vec<SourceElement<A::Element>>
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
{
    elements.sort_by_key(|entry| {
        (
            access.construction_order_key(solution, &entry.element),
            entry.source_index,
        )
    });
    elements
}

fn precedence_downstream<S, A>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    solution: &S,
    elements: &[SourceElement<A::Element>],
) -> Option<Vec<usize>>
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
{
    let mut position_by_source = vec![None; source_index.source_count()];
    for (position, entry) in elements.iter().enumerate() {
        *position_by_source.get_mut(entry.source_index)? = Some(position);
    }
    let durations = elements
        .iter()
        .map(|entry| access.precedence_duration(solution, &entry.element))
        .collect::<Option<Vec<_>>>()?;
    let mut successors = vec![Vec::new(); elements.len()];
    let mut predecessor_counts = vec![0usize; elements.len()];
    let mut scratch = Vec::new();
    for (from, entry) in elements.iter().enumerate() {
        scratch.clear();
        if !access.extend_precedence_successor_source_indices(
            solution,
            &entry.element,
            source_index,
            &mut scratch,
        ) {
            return None;
        }
        for successor_source in &scratch {
            let Some(Some(to)) = position_by_source.get(*successor_source) else {
                continue;
            };
            successors[from].push(*to);
            predecessor_counts[*to] = predecessor_counts[*to].saturating_add(1);
        }
    }
    let mut ready = predecessor_counts
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut topo = Vec::with_capacity(elements.len());
    while let Some(index) = ready.pop() {
        topo.push(index);
        for &successor in &successors[index] {
            predecessor_counts[successor] = predecessor_counts[successor].saturating_sub(1);
            if predecessor_counts[successor] == 0 {
                ready.push(successor);
            }
        }
    }
    if topo.len() != elements.len() {
        return None;
    }
    let mut downstream = durations.clone();
    for &index in topo.iter().rev() {
        let tail = successors[index]
            .iter()
            .map(|&successor| downstream[successor])
            .max()
            .unwrap_or(0);
        downstream[index] = durations[index].saturating_add(tail);
    }
    Some(downstream)
}

fn legal_entities<S, A>(
    access: &A,
    solution: &S,
    element: &A::Element,
    entity_count: usize,
) -> Vec<usize>
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
{
    match access.owner_restriction(solution, entity_count, element) {
        OwnerRestriction::Unrestricted => (0..entity_count).collect(),
        OwnerRestriction::Fixed(owner) if owner < entity_count => vec![owner],
        OwnerRestriction::Fixed(_) | OwnerRestriction::Invalid => Vec::new(),
    }
}
