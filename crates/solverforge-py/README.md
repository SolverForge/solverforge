# SolverForge Python Bindings

Python bindings for the SolverForge constraint optimization solver.

## Installation

```bash
pip install solverforge
```

## Quick Start

```python
from solverforge import Solver

# Create a solver
s = Solver()

# Define an entity class with a planning variable
s.entity_class("Queen", [
    ("column", "int"),
    ("row", "int", {"planning_variable": True, "value_range": "rows"})
])

# Define the value range
s.int_range("rows", 0, 8)

# Add entities
s.add_entities("Queen", [{"column": i} for i in range(8)])

# Define constraints
row_conflict = s.constraint("row_conflict", "1hard") \
    .for_each("Queen", s) \
    .join("Queen", "A.row == B.row", solver=s) \
    .distinct_pair() \
    .penalize()
s.add_constraint(row_conflict)

# Solve
result = s.solve(time_limit_seconds=30)

print(f"Score: {result.score}")
print(f"Feasible: {result.is_feasible}")

# Get solution
for queen in result.get_entities("Queen"):
    print(f"Queen at column {queen['column']} -> row {queen['row']}")
```

## Building from Source

Requires Rust and maturin:

```bash
pip install maturin
cd crates/solverforge-py
maturin develop
```
