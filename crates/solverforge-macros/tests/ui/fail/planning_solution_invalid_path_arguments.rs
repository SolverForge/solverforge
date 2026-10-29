use solverforge_macros::planning_solution;

#[planning_solution(constraints = "crate::constraints::create constraints")]
struct Plan {
    #[planning_score]
    score: Option<HardSoftScore>,
}

#[planning_solution(config = "crate::my config")]
struct ConfigPlan {
    #[planning_score]
    score: Option<HardSoftScore>,
}

#[planning_solution(conflict_repairs = "repairs()")]
struct RepairPlan {
    #[planning_score]
    score: Option<HardSoftScore>,
}

#[planning_solution(scalar_groups = "groups-fn")]
struct GroupPlan {
    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {}
