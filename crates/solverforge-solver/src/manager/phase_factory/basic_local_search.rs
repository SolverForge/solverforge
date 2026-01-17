//! Basic local search phase for improving assignments.
//!
//! Provides late acceptance local search for basic variables.
//!
//! # Zero-Erasure Design
//!
//! Uses VariableOperations trait for value access. No function pointers required.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::operations::VariableOperations;
use crate::phase::localsearch::{
    AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchPhase,
};

use super::super::SolverPhaseFactory;

/// Type alias for the move selector used by BasicLocalSearchPhase.
type BasicMoveSelector<S> = ChangeMoveSelector<S, FromSolutionEntitySelector>;

/// Type alias for the concrete local search phase with late acceptance.
pub type BasicLocalSearchPhase<S> = LocalSearchPhase<
    S,
    ChangeMove<S>,
    BasicMoveSelector<S>,
    LateAcceptanceAcceptor<S>,
    AcceptedCountForager<S, ChangeMove<S>>,
>;

/// Builder for creating basic variable local search phases.
///
/// This builder creates phases that improve solutions by changing basic
/// planning variable values using late acceptance.
///
/// # Type Parameters
///
/// * `S` - The planning solution type (must implement VariableOperations)
pub struct BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    variable_name: &'static str,
    descriptor_index: usize,
    late_acceptance_size: usize,
    _marker: PhantomData<fn() -> S>,
}

impl<S> BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    /// Creates a new basic local search phase builder.
    ///
    /// # Arguments
    /// * `variable_name` - Name of the planning variable
    /// * `descriptor_index` - Index of the entity descriptor
    /// * `late_acceptance_size` - Size of the late acceptance buffer
    pub fn new(
        variable_name: &'static str,
        descriptor_index: usize,
        late_acceptance_size: usize,
    ) -> Self {
        Self {
            variable_name,
            descriptor_index,
            late_acceptance_size,
            _marker: PhantomData,
        }
    }
}

impl<S> BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution + VariableOperations,
{
    /// Creates the local search phase.
    pub fn create_phase(&self) -> BasicLocalSearchPhase<S> {
        let entity_selector = FromSolutionEntitySelector::new(self.descriptor_index);

        let move_selector = ChangeMoveSelector::new(
            entity_selector,
            self.variable_name,
            self.descriptor_index,
        );

        let acceptor = LateAcceptanceAcceptor::new(self.late_acceptance_size);
        let forager = AcceptedCountForager::new(1);

        LocalSearchPhase::new(move_selector, acceptor, forager, None)
    }
}

impl<S, D> SolverPhaseFactory<S, D, BasicLocalSearchPhase<S>> for BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution + VariableOperations,
    D: ScoreDirector<S>,
{
    fn create_phase(&self) -> BasicLocalSearchPhase<S> {
        BasicLocalSearchPhaseBuilder::create_phase(self)
    }
}
