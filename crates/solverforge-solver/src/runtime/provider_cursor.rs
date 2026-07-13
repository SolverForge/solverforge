//! The one lazy cursor for callback and native compound-provider results.
//!
//! It deliberately owns provider scheduling rather than delegating to the
//! older typed group/repair selectors. A source policy is data in the compiled
//! graph; this cursor is the only code that interprets it.

use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{
    ProviderNormalizationState, ProviderReasonArena, RawProviderCandidate,
    ResolvedProviderCandidate, RuntimeProviderLimits, RuntimeProviderRegistry,
};
use crate::heuristic::r#move::{RuntimeCompoundMove, RuntimeCompoundMoveKind};
use crate::heuristic::selector::move_selector::{CandidateId, CandidateStore, MoveStreamContext};
use crate::RepairLimits;

use super::compiler::{
    CompiledProviderPlan, ProviderBindingPlan, ProviderBindingPolicy, ProviderMoveKind,
    ProviderSchedule,
};

/// Cursor-owned, lazily initialized compound-provider stream.
///
/// A provider source is never pulled by `new`; its first pull happens only
/// when `next_candidate` is reached. For a static `OpenCursor` declaration,
/// that first reachable pull is the provider leaf's activation boundary. It
/// therefore retains eager-at-activation semantics without making an
/// unreached, limited-zero, or impossible Cartesian branch observable.
///
/// The cursor deliberately does not retain the solve-owned reason arena. The
/// recursive compiled-selector execution owns that arena and passes it only
/// at normalization/pull boundaries. This lets one execution interleave
/// nested provider leaves without shared ownership, interior mutability, or
/// duplicate per-leaf arenas while preserving one stable ID space.
pub(crate) struct RuntimeProviderCursor<S>
where
    S: PlanningSolution + 'static,
{
    plan: CompiledProviderPlan,
    solution: S,
    context: MoveStreamContext,
    require_hard_improvement: bool,
    store: CandidateStore<S, RuntimeCompoundMove<S>>,
    next_index: usize,
    prepared: bool,
}

