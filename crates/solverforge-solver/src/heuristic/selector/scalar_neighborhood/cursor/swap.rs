use solverforge_core::domain::PlanningSolution;

use crate::builder::{RuntimeScalarSlot, ValueSource};
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::nearby_support::{NearbyTopK, RankedNearbyCandidate};

use super::super::spec::RuntimeScalarRecipe;
use super::slot_identity;

const SWAP_LEFT_SALT: u64 = 0x5A09_5CA1_AA00_0001;
const SWAP_RIGHT_SALT: u64 = 0x5A09_5CA1_AA00_0002;
const NEARBY_SWAP_ENTITY_START_SALT: u64 = 0x5A09_5CA1_AAAA_0001;
const NEARBY_SWAP_ENTITY_STRIDE_SALT: u64 = 0x5A09_5CA1_AAAA_0002;
const NEARBY_SWAP_TARGET_SALT: u64 = 0x5A09_5CA1_AAAA_0003;
const ENTITY_STRIDE_MIX: u64 = 0xD1B5_4A32_D192_ED03;

/// Complete scalar exchange enumeration for one frozen slot.
///
/// Native slots eagerly snapshot their values and legal rows, matching the
/// historic selector's source timing. Dynamic slots retain their existing
/// direct legality calls while traversing the same candidate ordering.
pub(super) struct SwapCursor<S> {
    slot: RuntimeScalarSlot<S>,
    solution: S,
    snapshot: SwapSnapshot,
    context: MoveStreamContext,
    left_offset: usize,
    right_offset: usize,
}

enum SwapSnapshot {
    Static {
        current_values: Vec<Option<usize>>,
        legal_values: Vec<Vec<usize>>,
        empty_value_source: bool,
    },
    Dynamic,
}

impl<S> SwapCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(slot: RuntimeScalarSlot<S>, solution: S, context: MoveStreamContext) -> Self {
        let snapshot = match &slot {
            RuntimeScalarSlot::Static(static_slot) => {
                let entity_count = slot.entity_count(&solution);
                let current_values = (0..entity_count)
                    .map(|entity_index| slot.current_value(&solution, entity_index))
                    .collect();
                let legal_values = (0..entity_count)
                    .map(|entity_index| static_slot.values_for_entity(&solution, entity_index))
                    .collect();
                SwapSnapshot::Static {
                    current_values,
                    legal_values,
                    empty_value_source: matches!(static_slot.value_source, ValueSource::Empty),
                }
            }
            RuntimeScalarSlot::Dynamic(_) => SwapSnapshot::Dynamic,
        };
        Self {
            slot,
            solution,
            snapshot,
            context,
            left_offset: 0,
            right_offset: 0,
        }
    }

    fn entity_count(&self) -> usize {
        match &self.snapshot {
            SwapSnapshot::Static { current_values, .. } => current_values.len(),
            SwapSnapshot::Dynamic => self.slot.entity_count(&self.solution),
        }
    }

    fn ordered_left_entity(&self, count: usize, offset: usize, salt: u64) -> usize {
        if count <= 1 {
            return 0;
        }
        self.context
            .selection_index_without_replacement(offset, count, salt ^ ENTITY_STRIDE_MIX)
    }

    fn ordered_right_entity(&self, count: usize, offset: usize, salt: u64) -> usize {
        if count <= 1 {
            return 0;
        }
        self.context
            .selection_index(offset, count, salt ^ ENTITY_STRIDE_MIX)
    }

    fn current_value(&self, entity_index: usize) -> Option<usize> {
        match &self.snapshot {
            SwapSnapshot::Static { current_values, .. } => current_values[entity_index],
            SwapSnapshot::Dynamic => self.slot.current_value(&self.solution, entity_index),
        }
    }

    fn destination_is_legal(&self, entity_index: usize, value: Option<usize>) -> bool {
        match &self.snapshot {
            SwapSnapshot::Static {
                legal_values,
                empty_value_source,
                ..
            } => {
                if *empty_value_source {
                    value.is_some()
                } else {
                    match value {
                        Some(value) => legal_values[entity_index].contains(&value),
                        None => self.slot.allows_unassigned(),
                    }
                }
            }
            SwapSnapshot::Dynamic => {
                self.slot
                    .swap_destination_is_legal(&self.solution, entity_index, value)
            }
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        let entity_count = self.entity_count();
        let identity = slot_identity(&self.slot);
        while self.left_offset < entity_count {
            let left_entity_index =
                self.ordered_left_entity(entity_count, self.left_offset, SWAP_LEFT_SALT ^ identity);
            while self.right_offset < entity_count {
                let right_entity_index = self.ordered_right_entity(
                    entity_count,
                    self.right_offset,
                    SWAP_RIGHT_SALT ^ left_entity_index as u64 ^ self.slot.variable_index() as u64,
                );
                self.right_offset += 1;
                if left_entity_index >= right_entity_index {
                    continue;
                }
                let left_value = self.current_value(left_entity_index);
                let right_value = self.current_value(right_entity_index);
                if left_value == right_value
                    || !self.destination_is_legal(left_entity_index, right_value)
                    || !self.destination_is_legal(right_entity_index, left_value)
                {
                    continue;
                }
                return Some(RuntimeScalarRecipe::Swap {
                    slot: self.slot.clone(),
                    left_entity_index,
                    right_entity_index,
                });
            }
            self.left_offset += 1;
            self.right_offset = 0;
        }
        None
    }
}

