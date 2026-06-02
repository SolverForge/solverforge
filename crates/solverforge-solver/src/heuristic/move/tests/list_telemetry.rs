use smallvec::smallvec;

use super::*;
use crate::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;

#[derive(Clone, Debug)]
struct Plan {
    routes: Vec<Vec<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.routes.get(entity).map_or(0, Vec::len)
}

fn list_get(plan: &Plan, entity: usize, pos: usize) -> Option<usize> {
    plan.routes.get(entity)?.get(pos).copied()
}

fn list_set(plan: &mut Plan, entity: usize, pos: usize, value: usize) {
    plan.routes[entity][pos] = value;
}

fn list_remove(plan: &mut Plan, entity: usize, pos: usize) -> Option<usize> {
    (pos < plan.routes[entity].len()).then(|| plan.routes[entity].remove(pos))
}

fn list_remove_strict(plan: &mut Plan, entity: usize, pos: usize) -> usize {
    plan.routes[entity].remove(pos)
}

fn list_insert(plan: &mut Plan, entity: usize, pos: usize, value: usize) {
    plan.routes[entity].insert(pos, value);
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, pos: usize, values: Vec<usize>) {
    plan.routes[entity].splice(pos..pos, values);
}

#[test]
fn list_moves_report_specific_telemetry_labels() {
    let list_change = ListChangeMove::<Plan, usize>::new(
        0,
        0,
        0,
        2,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "routes",
        0,
    );
    let list_swap =
        ListSwapMove::<Plan, usize>::new(0, 0, 0, 1, list_len, list_get, list_set, "routes", 0);
    let list_permute = ListPermuteMove::<Plan, usize>::new(
        0,
        0,
        2,
        smallvec![1, 0],
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "routes",
        0,
    );
    let list_reverse =
        ListReverseMove::<Plan, usize>::new(0, 0, 2, list_len, list_get, list_reverse, "routes", 0);
    let list_ruin = ListRuinMove::<Plan, usize>::new(
        0,
        &[0],
        entity_count,
        list_len,
        list_get,
        list_remove_strict,
        list_insert,
        "routes",
        0,
    );
    let sublist_change = SublistChangeMove::<Plan, usize>::new(
        0,
        0,
        1,
        0,
        2,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "routes",
        0,
    );
    let sublist_swap = SublistSwapMove::<Plan, usize>::new(
        0,
        0,
        1,
        0,
        1,
        2,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "routes",
        0,
    );
    let k_opt = KOptMove::<Plan, usize>::new(
        &[
            CutPoint::new(0, 1),
            CutPoint::new(0, 2),
            CutPoint::new(0, 3),
        ],
        &THREE_OPT_RECONNECTIONS[3],
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "routes",
        0,
    );

    assert_eq!(list_change.telemetry_label(), "list_change");
    assert_eq!(list_swap.telemetry_label(), "list_swap");
    assert_eq!(list_permute.telemetry_label(), "list_permute");
    assert_eq!(list_reverse.telemetry_label(), "list_reverse");
    assert_eq!(list_ruin.telemetry_label(), "list_ruin");
    assert_eq!(sublist_change.telemetry_label(), "sublist_change");
    assert_eq!(sublist_swap.telemetry_label(), "sublist_swap");
    assert_eq!(k_opt.telemetry_label(), "k_opt");
}
