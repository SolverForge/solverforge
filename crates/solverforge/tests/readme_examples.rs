//! Every ```rust block in README.md is compiled. Each block is checked in
//! verbatim between `// readme:begin` and `// readme:end` lines in a file
//! under `readme_examples/`, inside whatever context it needs to compile, and
//! `every_readme_rust_block_is_compiled` fails as soon as README.md and
//! those files disagree in either direction.

// README code is shown as written: shadowed `let`s, ignored results.
#![allow(dead_code, unused_variables, unused_must_use, unused_mut)]

#[path = "readme_examples/nurses/mod.rs"]
mod nurses;
#[path = "readme_examples/routes/mod.rs"]
mod routes;
#[path = "readme_examples/staffing/mod.rs"]
mod staffing;
#[path = "readme_examples/weighted/mod.rs"]
mod weighted;

#[rustfmt::skip]
#[path = "readme_examples/configured.rs"]
mod configured;
#[rustfmt::skip]
#[path = "readme_examples/manager.rs"]
mod manager;
#[rustfmt::skip]
#[path = "readme_examples/night_shift.rs"]
mod night_shift;
#[rustfmt::skip]
#[path = "readme_examples/scores.rs"]
mod scores;
#[rustfmt::skip]
#[path = "readme_examples/streams.rs"]
mod streams;
#[rustfmt::skip]
#[path = "readme_examples/weights.rs"]
mod weights;

use solverforge::prelude::*;
use staffing::{Assignment, Employee, Schedule, Shift};

const README: &str = include_str!("../../../README.md");

const SOURCES: &[(&str, &str)] = &[
    (
        "configured.rs",
        include_str!("readme_examples/configured.rs"),
    ),
    ("manager.rs", include_str!("readme_examples/manager.rs")),
    (
        "night_shift.rs",
        include_str!("readme_examples/night_shift.rs"),
    ),
    (
        "nurses/schedule.rs",
        include_str!("readme_examples/nurses/schedule.rs"),
    ),
    (
        "routes/plan.rs",
        include_str!("readme_examples/routes/plan.rs"),
    ),
    ("scores.rs", include_str!("readme_examples/scores.rs")),
    ("streams.rs", include_str!("readme_examples/streams.rs")),
    (
        "weighted/schedule.rs",
        include_str!("readme_examples/weighted/schedule.rs"),
    ),
    ("weights.rs", include_str!("readme_examples/weights.rs")),
];

/// The README's ```rust blocks, with the line each starts on.
fn readme_blocks() -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, Vec<&str>)> = None;
    for (index, line) in README.lines().enumerate() {
        match open.as_mut() {
            None if line == "```rust" => open = Some((index + 2, Vec::new())),
            Some(_) if line == "```" => {
                let (start, lines) = open.take().unwrap();
                blocks.push((start, lines.join("\n")));
            }
            Some((_, lines)) => lines.push(line),
            None => {}
        }
    }
    assert!(
        open.is_none(),
        "README.md has an unterminated ```rust block"
    );
    blocks
}

/// The checked-in regions, with the file each comes from.
fn compiled_regions() -> Vec<(&'static str, String)> {
    let mut regions = Vec::new();
    for (file, source) in SOURCES {
        let mut open: Option<Vec<&str>> = None;
        for line in source.lines() {
            match (open.as_mut(), line) {
                (None, "// readme:begin") => open = Some(Vec::new()),
                (Some(_), "// readme:end") => {
                    regions.push((*file, open.take().unwrap().join("\n")))
                }
                (Some(lines), _) => lines.push(line),
                (None, _) => {}
            }
        }
        assert!(open.is_none(), "{file} has an unterminated readme region");
    }
    regions
}

#[test]
fn every_readme_rust_block_is_compiled() {
    let blocks = readme_blocks();
    let regions = compiled_regions();
    for (line, block) in &blocks {
        assert!(
            regions.iter().any(|(_, region)| region == block),
            "the ```rust block at README.md:{line} is not checked in under tests/readme_examples/"
        );
    }
    for (file, region) in &regions {
        assert!(
            blocks.iter().any(|(_, block)| block == region),
            "a readme region in readme_examples/{file} is no longer in README.md:\n{region}"
        );
    }
}

#[test]
fn readme_list_variable_model_builds() {
    let plan = routes::Plan {
        visits: (0..3).map(|id| routes::Visit { id }).collect(),
        routes: vec![routes::Route {
            id: 0,
            visit_order: vec![2, 0, 1],
        }],
        score: None,
    };
    assert_eq!(plan.routes[0].visit_order.len(), plan.visits.len());
}

#[test]
fn readme_scores_example_runs() {
    scores::scores();
}

#[test]
fn readme_constraint_weights_are_validated() {
    let mut weights = ConstraintWeights::new();
    weights.set("Travel time", 10).set("Overtime", 0);
    assert!(weighted::Schedule::validate_constraint_weights(&weights).is_ok());
    weights.set("Travel tme", 1);
    assert!(weighted::Schedule::validate_constraint_weights(&weights).is_err());
}

#[test]
fn readme_join_example_scores_night_shifts_with_a_matching_employee() {
    let schedule = Schedule {
        shifts: vec![
            Shift { id: 1, night: true },
            Shift {
                id: 2,
                night: false,
            },
        ],
        employees: vec![Employee {
            code: "e0".to_string(),
        }],
        assignments: vec![
            Assignment {
                id: 0,
                shift_id: 1,
                employee_code: "e0".to_string(),
            },
            Assignment {
                id: 1,
                shift_id: 2,
                employee_code: "e0".to_string(),
            },
            Assignment {
                id: 2,
                shift_id: 1,
                employee_code: "nobody".to_string(),
            },
        ],
        score: None,
    };
    let constraint = night_shift::constraint();
    assert_eq!(constraint.match_count(&schedule), 1);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
}
