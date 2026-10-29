use super::*;
use crate::test_utils::create_simple_nqueens_director;

#[test]
fn test_from_solution_entity_selector() {
    let director = create_simple_nqueens_director(4);

    let solution = director.working_solution();
    for (i, queen) in solution.queens.iter().enumerate() {
        assert_eq!(queen.column, i as i64);
    }

    let selector = FromSolutionEntitySelector::new(0);

    let refs: Vec<_> = selector.iter(&director).collect();
    assert_eq!(refs.len(), 4);
    assert_eq!(refs[0], EntityReference::new(0, 0));
    assert_eq!(refs[1], EntityReference::new(0, 1));
    assert_eq!(refs[2], EntityReference::new(0, 2));
    assert_eq!(refs[3], EntityReference::new(0, 3));

    assert_eq!(selector.size(&director), 4);
}
