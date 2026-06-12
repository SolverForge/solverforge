use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "visits",
        route_hooks = "route_hooks",
        savings_hooks = "savings_hooks"
    )]
    pub visits: Vec<usize>,
}

mod route_hooks {
    pub(crate) fn get<S>(_: &S, _: usize) -> Vec<usize> {
        Vec::new()
    }

    pub(crate) fn set<S>(_: &mut S, _: usize, _: Vec<usize>) {}

    pub(crate) fn depot<S>(_: &S, _: usize) -> usize {
        0
    }

    pub(crate) fn distance<S>(_: &S, _: usize, from: usize, to: usize) -> i64 {
        from.abs_diff(to) as i64
    }

    pub(crate) fn feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
        true
    }
}

mod savings_hooks {
    pub(crate) fn depot<S>(_: &S, _: usize) -> usize {
        0
    }

    pub(crate) fn distance<S>(_: &S, _: usize, from: usize, to: usize) -> i64 {
        from.abs_diff(to) as i64
    }

    pub(crate) fn feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
        true
    }
}

fn main() {
    let _ = Route {
        id: 0,
        visits: Vec::new(),
    };
}
