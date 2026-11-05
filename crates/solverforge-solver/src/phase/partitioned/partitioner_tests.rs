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
    assert_eq!(
        format!(
            "{}",
            ThreadCount::Specific(std::num::NonZeroUsize::new(4).unwrap())
        ),
        "4"
    );
}

#[test]
fn test_thread_count_resolve_specific() {
    assert_eq!(
        ThreadCount::Specific(std::num::NonZeroUsize::new(4).unwrap()).resolve(10, None),
        4
    );
    assert_eq!(
        ThreadCount::Specific(std::num::NonZeroUsize::new(10).unwrap()).resolve(4, None),
        4
    );
}

#[test]
fn test_thread_count_resolve_auto() {
    let count = ThreadCount::Auto.resolve(100, None);
    assert!(count > 0);
}

#[test]
fn a_recommended_partition_count_bounds_only_auto() {
    let two = std::num::NonZeroUsize::new(2);
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
    assert_eq!(ThreadCount::Auto.resolve(100, two), cores.min(2));
    assert_eq!(ThreadCount::Auto.resolve(1, two), 1);
    assert_eq!(
        ThreadCount::Specific(std::num::NonZeroUsize::new(4).unwrap()).resolve(10, two),
        4
    );
    assert_eq!(ThreadCount::Unlimited.resolve(10, two), cores);
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
    .with_recommended_count(std::num::NonZeroUsize::new(4).unwrap());

    let debug = format!("{:?}", partitioner);
    assert!(debug.contains("FunctionalPartitioner"));
    assert!(debug.contains("recommended_count"));
}
