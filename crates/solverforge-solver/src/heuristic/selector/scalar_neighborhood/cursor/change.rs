use solverforge_core::domain::PlanningSolution;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::nearby_support::{NearbyTopK, RankedNearbyCandidate};

use super::super::spec::RuntimeScalarRecipe;
use super::slot_identity;

const STATIC_CHANGE_VALUE_SALT: u64 = 0xC4A4_6E00_0000_0000;
const STATIC_CHANGE_ENTITY_SALT: u64 = 0xC4A4_6E00_0000_0001;
const DYNAMIC_CHANGE_VALUE_SALT: u64 = 0xD94E_5CA1_0000_0000;
const DYNAMIC_CHANGE_ENTITY_SALT: u64 = 0xD94E_5CA1_0000_0001;
const NEARBY_CHANGE_ENTITY_START_SALT: u64 = 0xC4A4_6E00_AAAA_0001;
const NEARBY_CHANGE_ENTITY_STRIDE_SALT: u64 = 0xC4A4_6E00_AAAA_0002;
const NEARBY_CHANGE_VALUE_SALT: u64 = 0xC4A4_6E00_AAAA_0003;

struct ChangeRow {
    entity_index: usize,
    values: Vec<usize>,
    current_assigned: bool,
}

/// Ordinary change keeps historic eager source collection for both native and
/// dynamic direct selectors. Its recipe stream is still one RuntimeScalarSlot
/// implementation; only the established seed salts differ by physical carrier.
pub(super) struct ChangeCursor<S> {
    slot: RuntimeScalarSlot<S>,
    rows: Vec<ChangeRow>,
    row_offset: usize,
    value_offset: usize,
    unassigned_pending: bool,
}

impl<S> ChangeCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: S,
        context: MoveStreamContext,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        let dynamic = slot.is_dynamic();
        let identity = slot_identity(&slot);
        let value_salt = if dynamic {
            DYNAMIC_CHANGE_VALUE_SALT
        } else {
            STATIC_CHANGE_VALUE_SALT
        };
        let entity_salt = if dynamic {
            DYNAMIC_CHANGE_ENTITY_SALT
        } else {
            STATIC_CHANGE_ENTITY_SALT
        };
        let entity_count = slot.entity_count(&solution);
        let rows = (0..entity_count)
            .map(|entity_offset| {
                let entity_index = context.selection_index_without_replacement(
                    entity_offset,
                    entity_count,
                    entity_salt ^ identity,
                );
                let mut canonical_values = Vec::new();
                slot.visit_candidate_values(
                    &solution,
                    entity_index,
                    value_candidate_limit,
                    &mut |value| canonical_values.push(value),
                );
                let value_count = canonical_values.len();
                let values = (0..value_count)
                    .map(|value_offset| {
                        canonical_values[context.selection_index(
                            value_offset,
                            value_count,
                            value_salt ^ entity_index as u64 ^ identity,
                        )]
                    })
                    .collect();
                ChangeRow {
                    entity_index,
                    values,
                    current_assigned: slot.current_value(&solution, entity_index).is_some(),
                }
            })
            .collect::<Vec<_>>();
        Self {
            slot,
            rows,
            row_offset: 0,
            value_offset: 0,
            unassigned_pending: false,
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        loop {
            let row = self.rows.get(self.row_offset)?;
            if let Some(&value) = row.values.get(self.value_offset) {
                self.value_offset += 1;
                return Some(RuntimeScalarRecipe::Change {
                    slot: self.slot.clone(),
                    entity_index: row.entity_index,
                    to_value: Some(value),
                });
            }
            if !self.unassigned_pending && self.slot.allows_unassigned() && row.current_assigned {
                self.unassigned_pending = true;
                return Some(RuntimeScalarRecipe::Change {
                    slot: self.slot.clone(),
                    entity_index: row.entity_index,
                    to_value: None,
                });
            }
            self.row_offset += 1;
            self.value_offset = 0;
            self.unassigned_pending = false;
        }
    }
}

struct NearbyChangeRow {
    entity_index: usize,
    values: Vec<usize>,
    unassigned_pending: bool,
}

enum NearbyChangeMode<S> {
    /// Native nearby source callbacks historically run at cursor open.
    Eager {
        rows: Vec<NearbyChangeRow>,
        row_offset: usize,
        value_offset: usize,
    },
    /// Dynamic sources remain lazy: only the reached row invokes the bridge.
    Lazy {
        solution: S,
        context: MoveStreamContext,
        entity_count: usize,
        entity_offset: usize,
        values: Vec<usize>,
        value_offset: usize,
        source_loaded: bool,
        unassigned_pending: bool,
    },
}

/// Nearby change preserves source timing by carrier while sharing ranking,
/// ordering, recipe ownership, and no-substitution behavior.
pub(super) struct NearbyChangeCursor<S> {
    slot: RuntimeScalarSlot<S>,
    max_nearby: usize,
    source_limit: usize,
    mode: NearbyChangeMode<S>,
}

