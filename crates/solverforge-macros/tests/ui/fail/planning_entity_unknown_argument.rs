use solverforge_macros::planning_entity;

#[planning_entity(debug)]
struct Task {
    #[planning_id]
    id: usize,
}

fn main() {}
