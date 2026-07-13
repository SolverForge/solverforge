use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use super::super::{
    ConflictRepair, RuntimeScalarSlot, RuntimeScalarSlotId, ScalarGroupBinding,
    ScalarGroupBindingKind,
};
use super::native::{pull_static_group, pull_static_repair};
use super::types::PanicProviderErrorBoundary;
use super::{
    ProviderNormalizationState, ProviderReasonArena, RawProviderCandidate,
    ResolvedProviderCandidate, RuntimeConflictRepairProviderBinding,
    RuntimeHostProviderErrorBoundary, RuntimeProviderHandle, RuntimeProviderLimits,
    RuntimeProviderSlotResolver, RuntimeScalarGroupProviderBinding,
    StaticConflictRepairProviderBinding, StaticScalarGroupProviderBinding,
};
use crate::{RepairCandidate, RepairLimits, ScalarCandidate};

/// Frozen schema-order registry. It does not contain a mutable host lookup;
/// plans store immutable declaration indexes and cursor dispatch rebuilds no
/// schema state at solve time.
pub struct RuntimeProviderRegistry<S> {
    groups: Vec<RuntimeScalarGroupProviderBinding<S>>,
    repairs: Vec<RuntimeConflictRepairProviderBinding<S>>,
    static_groups: Vec<StaticScalarGroupProviderBinding<S>>,
    static_repairs: Vec<StaticConflictRepairProviderBinding<S>>,
    error_boundary: Arc<dyn RuntimeHostProviderErrorBoundary>,
    resolver: Option<RuntimeProviderSlotResolver<S>>,
}

impl<S> Default for RuntimeProviderRegistry<S> {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            repairs: Vec::new(),
            static_groups: Vec::new(),
            static_repairs: Vec::new(),
            error_boundary: Arc::new(PanicProviderErrorBoundary),
            resolver: None,
        }
    }
}

impl<S> Clone for RuntimeProviderRegistry<S> {
    fn clone(&self) -> Self {
        Self {
            groups: self.groups.clone(),
            repairs: self.repairs.clone(),
            static_groups: self.static_groups.clone(),
            static_repairs: self.static_repairs.clone(),
            error_boundary: Arc::clone(&self.error_boundary),
            resolver: self.resolver.clone(),
        }
    }
}

impl<S> fmt::Debug for RuntimeProviderRegistry<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeProviderRegistry")
            .field("groups", &self.groups)
            .field("repairs", &self.repairs)
            .field("static_groups", &self.static_groups)
            .field("static_repairs", &self.static_repairs)
            .field("resolver_bound", &self.resolver.is_some())
            .finish()
    }
}

