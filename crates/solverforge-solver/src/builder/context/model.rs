use std::fmt;
use std::marker::PhantomData;

use super::{
    ConflictRepair, CoverageGroupBinding, ListVariableSlot, ScalarGroupBinding, ScalarVariableSlot,
};

pub enum VariableSlot<S, V, DM, IDM> {
    Scalar(ScalarVariableSlot<S>),
    List(ListVariableSlot<S, V, DM, IDM>),
}

impl<S, V, DM: Clone, IDM: Clone> Clone for VariableSlot<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(variable) => Self::Scalar(*variable),
            Self::List(variable) => Self::List(variable.clone()),
        }
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for VariableSlot<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(variable) => variable.fmt(f),
            Self::List(variable) => variable.fmt(f),
        }
    }
}

pub struct RuntimeModel<S, V, DM, IDM> {
    variables: Vec<VariableSlot<S, V, DM, IDM>>,
    scalar_groups: Vec<ScalarGroupBinding<S>>,
    coverage_groups: Vec<CoverageGroupBinding<S>>,
    conflict_repairs: Vec<ConflictRepair<S>>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM: Clone, IDM: Clone> Clone for RuntimeModel<S, V, DM, IDM> {
    fn clone(&self) -> Self {
        let mut coverage_groups = Vec::with_capacity(self.coverage_groups.len());
        coverage_groups.extend(self.coverage_groups.iter().copied());
        Self {
            variables: self.variables.clone(),
            scalar_groups: self.scalar_groups.clone(),
            coverage_groups,
            conflict_repairs: self.conflict_repairs.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM, IDM> RuntimeModel<S, V, DM, IDM> {
    pub fn new(variables: Vec<VariableSlot<S, V, DM, IDM>>) -> Self {
        Self {
            variables,
            scalar_groups: Vec::new(),
            coverage_groups: Vec::new(),
            conflict_repairs: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_scalar_groups(mut self, groups: Vec<ScalarGroupBinding<S>>) -> Self {
        self.scalar_groups = groups;
        self
    }

    pub fn with_coverage_groups(mut self, groups: Vec<CoverageGroupBinding<S>>) -> Self {
        self.coverage_groups = groups;
        self
    }

    pub fn with_conflict_repairs(mut self, repairs: Vec<ConflictRepair<S>>) -> Self {
        self.conflict_repairs = repairs;
        self
    }

    pub fn variables(&self) -> &[VariableSlot<S, V, DM, IDM>] {
        &self.variables
    }

    pub fn scalar_groups(&self) -> &[ScalarGroupBinding<S>] {
        &self.scalar_groups
    }

    pub fn coverage_groups(&self) -> &[CoverageGroupBinding<S>] {
        &self.coverage_groups
    }

    pub fn conflict_repairs(&self) -> &[ConflictRepair<S>] {
        &self.conflict_repairs
    }

    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    pub fn has_list_variables(&self) -> bool {
        self.variables
            .iter()
            .any(|variable| matches!(variable, VariableSlot::List(_)))
    }

    pub fn scalar_variables(&self) -> impl Iterator<Item = &ScalarVariableSlot<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::Scalar(ctx) => Some(ctx),
            VariableSlot::List(_) => None,
        })
    }

    pub fn list_variables(&self) -> impl Iterator<Item = &ListVariableSlot<S, V, DM, IDM>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::List(ctx) => Some(ctx),
            VariableSlot::Scalar(_) => None,
        })
    }
}

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for RuntimeModel<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeModel")
            .field("variables", &self.variables)
            .field("scalar_groups", &self.scalar_groups)
            .field("coverage_groups", &self.coverage_groups)
            .field("conflict_repairs", &self.conflict_repairs)
            .finish()
    }
}
