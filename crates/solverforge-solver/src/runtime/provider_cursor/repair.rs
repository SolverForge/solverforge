//! Conflict-repair scheduling for the provider cursor.
//!
//! Every conflict-repair outcome is counted here, where the cursor decides
//! it: raw provider results, normalization drops and not-doable drops.

use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{
    ProviderNormalizationState, ProviderReasonArena, RuntimeProviderLimits, RuntimeProviderRegistry,
};
use crate::runtime::compiler::ProviderBindingPolicy;
use crate::runtime::provider_telemetry::ConflictRepairCounts;
use crate::RepairLimits;

use super::RuntimeProviderCursor;

impl<S> RuntimeProviderCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_repair(
        &mut self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        constraints: &[String],
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
        reason_arena: &mut ProviderReasonArena,
        telemetry: &mut ConflictRepairCounts,
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
            solution,
            registry,
            constraints,
            RepairLimits {
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
            },
            include_soft_matches,
            &mut provider_invocations,
            reason_arena,
            telemetry,
        );
        if self.store.len() < max_moves_per_step && provider_invocations < max_matches_per_step {
            let mut normalization = ProviderNormalizationState::default();
            self.prepare_static_repairs(
                solution,
                registry,
                constraints,
                RepairLimits {
                    max_matches_per_step,
                    max_repairs_per_match,
                    max_moves_per_step,
                },
                &mut provider_invocations,
                &mut normalization,
                reason_arena,
                telemetry,
            );
            telemetry.record_normalization(&normalization);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_callback_repairs(
        &mut self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        constraints: &[String],
        limits: RepairLimits,
        include_soft_matches: bool,
        provider_invocations: &mut usize,
        reason_arena: &mut ProviderReasonArena,
        telemetry: &mut ConflictRepairCounts,
    ) {
        let RepairLimits {
            max_matches_per_step,
            max_repairs_per_match,
            max_moves_per_step,
        } = limits;
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
        let constraint_limits = Arc::<[Arc<str>]>::from(
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
                solution,
                RuntimeProviderLimits::Repair {
                    constraints: Arc::clone(&constraint_limits),
                    max_matches_per_step,
                    max_repairs_per_match,
                    max_moves_per_step,
                    include_soft_matches,
                },
            );
            telemetry.provider_generated += raw.len() as u64;
            let mut normalization = ProviderNormalizationState::default();
            let mut candidates = self.normalize(
                solution,
                registry,
                binding,
                raw,
                &mut normalization,
                reason_arena,
            );
            telemetry.record_normalization(&normalization);
            candidates.truncate(max_repairs_per_match);
            for candidate in candidates {
                if self.store.len() >= max_moves_per_step {
                    break;
                }
                if !self.push_candidate(solution, candidate) {
                    telemetry.not_doable_filtered += 1;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_static_repairs(
        &mut self,
        solution: &S,
        registry: &RuntimeProviderRegistry<S>,
        constraints: &[String],
        limits: RepairLimits,
        provider_invocations: &mut usize,
        normalization: &mut ProviderNormalizationState,
        reason_arena: &mut ProviderReasonArena,
        telemetry: &mut ConflictRepairCounts,
    ) {
        let RepairLimits {
            max_matches_per_step,
            max_repairs_per_match,
            max_moves_per_step,
        } = limits;
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
                let mut native = registry.pull_static_repair(provider_index, solution, limits);
                telemetry.provider_generated += native.len() as u64;
                self.context.apply_selection_order(
                    &mut native,
                    spec_rotation_seed_salt ^ binding.declared_schema_index as u64,
                );
                native.truncate(max_repairs_per_match);
                for candidate in registry.normalize_static_repair(
                    solution,
                    native,
                    &binding.allowed_slots,
                    normalization,
                    reason_arena,
                ) {
                    if self.store.len() >= max_moves_per_step {
                        return;
                    }
                    if !self.push_candidate(solution, candidate) {
                        telemetry.not_doable_filtered += 1;
                    }
                }
            }
        }
    }
}
