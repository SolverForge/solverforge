use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

use crate::builder::context::{
    ProviderReasonArena, RawProviderCandidate, RawProviderEdit, RuntimeProviderSlotResolver,
    RuntimeScalarSlot, ScalarVariableSlot, ValueSource,
};

#[derive(Clone, Debug)]
struct ProviderTestSolution {
    score: Option<SoftScore>,
    first: Vec<Option<usize>>,
    second: Vec<Option<usize>>,
}

impl PlanningSolution for ProviderTestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn first_count(solution: &ProviderTestSolution) -> usize {
    solution.first.len()
}

fn second_count(solution: &ProviderTestSolution) -> usize {
    solution.second.len()
}

fn first_get(solution: &ProviderTestSolution, row: usize, _variable: usize) -> Option<usize> {
    solution.first.get(row).copied().flatten()
}

fn second_get(solution: &ProviderTestSolution, row: usize, _variable: usize) -> Option<usize> {
    solution.second.get(row).copied().flatten()
}

fn first_set(
    solution: &mut ProviderTestSolution,
    row: usize,
    _variable: usize,
    value: Option<usize>,
) {
    solution.first[row] = value;
}

fn second_set(
    solution: &mut ProviderTestSolution,
    row: usize,
    _variable: usize,
    value: Option<usize>,
) {
    solution.second[row] = value;
}

fn first_slot() -> RuntimeScalarSlot<ProviderTestSolution> {
    RuntimeScalarSlot::Static(ScalarVariableSlot::new(
        0,
        0,
        "First",
        first_count,
        "value",
        first_get,
        first_set,
        ValueSource::CountableRange { from: 0, to: 3 },
        false,
    ))
}

fn second_slot() -> RuntimeScalarSlot<ProviderTestSolution> {
    RuntimeScalarSlot::Static(ScalarVariableSlot::new(
        1,
        0,
        "Second",
        second_count,
        "value",
        second_get,
        second_set,
        ValueSource::CountableRange { from: 0, to: 3 },
        false,
    ))
}

fn unqualified_value_edit() -> RawProviderEdit {
    RawProviderEdit {
        entity_class: None,
        variable_name: Arc::from("value"),
        entity_index: 0,
        to_value: Some(1),
    }
}

#[test]
fn unqualified_name_resolves_in_allowed_model_order_not_global_first_match() {
    let first = first_slot();
    let second = second_slot();
    let allowed = vec![second.id()];
    let resolver = RuntimeProviderSlotResolver::new(vec![first, second]).unwrap();
    let solution = ProviderTestSolution {
        score: None,
        first: vec![Some(0)],
        second: vec![Some(0)],
    };

    let mut reasons = ProviderReasonArena::default();
    let resolved = resolver
        .resolve_and_normalize(
            &solution,
            vec![RawProviderCandidate {
                reason: Arc::from("candidate"),
                edits: vec![unqualified_value_edit()],
            }],
            &allowed,
            &mut reasons,
        )
        .unwrap();

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].edits[0].descriptor_index, 1);
    assert!(matches!(
        &resolved[0].edits[0].slot,
        RuntimeScalarSlot::Static(slot) if slot.entity_type_name == "Second"
    ));
}

#[test]
fn aliases_resolve_before_duplicate_target_filtering() {
    let first = first_slot();
    let second = second_slot();
    let allowed = vec![second.id()];
    let resolver = RuntimeProviderSlotResolver::new(vec![first, second]).unwrap();
    let solution = ProviderTestSolution {
        score: None,
        first: vec![Some(0)],
        second: vec![Some(0)],
    };
    let qualified_alias = RawProviderEdit {
        entity_class: Some(Arc::from("Second")),
        ..unqualified_value_edit()
    };

    let mut reasons = ProviderReasonArena::default();
    let resolved = resolver
        .resolve_and_normalize(
            &solution,
            vec![RawProviderCandidate {
                reason: Arc::from("candidate"),
                edits: vec![unqualified_value_edit(), qualified_alias],
            }],
            &allowed,
            &mut reasons,
        )
        .unwrap();

    assert!(resolved.is_empty());
}