impl<S: 'static> RuntimeProviderRegistry<S> {
    pub fn new(
        groups: Vec<RuntimeScalarGroupProviderBinding<S>>,
        repairs: Vec<RuntimeConflictRepairProviderBinding<S>>,
        error_boundary: Arc<dyn RuntimeHostProviderErrorBoundary>,
    ) -> Result<Self, String> {
        let mut group_names = HashSet::new();
        let mut previous_group_schema_index = None;
        for group in &groups {
            if group.group_name.is_empty() {
                return Err("runtime provider registry has an empty group name".to_string());
            }
            if previous_group_schema_index.is_some_and(|previous| previous >= group.declared_index)
            {
                return Err(format!(
                    "runtime provider group `{}` has non-monotonic declared schema index {}",
                    group.group_name, group.declared_index
                ));
            }
            previous_group_schema_index = Some(group.declared_index);
            if !group_names.insert(Arc::clone(&group.group_name)) {
                return Err(format!(
                    "runtime provider registry declares callback group `{}` more than once",
                    group.group_name
                ));
            }
        }
        let mut previous_repair_schema_index = None;
        for repair in &repairs {
            if previous_repair_schema_index
                .is_some_and(|previous| previous >= repair.declared_index)
            {
                return Err(format!(
                    "runtime repair provider has non-monotonic declared schema index {}",
                    repair.declared_index
                ));
            }
            previous_repair_schema_index = Some(repair.declared_index);
        }
        Ok(Self {
            groups,
            repairs,
            static_groups: Vec::new(),
            static_repairs: Vec::new(),
            error_boundary,
            resolver: None,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
            && self.repairs.is_empty()
            && self.static_groups.is_empty()
            && self.static_repairs.is_empty()
    }

    pub fn groups(&self) -> &[RuntimeScalarGroupProviderBinding<S>] {
        &self.groups
    }

    pub fn repairs(&self) -> &[RuntimeConflictRepairProviderBinding<S>] {
        &self.repairs
    }

    pub fn static_groups(&self) -> &[StaticScalarGroupProviderBinding<S>] {
        &self.static_groups
    }

    pub fn static_repairs(&self) -> &[StaticConflictRepairProviderBinding<S>] {
        &self.static_repairs
    }

    pub fn group_indices(&self, group_name: &str) -> Vec<usize> {
        self.groups
            .iter()
            .enumerate()
            .filter_map(|(index, group)| (group.group_name.as_ref() == group_name).then_some(index))
            .collect()
    }

    /// Whether one frozen repair source declares a configured constraint.
    pub fn declares_constraint(&self, handle: RuntimeProviderHandle, constraint: &str) -> bool {
        match handle {
            RuntimeProviderHandle::CallbackRepair(index) => self
                .repairs
                .get(index)
                .unwrap_or_else(|| {
                    panic!("compiled callback repair provider index {index} no longer exists")
                })
                .declared_constraints
                .iter()
                .any(|name| name.as_ref() == constraint),
            RuntimeProviderHandle::StaticRepair(index) => {
                self.static_repairs
                    .get(index)
                    .unwrap_or_else(|| {
                        panic!("compiled static repair provider index {index} no longer exists")
                    })
                    .repair
                    .constraint_name()
                    == constraint
            }
            RuntimeProviderHandle::CallbackGroup(_) | RuntimeProviderHandle::StaticGroup(_) => {
                false
            }
        }
    }

    pub fn declares_any_constraint(
        &self,
        handle: RuntimeProviderHandle,
        constraints: &[String],
    ) -> bool {
        constraints
            .iter()
            .any(|constraint| self.declares_constraint(handle, constraint))
    }

    pub(crate) fn freeze(
        &mut self,
        scalar_slots: &[RuntimeScalarSlot<S>],
        scalar_groups: &[ScalarGroupBinding<S>],
        conflict_repairs: &[ConflictRepair<S>],
    ) -> Result<(), String> {
        self.static_groups.clear();
        self.static_repairs.clear();
        for (declared_index, group) in scalar_groups.iter().enumerate() {
            let ScalarGroupBindingKind::Candidates { candidate_provider } = &group.kind else {
                continue;
            };
            self.static_groups.push(StaticScalarGroupProviderBinding {
                declared_index,
                group_name: group.group_name,
                provider: *candidate_provider,
                declared_limits: group.limits,
            });
        }
        for (declared_index, repair) in conflict_repairs.iter().enumerate() {
            self.static_repairs
                .push(StaticConflictRepairProviderBinding {
                    declared_index,
                    repair: *repair,
                });
        }
        if self.is_empty() {
            return Ok(());
        }
        self.resolver = Some(RuntimeProviderSlotResolver::new(scalar_slots.to_vec())?);
        Ok(())
    }

    fn resolver(&self) -> &RuntimeProviderSlotResolver<S> {
        self.resolver.as_ref().unwrap_or_else(|| {
            panic!("runtime provider registry was invoked before descriptor resolution")
        })
    }

    /// Pulls exactly one source without normalizing or reordering its result.
    pub fn pull_callback_raw(
        &self,
        handle: RuntimeProviderHandle,
        solution: &S,
        limits: RuntimeProviderLimits,
    ) -> Vec<RawProviderCandidate> {
        let _ = self.resolver();
        let callback = match handle {
            RuntimeProviderHandle::CallbackGroup(index) => self
                .groups
                .get(index)
                .unwrap_or_else(|| {
                    panic!("compiled callback group provider index {index} no longer exists")
                })
                .callback
                .as_ref(),
            RuntimeProviderHandle::CallbackRepair(index) => self
                .repairs
                .get(index)
                .unwrap_or_else(|| {
                    panic!("compiled callback repair provider index {index} no longer exists")
                })
                .callback
                .as_ref(),
            RuntimeProviderHandle::StaticGroup(_) | RuntimeProviderHandle::StaticRepair(_) => {
                panic!("static providers must use their concrete pull path")
            }
        };
        callback.pull(solution, limits)
    }

    pub(crate) fn pull_static_group(
        &self,
        index: usize,
        solution: &S,
        value_candidate_limit: Option<usize>,
        max_moves_per_step: Option<usize>,
    ) -> Vec<ScalarCandidate<S>> {
        let _ = self.resolver();
        let binding = self.static_groups.get(index).unwrap_or_else(|| {
            panic!("compiled static group provider index {index} no longer exists")
        });
        pull_static_group(binding, solution, value_candidate_limit, max_moves_per_step)
    }

    pub(crate) fn pull_static_repair(
        &self,
        index: usize,
        solution: &S,
        limits: RepairLimits,
    ) -> Vec<RepairCandidate<S>> {
        let _ = self.resolver();
        let binding = self.static_repairs.get(index).unwrap_or_else(|| {
            panic!("compiled static repair provider index {index} no longer exists")
        });
        pull_static_repair(binding, solution, limits)
    }

    /// Resolves one raw result in the caller-owned deduplication scope.
    /// Callback panics propagate from [`Self::pull_callback_raw`]; only structured core
    /// normalization failures cross the host error boundary here.
    pub fn normalize_or_raise(
        &self,
        solution: &S,
        raw: Vec<RawProviderCandidate>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Vec<ResolvedProviderCandidate<S>> {
        match self.resolver().resolve_and_normalize_with_state(
            solution,
            raw,
            allowed_slots,
            state,
            reasons,
        ) {
            Ok(candidates) => candidates,
            Err(error) => self.error_boundary.raise(error),
        }
    }

    pub(crate) fn normalize_static_group(
        &self,
        solution: &S,
        candidates: Vec<ScalarCandidate<S>>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Vec<ResolvedProviderCandidate<S>> {
        self.resolver()
            .resolve_static_group_and_normalize_with_state(
                solution,
                candidates,
                allowed_slots,
                state,
                reasons,
            )
            .unwrap_or_else(|error| panic!("{error}"))
    }

    pub(crate) fn normalize_static_repair(
        &self,
        solution: &S,
        candidates: Vec<RepairCandidate<S>>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Vec<ResolvedProviderCandidate<S>> {
        self.resolver()
            .resolve_static_repair_and_normalize_with_state(
                solution,
                candidates,
                allowed_slots,
                state,
                reasons,
            )
            .unwrap_or_else(|error| panic!("{error}"))
    }
}
