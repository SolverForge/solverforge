#[path = "fixtures/standard_multi_module_plan.rs"]
mod plan;
#[path = "fixtures/standard_multi_module_task.rs"]
mod task;
#[path = "fixtures/standard_multi_module_worker.rs"]
mod worker;

pub use plan::Plan;
pub use task::Task;
pub use worker::Worker;

fn main() {
    let _ = Plan {
        workers: Vec::new(),
        tasks: Vec::new(),
        score: None,
    };
}
