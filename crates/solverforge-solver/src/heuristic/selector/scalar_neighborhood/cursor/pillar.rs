use solverforge_core::domain::PlanningSolution;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::pillar::SubPillarConfig;
use crate::heuristic::selector::pillar_support::{
    collect_pillar_groups, intersect_legal_values_for_pillar, PillarGroup,
};

use super::super::spec::RuntimeScalarRecipe;

fn sub_pillar_config(minimum_size: usize, maximum_size: usize) -> SubPillarConfig {
    if minimum_size == 0 || maximum_size == 0 {
        SubPillarConfig::none()
    } else {
        let minimum_size = minimum_size.max(2);
        SubPillarConfig {
            enabled: true,
            minimum_size,
            maximum_size: maximum_size.max(minimum_size),
        }
    }
}

fn collect_groups<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    minimum_size: usize,
    maximum_size: usize,
) -> Vec<PillarGroup<usize>>
where
    S: PlanningSolution,
{
    let descriptor_index = slot.descriptor_index();
    collect_pillar_groups(
        (0..slot.entity_count(solution)).map(|entity_index| {
            (
                EntityReference::new(descriptor_index, entity_index),
                slot.current_value(solution, entity_index),
            )
        }),
        &sub_pillar_config(minimum_size, maximum_size),
    )
}

fn candidate_values<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    entity_index: usize,
    value_candidate_limit: Option<usize>,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    let mut values = Vec::new();
    slot.visit_candidate_values(
        solution,
        entity_index,
        value_candidate_limit,
        &mut |value| values.push(value),
    );
    values
}

#[derive(Clone)]
struct PillarChangeInput {
    entity_indices: Vec<usize>,
    values: Vec<usize>,
}

/// Materializes the historic pillar-change source phase at cursor open, then
/// streams owned recipes. The eager preparation is deliberate: native scalar
/// value sources were previously visited before the first candidate pull.
pub(super) struct PillarChangeCursor<S> {
    slot: RuntimeScalarSlot<S>,
    inputs: Vec<PillarChangeInput>,
    input_offset: usize,
    value_offset: usize,
}

impl<S> PillarChangeCursor<S>
where
    S: PlanningSolution,
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        let mut inputs: Vec<PillarChangeInput> = collect_groups(
            &slot,
            &solution,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        )
        .into_iter()
        .map(|group| {
            let values = intersect_legal_values_for_pillar(&group.pillar, |entity_index| {
                candidate_values(&slot, &solution, entity_index, value_candidate_limit)
            })
            .into_iter()
            .filter(|value| *value != group.shared_value)
            .collect();
            PillarChangeInput {
                entity_indices: group
                    .pillar
                    .iter()
                    .map(|entity| entity.entity_index)
                    .collect(),
                values,
            }
        })
        .collect();
        context.apply_selection_order(
            &mut inputs,
            0x5CA1_A2C0_711A_0001 ^ super::slot_identity(&slot),
        );
        for (offset, input) in inputs.iter_mut().enumerate() {
            context.apply_selection_order(
                &mut input.values,
                0x5CA1_A2C0_711A_0002
                    ^ super::slot_identity(&slot)
                    ^ (offset as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
        }
        Self {
            slot,
            inputs,
            input_offset: 0,
            value_offset: 0,
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        while let Some(input) = self.inputs.get(self.input_offset) {
            if let Some(&value) = input.values.get(self.value_offset) {
                self.value_offset += 1;
                return Some(RuntimeScalarRecipe::PillarChange {
                    slot: self.slot.clone(),
                    entity_indices: input.entity_indices.clone(),
                    to_value: Some(value),
                });
            }
            self.input_offset += 1;
            self.value_offset = 0;
        }
        None
    }
}

#[derive(Clone)]
struct PillarSwapInput {
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
}

/// Materializes compatible pillar pairs at open, exactly as the established
/// static selector did. A candidate contains only owned indices, so it cannot
/// retain a callback row or a cursor borrow.
pub(super) struct PillarSwapCursor<S> {
    slot: RuntimeScalarSlot<S>,
    inputs: std::vec::IntoIter<PillarSwapInput>,
}

impl<S> PillarSwapCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
    ) -> Self {
        let groups = collect_groups(
            &slot,
            &solution,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        );
        let mut inputs = Vec::new();
        for left_offset in 0..groups.len() {
            let left = &groups[left_offset];
            for right in groups.iter().skip(left_offset + 1) {
                if !groups_are_swap_compatible(&slot, &solution, left, right) {
                    continue;
                }
                inputs.push(PillarSwapInput {
                    left_indices: left
                        .pillar
                        .iter()
                        .map(|entity| entity.entity_index)
                        .collect(),
                    right_indices: right
                        .pillar
                        .iter()
                        .map(|entity| entity.entity_index)
                        .collect(),
                });
            }
        }
        context.apply_selection_order(
            &mut inputs,
            0x5CA1_A2C0_711A_0003 ^ super::slot_identity(&slot),
        );
        Self {
            slot,
            inputs: inputs.into_iter(),
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        let input = self.inputs.next()?;
        Some(RuntimeScalarRecipe::PillarSwap {
            slot: self.slot.clone(),
            left_indices: input.left_indices,
            right_indices: input.right_indices,
        })
    }
}

fn groups_are_swap_compatible<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    left: &PillarGroup<usize>,
    right: &PillarGroup<usize>,
) -> bool
where
    S: PlanningSolution,
{
    left.pillar.iter().all(|entity| {
        slot.swap_destination_is_legal(solution, entity.entity_index, Some(right.shared_value))
    }) && right.pillar.iter().all(|entity| {
        slot.swap_destination_is_legal(solution, entity.entity_index, Some(left.shared_value))
    })
}
