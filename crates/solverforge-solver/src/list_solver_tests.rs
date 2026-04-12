use super::*;
use solverforge_config::VariableTargetConfig;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn config(kind: ConstructionHeuristicType) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: kind,
        target: VariableTargetConfig::default(),
        k: 2,
        termination: None,
    }
}

#[test]
fn list_builder_rejects_unnormalized_generic_construction() {
    let panic = std::panic::catch_unwind(|| {
        let _ = build_list_construction::<TestSolution, usize>(
            Some(&config(ConstructionHeuristicType::FirstFit)),
            |_| 0,
            |_| Vec::new(),
            |_| 0,
            |_, _| 0,
            |_, _, _, _| {},
            |_, _, _| 0,
            |_, _| 0,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
    })
    .expect_err("unnormalized generic construction should panic");

    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(message.contains("must be normalized before list construction"));
}
