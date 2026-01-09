//! Tests for SolverManagerBuilder.

use super::{ConstructionType, LocalSearchType};

#[test]
fn test_local_search_types() {
    assert_eq!(LocalSearchType::default(), LocalSearchType::HillClimbing);

    let tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
    assert!(matches!(
        tabu,
        LocalSearchType::TabuSearch { tabu_size: 10 }
    ));

    let sa = LocalSearchType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    assert!(matches!(
        sa,
        LocalSearchType::SimulatedAnnealing {
            starting_temp: 1.0,
            decay_rate: 0.99
        }
    ));
}

#[test]
fn test_construction_types() {
    assert_eq!(ConstructionType::default(), ConstructionType::FirstFit);
}
