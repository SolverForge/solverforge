use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "visits",
        domain = "cvrp",
        route_hooks = "custom_hooks"
    )]
    pub visits: Vec<usize>,
}

mod custom_hooks {
    pub fn get<S>(_: &S, _: usize) -> Vec<usize> {
        Vec::new()
    }

    pub fn set<S>(_: &mut S, _: usize, _: Vec<usize>) {}

    pub fn depot<S>(_: &S, _: usize) -> usize {
        0
    }

    pub fn distance<S>(_: &S, _: usize, _: usize, _: usize) -> i64 {
        0
    }

    pub fn feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
        true
    }
}

fn main() {}
