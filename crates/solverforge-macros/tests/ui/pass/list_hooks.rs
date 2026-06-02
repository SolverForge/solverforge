#[path = "list_hooks/domain/mod.rs"]
mod domain;

use domain::*;
use solverforge::stream::CollectionExtract;

fn main() {
    let plan = Plan {
        operations: vec![Operation {
            id: 0,
            route_id: 0,
            duration: 3,
            next: None,
        }],
        routes: vec![Route {
            id: 0,
            operations: Vec::new(),
        }],
        score: None,
    };

    let _ = Plan::operations().extract(&plan);
    let _ = Plan::routes().extract(&plan);
}
