#[path = "multi_list_owners/domain/mod.rs"]
mod domain;

use domain::*;
use solverforge::stream::CollectionExtract;

fn main() {
    let plan = Plan {
        routes: Vec::new(),
        shifts: Vec::new(),
        route_tasks: Vec::new(),
        shift_tasks: Vec::new(),
        score: None,
    };

    let _ = Plan::routes().extract(&plan);
    let _ = Plan::shifts().extract(&plan);
    let _ = Plan::route_tasks().extract(&plan);
    let _ = Plan::shift_tasks().extract(&plan);
}
