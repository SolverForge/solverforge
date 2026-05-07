use solverforge_macros::problem_fact;

#[problem_fact(debug)]
struct Worker {
    #[planning_id]
    id: usize,
}

fn main() {}