impl<S> NearbyChangeCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: S,
        context: MoveStreamContext,
        max_nearby: usize,
        source_limit: usize,
    ) -> Self {
        let mode = if slot.is_dynamic() {
            NearbyChangeMode::Lazy {
                entity_count: slot.entity_count(&solution),
                solution,
                context,
                entity_offset: 0,
                values: Vec::new(),
                value_offset: 0,
                source_loaded: false,
                unassigned_pending: false,
            }
        } else {
            let rows = eager_nearby_rows(&slot, &solution, context, max_nearby, source_limit);
            NearbyChangeMode::Eager {
                rows,
                row_offset: 0,
                value_offset: 0,
            }
        };
        Self {
            slot,
            max_nearby,
            source_limit,
            mode,
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        match &mut self.mode {
            NearbyChangeMode::Eager {
                rows,
                row_offset,
                value_offset,
            } => loop {
                let (entity_index, value, unassigned_pending) = {
                    let row = rows.get(*row_offset)?;
                    (
                        row.entity_index,
                        row.values.get(*value_offset).copied(),
                        row.unassigned_pending,
                    )
                };
                if let Some(value) = value {
                    *value_offset += 1;
                    return Some(RuntimeScalarRecipe::Change {
                        slot: self.slot.clone(),
                        entity_index,
                        to_value: Some(value),
                    });
                }
                if unassigned_pending {
                    // Consume this one terminal candidate by replacing the row
                    // marker; no source is reread.
                    rows[*row_offset].unassigned_pending = false;
                    return Some(RuntimeScalarRecipe::Change {
                        slot: self.slot.clone(),
                        entity_index,
                        to_value: None,
                    });
                }
                *row_offset += 1;
                *value_offset = 0;
            },
            NearbyChangeMode::Lazy {
                solution,
                context,
                entity_count,
                entity_offset,
                values,
                value_offset,
                source_loaded,
                unassigned_pending,
            } => {
                while *entity_offset < *entity_count {
                    let entity_index = ordered_entity(
                        *entity_count,
                        *entity_offset,
                        *context,
                        NEARBY_CHANGE_ENTITY_START_SALT,
                        NEARBY_CHANGE_ENTITY_STRIDE_SALT,
                        slot_identity(&self.slot),
                    );
                    if !*source_loaded {
                        *values = rank_nearby_values(
                            &self.slot,
                            solution,
                            entity_index,
                            self.max_nearby,
                            self.source_limit,
                            *context,
                        );
                        *value_offset = 0;
                        *unassigned_pending = self.slot.allows_unassigned()
                            && self.slot.current_value(solution, entity_index).is_some();
                        *source_loaded = true;
                    }
                    if let Some(&value) = values.get(*value_offset) {
                        *value_offset += 1;
                        return Some(RuntimeScalarRecipe::Change {
                            slot: self.slot.clone(),
                            entity_index,
                            to_value: Some(value),
                        });
                    }
                    if *unassigned_pending {
                        *unassigned_pending = false;
                        return Some(RuntimeScalarRecipe::Change {
                            slot: self.slot.clone(),
                            entity_index,
                            to_value: None,
                        });
                    }
                    *entity_offset += 1;
                    values.clear();
                    *value_offset = 0;
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
    source_limit: usize,
) -> Vec<NearbyChangeRow>
where
    S: PlanningSolution,
{
    let entity_count = slot.entity_count(solution);
    (0..entity_count)
        .map(|offset| {
            let entity_index = ordered_entity(
                entity_count,
                offset,
                context,
                NEARBY_CHANGE_ENTITY_START_SALT,
                NEARBY_CHANGE_ENTITY_STRIDE_SALT,
                slot_identity(slot),
            );
            NearbyChangeRow {
                entity_index,
                values: rank_nearby_values(
                    slot,
                    solution,
                    entity_index,
                    max_nearby,
                    source_limit,
                    context,
                ),
                unassigned_pending: slot.allows_unassigned()
                    && slot.current_value(solution, entity_index).is_some(),
            }
        })
        .collect()
}

fn rank_nearby_values<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    entity_index: usize,
    max_nearby: usize,
    source_limit: usize,
    context: MoveStreamContext,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    if max_nearby == 0 || source_limit == 0 {
        return Vec::new();
    }
    let current = slot.current_value(solution, entity_index);
    let dynamic = slot.is_dynamic();
    let mut selected = NearbyTopK::new(max_nearby);
    let mut order = 0usize;
    let mut visit = |value| {
        let source_order = order;
        order += 1;
        if current == Some(value)
            || (dynamic && !slot.value_is_legal(solution, entity_index, Some(value)))
        {
            return;
        }
        selected.push(RankedNearbyCandidate {
            candidate: value,
            distance: slot
                .nearby_value_distance(solution, entity_index, value)
                .unwrap_or(source_order as f64),
            order: source_order,
        });
    };
    let row_supplied =
        slot.visit_nearby_value_candidates(solution, entity_index, source_limit, &mut visit);
    if !row_supplied {
        slot.visit_candidate_values(solution, entity_index, Some(source_limit), &mut visit);
    }
    let mut values = selected.finish();
    context.apply_selection_order(
        &mut values,
        NEARBY_CHANGE_VALUE_SALT ^ entity_index as u64 ^ slot_identity(slot),
    );
    values
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
