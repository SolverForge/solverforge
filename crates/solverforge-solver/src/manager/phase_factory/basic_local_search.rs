//! Basic local search phase for improving assignments.
//!
//! Provides late acceptance local search for basic variables.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::typed_value::FromSolutionTypedValueSelector;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::heuristic::MoveSelector;
use crate::phase::localsearch::{
    AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchForager, LocalSearchPhase,
};
use crate::phase::Phase;

use super::super::config::LocalSearchType;
use super::super::SolverPhaseFactory;

/// Builder for creating basic variable local search phases.
///
/// This builder creates phases that improve solutions by changing basic
/// planning variable values using late acceptance.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{BasicLocalSearchPhaseBuilder, SolverPhaseFactory, LocalSearchType};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Shift { employee_idx: Option<usize> }
///
/// #[derive(Clone)]
/// struct Schedule { shifts: Vec<Shift>, employees: Vec<()>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_employee(s: &Schedule, idx: usize) -> Option<usize> {
///     s.shifts.get(idx).and_then(|shift| shift.employee_idx)
/// }
///
/// fn set_employee(s: &mut Schedule, idx: usize, v: Option<usize>) {
///     if let Some(shift) = s.shifts.get_mut(idx) {
///         shift.employee_idx = v;
///     }
/// }
///
/// fn value_count(s: &Schedule) -> usize { s.employees.len() }
///
/// let factory = BasicLocalSearchPhaseBuilder::<Schedule>::new(
///     get_employee,
///     set_employee,
///     value_count,
///     "employee_idx",
///     0,
///     LocalSearchType::LateAcceptance { size: 400 },
/// );
/// ```
pub struct BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    /// Typed getter for the planning variable
    getter: fn(&S, usize) -> Option<usize>,
    /// Typed setter for the planning variable
    setter: fn(&mut S, usize, Option<usize>),
    /// Returns the number of valid values (0..value_count)
    value_count: fn(&S) -> usize,
    /// Variable name for change notification
    variable_name: &'static str,
    /// Descriptor index for entity collection
    descriptor_index: usize,
    /// Local search algorithm type
    search_type: LocalSearchType,
    _marker: PhantomData<S>,
}

impl<S> BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    /// Creates a new basic local search phase builder.
    ///
    /// # Arguments
    ///
    /// * `getter` - Function to get the variable value for an entity
    /// * `setter` - Function to set the variable value for an entity
    /// * `value_count` - Function returning the number of valid values
    /// * `variable_name` - Name of the variable for change notification
    /// * `descriptor_index` - Entity descriptor index
    /// * `search_type` - Local search algorithm configuration
    pub fn new(
        getter: fn(&S, usize) -> Option<usize>,
        setter: fn(&mut S, usize, Option<usize>),
        value_count: fn(&S) -> usize,
        variable_name: &'static str,
        descriptor_index: usize,
        search_type: LocalSearchType,
    ) -> Self {
        Self {
            getter,
            setter,
            value_count,
            variable_name,
            descriptor_index,
            search_type,
            _marker: PhantomData,
        }
    }
}

impl<S> SolverPhaseFactory<S> for BasicLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        let getter = self.getter;
        let setter = self.setter;
        let value_count = self.value_count;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        // Create move selector with dynamic values from solution
        let entity_selector = FromSolutionEntitySelector::new(descriptor_index);
        let value_selector =
            FromSolutionTypedValueSelector::new(move |sd| (0..(value_count)(sd.working_solution())).collect());

        let move_selector = ChangeMoveSelector::<S, usize>::new(
            Box::new(entity_selector),
            Box::new(value_selector),
            getter,
            setter,
            descriptor_index,
            variable_name,
        );

        // Create acceptor based on search type
        let (acceptor, forager): (
            Box<dyn crate::phase::localsearch::Acceptor<S>>,
            Box<dyn LocalSearchForager<S, ChangeMove<S, usize>>>,
        ) = match self.search_type {
            LocalSearchType::LateAcceptance { size } => (
                Box::new(LateAcceptanceAcceptor::new(size)),
                Box::new(AcceptedCountForager::new(1)),
            ),
            LocalSearchType::HillClimbing => (
                Box::new(crate::phase::localsearch::HillClimbingAcceptor::new()),
                Box::new(AcceptedCountForager::new(1)),
            ),
            LocalSearchType::TabuSearch { tabu_size } => (
                Box::new(crate::phase::localsearch::TabuSearchAcceptor::new(
                    tabu_size,
                )),
                Box::new(AcceptedCountForager::new(1)),
            ),
            LocalSearchType::SimulatedAnnealing {
                starting_temp,
                decay_rate,
            } => (
                Box::new(crate::phase::localsearch::SimulatedAnnealingAcceptor::new(
                    starting_temp,
                    decay_rate,
                )),
                Box::new(AcceptedCountForager::new(1)),
            ),
            _ => {
                // Other types (KOpt, ValueTabu, MoveTabu) - fall back to late acceptance
                (
                    Box::new(LateAcceptanceAcceptor::new(400)),
                    Box::new(AcceptedCountForager::new(1)),
                )
            }
        };

        Box::new(LocalSearchPhase::new(
            Box::new(move_selector) as Box<dyn MoveSelector<S, ChangeMove<S, usize>>>,
            acceptor,
            forager,
            None, // No step limit - termination via solver config
        ))
    }
}
