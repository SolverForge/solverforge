//! Change local search phase for improving assignments.
//!
//! Provides late acceptance local search using change moves.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::typed_value::RangeValueSelector;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::phase::localsearch::{AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchPhase};

use super::super::PhaseFactory;

/// Type alias for the move selector used by ChangeLocalSearchPhase.
type ChangeMoveSelector_<S> =
    ChangeMoveSelector<S, usize, FromSolutionEntitySelector, RangeValueSelector<S>>;

/// Type alias for the concrete local search phase with late acceptance.
pub type ChangeLocalSearchPhase<S> = LocalSearchPhase<
    S,
    ChangeMove<S, usize>,
    ChangeMoveSelector_<S>,
    LateAcceptanceAcceptor<S>,
    AcceptedCountForager<S>,
>;

/// Builder for creating change local search phases.
///
/// This builder creates phases that improve solutions by changing
/// planning variable values using late acceptance.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::ChangeLocalSearchPhaseBuilder;
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
/// let builder = ChangeLocalSearchPhaseBuilder::<Schedule>::new(
///     get_employee,
///     set_employee,
///     value_count,
///     "employee_idx",
///     0,
///     400, // late acceptance size
/// );
/// ```
pub struct ChangeLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    getter: fn(&S, usize) -> Option<usize>,
    setter: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    late_acceptance_size: usize,
    _marker: PhantomData<S>,
}

impl<S> ChangeLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution,
{
    /// Creates a new change local search phase builder.
    pub fn new(
        getter: fn(&S, usize) -> Option<usize>,
        setter: fn(&mut S, usize, Option<usize>),
        value_count: fn(&S) -> usize,
        variable_name: &'static str,
        descriptor_index: usize,
        late_acceptance_size: usize,
    ) -> Self {
        Self {
            getter,
            setter,
            value_count,
            variable_name,
            descriptor_index,
            late_acceptance_size,
            _marker: PhantomData,
        }
    }

    /// Creates the local search phase.
    pub fn create_phase(&self) -> ChangeLocalSearchPhase<S> {
        let entity_selector = FromSolutionEntitySelector::new(self.descriptor_index);
        let value_selector = RangeValueSelector::new(self.value_count);

        let move_selector = ChangeMoveSelector::new(
            entity_selector,
            value_selector,
            self.getter,
            self.setter,
            self.descriptor_index,
            self.variable_name,
        );

        let acceptor = LateAcceptanceAcceptor::new(self.late_acceptance_size);
        let forager = AcceptedCountForager::new(1);

        LocalSearchPhase::new(move_selector, acceptor, forager, None)
    }
}

impl<S, C> PhaseFactory<S, C> for ChangeLocalSearchPhaseBuilder<S>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    type Phase = ChangeLocalSearchPhase<S>;

    fn create(&self) -> Self::Phase {
        ChangeLocalSearchPhaseBuilder::create_phase(self)
    }
}
