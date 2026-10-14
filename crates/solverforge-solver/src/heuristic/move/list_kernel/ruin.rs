//! Shared ruin-and-recreate mutation mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedValueTabuToken,
};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::heuristic::selector::precedence_route::node_index;

use super::{ListMoveAccess, ListRuinAccess};

pub(crate) type RuinSources = SmallVec<[(usize, SmallVec<[usize; 8]>); 4]>;
pub(crate) type RuinUndo = SmallVec<[(usize, usize, usize); 8]>;

pub(crate) fn single_ruin_source(entity: usize, indices: &[usize]) -> RuinSources {
    let mut sorted = SmallVec::<[usize; 8]>::from_slice(indices);
    sorted.sort_unstable();
    smallvec![(entity, sorted)]
}

pub(crate) fn merged_ruin_sources(sources: &[(usize, SmallVec<[usize; 8]>)]) -> RuinSources {
    let mut merged = RuinSources::new();
    for (entity, indices) in sources {
        let mut sorted = indices.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.is_empty() {
            continue;
        }
        if let Some((_, existing)) = merged.iter_mut().find(|(candidate, _)| candidate == entity) {
            existing.extend(sorted);
            existing.sort_unstable();
            existing.dedup();
        } else {
            merged.push((*entity, sorted));
        }
    }
    merged.sort_by_key(|(entity, _)| *entity);
    merged
}

pub(crate) fn ruin_entity_indices(sources: &RuinSources) -> SmallVec<[usize; 8]> {
    sources.iter().map(|(entity, _)| *entity).collect()
}

pub(crate) fn ruin_count(sources: &RuinSources) -> usize {
    sources.iter().map(|(_, indices)| indices.len()).sum()
}

#[cfg(test)]
pub(crate) fn final_positions_after_insertions(
    placements: &SmallVec<[(usize, usize); 8]>,
) -> SmallVec<[usize; 8]> {
    let mut current_positions = SmallVec::with_capacity(placements.len());
    for i in 0..placements.len() {
        let (entity, insert_position) = placements[i];
        for j in 0..i {
            let (previous_entity, _) = placements[j];
            if previous_entity == entity && current_positions[j] >= insert_position {
                current_positions[j] += 1;
            }
        }
        current_positions.push(insert_position);
    }
    current_positions
}

fn final_positions_after_ordered_insertions(placements: &RuinUndo) -> SmallVec<[usize; 8]> {
    let mut current_positions = SmallVec::with_capacity(placements.len());
    for i in 0..placements.len() {
        let (entity, insert_position, _) = placements[i];
        for j in 0..i {
            let (previous_entity, _, _) = placements[j];
            if previous_entity == entity && current_positions[j] >= insert_position {
                current_positions[j] += 1;
            }
        }
        current_positions.push(insert_position);
    }
    current_positions
}

pub(crate) fn ruin_is_doable<S, A, D>(access: &A, sources: &RuinSources, score_director: &D) -> bool
where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    if sources.is_empty() || sources.iter().all(|(_, indices)| indices.is_empty()) {
        return false;
    }
    let solution = score_director.working_solution();
    if sources.iter().any(|(entity, _)| {
        crate::pinning::entity_is_pinned(score_director, access.descriptor_index(), *entity)
    }) {
        return false;
    }
    let entity_count = access.entity_count(solution);
    if !access.has_owner_binding() {
        return (0..entity_count).any(|destination| {
            !crate::pinning::entity_is_pinned(
                score_director,
                access.descriptor_index(),
                destination,
            )
        }) && sources.iter().all(|(entity, indices)| {
            let len = access.list_len(solution, *entity);
            indices.iter().all(|&index| index < len)
        });
    }

    sources.iter().all(|(entity, indices)| {
        let len = access.list_len(solution, *entity);
        indices.iter().all(|&index| {
            if index >= len {
                return false;
            }
            let Some(element) = access.list_get(solution, *entity, index) else {
                return false;
            };
            let restriction = access.owner_restriction(solution, entity_count, &element);
            (0..entity_count).any(|destination| {
                restriction.allows(destination)
                    && !crate::pinning::entity_is_pinned(
                        score_director,
                        access.descriptor_index(),
                        destination,
                    )
            })
        })
    })
}

