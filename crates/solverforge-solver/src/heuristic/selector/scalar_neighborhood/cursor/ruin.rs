use std::collections::HashMap;

use rand::rngs::SmallRng;
use rand::RngExt;
use solverforge_config::RecreateHeuristicType;
use solverforge_core::domain::PlanningSolution;

use crate::builder::RuntimeScalarSlot;

use super::super::spec::RuntimeScalarRecipe;

/// Generates the exact finite batch shape of the established scalar
/// ruin/recreate selector. The caller owns the one mutable leaf stream state;
/// this function only consumes it while creating cursor-owned batches.
pub(super) fn generate_ruin_batches<S>(
    solution: &S,
    slot: &RuntimeScalarSlot<S>,
    min_ruin_count: usize,
    max_ruin_count: usize,
    moves_per_step: usize,
    rng: &mut SmallRng,
) -> Vec<Vec<usize>>
where
    S: PlanningSolution,
{
    ruin_batches(
        slot.entity_count(solution),
        min_ruin_count,
        max_ruin_count,
        moves_per_step,
        rng,
    )
}

/// Draws `moves_per_step` batches of distinct entity indexes, each the prefix
/// of a partial Fisher-Yates shuffle of the identity permutation.
///
/// The shuffle is simulated without the permutation: `displaced` records only
/// the positions whose value a swap has changed, and every other position
/// holds its own index. A batch therefore costs O(ruin count) time and memory,
/// reads no entity-sized array, and yields exactly the batches and random
/// draws of shuffling a full identity permutation.
fn ruin_batches(
    entity_count: usize,
    min_ruin_count: usize,
    max_ruin_count: usize,
    moves_per_step: usize,
    rng: &mut SmallRng,
) -> Vec<Vec<usize>> {
    let min = min_ruin_count.min(entity_count);
    let max = max_ruin_count.min(entity_count);
    let mut displaced: HashMap<usize, usize> = HashMap::with_capacity(max);
    (0..moves_per_step)
        .map(|_| {
            if entity_count == 0 {
                return Vec::new();
            }
            let ruin_count = if min == max {
                min
            } else {
                rng.random_range(min..=max)
            };
            displaced.clear();
            let mut batch = Vec::with_capacity(ruin_count);
            for index in 0..ruin_count {
                let swap_index = rng.random_range(index..entity_count);
                let at_index = displaced.get(&index).copied().unwrap_or(index);
                let at_swap = displaced.get(&swap_index).copied().unwrap_or(swap_index);
                batch.push(at_swap);
                // Position `index` is never read again: later swaps draw from
                // later positions only.
                displaced.insert(swap_index, at_index);
            }
            batch
        })
        .collect()
}

#[cfg(test)]
#[path = "ruin_tests.rs"]
mod tests;

struct RuinSnapshot {
    assigned: Vec<bool>,
    recreatable: Vec<bool>,
}

/// Streams pre-generated subsets after snapshotting their eligibility. A
/// selected move owns its recipe and performs score-aware recreation only
/// when applied by the local-search engine.
pub(super) struct RuinRecreateCursor<S> {
    slot: RuntimeScalarSlot<S>,
    batches: std::vec::IntoIter<Vec<usize>>,
    snapshot: RuinSnapshot,
    value_candidate_limit: Option<usize>,
    recreate_heuristic_type: RecreateHeuristicType,
}

impl<S> RuinRecreateCursor<S>
where
    S: PlanningSolution,
{
    pub(super) fn new(
        slot: RuntimeScalarSlot<S>,
        solution: &S,
        batches: Vec<Vec<usize>>,
        value_candidate_limit: Option<usize>,
        recreate_heuristic_type: RecreateHeuristicType,
    ) -> Self {
        let snapshot = snapshot_slot(&slot, solution, value_candidate_limit);
        Self {
            slot,
            batches: batches.into_iter(),
            snapshot,
            value_candidate_limit,
            recreate_heuristic_type,
        }
    }

    pub(super) fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        while let Some(entity_indices) = self.batches.next() {
            let has_assigned = entity_indices
                .iter()
                .any(|&entity| self.snapshot.assigned.get(entity).copied().unwrap_or(false));
            let required_recreatable = self.slot.allows_unassigned()
                || entity_indices.iter().all(|&entity| {
                    !self.snapshot.assigned.get(entity).copied().unwrap_or(false)
                        || self
                            .snapshot
                            .recreatable
                            .get(entity)
                            .copied()
                            .unwrap_or(false)
                });
            if !has_assigned || !required_recreatable {
                continue;
            }
            return Some(RuntimeScalarRecipe::RuinRecreate {
                slot: self.slot.clone(),
                entity_indices,
                value_candidate_limit: self.value_candidate_limit,
                recreate_heuristic_type: self.recreate_heuristic_type,
            });
        }
        None
    }
}

fn snapshot_slot<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    value_candidate_limit: Option<usize>,
) -> RuinSnapshot
where
    S: PlanningSolution,
{
    let entity_count = slot.entity_count(solution);
    let assigned = (0..entity_count)
        .map(|entity| slot.current_value(solution, entity).is_some())
        .collect();
    let recreatable = (0..entity_count)
        .map(|entity| {
            let mut found = false;
            slot.visit_candidate_values(solution, entity, value_candidate_limit, &mut |_| {
                found = true
            });
            found
        })
        .collect();
    RuinSnapshot {
        assigned,
        recreatable,
    }
}
