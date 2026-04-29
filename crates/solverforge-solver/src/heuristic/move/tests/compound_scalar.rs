use super::*;

#[derive(Clone, Debug)]
struct CompoundSolution {
    left: Vec<Option<usize>>,
    right: Vec<Option<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for CompoundSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_left(
    solution: &CompoundSolution,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.left[entity_index]
}

fn set_left(
    solution: &mut CompoundSolution,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.left[entity_index] = value;
}

fn get_right(
    solution: &CompoundSolution,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.right[entity_index]
}

fn set_right(
    solution: &mut CompoundSolution,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.right[entity_index] = value;
}

fn legal_to_three(
    _solution: &CompoundSolution,
    _entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) -> bool {
    value == Some(3)
}

fn compound_edit(
    descriptor_index: usize,
    entity_index: usize,
    variable_index: usize,
    variable_name: &'static str,
    to_value: Option<usize>,
    getter: fn(&CompoundSolution, usize, usize) -> Option<usize>,
    setter: fn(&mut CompoundSolution, usize, usize, Option<usize>),
) -> CompoundScalarEdit<CompoundSolution> {
    CompoundScalarEdit {
        descriptor_index,
        entity_index,
        variable_index,
        variable_name,
        to_value,
        getter,
        setter,
        value_is_legal: None,
    }
}

#[test]
fn compound_scalar_applies_and_undoes_multiple_edits_atomically() {
    let solution = CompoundSolution {
        left: vec![Some(0); 8],
        right: vec![Some(1); 8],
        score: None,
    };
    let mut director = ScoreDirector::simple_zero(solution);
    let mov = CompoundScalarMove::new(
        "pair",
        vec![
            compound_edit(0, 0, 0, "left", Some(2), get_left, set_left),
            compound_edit(1, 0, 0, "right", Some(3), get_right, set_right),
        ],
    );

    let mut recording = RecordingDirector::new(&mut director);
    assert!(mov.is_doable(&recording));
    mov.do_move(&mut recording);
    assert_eq!(recording.working_solution().left[0], Some(2));
    assert_eq!(recording.working_solution().right[0], Some(3));
    recording.undo_changes();

    assert_eq!(director.working_solution().left[0], Some(0));
    assert_eq!(director.working_solution().right[0], Some(1));
}

#[test]
fn compound_scalar_reports_each_affected_scope_and_tabu_token() {
    let solution = CompoundSolution {
        left: vec![Some(0); 8],
        right: vec![Some(1); 8],
        score: None,
    };
    let director = ScoreDirector::simple_zero(solution);
    let mov = CompoundScalarMove::new(
        "pair",
        vec![
            compound_edit(0, 4, 0, "left", Some(2), get_left, set_left),
            compound_edit(1, 7, 0, "right", Some(3), get_right, set_right),
        ],
    );

    let mut affected = Vec::new();
    mov.for_each_affected_entity(&mut |entity| {
        affected.push((
            entity.descriptor_index,
            entity.entity_index,
            entity.variable_name.to_string(),
        ));
    });
    assert_eq!(
        affected,
        vec![(0, 4, "left".to_string()), (1, 7, "right".to_string())]
    );

    let signature = mov.tabu_signature(&director);
    let left_scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "left");
    let right_scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(1, "right");
    assert!(signature
        .entity_tokens
        .contains(&left_scope.entity_token(4)));
    assert!(signature
        .entity_tokens
        .contains(&right_scope.entity_token(7)));
    assert!(signature
        .destination_value_tokens
        .contains(&left_scope.value_token(2)));
    assert!(signature
        .destination_value_tokens
        .contains(&right_scope.value_token(3)));
}

#[test]
fn compound_scalar_rejects_noop_and_illegal_edits() {
    let solution = CompoundSolution {
        left: vec![Some(0)],
        right: vec![Some(1)],
        score: None,
    };
    let director = ScoreDirector::simple_zero(solution);
    let noop = CompoundScalarMove::new(
        "noop",
        vec![compound_edit(0, 0, 0, "left", Some(0), get_left, set_left)],
    );
    assert!(!noop.is_doable(&director));

    let mut illegal_edit = compound_edit(0, 0, 0, "left", Some(2), get_left, set_left);
    illegal_edit.value_is_legal = Some(legal_to_three);
    let illegal = CompoundScalarMove::new("illegal", vec![illegal_edit]);
    assert!(!illegal.is_doable(&director));
}