pub(crate) fn ruin_do_move<S, A, D>(
    access: &A,
    sources: &RuinSources,
    skip_empty_destinations: bool,
    score_director: &mut D,
) -> RuinUndo
where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let mut removed = SmallVec::<[(usize, usize, A::Element); 8]>::new();
    for (source, indices) in sources {
        score_director.before_variable_changed(descriptor_index, *source);
        let mut source_removed = SmallVec::<[(usize, A::Element); 8]>::new();
        for &index in indices.iter().rev() {
            source_removed.push((
                index,
                access.list_remove(score_director.working_solution_mut(), *source, index),
            ));
        }
        source_removed.reverse();
        removed.extend(
            source_removed
                .into_iter()
                .map(|(index, value)| (*source, index, value)),
        );
        score_director.after_variable_changed(descriptor_index, *source);
    }

    let mut placements = RuinUndo::new();
    // `removed` owns every displaced element. The placement loop takes one
    // element out into an Option slot at a time, circulates that owned value
    // through every trial insert/remove, and transfers the winning insert by
    // ownership. No element is ever cloned; the original-order entry keeps
    // its identity for the undo bookkeeping.
    let mut remaining = removed
        .into_iter()
        .enumerate()
        .map(|(index, (source, original_position, value))| {
            (index, source, original_position, Some(value))
        })
        .collect::<SmallVec<[(usize, usize, usize, Option<A::Element>); 8]>>();
    let entity_count = access.entity_count(score_director.working_solution());

    while remaining.iter().any(|entry| entry.3.is_some()) {
        let mut precedence_graph =
            access.recreate_precedence_graph(score_director.working_solution());
        let mut best_choice: Option<(usize, usize, usize, S::Score)> = None;

        for remaining_index in 0..remaining.len() {
            let Some(mut element) = remaining[remaining_index].3.take() else {
                continue;
            };
            let restriction =
                access.owner_restriction(score_director.working_solution(), entity_count, &element);
            for destination_entity in 0..entity_count {
                if !restriction.allows(destination_entity)
                    || crate::pinning::entity_is_pinned(
                        score_director,
                        descriptor_index,
                        destination_entity,
                    )
                {
                    continue;
                }
                let destination_len =
                    access.list_len(score_director.working_solution(), destination_entity);
                if skip_empty_destinations && !access.has_owner_binding() && destination_len == 0 {
                    continue;
                }
                for destination_position in 0..=destination_len {
                    if precedence_graph.as_mut().is_some_and(|(elements, graph)| {
                        let Some(element_node) = node_index(elements, &element) else {
                            return false;
                        };
                        let previous = (destination_position > 0)
                            .then(|| {
                                access.list_get(
                                    score_director.working_solution(),
                                    destination_entity,
                                    destination_position - 1,
                                )
                            })
                            .flatten();
                        let next = (destination_position < destination_len)
                            .then(|| {
                                access.list_get(
                                    score_director.working_solution(),
                                    destination_entity,
                                    destination_position,
                                )
                            })
                            .flatten();
                        graph.insertion_introduces_cycle(
                            previous
                                .as_ref()
                                .and_then(|value| node_index(elements, value)),
                            element_node,
                            next.as_ref().and_then(|value| node_index(elements, value)),
                        )
                    }) {
                        continue;
                    }

                    score_director.before_variable_changed(descriptor_index, destination_entity);
                    access.list_insert(
                        score_director.working_solution_mut(),
                        destination_entity,
                        destination_position,
                        element,
                    );
                    score_director.after_variable_changed(descriptor_index, destination_entity);

                    let candidate_score = score_director.calculate_score();
                    if best_choice.is_none_or(|(_, _, _, best_score)| candidate_score > best_score)
                    {
                        best_choice = Some((
                            remaining_index,
                            destination_entity,
                            destination_position,
                            candidate_score,
                        ));
                    }

                    score_director.before_variable_changed(descriptor_index, destination_entity);
                    let recovered = access.list_remove(
                        score_director.working_solution_mut(),
                        destination_entity,
                        destination_position,
                    );
                    score_director.after_variable_changed(descriptor_index, destination_entity);
                    element = recovered;
                }
            }
            remaining[remaining_index].3 = Some(element);
        }

        let Some((best_index, entity, position, _)) = best_choice else {
            restore_remaining_elements(access, &placements, &mut remaining, score_director);
            return RuinUndo::new();
        };
        let (removed_index, _, _, element) = &mut remaining[best_index];
        let Some(transferred) = element.take() else {
            unreachable!("the winning element must remain in its Option slot");
        };
        let removed_index = *removed_index;
        score_director.before_variable_changed(descriptor_index, entity);
        access.list_insert(
            score_director.working_solution_mut(),
            entity,
            position,
            transferred,
        );
        score_director.after_variable_changed(descriptor_index, entity);
        placements.push((entity, position, removed_index));
    }
    placements
}

