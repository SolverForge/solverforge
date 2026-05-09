#[path = "custom_search/domain/mod.rs"]
mod domain;

use domain::*;

fn main() {
    let _ = Plan {
        workers: Vec::new(),
        tasks: Vec::new(),
        score: None,
    };
}