struct NearbySwapRow {
    left_entity_index: usize,
    right_entities: Vec<usize>,
}

enum NearbySwapMode<S> {
    /// Native nearby callbacks historically ran for every row at cursor open.
    Eager {
        rows: Vec<NearbySwapRow>,
        row_offset: usize,
        right_offset: usize,
    },
    /// Dynamic nearby callbacks remain lazy by public contract.
    Lazy {
        solution: S,
        context: MoveStreamContext,
        entity_count: usize,
        left_offset: usize,
        right_entities: Vec<usize>,
        right_offset: usize,
        source_loaded: bool,
    },
}

/// Nearby scalar exchange with explicit native-eager/dynamic-lazy source
/// timing. Structural source availability is validated before this cursor is
/// constructed; an empty row is simply an empty row, never an all-entity
/// substitute.
pub(super) struct NearbySwapCursor<S> {
    slot: RuntimeScalarSlot<S>,
    max_nearby: usize,
    mode: NearbySwapMode<S>,
}

impl<S> NearbySwapCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: S,
        context: MoveStreamContext,
        max_nearby: usize,
    ) -> Self {
        let mode = if slot.is_dynamic() {
            NearbySwapMode::Lazy {
                entity_count: slot.entity_count(&solution),
                solution,
                context,
                left_offset: 0,
                right_entities: Vec::new(),
                right_offset: 0,
                source_loaded: false,
            }
        } else {
            let rows = eager_nearby_rows(&slot, &solution, context, max_nearby);
            NearbySwapMode::Eager {
                rows,
                row_offset: 0,
                right_offset: 0,
            }
        };
        Self {
            slot,
            max_nearby,
            mode,
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        match &mut self.mode {
            NearbySwapMode::Eager {
                rows,
                row_offset,
                right_offset,
            } => loop {
                let row = rows.get(*row_offset)?;
                if let Some(&right_entity_index) = row.right_entities.get(*right_offset) {
                    *right_offset += 1;
                    return Some(RuntimeScalarRecipe::Swap {
                        slot: self.slot.clone(),
                        left_entity_index: row.left_entity_index,
                        right_entity_index,
                    });
                }
                *row_offset += 1;
                *right_offset = 0;
            },
            NearbySwapMode::Lazy {
                solution,
                context,
                entity_count,
                left_offset,
                right_entities,
                right_offset,
                source_loaded,
            } => {
                while *left_offset < *entity_count {
                    let left_entity_index = ordered_entity(
                        *entity_count,
                        *left_offset,
                        *context,
                        NEARBY_SWAP_ENTITY_START_SALT,
                        NEARBY_SWAP_ENTITY_STRIDE_SALT,
                        slot_identity(&self.slot),
                    );
                    if !*source_loaded {
                        *right_entities = rank_nearby_entities(
                            &self.slot,
                            solution,
                            left_entity_index,
                            *entity_count,
                            self.max_nearby,
                            NearbyOrientation::DynamicDirectional,
                            *context,
                        );
                        *right_offset = 0;
                        *source_loaded = true;
                    }
                    if let Some(&right_entity_index) = right_entities.get(*right_offset) {
                        *right_offset += 1;
                        return Some(RuntimeScalarRecipe::Swap {
                            slot: self.slot.clone(),
                            left_entity_index,
                            right_entity_index,
                        });
                    }
                    *left_offset += 1;
                    right_entities.clear();
                    *right_offset = 0;
                    *source_loaded = false;
                }
                None
            }
        }
    }
}