pub(crate) fn ruin_undo_move<S, A, D>(
    access: &A,
    sources: &RuinSources,
    placements: RuinUndo,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let mut current_positions = final_positions_after_ordered_insertions(&placements);
    let mut values = SmallVec::<[(usize, usize, A::Element); 8]>::with_capacity(placements.len());
    let descriptor_index = access.descriptor_index();
    for i in (0..placements.len()).rev() {
        let (entity, _, removed_index) = placements[i];
        let actual_position = current_positions[i];
        score_director.before_variable_changed(descriptor_index, entity);
        let value = access.list_remove(
            score_director.working_solution_mut(),
            entity,
            actual_position,
        );
        let (source, original_position) = removed_source_entry(sources, removed_index)
            .expect("list ruin undo placement index must map to an original source entry");
        values.push((source, original_position, value));
        score_director.after_variable_changed(descriptor_index, entity);

        for j in 0..i {
            let (other_entity, _, _) = placements[j];
            if other_entity == entity && current_positions[j] > actual_position {
                current_positions[j] -= 1;
            }
        }
    }
    restore_values(access, values, score_director);
}

pub(crate) fn ruin_tabu_signature<S, A, D>(
    access: &A,
    sources: &RuinSources,
    entity_indices: &[usize],
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let mut value_ids = SmallVec::<[u64; 2]>::new();
    for (entity, indices) in sources {
        for &index in indices {
            let value = access.list_get(score_director.working_solution(), *entity, index);
            value_ids.push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
        }
    }
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = value_ids
        .iter()
        .copied()
        .map(|value| scope.value_token(value))
        .collect();
    let mut move_id = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        encode_usize(sources.len()),
        encode_usize(ruin_count(sources))
    ];
    for (entity, indices) in sources {
        move_id.push(encode_usize(*entity));
        move_id.push(encode_usize(indices.len()));
        move_id.extend(indices.iter().map(|&index| encode_usize(index)));
    }
    move_id.extend(value_ids.iter().copied());
    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens(
            entity_indices
                .iter()
                .copied()
                .map(encode_usize)
                .map(|entity| scope.entity_token(entity)),
        )
        .with_destination_value_tokens(destination_value_tokens)
}

fn restore_remaining_elements<S, A, D>(
    access: &A,
    placements: &RuinUndo,
    remaining: &mut SmallVec<[(usize, usize, usize, Option<A::Element>); 8]>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    let mut current_positions = final_positions_after_ordered_insertions(placements);
    let descriptor_index = access.descriptor_index();
    for i in (0..placements.len()).rev() {
        let (entity, _, _) = placements[i];
        let actual_position = current_positions[i];
        score_director.before_variable_changed(descriptor_index, entity);
        let _ = access.list_remove(
            score_director.working_solution_mut(),
            entity,
            actual_position,
        );
        score_director.after_variable_changed(descriptor_index, entity);
        for j in 0..i {
            let (other_entity, _, _) = placements[j];
            if other_entity == entity && current_positions[j] > actual_position {
                current_positions[j] -= 1;
            }
        }
    }
    let values = remaining
        .iter_mut()
        .filter_map(|(_, source, original_position, element)| {
            element
                .take()
                .map(|value| (*source, *original_position, value))
        })
        .collect::<SmallVec<[(usize, usize, A::Element); 8]>>();
    restore_values(access, values, score_director);
}

fn restore_values<S, A, D>(
    access: &A,
    mut values: SmallVec<[(usize, usize, A::Element); 8]>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListRuinAccess<S>,
    D: Director<S>,
{
    values.sort_by_key(|(entity, original_position, _)| (*entity, *original_position));
    let descriptor_index = access.descriptor_index();
    let mut current_entity = None;
    for (entity, original_position, value) in values {
        if current_entity != Some(entity) {
            if let Some(previous_entity) = current_entity {
                score_director.after_variable_changed(descriptor_index, previous_entity);
            }
            score_director.before_variable_changed(descriptor_index, entity);
            current_entity = Some(entity);
        }
        access.list_insert(
            score_director.working_solution_mut(),
            entity,
            original_position,
            value,
        );
    }
    if let Some(entity) = current_entity {
        score_director.after_variable_changed(descriptor_index, entity);
    }
}

fn removed_source_entry(sources: &RuinSources, target_index: usize) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    for (entity, indices) in sources {
        if target_index < offset + indices.len() {
            return Some((*entity, indices[target_index - offset]));
        }
        offset += indices.len();
    }
    None
}
