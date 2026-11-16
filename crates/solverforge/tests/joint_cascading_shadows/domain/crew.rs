use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Technician, Visit};

/// Technicians whose jobs may need several of them at once.
#[planning_solution(constraints = "define_constraints", config = "crew_config")]
#[shadow_variable_updates(
    list_owner = "technicians",
    inverse_field = "technician",
    previous_field = "previous",
    next_field = "next",
    cascading_field = "start",
    cascading_compute = "compute_start",
    cascading_peers = "visit_partners"
)]
pub struct Crew {
    /// The technicians.
    #[planning_entity_collection]
    pub technicians: Vec<Technician>,

    /// The visits the technicians make.
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    /// The crew's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Crew {
    // When the technician can be at `visit`: one unit of travel after the
    // previous visit ends.
    fn arrival(&self, visit: usize) -> Option<i64> {
        match self.visits[visit].previous {
            Some(previous) => {
                let previous = &self.visits[previous];
                Some(previous.start? + previous.duration + 1)
            }
            None => Some(0),
        }
    }

    /// A visit starts when the last assigned technician of its job arrives.
    pub fn compute_start(&self, visit: usize) -> Option<i64> {
        let mut start = self.arrival(visit)?;
        for &partner in &self.visits[visit].partners {
            if self.visits[partner].technician.is_some() {
                start = start.max(self.arrival(partner)?);
            }
        }
        Some(start)
    }

    /// The visits started together with `visit`.
    pub fn visit_partners(&self, visit: usize) -> Vec<usize> {
        self.visits[visit].partners.clone()
    }
}

/// Unassigned and unscheduled visits are hard; every visit's end is soft,
/// so any stale start changes the score.
pub fn define_constraints() -> impl ConstraintSet<Crew, HardSoftScore> {
    (
        ConstraintFactory::<Crew, HardSoftScore>::new()
            .for_each(Crew::visits())
            .filter(|visit: &Visit| visit.start.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unscheduled visit"),
        ConstraintFactory::<Crew, HardSoftScore>::new()
            .for_each(Crew::visits())
            .filter(|visit: &Visit| visit.start.is_some())
            .penalize(|visit: &Visit| {
                HardSoftScore::of_soft(visit.start.unwrap_or(0) + visit.duration)
            })
            .named("visit end"),
    )
}

// Full assert checks every incremental score against a fresh calculation.
const CREW_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 9

[[phases]]
type = "construction_heuristic"

[[phases]]
type = "local_search"

[phases.termination]
step_count_limit = 80
"#;

fn crew_config(_solution: &Crew, _base: SolverConfig) -> SolverConfig {
    SolverConfig::from_toml_str(CREW_TOML).expect("crew fixture config must parse")
}
