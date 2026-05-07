#[path = "field_only_solution/domain/mod.rs"]
mod domain;

use domain::*;
use solverforge::stream::CollectionExtract;

fn main() {
    let plan = Plan {
        routes: Vec::new(),
        visits: Vec::new(),
        score: None,
    };

    let _ = Plan::routes().extract(&plan);
    let _ = Plan::visits().extract(&plan);
}