fn eager_nearby_rows<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    context: MoveStreamContext,
    max_nearby: usize,
) -> Vec<NearbySwapRow>
where
    S: PlanningSolution,
{
    let entity_count = slot.entity_count(solution);
    (0..entity_count)
        .map(|left_offset| {
            let left_entity_index = ordered_entity(
                entity_count,
                left_offset,
                context,
                NEARBY_SWAP_ENTITY_START_SALT,
                NEARBY_SWAP_ENTITY_STRIDE_SALT,
                slot_identity(slot),
            );
            NearbySwapRow {
                left_entity_index,
                right_entities: rank_nearby_entities(
                    slot,
                    solution,
                    left_entity_index,
                    entity_count,
                    max_nearby,
                    NearbyOrientation::StaticCanonical,
                    context,
                ),
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
enum NearbyOrientation {
    StaticCanonical,
    DynamicDirectional,
}

fn rank_nearby_entities<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    left_entity_index: usize,
    entity_count: usize,
    max_nearby: usize,
    orientation: NearbyOrientation,
    context: MoveStreamContext,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    if max_nearby == 0 {
        return Vec::new();
    }
    let left_value = slot.current_value(solution, left_entity_index);
    let mut selected = NearbyTopK::new(max_nearby);
    let mut order = 0usize;
    let mut visit = |right_entity_index| {
        let source_order = order;
        order += 1;
        let allowed_orientation = match orientation {
            NearbyOrientation::StaticCanonical => right_entity_index > left_entity_index,
            NearbyOrientation::DynamicDirectional => right_entity_index != left_entity_index,
        };
        if !allowed_orientation || right_entity_index >= entity_count {
            return;
        }
        let right_value = slot.current_value(solution, right_entity_index);
        if left_value == right_value
            || !slot.swap_destination_is_legal(solution, left_entity_index, right_value)
            || !slot.swap_destination_is_legal(solution, right_entity_index, left_value)
        {
            return;
        }
        selected.push(RankedNearbyCandidate {
            candidate: right_entity_index,
            distance: slot
                .nearby_entity_distance(solution, left_entity_index, right_entity_index)
                .unwrap_or(source_order as f64),
            order: source_order,
        });
    };
    let row_supplied =
        slot.visit_nearby_entity_candidates(solution, left_entity_index, entity_count, &mut visit);
    if !row_supplied {
        for right_entity_index in 0..entity_count {
            visit(right_entity_index);
        }
    }
    let mut entities = selected.finish();
    context.apply_selection_order(
        &mut entities,
        NEARBY_SWAP_TARGET_SALT ^ left_entity_index as u64 ^ slot_identity(slot),
    );
    entities
}

fn ordered_entity(
    entity_count: usize,
    entity_offset: usize,
    context: MoveStreamContext,
    start_salt: u64,
    stride_salt: u64,
    identity: u64,
) -> usize {
    if entity_count <= 1 {
        return 0;
    }
    let start = context.start_offset(entity_count, start_salt ^ identity);
    let stride = context.stride(entity_count, stride_salt ^ identity);
    (start + entity_offset * stride) % entity_count
}
