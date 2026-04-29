use super::*;

#[derive(Clone, Debug)]
struct RepairSolution {
    left: Vec<Option<usize>>,
    right: Vec<Option<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for RepairSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_left(
    solution: &RepairSolution,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.left[entity_index]
}

fn set_left(
    solution: &mut RepairSolution,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.left[entity_index] = value;
}

fn get_right(
    solution: &RepairSolution,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.right[entity_index]
}

fn set_right(
    solution: &mut RepairSolution,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.right[entity_index] = value;
}

#[test]
fn conflict_repair_tabu_tokens_use_each_edit_scope() {
    let solution = RepairSolution {
        left: vec![Some(0)],
        right: vec![Some(1)],
        score: None,
    };
    let director = ScoreDirector::simple_zero(solution);
    let repair = ConflictRepairMove::new(
        "twoDescriptorRepair",
        vec![
            ConflictRepairScalarEdit {
                descriptor_index: 0,
                entity_index: 0,
                variable_index: 0,
                variable_name: "left",
                to_value: Some(2),
                getter: get_left,
                setter: set_left,
            },
            ConflictRepairScalarEdit {
                descriptor_index: 1,
                entity_index: 0,
                variable_index: 0,
                variable_name: "right",
                to_value: Some(3),
                getter: get_right,
                setter: set_right,
            },
        ],
    );

    let signature = repair.tabu_signature(&director);
    let left_scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "left");
    let right_scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(1, "right");

    assert!(signature
        .entity_tokens
        .contains(&left_scope.entity_token(0)));
    assert!(signature
        .entity_tokens
        .contains(&right_scope.entity_token(0)));
    assert!(signature
        .destination_value_tokens
        .contains(&left_scope.value_token(2)));
    assert!(signature
        .destination_value_tokens
        .contains(&right_scope.value_token(3)));
    assert_eq!(signature.entity_tokens.len(), 2);
    assert_eq!(signature.destination_value_tokens.len(), 2);
}

#[test]
fn conflict_repair_affected_entities_keep_each_edit_scope() {
    let repair = ConflictRepairMove::<RepairSolution>::new(
        "twoDescriptorRepair",
        vec![
            ConflictRepairScalarEdit {
                descriptor_index: 0,
                entity_index: 4,
                variable_index: 0,
                variable_name: "left",
                to_value: Some(2),
                getter: get_left,
                setter: set_left,
            },
            ConflictRepairScalarEdit {
                descriptor_index: 1,
                entity_index: 7,
                variable_index: 0,
                variable_name: "right",
                to_value: Some(3),
                getter: get_right,
                setter: set_right,
            },
        ],
    );

    let mut affected = Vec::new();
    let mut collect_affected = |entity: MoveAffectedEntity<'_>| {
        affected.push((
            entity.descriptor_index,
            entity.variable_name.to_string(),
            entity.entity_index,
        ));
    };
    repair.for_each_affected_entity(&mut collect_affected);

    assert_eq!(
        affected,
        vec![(0, "left".to_string(), 4), (1, "right".to_string(), 7)]
    );
}
