#[path = "basic_macros/domain/mod.rs"]
mod domain;

use domain::*;

fn main() {
    let plan = Plan {
        tasks: vec![Task {
            id: "pinned".into(),
            pinned: true,
            worker_idx: Some(0),
        }],
        workers: Vec::new(),
        score: None,
    };
    assert!(Plan::descriptor().entity_descriptors[0].is_pinned(&plan, 0));
}
