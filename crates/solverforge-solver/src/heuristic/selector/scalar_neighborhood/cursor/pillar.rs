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

struct PillarChangeInput {
    entity_indices: Vec<usize>,
    shared_value: usize,
}

/// Streams pillar-change inputs. Cursor open collects the pure group
/// snapshot and fixes the seeded input order; a reached input visits its
/// legal values into reusable scratch at entry, shuffles them with the same
/// salt schedule, and yields owned recipes. Enumerated inputs, values, and
/// orders are identical to the historical eager collection.
pub(super) struct PillarChangeCursor<'a, S> {
    slot: &'a RuntimeScalarSlot<S>,
    inputs: Vec<PillarChangeInput>,
    current_values: Vec<usize>,
    legal_value_scratch: Vec<usize>,
    value_candidate_limit: Option<usize>,
    context: MoveStreamContext,
    identity: u64,
    input_offset: usize,
    value_offset: usize,
}

impl<'a, S> PillarChangeCursor<'a, S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: &'a RuntimeScalarSlot<S>,
        solution: &S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        let identity = super::slot_identity(slot);
        let inputs = collect_groups(
            slot,
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
            legal_value_scratch: Vec::new(),
            value_candidate_limit,
            context,
            identity,
            input_offset: 0,
            value_offset: 0,
        }
    }

    pub(super) fn next_recipe(&mut self, solution: &S) -> Option<RuntimeScalarRecipe<'a, S>> {
        while self.input_offset < self.inputs.len() {
            if self.value_offset == 0 && self.current_values.is_empty() {
                let input = &self.inputs[self.input_offset];
                intersect_legal_values_for_pillar(
                    &input.entity_indices,
                    |entity, values| {
                        self.slot.visit_candidate_values(
                            solution,
                            entity,
                            self.value_candidate_limit,
                            &mut |value| values.push(value),
                        );
                    },
                    &mut self.current_values,
                    &mut self.legal_value_scratch,
                );
                self.current_values
                    .retain(|value| *value != input.shared_value);
                let offset = self.input_offset;
                self.context.apply_selection_order(
                    &mut self.current_values,
                    0x5CA1_A2C0_711A_0002
                        ^ self.identity
                        ^ (offset as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                );
                if self.current_values.is_empty() {
                    self.input_offset += 1;
                    continue;
                }
            }
            let Some(&value) = self.current_values.get(self.value_offset) else {
                self.input_offset += 1;
                self.value_offset = 0;
                self.current_values.clear();
                continue;
            };
            self.value_offset += 1;
            return Some(RuntimeScalarRecipe::PillarChange {
                slot: self.slot,
                entity_indices: self.inputs[self.input_offset].entity_indices.clone(),
                to_value: Some(value),
            });
        }
        None
    }
}

/// Streams compatible pillar pairs. Cursor open collects the pure group
/// snapshot, materializes the pair-order grid (small owned `(left, right)`
/// index tuples), and applies the same seeded order shuffle to that grid;
/// the compatibility callbacks then run at pull time in shuffled grid
/// order. Yielded pairs are identical to the eager walk; the per-open
/// collector of full index vecs is gone.
pub(super) struct PillarSwapCursor<'a, S> {
    slot: &'a RuntimeScalarSlot<S>,
    groups: Vec<PillarGroup<usize>>,
    pair_grid: Vec<(usize, usize)>,
    grid_offset: usize,
}

impl<'a, S> PillarSwapCursor<'a, S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: &'a RuntimeScalarSlot<S>,
        solution: &S,
        context: MoveStreamContext,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
    ) -> Self {
        let identity = super::slot_identity(slot);
        let groups = collect_groups(
            slot,
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

    pub(super) fn next_recipe(&mut self, solution: &S) -> Option<RuntimeScalarRecipe<'a, S>> {
        while self.grid_offset < self.pair_grid.len() {
            let (left_offset, right_offset) = self.pair_grid[self.grid_offset];
            self.grid_offset += 1;
            let left = &self.groups[left_offset];
            let right = &self.groups[right_offset];
            if !groups_are_swap_compatible(self.slot, solution, left, right) {
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
                slot: self.slot,
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
