//! The planning macros document or hide every public item they generate, so a
//! crate that denies `missing_docs` compiles with them.
#![deny(missing_docs)]

#[path = "generated_items_documented/mod.rs"]
mod domain;

pub use domain::{Plan, PlanConstraintStreams, Route, Visit};

#[test]
fn documented_model_exposes_its_collection_sources() {
    let descriptor = Plan::descriptor();
    assert!(descriptor.find_entity_descriptor("Route").is_some());
    let _routes = Plan::routes();
    let _visits = Plan::visits();
}
