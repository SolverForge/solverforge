#[test]
fn expansion_tracks_every_manifest_module_as_include_dependency() {
    let expanded = expand(quote! {
        root = "tests/ui/pass/scalar_multi_module/domain";

        mod plan;
        mod task;
        mod worker;

        pub use plan::Plan;
        pub use task::Task;
        pub use worker::Worker;
    })
    .expect("planning_model! should expand")
    .to_string();

    assert!(expanded.contains("include_str !"));
    assert!(expanded.contains("plan.rs"));
    assert!(expanded.contains("task.rs"));
    assert!(expanded.contains("worker.rs"));
}
