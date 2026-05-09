use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

pub struct PhaseSequence<P> {
    phases: Vec<P>,
}

impl<P> PhaseSequence<P> {
    pub fn new(phases: Vec<P>) -> Self {
        Self { phases }
    }

    pub fn phases(&self) -> &[P] {
        &self.phases
    }

    pub fn into_phases(self) -> Vec<P> {
        self.phases
    }
}

impl<P: Debug> Debug for PhaseSequence<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PhaseSequence")
            .field("phases", &self.phases)
            .finish()
    }
}

impl<S, D, ProgressCb, P> Phase<S, D, ProgressCb> for PhaseSequence<P>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    P: Phase<S, D, ProgressCb>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        for phase in &mut self.phases {
            phase.solve(solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseSequence"
    }
}
