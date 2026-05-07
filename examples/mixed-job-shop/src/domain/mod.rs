solverforge::planning_model! {
    root = "examples/mixed-job-shop/src/domain";

    mod job_shop_plan;
    mod machine;
    mod machine_sequence;
    mod operation;

    pub use job_shop_plan::JobShopPlan;
    pub use machine::Machine;
    pub use machine_sequence::MachineSequence;
    pub use operation::Operation;
}
