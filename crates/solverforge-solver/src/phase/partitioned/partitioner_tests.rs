use super::*;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct TestSolution {
    values: Vec<i32>,
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

#[test]
fn test_thread_count_default() {
    assert_eq!(ThreadCount::default(), ThreadCount::Auto);
}

#[test]
fn test_thread_count_display() {
    assert_eq!(format!("{}", ThreadCount::Auto), "Auto");
    assert_eq!(format!("{}", ThreadCount::Unlimited), "Unlimited");
    assert_eq!(format!("{}", ThreadCount::Specific(4)), "4");
}

#[test]
fn test_thread_count_resolve_specific() {
    assert_eq!(ThreadCount::Specific(4).resolve(10), 4);
    assert_eq!(ThreadCount::Specific(10).resolve(4), 4);
}

#[test]
fn test_thread_count_resolve_auto() {
    let count = ThreadCount::Auto.resolve(100);
    assert!(count > 0);
}

#[test]
fn test_functional_partitioner() {
    let partitioner = FunctionalPartitioner::new(
        |s: &TestSolution| {
            let mid = s.values.len() / 2;
            vec![
                TestSolution {
                    values: s.values[..mid].to_vec(),
                    score: None,
                },
                TestSolution {
                    values: s.values[mid..].to_vec(),
                    score: None,
                },
            ]
        },
        |_original, partitions| {
            let mut values = Vec::new();
            for p in partitions {
                values.extend(p.values);
            }
            TestSolution {
                values,
                score: None,
            }
        },
    );

    let solution = TestSolution {
        values: vec![1, 2, 3, 4],
        score: None,
    };

    let partitions = partitioner.partition(&solution);
    assert_eq!(partitions.len(), 2);
    assert_eq!(partitions[0].values, vec![1, 2]);
    assert_eq!(partitions[1].values, vec![3, 4]);

    let merged = partitioner.merge(&solution, partitions);
    assert_eq!(merged.values, vec![1, 2, 3, 4]);
}

#[test]
fn test_partitioner_debug() {
    let partitioner = FunctionalPartitioner::new(
        |_: &TestSolution| Vec::new(),
        |original: &TestSolution, _| original.clone(),
    )
    .with_recommended_count(4);

    let debug = format!("{:?}", partitioner);
    assert!(debug.contains("FunctionalPartitioner"));
    assert!(debug.contains("recommended_count"));
}
