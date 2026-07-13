use crate::builder::context::RuntimeListElement;
use crate::heuristic::r#move::metadata::{encode_option_debug, encode_runtime_dynamic_list_source};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCursor;

use super::super::super::super::graph::ListLeafKind;
use super::super::spec::RuntimeListRecipe;
use super::super::RuntimeListMove;
use super::clone_tracking::{
    meter_clones, reset_meter_clones, reset_solution_clones, solution_clones,
};
use super::support::{
    director, dynamic_slot, dynamic_slot_for, initial_plan, selector, static_slot, static_slot_for,
    ListPlan, PositionMetric, ALL_KINDS,
};

type RuntimeMove = RuntimeListMove<ListPlan, usize, PositionMetric, PositionMetric>;

fn first_doable(
    selector: &super::super::RuntimeListNeighborhoodSelector<
        ListPlan,
        usize,
        PositionMetric,
        PositionMetric,
    >,
    score_director: &mut solverforge_scoring::ScoreDirector<ListPlan, ()>,
) -> RuntimeMove {
    let mut stream_state = selector.new_stream_state();
    let mut cursor = selector.open_cursor_with_stream_state(
        &mut stream_state,
        score_director,
        crate::heuristic::selector::move_selector::MoveStreamContext::default(),
    );
    loop {
        let id = cursor
            .next_candidate()
            .expect("test neighborhood must contain a doable candidate");
        let doable = cursor
            .candidate(id)
            .expect("newly emitted candidate remains borrowable")
            .is_doable(score_director);
        if doable {
            return cursor.take_candidate(id);
        }
        assert!(cursor.release_candidate(id));
    }
}

fn change_source_value(move_: &RuntimeMove, plan: &ListPlan) -> usize {
    let RuntimeListRecipe::Change { coordinates, .. } = move_.clone().into_recipe() else {
        panic!("source-key regression must use a change recipe");
    };
    plan.routes[coordinates.source_entity][coordinates.source_position]
}

#[test]
fn every_list_family_applies_and_undoes_for_typed_and_dynamic_slots() {
    for kind in ALL_KINDS {
        for (source, slot) in [
            ("typed", static_slot_for(kind)),
            ("dynamic", dynamic_slot_for(kind)),
        ] {
            let selector = selector(kind, slot, Some(71));
            let mut score_director = director(initial_plan());
            let runtime_move = first_doable(&selector, &mut score_director);
            let before = score_director.working_solution().clone();

            assert!(
                runtime_move.is_doable(&score_director),
                "{kind:?}/{source} remains doable"
            );
            let undo = runtime_move.do_move(&mut score_director);
            runtime_move.undo_move(&mut score_director, undo);
            assert_eq!(
                score_director.working_solution().routes,
                before.routes,
                "{kind:?}/{source} undo restores every route"
            );
            assert_eq!(
                score_director.working_solution().elements,
                before.elements,
                "{kind:?}/{source} undo preserves element inventory"
            );
        }
    }
}

