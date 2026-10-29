use solverforge::prelude::*;
use solverforge_macros::planning_solution;

#[planning_solution]
struct Plan(#[planning_score] Option<HardSoftScore>);

fn main() {}
