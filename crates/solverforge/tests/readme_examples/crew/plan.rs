use solverforge::prelude::*;

use super::{Technician, Visit};

// readme:begin
#[planning_solution]
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
    #[planning_entity_collection]
    pub technicians: Vec<Technician>,

    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Crew {
    // When the technician can be at the visit.
    fn arrival(&self, visit: usize) -> Option<i64> {
        match self.visits[visit].previous {
            Some(previous) => {
                let left = self.visits[previous].start? + self.visits[previous].duration;
                Some(left + self.visits[visit].travel_from[previous])
            }
            None => Some(0),
        }
    }

    // When the last assigned technician of the job arrives.
    pub fn compute_start(&self, visit: usize) -> Option<i64> {
        let mut start = self.arrival(visit)?;
        for &partner in &self.visits[visit].partners {
            if self.visits[partner].technician.is_some() {
                start = start.max(self.arrival(partner)?);
            }
        }
        Some(start)
    }

    pub fn visit_partners(&self, visit: usize) -> Vec<usize> {
        self.visits[visit].partners.clone()
    }
}
// readme:end
