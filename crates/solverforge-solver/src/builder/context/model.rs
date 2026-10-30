use std::fmt;
use std::marker::PhantomData;

use solverforge_core::domain::{DynamicListVariableSlot, DynamicScalarVariableSlot};

use super::{
    ConflictRepair, ListVariableSlot, RuntimeCandidateMetricRegistry, RuntimeProviderRegistry,
    ScalarGroupBinding, ScalarVariableSlot,
};

/// One planning variable of a [`RuntimeModel`].
pub enum VariableSlot<S, V, DM, IDM> {
    /// A typed scalar variable.
    Scalar(ScalarVariableSlot<S>),
    /// A typed list variable.
    List(ListVariableSlot<S, V, DM, IDM>),
    /// A scalar variable declared at runtime by a host-language binding.
    DynamicScalar(DynamicScalarVariableSlot<S>),
    /// A list variable declared at runtime by a host-language binding.
    DynamicList(DynamicListVariableSlot<S>),
}

impl<S, V, DM: Clone, IDM: Clone> Clone for VariableSlot<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(variable) => Self::Scalar(*variable),
            Self::List(variable) => Self::List(variable.clone()),
            Self::DynamicScalar(variable) => Self::DynamicScalar(variable.clone()),
            Self::DynamicList(variable) => Self::DynamicList(variable.clone()),
        }
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for VariableSlot<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(variable) => variable.fmt(f),
            Self::List(variable) => variable.fmt(f),
            Self::DynamicScalar(variable) => variable.fmt(f),
            Self::DynamicList(variable) => variable.fmt(f),
        }
    }
}

