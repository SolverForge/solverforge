//! The one lazy cursor for callback and native compound-provider results.
//!
//! It deliberately owns provider scheduling rather than delegating to the
//! older typed group/repair selectors. A source policy is data in the compiled
//! graph; this cursor is the only code that interprets it.

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{
    ProviderNormalizationState, ProviderReasonArena, RawProviderCandidate,
    ResolvedProviderCandidate, RuntimeProviderLimits, RuntimeProviderRegistry,
};
use crate::heuristic::r#move::{RuntimeCompoundMove, RuntimeCompoundMoveKind};
use crate::heuristic::selector::move_selector::{CandidateId, CandidateStore, MoveStreamContext};

use super::compiler::{
    CompiledProviderPlan, ProviderBindingPlan, ProviderBindingPolicy, ProviderMoveKind,
    ProviderSchedule,
};
use super::provider_telemetry::ConflictRepairCounts;

#[path = "provider_cursor/repair.rs"]
mod repair;

/// Cursor-owned, lazily initialized compound-provider stream.
///
/// Sources activate only on a scheduler-reached pull, including static
/// `OpenCursor` declarations. Unreached or limited-zero branches remain inert.
/// The caller lends the preview-relative solution, registry and solve-owned
/// reason arena at each pull; the cursor retains none of their borrows.
pub(crate) struct RuntimeProviderCursor<'p, S>
where
    S: PlanningSolution + 'static,
{
    plan: &'p CompiledProviderPlan,
    context: MoveStreamContext,
    require_hard_improvement: bool,
    store: CandidateStore<S, RuntimeCompoundMove<S>>,
    next_index: usize,
    prepared: bool,
}

impl<'p, S> RuntimeProviderCursor<'p, S>
where
    S: PlanningSolution + 'static,
{
    pub(crate) fn new(
        plan: &'p CompiledProviderPlan,
        context: MoveStreamContext,
        require_hard_improvement: bool,
    ) -> Self {
        Self {
            plan,
            context,
            require_hard_improvement,
            store: CandidateStore::new(),
            next_index: 0,
            prepared: false,
        }
    }

    fn prepare(
        &mut self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        reason_arena: &mut ProviderReasonArena,
        telemetry: &mut ConflictRepairCounts,
    ) {
        if self.prepared {
            return;
        }
        self.prepared = true;
        match &self.plan.schedule {
            ProviderSchedule::Group {
                value_candidate_limit,
                requested_max_moves_per_step,
            } => self.prepare_group(
                solution,
                registry,
                *value_candidate_limit,
                *requested_max_moves_per_step,
                reason_arena,
            ),
            ProviderSchedule::Repair {
                constraints,
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
                include_soft_matches,
            } => self.prepare_repair(
                solution,
                registry,
                constraints,
                *max_matches_per_step,
                *max_repairs_per_match,
                *max_moves_per_step,
                *include_soft_matches,
                reason_arena,
                telemetry,
            ),
        }
    }

    fn prepare_group(
        &mut self,
        solution: &S,
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
                    let raw = registry.pull_callback_raw(binding.handle, solution, limits);
                    let mut candidates = self.normalize(
                        solution,
                        registry,
                        binding,
                        raw,
                        &mut normalization,
                        reason_arena,
                    );
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
                        solution,
                        value_candidate_limit,
                        Some(max_moves),
                    );
                    // Native groups historically rotate provider output before
                    // validity/dedup filtering. Keep that source policy while
                    // resolving typed edits through the shared semantic kernel.
                    self.context
                        .apply_selection_order(&mut native, rotation_seed_salt);
                    registry.normalize_static_group(
                        solution,
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
                self.push_candidate(solution, candidate);
            }
        }
    }

    fn normalize(
        &self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        binding: &ProviderBindingPlan,
        raw: Vec<RawProviderCandidate>,
        state: &mut ProviderNormalizationState,
        reason_arena: &mut ProviderReasonArena,
    ) -> Vec<ResolvedProviderCandidate<S>> {
        registry.normalize_or_raise(solution, raw, &binding.allowed_slots, state, reason_arena)
    }

    /// Stores `candidate` when it is doable on `solution`; returns whether it
    /// was stored.
    fn push_candidate(&mut self, solution: &S, candidate: ResolvedProviderCandidate<S>) -> bool {
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
        let doable = mov.is_doable_on(solution);
        if doable {
            self.store.push(mov);
        }
        doable
    }
}

impl<S> RuntimeProviderCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    /// Pulls one candidate through the caller-owned per-execution reason
    /// arena. The arena is borrowed only while a provider is prepared; the
    /// cursor never retains it between pulls. Conflict-repair outcomes are
    /// counted into the caller-owned `telemetry`; group providers count none.
    pub(crate) fn next_candidate(
        &mut self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        reason_arena: &mut ProviderReasonArena,
        telemetry: &mut ConflictRepairCounts,
    ) -> Option<CandidateId> {
        self.prepare(solution, registry, reason_arena, telemetry);
        while self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            if self.store.candidate(id).is_some() {
                if matches!(self.plan.schedule, ProviderSchedule::Repair { .. }) {
                    telemetry.exposed += 1;
                }
                return Some(id);
            }
        }
        None
    }

    pub(crate) fn take_candidate(&mut self, id: CandidateId) -> RuntimeCompoundMove<S> {
        self.store.take_candidate(id)
    }
}

fn runtime_move_kind(kind: ProviderMoveKind) -> RuntimeCompoundMoveKind {
    match kind {
        ProviderMoveKind::Grouped => RuntimeCompoundMoveKind::Grouped,
        ProviderMoveKind::ConflictRepair => RuntimeCompoundMoveKind::ConflictRepair,
        ProviderMoveKind::CompoundConflictRepair => RuntimeCompoundMoveKind::CompoundConflictRepair,
    }
}