impl<S> RuntimeProviderCursor<S>
where
    S: PlanningSolution + 'static,
{
    pub(crate) fn new(
        plan: CompiledProviderPlan,
        solution: S,
        context: MoveStreamContext,
        require_hard_improvement: bool,
    ) -> Self {
        Self {
            plan,
            solution,
            context,
            require_hard_improvement,
            store: CandidateStore::new(),
            next_index: 0,
            prepared: false,
        }
    }

    fn prepare(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        reason_arena: &mut ProviderReasonArena,
    ) {
        if self.prepared {
            return;
        }
        self.prepared = true;
        match self.plan.schedule.clone() {
            ProviderSchedule::Group {
                value_candidate_limit,
                requested_max_moves_per_step,
            } => self.prepare_group(
                registry,
                value_candidate_limit,
                requested_max_moves_per_step,
                reason_arena,
            ),
            ProviderSchedule::Repair {
                constraints,
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
                include_soft_matches,
            } => self.prepare_repair(
                registry,
                constraints,
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
                include_soft_matches,
                reason_arena,
            ),
        }
    }

    fn prepare_group(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        value_candidate_limit: Option<usize>,
        requested_max_moves_per_step: Option<usize>,
        reason_arena: &mut ProviderReasonArena,
    ) {
        for binding_index in 0..self.plan.bindings.len() {
            let binding = &self.plan.bindings[binding_index];
            let max_moves = match binding.policy {
                ProviderBindingPolicy::CallbackGroup { .. } => {
                    // Python's public group callback contract treats an
                    // explicit zero as one candidate rather than a no-op.
                    requested_max_moves_per_step.unwrap_or(256).max(1)
                }
                ProviderBindingPolicy::StaticGroup {
                    declared_max_moves_per_step,
                    ..
                } => requested_max_moves_per_step
                    .or(declared_max_moves_per_step)
                    .unwrap_or(256),
                ProviderBindingPolicy::CallbackRepair { .. }
                | ProviderBindingPolicy::StaticRepair { .. } => continue,
            };
            if max_moves == 0 {
                continue;
            }
            let mut normalization = ProviderNormalizationState::default();
            let mut candidates = match binding.policy {
                ProviderBindingPolicy::CallbackGroup {
                    rotation_seed_salt, ..
                } => {
                    let limits = RuntimeProviderLimits::Group {
                        value_candidate_limit,
                        max_moves_per_step: Some(max_moves),
                    };
                    let raw = registry.pull_callback_raw(binding.handle, &self.solution, limits);
                    let mut candidates =
                        self.normalize(registry, binding, raw, &mut normalization, reason_arena);
                    // Callback order is normalized/deduped first, capped in
                    // callback order, then step-rotated.
                    candidates.truncate(max_moves);
                    self.context
                        .apply_selection_order(&mut candidates, rotation_seed_salt);
                    candidates
                }
                ProviderBindingPolicy::StaticGroup {
                    rotation_seed_salt, ..
                } => {
                    let crate::builder::RuntimeProviderHandle::StaticGroup(provider_index) =
                        binding.handle
                    else {
                        unreachable!("static group policy must retain a static group handle")
                    };
                    let mut native = registry.pull_static_group(
                        provider_index,
                        &self.solution,
                        value_candidate_limit,
                        Some(max_moves),
                    );
                    // Native groups historically rotate provider output before
                    // validity/dedup filtering. Keep that source policy while
                    // resolving typed edits through the shared semantic kernel.
                    self.context
                        .apply_selection_order(&mut native, rotation_seed_salt);
                    registry.normalize_static_group(
                        &self.solution,
                        native,
                        &binding.allowed_slots,
                        &mut normalization,
                        reason_arena,
                    )
                }
                ProviderBindingPolicy::CallbackRepair { .. }
                | ProviderBindingPolicy::StaticRepair { .. } => unreachable!(),
            };
            for candidate in candidates.drain(..) {
                if self.store.len() >= max_moves {
                    break;
                }
                self.push_candidate(candidate);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_repair(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        constraints: Vec<String>,
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
        reason_arena: &mut ProviderReasonArena,
    ) {
        if constraints.is_empty()
            || max_matches_per_step == 0
            || max_repairs_per_match == 0
            || max_moves_per_step == 0
        {
            return;
        }
        let mut provider_invocations = 0usize;
        self.prepare_callback_repairs(
            registry,
            &constraints,
            max_matches_per_step,
            max_repairs_per_match,
            max_moves_per_step,
            include_soft_matches,
            &mut provider_invocations,
            reason_arena,
        );
        if self.store.len() < max_moves_per_step && provider_invocations < max_matches_per_step {
            self.prepare_static_repairs(
                registry,
                &constraints,
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
                &mut provider_invocations,
                reason_arena,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_callback_repairs(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        constraints: &[String],
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
        provider_invocations: &mut usize,
        reason_arena: &mut ProviderReasonArena,
    ) {
        let mut indexes = self
            .plan
            .bindings
            .iter()
            .enumerate()
            .filter_map(|(index, binding)| {
                matches!(binding.policy, ProviderBindingPolicy::CallbackRepair { .. })
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let Some(&first_index) = indexes.first() else {
            return;
        };
        let ProviderBindingPolicy::CallbackRepair {
            rotation_seed_salt, ..
        } = self.plan.bindings[first_index].policy
        else {
            unreachable!();
        };
        let limits = Arc::<[Arc<str>]>::from(
            constraints
                .iter()
                .map(|constraint| Arc::from(constraint.as_str()))
                .collect::<Vec<_>>(),
        );
        // Rotate the complete callback declaration stream before testing
        // constraint membership. A multi-constraint provider is called once.
        self.context
            .apply_selection_order(&mut indexes, rotation_seed_salt);
        for binding_index in indexes {
            if self.store.len() >= max_moves_per_step
                || *provider_invocations >= max_matches_per_step
            {
                break;
            }
            let binding = &self.plan.bindings[binding_index];
            if !registry.declares_any_constraint(binding.handle, constraints) {
                continue;
            }
            *provider_invocations += 1;
            let raw = registry.pull_callback_raw(
                binding.handle,
                &self.solution,
                repair_limits(
                    &limits,
                    max_matches_per_step,
                    max_repairs_per_match,
                    max_moves_per_step,
                    include_soft_matches,
                ),
            );
            let mut normalization = ProviderNormalizationState::default();
            let mut candidates =
                self.normalize(registry, binding, raw, &mut normalization, reason_arena);
            candidates.truncate(max_repairs_per_match);
            for candidate in candidates {
                if self.store.len() >= max_moves_per_step {
                    break;
                }
                self.push_candidate(candidate);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_static_repairs(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        constraints: &[String],
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        provider_invocations: &mut usize,
        reason_arena: &mut ProviderReasonArena,
    ) {
        let static_indexes = self
            .plan
            .bindings
            .iter()
            .enumerate()
            .filter_map(|(index, binding)| {
                matches!(binding.policy, ProviderBindingPolicy::StaticRepair { .. })
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let Some(&first_index) = static_indexes.first() else {
            return;
        };
        let ProviderBindingPolicy::StaticRepair {
            constraint_rotation_seed_salt,
            provider_rotation_seed_salt,
            spec_rotation_seed_salt,
            ..
        } = self.plan.bindings[first_index].policy
        else {
            unreachable!();
        };
        let mut constraint_indexes = (0..constraints.len()).collect::<Vec<_>>();
        self.context.apply_selection_order(
            &mut constraint_indexes,
            constraint_rotation_seed_salt ^ max_moves_per_step as u64,
        );
        let mut normalization = ProviderNormalizationState::default();
        for constraint_index in constraint_indexes {
            let constraint = &constraints[constraint_index];
            let mut indexes = static_indexes
                .iter()
                .copied()
                .filter(|&binding_index| {
                    registry
                        .declares_constraint(self.plan.bindings[binding_index].handle, constraint)
                })
                .collect::<Vec<_>>();
            self.context.apply_selection_order(
                &mut indexes,
                provider_rotation_seed_salt ^ constraint_index as u64,
            );
            for binding_index in indexes {
                if self.store.len() >= max_moves_per_step
                    || *provider_invocations >= max_matches_per_step
                {
                    return;
                }
                *provider_invocations += 1;
                let binding = &self.plan.bindings[binding_index];
                let crate::builder::RuntimeProviderHandle::StaticRepair(provider_index) =
                    binding.handle
                else {
                    unreachable!("static repair policy must retain a static repair handle")
                };
                let mut native = registry.pull_static_repair(
                    provider_index,
                    &self.solution,
                    RepairLimits {
                        max_matches_per_step,
                        max_repairs_per_match,
                        max_moves_per_step,
                    },
                );
                self.context.apply_selection_order(
                    &mut native,
                    spec_rotation_seed_salt ^ binding.declared_schema_index as u64,
                );
                native.truncate(max_repairs_per_match);
                for candidate in registry.normalize_static_repair(
                    &self.solution,
                    native,
                    &binding.allowed_slots,
                    &mut normalization,
                    reason_arena,
                ) {
                    if self.store.len() >= max_moves_per_step {
                        return;
                    }
                    self.push_candidate(candidate);
                }
            }
        }
    }

    fn normalize(
        &self,
        registry: &RuntimeProviderRegistry<S>,
        binding: &ProviderBindingPlan,
        raw: Vec<RawProviderCandidate>,
        state: &mut ProviderNormalizationState,
        reason_arena: &mut ProviderReasonArena,
    ) -> Vec<ResolvedProviderCandidate<S>> {
        registry.normalize_or_raise(
            &self.solution,
            raw,
            &binding.allowed_slots,
            state,
            reason_arena,
        )
    }

    fn push_candidate(&mut self, candidate: ResolvedProviderCandidate<S>) {
        let edits = candidate
            .edits
            .into_iter()
            .map(|edit| edit.slot.edit(edit.entity_index, edit.to_value))
            .collect();
        let mov = RuntimeCompoundMove::new(
            runtime_move_kind(self.plan.move_kind),
            candidate.reason,
            edits,
            self.require_hard_improvement,
        );
        if mov.is_doable_on(&self.solution) {
            self.store.push(mov);
        }
    }
}

impl<S> RuntimeProviderCursor<S>
where
    S: PlanningSolution + 'static,
{
    /// Pulls one candidate through the caller-owned per-execution reason
    /// arena. The arena is borrowed only while a provider is prepared; the
    /// cursor never retains it between pulls.
    pub(crate) fn next_candidate(
        &mut self,
        registry: &RuntimeProviderRegistry<S>,
        reason_arena: &mut ProviderReasonArena,
    ) -> Option<CandidateId> {
        self.prepare(registry, reason_arena);
        while self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            if self.store.candidate(id).is_some() {
                return Some(id);
            }
        }
        None
    }

    pub(crate) fn take_candidate(&mut self, id: CandidateId) -> RuntimeCompoundMove<S> {
        self.store.take_candidate(id)
    }
}

fn repair_limits(
    constraints: &Arc<[Arc<str>]>,
    max_matches_per_step: usize,
    max_repairs_per_match: usize,
    max_moves_per_step: usize,
    include_soft_matches: bool,
) -> RuntimeProviderLimits {
    RuntimeProviderLimits::Repair {
        constraints: Arc::clone(constraints),
        max_matches_per_step,
        max_repairs_per_match,
        max_moves_per_step,
        include_soft_matches,
    }
}

fn runtime_move_kind(kind: ProviderMoveKind) -> RuntimeCompoundMoveKind {
    match kind {
        ProviderMoveKind::Grouped => RuntimeCompoundMoveKind::Grouped,
        ProviderMoveKind::ConflictRepair => RuntimeCompoundMoveKind::ConflictRepair,
        ProviderMoveKind::CompoundConflictRepair => RuntimeCompoundMoveKind::CompoundConflictRepair,
    }
}
