#!/usr/bin/env python3
"""Test N-Queens with solverforge Python bindings."""

from solverforge import Solver


def test_4_queens():
    """Test solving 4-Queens problem."""
    print("=== Testing 4-Queens ===")

    s = Solver()

    # Define Queen entity class
    s.entity_class(
        "Queen",
        [
            ("column", "int"),
            ("row", "int", {"planning_variable": True, "value_range": "rows"}),
        ],
    )

    # Define value range
    s.int_range("rows", 0, 4)

    # Add 4 queens
    s.add_entities("Queen", [{"column": i} for i in range(4)])

    # Row conflict constraint
    row_conflict = (
        s.constraint("row_conflict", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row == B.row", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(row_conflict)

    # Ascending diagonal: row1 - row2 == col1 - col2
    asc_diag = (
        s.constraint("ascending_diagonal", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row - B.row == A.column - B.column", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(asc_diag)

    # Descending diagonal: row1 - row2 == col2 - col1
    desc_diag = (
        s.constraint("descending_diagonal", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row - B.row == B.column - A.column", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(desc_diag)

    # Solve
    result = s.solve(time_limit_seconds=5)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")
    print(f"Duration: {result.duration_ms}ms")
    print(f"Steps: {result.steps}")
    print(f"Moves evaluated: {result.moves_evaluated}")

    # Print solution
    print("\nSolution:")
    queens = result.get_entities("Queen")
    for queen in queens:
        print(f"  Queen at column {queen['column']} -> row {queen['row']}")

    # Verify solution
    assert result.is_feasible, f"Expected feasible solution, got score {result.score}"
    print("\n4-Queens: PASSED")
    return True


def test_8_queens():
    """Test solving 8-Queens problem."""
    print("\n=== Testing 8-Queens ===")

    s = Solver()

    # Define Queen entity class
    s.entity_class(
        "Queen",
        [
            ("column", "int"),
            ("row", "int", {"planning_variable": True, "value_range": "rows"}),
        ],
    )

    # Define value range
    s.int_range("rows", 0, 8)

    # Add 8 queens
    s.add_entities("Queen", [{"column": i} for i in range(8)])

    # Row conflict constraint
    row_conflict = (
        s.constraint("row_conflict", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row == B.row", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(row_conflict)

    # Ascending diagonal: row1 - row2 == col1 - col2
    asc_diag = (
        s.constraint("ascending_diagonal", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row - B.row == A.column - B.column", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(asc_diag)

    # Descending diagonal: row1 - row2 == col2 - col1
    desc_diag = (
        s.constraint("descending_diagonal", "1hard")
        .for_each("Queen", s)
        .join("Queen", "A.row - B.row == B.column - A.column", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(desc_diag)

    # Solve
    result = s.solve(time_limit_seconds=10)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")
    print(f"Duration: {result.duration_ms}ms")
    print(f"Steps: {result.steps}")
    print(f"Moves evaluated: {result.moves_evaluated}")

    # Print solution
    print("\nSolution:")
    queens = result.get_entities("Queen")
    for queen in queens:
        print(f"  Queen at column {queen['column']} -> row {queen['row']}")

    # Visualize board
    print("\nBoard:")
    board = [["."] * 8 for _ in range(8)]
    for queen in queens:
        row = queen["row"]
        col = queen["column"]
        if row is not None:
            board[row][col] = "Q"
    for row in board:
        print("  " + " ".join(row))

    # Verify solution
    assert result.is_feasible, f"Expected feasible solution, got score {result.score}"
    print("\n8-Queens: PASSED")
    return True


if __name__ == "__main__":
    test_4_queens()
    test_8_queens()
    print("\n=== All tests passed! ===")