/// Every planning variable of a model plus its scalar groups and conflict
/// repairs: the input the runtime compiles phases from.
pub struct RuntimeModel<S, V, DM, IDM> {
    pub(super) variables: Vec<VariableSlot<S, V, DM, IDM>>,
    pub(super) scalar_groups: Vec<ScalarGroupBinding<S>>,
    pub(super) conflict_repairs: Vec<ConflictRepair<S>>,
    pub(super) runtime_provider_registry: RuntimeProviderRegistry<S>,
    pub(super) candidate_metrics: RuntimeCandidateMetricRegistry<S>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for RuntimeModel<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            scalar_groups: self.scalar_groups.clone(),
            conflict_repairs: self.conflict_repairs.clone(),
            runtime_provider_registry: self.runtime_provider_registry.clone(),
            candidate_metrics: self.candidate_metrics.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> RuntimeModel<S, V, DM, IDM> {
    /// Creates a model over `variables` with no scalar groups, conflict repairs,
    /// host providers or candidate metrics.
    pub fn new(variables: Vec<VariableSlot<S, V, DM, IDM>>) -> Self {
        Self {
            variables,
            scalar_groups: Vec::new(),
            conflict_repairs: Vec::new(),
            runtime_provider_registry: RuntimeProviderRegistry::default(),
            candidate_metrics: RuntimeCandidateMetricRegistry::default(),
            _phantom: PhantomData,
        }
    }

    /// Replaces the scalar groups.
    pub fn with_scalar_groups(mut self, groups: Vec<ScalarGroupBinding<S>>) -> Self {
        self.scalar_groups = groups;
        self
    }

    /// Replaces the conflict repairs.
    pub fn with_conflict_repairs(mut self, repairs: Vec<ConflictRepair<S>>) -> Self {
        self.conflict_repairs = repairs;
        self
    }

    /// Attaches the immutable host-language generic-provider registry.  It is
    /// frozen against resolved dynamic slot identities during model resolution
    /// and is never consulted through a mutable schema/TLS lookup at solve
    /// time. Host-language bindings reach the registry type through
    /// `solverforge_bridge`.
    #[doc(hidden)]
    pub fn with_runtime_provider_registry(mut self, registry: RuntimeProviderRegistry<S>) -> Self {
        self.runtime_provider_registry = registry;
        self
    }

    /// Attaches the host-language candidate metrics that `Sorted` and
    /// `Probabilistic` selector ordering read. Host-language bindings reach
    /// the registry type through `solverforge_bridge`.
    #[doc(hidden)]
    pub fn with_candidate_metrics(mut self, metrics: RuntimeCandidateMetricRegistry<S>) -> Self {
        self.candidate_metrics = metrics;
        self
    }

    /// Every variable, in declaration order.
    pub fn variables(&self) -> &[VariableSlot<S, V, DM, IDM>] {
        &self.variables
    }

    /// The bound scalar groups.
    pub fn scalar_groups(&self) -> &[ScalarGroupBinding<S>] {
        &self.scalar_groups
    }

    /// The conflict repairs.
    pub fn conflict_repairs(&self) -> &[ConflictRepair<S>] {
        &self.conflict_repairs
    }

    pub(crate) fn runtime_provider_registry(&self) -> &RuntimeProviderRegistry<S> {
        &self.runtime_provider_registry
    }

    pub(crate) fn candidate_metrics(&self) -> &RuntimeCandidateMetricRegistry<S> {
        &self.candidate_metrics
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for RuntimeModel<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeModel")
            .field("variables", &self.variables)
            .field("scalar_groups", &self.scalar_groups)
            .field("conflict_repairs", &self.conflict_repairs)
            .field("runtime_provider_registry", &self.runtime_provider_registry)
            .field("candidate_metrics", &self.candidate_metrics)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use solverforge_core::domain::{
        DynamicListVariableSlot, DynamicModelBackend, DynamicScalarVariableSlot, EntityClassId,
        VariableId,
    };
    use solverforge_core::score::SoftScore;

    use super::{RuntimeModel, VariableSlot};

    #[derive(Clone)]
    struct DynamicRows;

    impl DynamicModelBackend for DynamicRows {
        type Score = SoftScore;

        fn entity_count(&self, _entity: EntityClassId) -> usize {
            0
        }

        fn get_scalar(
            &self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
        ) -> Option<usize> {
            None
        }

        fn set_scalar(
            &mut self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
            _value: Option<usize>,
        ) {
        }

        fn list_len(&self, _entity: EntityClassId, _row: usize, _variable: VariableId) -> usize {
            0
        }

        fn list_get(
            &self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
            _pos: usize,
        ) -> Option<usize> {
            None
        }

        fn list_insert(
            &mut self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
            _pos: usize,
            _value: usize,
        ) {
        }

        fn list_remove(
            &mut self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
            _pos: usize,
        ) -> Option<usize> {
            None
        }

        fn candidate_values(
            &self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
        ) -> &[usize] {
            &[]
        }

        fn scalar_value_is_legal(
            &self,
            _entity: EntityClassId,
            _row: usize,
            _variable: VariableId,
            _value: usize,
        ) -> bool {
            false
        }
    }

    #[test]
    fn runtime_model_tracks_dynamic_variable_slots() {
        let scalar =
            DynamicScalarVariableSlot::new(EntityClassId(0), VariableId(0), "Task", "worker", true);
        let list =
            DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits");

        let model: RuntimeModel<DynamicRows, usize, (), ()> = RuntimeModel::new(vec![
            VariableSlot::DynamicScalar(scalar.clone()),
            VariableSlot::DynamicList(list.clone()),
        ]);

        assert!(model.has_scalar_variables());
        assert!(model.has_list_variables());
        assert_eq!(
            model
                .dynamic_scalar_variables()
                .map(|slot| slot.variable_name)
                .collect::<Vec<_>>(),
            vec!["worker"]
        );
        assert_eq!(
            model
                .dynamic_list_variables()
                .map(|slot| slot.variable_name)
                .collect::<Vec<_>>(),
            vec!["visits"]
        );
        assert_eq!(model.scalar_variables().count(), 0);
        assert_eq!(model.list_variables().count(), 0);
    }

    #[test]
    #[should_panic(
        expected = "dynamic scalar variable Task.worker has not been resolved against a SolutionDescriptor"
    )]
    fn runtime_model_rejects_unresolved_dynamic_slots_for_selector_use() {
        let scalar =
            DynamicScalarVariableSlot::new(EntityClassId(0), VariableId(0), "Task", "worker", true);
        let model: RuntimeModel<DynamicRows, usize, (), ()> =
            RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);

        model.assert_dynamic_descriptor_indexes_resolved();
    }
}
