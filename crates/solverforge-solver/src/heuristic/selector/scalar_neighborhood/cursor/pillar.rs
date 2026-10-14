use solverforge_core::domain::PlanningSolution;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::pillar::{Pillar, SubPillarConfig};
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

#[derive(Clone)]
struct PillarChangeInput {
    entity_indices: Vec<usize>,
    shared_value: usize,
}

/// Streams pillar-change inputs. Cursor open collects the pure group
/// snapshot and fixes the seeded input order; a reached input visits its
/// legal values into reusable scratch at entry, shuffles them with the same
/// salt schedule, and yields owned recipes. Enumerated inputs, values, and
/// orders are identical to the historical eager collection.
pub(super) struct PillarChangeCursor<S> {
    slot: RuntimeScalarSlot<S>,
    inputs: Vec<PillarChangeInput>,
    current_values: Vec<usize>,
    value_candidate_limit: Option<usize>,
    context: MoveStreamContext,
    identity: u64,
    input_offset: usize,
    value_offset: usize,
}

impl<S> PillarChangeCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: &S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        let identity = super::slot_identity(&slot);
        let inputs = collect_groups(
            &slot,
            solution,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        )
        .into_iter()
        .map(|group| PillarChangeInput {
            entity_indices: group
                .pillar
                .iter()
                .map(|entity| entity.entity_index)
                .collect(),
            shared_value: group.shared_value,
        })
        .collect::<Vec<_>>();
        Self {
            slot,
            inputs,
            current_values: Vec::new(),
            value_candidate_limit,
            context,
            identity,
            input_offset: 0,
            value_offset: 0,
        }
    }

    pub(super) fn next_recipe(&mut self, solution: &S) -> Option<RuntimeScalarRecipe<S>> {
        while self.input_offset < self.inputs.len() {
            if self.value_offset == 0 && self.current_values.is_empty() {
                let input = &self.inputs[self.input_offset];
                let values =
                    pillar_change_values(&self.slot, solution, input, self.value_candidate_limit);
                let offset = self.input_offset;
                let mut values = values;
                self.context.apply_selection_order(
                    &mut values,
                    0x5CA1_A2C0_711A_0002
                        ^ self.identity
                        ^ (offset as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                );
                if values.is_empty() {
                    self.input_offset += 1;
                    continue;
                }
                self.current_values = values;
            }
            let entity_indices = self.inputs[self.input_offset].entity_indices.clone();
            let Some(&value) = self.current_values.get(self.value_offset) else {
                self.input_offset += 1;
                self.value_offset = 0;
                self.current_values.clear();
                continue;
            };
            self.value_offset += 1;
            return Some(RuntimeScalarRecipe::PillarChange {
                slot: self.slot.clone(),
                entity_indices,
                to_value: Some(value),
            });
        }
        None
    }
}

fn pillar_change_values<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    input: &PillarChangeInput,
    value_candidate_limit: Option<usize>,
) -> Vec<usize> {
    let mut scratch = Vec::new();
    intersect_legal_values_for_pillar(
        &Pillar::new(
            input
                .entity_indices
                .iter()
                .map(|&entity_index| EntityReference::new(slot.descriptor_index(), entity_index))
                .collect(),
        ),
        |entity_index| {
            scratch.clear();
            slot.visit_candidate_values(
                solution,
                entity_index,
                value_candidate_limit,
                &mut |value| scratch.push(value),
            );
            scratch.clone()
        },
    )
    .into_iter()
    .filter(|value| *value != input.shared_value)
    .collect()
}

/// Streams compatible pillar pairs. Cursor open collects the pure group
/// snapshot, materializes the pair-order grid (small owned `(left, right)`
/// index tuples), and applies the same seeded order shuffle to that grid;
/// the compatibility callbacks then run at pull time in shuffled grid
/// order. Yielded pairs are identical to the eager walk; the per-open
/// collector of full index vecs is gone.
pub(super) struct PillarSwapCursor<S> {
    slot: RuntimeScalarSlot<S>,
    groups: Vec<PillarGroup<usize>>,
    pair_grid: Vec<(usize, usize)>,
    grid_offset: usize,
}

impl<S> PillarSwapCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: &S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
    ) -> Self {
        let identity = super::slot_identity(&slot);
        let groups = collect_groups(
            &slot,
            solution,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        );
        let mut pair_grid = (0..groups.len())
            .flat_map(|left_offset| {
                (left_offset + 1..groups.len()).map(move |right_offset| (left_offset, right_offset))
            })
            .collect::<Vec<_>>();
        context.apply_selection_order(&mut pair_grid, 0x5CA1_A2C0_711A_0003 ^ identity);
        Self {
            slot,
            groups,
            pair_grid,
            grid_offset: 0,
        }
    }

    pub(super) fn next_recipe(&mut self, solution: &S) -> Option<RuntimeScalarRecipe<S>> {
        while self.grid_offset < self.pair_grid.len() {
            let (left_offset, right_offset) = self.pair_grid[self.grid_offset];
            self.grid_offset += 1;
            let left = &self.groups[left_offset];
            let right = &self.groups[right_offset];
            if !groups_are_swap_compatible(&self.slot, solution, left, right) {
                continue;
            }
            let left_indices = left
                .pillar
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            let right_indices = right
                .pillar
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            return Some(RuntimeScalarRecipe::PillarSwap {
                slot: self.slot.clone(),
                left_indices,
                right_indices,
            });
        }
        None
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
