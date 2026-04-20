#[path = "fixtures/list_multi_module_plan.rs"]
mod plan;
#[path = "fixtures/list_multi_module_container.rs"]
mod container;
#[path = "fixtures/list_multi_module_item.rs"]
mod item;

pub use container::Container;
pub use item::Item;
pub use plan::Plan;

fn main() {
    let _ = Plan {
        items: Vec::new(),
        containers: Vec::new(),
        score: None,
    };
}