#[test]
fn typed_and_dynamic_moves_keep_trace_identity_while_tabu_uses_source_identity() {
    for kind in ALL_KINDS {
        let typed_selector = selector(kind, static_slot_for(kind), Some(71));
        let dynamic_selector = selector(kind, dynamic_slot_for(kind), Some(71));
        let mut typed_director = director(initial_plan());
        let mut dynamic_director = director(initial_plan());
        let typed = first_doable(&typed_selector, &mut typed_director);
        let dynamic = first_doable(&dynamic_selector, &mut dynamic_director);

        assert_eq!(
            typed.candidate_trace_identity(),
            dynamic.candidate_trace_identity(),
            "{kind:?} trace identity"
        );
        assert_eq!(
            dynamic.tabu_signature(&dynamic_director),
            dynamic.tabu_signature(&dynamic_director),
            "{kind:?} dynamic tabu is stable"
        );
    }

    let typed_selector = selector(ListLeafKind::Change, static_slot(), Some(71));
    let dynamic_selector = selector(ListLeafKind::Change, dynamic_slot(), Some(71));
    let mut typed_director = director(initial_plan());
    let mut dynamic_director = director(initial_plan());
    let typed = first_doable(&typed_selector, &mut typed_director);
    let dynamic = first_doable(&dynamic_selector, &mut dynamic_director);
    let typed_value = change_source_value(&typed, typed_director.working_solution());
    let dynamic_value = change_source_value(&dynamic, dynamic_director.working_solution());
    assert_eq!(typed_value, dynamic_value);

    let typed_value_id = typed
        .tabu_signature(&typed_director)
        .destination_value_tokens[0]
        .value_id;
    let dynamic_value_id = dynamic
        .tabu_signature(&dynamic_director)
        .destination_value_tokens[0]
        .value_id;
    assert_eq!(typed_value_id, encode_option_debug(Some(&typed_value)));
    assert_eq!(
        dynamic_value_id,
        encode_runtime_dynamic_list_source(dynamic_value)
    );
    assert_ne!(
        dynamic_value_id,
        encode_option_debug(Some(&RuntimeListElement::<usize>::Dynamic(dynamic_value))),
        "the internal dynamic wrapper must never define a tabu value ID"
    );
}

#[test]
fn borrowed_candidates_release_once_and_selected_moves_outlive_every_cursor() {
    for kind in ALL_KINDS {
        let selector = selector(kind, static_slot_for(kind), Some(71));
        let score_director = director(initial_plan());
        let selected = {
            let mut stream_state = selector.new_stream_state();
            let mut cursor = selector.open_cursor_with_stream_state(
                &mut stream_state,
                &score_director,
                crate::heuristic::selector::move_selector::MoveStreamContext::default(),
            );
            let rejected = cursor
                .next_candidate()
                .unwrap_or_else(|| panic!("{kind:?} has a first candidate"));
            assert!(cursor.candidate(rejected).is_some());
            assert!(cursor.release_candidate(rejected));
            assert!(cursor.candidate(rejected).is_none());
            assert!(!cursor.release_candidate(rejected));
            cursor
                .next_owned_candidate()
                .unwrap_or_else(|| panic!("{kind:?} retains an owned candidate after release"))
        };
        assert!(
            selected.candidate_trace_identity().is_some(),
            "{kind:?} owns trace data"
        );
    }
}

#[test]
fn cursor_setup_clones_solutions_only_for_lazy_nearby_probes() {
    for kind in ALL_KINDS {
        let selector = selector(kind, static_slot_for(kind), Some(71));
        let score_director = director(initial_plan());
        let mut stream_state = selector.new_stream_state();
        reset_solution_clones();

        let mut cursor = selector.open_cursor_with_stream_state(
            &mut stream_state,
            &score_director,
            crate::heuristic::selector::move_selector::MoveStreamContext::default(),
        );
        let setup_clones = solution_clones();
        let expected = usize::from(matches!(
            kind,
            ListLeafKind::NearbyChange | ListLeafKind::NearbySwap
        ));
        assert_eq!(setup_clones, expected, "{kind:?} setup clone count");

        while let Some(id) = cursor.next_candidate() {
            assert!(cursor.release_candidate(id));
        }
        assert_eq!(
            solution_clones(),
            setup_clones,
            "{kind:?} candidate enumeration must not clone the solution"
        );
    }
}

#[test]
fn candidate_enumeration_never_clones_full_slot_meters() {
    let selector = selector(ListLeafKind::Change, static_slot(), Some(71));
    let score_director = director(initial_plan());
    let mut stream_state = selector.new_stream_state();
    reset_meter_clones();

    let mut cursor = selector.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        crate::heuristic::selector::move_selector::MoveStreamContext::default(),
    );
    let setup_clones = meter_clones();
    assert_eq!(setup_clones, 2, "the cursor clones each plan meter once");

    while let Some(id) = cursor.next_candidate() {
        assert!(cursor.release_candidate(id));
    }
    assert_eq!(
        meter_clones(),
        setup_clones,
        "candidate moves retain meter-free access instead of cloning the full slot"
    );
}
