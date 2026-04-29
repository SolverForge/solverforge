use std::fmt;
use std::marker::PhantomData;

use super::{
    ConflictRepairProviderEntry, ListVariableContext, ScalarGroupContext, ScalarVariableContext,
};

pub enum VariableContext<S, V, DM, IDM> {
    Scalar(ScalarVariableContext<S>),
    List(ListVariableContext<S, V, DM, IDM>),
}

impl<S, V, DM: Clone, IDM: Clone> Clone for VariableContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(variable) => Self::Scalar(*variable),
            Self::List(variable) => Self::List(variable.clone()),
        }
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for VariableContext<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(variable) => variable.fmt(f),
            Self::List(variable) => variable.fmt(f),
        }
    }
}

pub struct ModelContext<S, V, DM, IDM> {
    variables: Vec<VariableContext<S, V, DM, IDM>>,
    scalar_groups: Vec<ScalarGroupContext<S>>,
    conflict_repair_providers: Vec<ConflictRepairProviderEntry<S>>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for ModelContext<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            scalar_groups: self.scalar_groups.clone(),
            conflict_repair_providers: self.conflict_repair_providers.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> ModelContext<S, V, DM, IDM> {
    pub fn new(variables: Vec<VariableContext<S, V, DM, IDM>>) -> Self {
        Self {
            variables,
            scalar_groups: Vec::new(),
            conflict_repair_providers: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_scalar_groups(mut self, groups: Vec<ScalarGroupContext<S>>) -> Self {
        self.scalar_groups = groups;
        self
    }

    pub fn with_conflict_repair_providers(
        mut self,
        providers: Vec<ConflictRepairProviderEntry<S>>,
    ) -> Self {
        self.conflict_repair_providers = providers;
        self
    }

    pub fn variables(&self) -> &[VariableContext<S, V, DM, IDM>] {
        &self.variables
    }

    pub fn scalar_groups(&self) -> &[ScalarGroupContext<S>] {
        &self.scalar_groups
    }

    pub fn conflict_repair_providers(&self) -> &[ConflictRepairProviderEntry<S>] {
        &self.conflict_repair_providers
    }

    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    pub fn has_list_variables(&self) -> bool {
        self.variables
            .iter()
            .any(|variable| matches!(variable, VariableContext::List(_)))
    }

    pub fn scalar_variables(&self) -> impl Iterator<Item = &ScalarVariableContext<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableContext::Scalar(ctx) => Some(ctx),
            VariableContext::List(_) => None,
        })
    }

    pub fn list_variables(&self) -> impl Iterator<Item = &ListVariableContext<S, V, DM, IDM>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableContext::List(ctx) => Some(ctx),
            VariableContext::Scalar(_) => None,
        })
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for ModelContext<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelContext")
            .field("variables", &self.variables)
            .field("scalar_groups", &self.scalar_groups)
            .field("conflict_repair_providers", &self.conflict_repair_providers)
            .finish()
    }
}
