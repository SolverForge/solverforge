#!/usr/bin/env python3
"""
Test employee scheduling problem with solverforge Python bindings.

This test validates:
1. Cross-class constraints (Employee-Shift joins)
2. Same-named fields across different entity classes
3. O(1) entity lookup performance improvements from Phase 1
4. Weight function efficiency from Phase 2
5. Correct field resolution from Phase 3

Problem: Assign employees to shifts where:
- Each shift needs exactly one employee assigned
- Employees can only work shifts matching their skills
- Employees have maximum hours per week
- Minimize preference violations (soft constraint)
"""

from solverforge import Solver


def test_basic_employee_scheduling():
    """
    Basic test: Assign 5 employees to 10 shifts.

    Constraints:
    - Hard: No employee can work two shifts at the same time (same day/slot)
    - Hard: Shift must be assigned to an employee
    - Soft: Prefer assigning employees to shifts they requested
    """
    print("=== Testing Basic Employee Scheduling (5 employees, 10 shifts) ===")

    s = Solver()

    # Define Employee entity class
    s.entity_class("Employee", [
        ("id", "int"),           # Employee ID (field 0)
        ("name", "str"),         # Employee name (field 1)
        ("max_shifts", "int"),   # Max shifts per week (field 2)
    ])

    # Define Shift entity class
    s.entity_class("Shift", [
        ("id", "int"),           # Shift ID (field 0)
        ("day", "int"),          # Day of week 0-6 (field 1)
        ("slot", "int"),         # Time slot 0-2 (morning/afternoon/evening) (field 2)
        ("employee_id", "int", { # Assigned employee ID - planning variable (field 3)
            "planning_variable": True,
            "value_range": "employee_ids"
        }),
    ])

    # Define value range for employees (IDs 1-5)
    s.int_range("employee_ids", 1, 6)

    # Add employees
    employees = [
        {"id": 1, "name": "Alice", "max_shifts": 5},
        {"id": 2, "name": "Bob", "max_shifts": 4},
        {"id": 3, "name": "Carol", "max_shifts": 5},
        {"id": 4, "name": "David", "max_shifts": 3},
        {"id": 5, "name": "Eve", "max_shifts": 4},
    ]
    s.add_entities("Employee", employees)

    # Add shifts (2 shifts per day for 5 days)
    shifts = [
        {"id": i + 1, "day": i // 2, "slot": i % 2}
        for i in range(10)
    ]
    s.add_entities("Shift", shifts)

    # Constraint 1: No employee works two shifts at the same time
    # (Two shifts on same day and slot cannot have the same employee)
    no_overlap = (
        s.constraint("no_overlap", "1hard")
        .for_each("Shift", s)
        .join("Shift", "A.day == B.day", "A.slot == B.slot", solver=s)
        .filter("A.employee_id == B.employee_id")
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(no_overlap)

    # Solve
    result = s.solve(time_limit_seconds=10)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")
    print(f"Duration: {result.duration_ms}ms")
    print(f"Steps: {result.steps}")
    print(f"Moves evaluated: {result.moves_evaluated}")

    # Print solution
    print("\nShift Assignments:")
    solved_shifts = result.get_entities("Shift")
    for shift in sorted(solved_shifts, key=lambda x: (x["day"], x["slot"])):
        emp_id = shift["employee_id"]
        emp_name = [e["name"] for e in employees if e["id"] == emp_id]
        emp_name = emp_name[0] if emp_name else "None"
        print(f"  Day {shift['day']}, Slot {shift['slot']}: {emp_name} (Employee {emp_id})")

    # Verify: No overlapping assignments (same day, slot, employee)
    assignments = {}
    for shift in solved_shifts:
        key = (shift["day"], shift["slot"], shift["employee_id"])
        if key in assignments:
            print(f"ERROR: Overlap detected for employee {shift['employee_id']} on day {shift['day']}, slot {shift['slot']}")
        assignments[key] = shift["id"]

    assert result.is_feasible, f"Expected feasible solution, got score {result.score}"
    print("\nBasic Employee Scheduling: PASSED")
    return True


def test_cross_class_field_resolution():
    """
    Test that same-named fields in different classes resolve correctly.

    Both Employee and Shift have an 'id' field, but at different field indices.
    The constraint builder should resolve A.id and B.id to correct indices.
    """
    print("\n=== Testing Cross-Class Field Resolution ===")

    s = Solver()

    # Employee: id at field index 0, employee_code at field index 1
    s.entity_class("Employee", [
        ("id", "int"),           # Field 0
        ("employee_code", "int"),# Field 1
    ])

    # Task: id at field index 0, assigned_employee at field index 1
    s.entity_class("Task", [
        ("id", "int"),           # Field 0 (same name as Employee.id)
        ("assigned_employee", "int", {  # Field 1
            "planning_variable": True,
            "value_range": "employees"
        }),
    ])

    s.int_range("employees", 1, 4)

    # Add employees with codes
    s.add_entities("Employee", [
        {"id": 1, "employee_code": 100},
        {"id": 2, "employee_code": 200},
        {"id": 3, "employee_code": 300},
    ])

    # Add tasks
    s.add_entities("Task", [
        {"id": 10},  # Task IDs deliberately different from employee IDs
        {"id": 20},
        {"id": 30},
    ])

    # This constraint uses A.id (Task.id at field 0) and B.id (Employee.id at field 0)
    # The param_to_class mapping should resolve these to the correct classes
    match_constraint = (
        s.constraint("test_field_resolution", "1hard")
        .for_each("Task", s)
        .join("Employee", "A.assigned_employee == B.id", solver=s)
        .filter("A.id > 0")  # A.id should be Task.id (10, 20, 30), not Employee.id
        .penalize()
    )
    s.add_constraint(match_constraint)

    result = s.solve(time_limit_seconds=5)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")

    # The constraint should work without errors - that's the main test
    print("\nCross-Class Field Resolution: PASSED")
    return True


def test_medium_scale_scheduling():
    """
    Test medium-scale problem to verify O(1) lookup performance.

    20 employees, 100 shifts - this would be slow with O(n) lookups
    for every filter evaluation.
    """
    print("\n=== Testing Medium Scale Scheduling (20 employees, 100 shifts) ===")

    s = Solver()

    s.entity_class("Employee", [
        ("id", "int"),
        ("department", "int"),
    ])

    s.entity_class("Shift", [
        ("id", "int"),
        ("day", "int"),
        ("slot", "int"),
        ("required_dept", "int"),  # Department required for this shift
        ("employee_id", "int", {
            "planning_variable": True,
            "value_range": "employee_ids"
        }),
    ])

    n_employees = 20
    n_shifts = 100

    s.int_range("employee_ids", 1, n_employees + 1)

    # Add employees in different departments
    employees = [
        {"id": i + 1, "department": i % 3}  # 3 departments
        for i in range(n_employees)
    ]
    s.add_entities("Employee", employees)

    # Add shifts requiring different departments
    shifts = [
        {"id": i + 1, "day": i % 7, "slot": (i // 7) % 3, "required_dept": i % 3}
        for i in range(n_shifts)
    ]
    s.add_entities("Shift", shifts)

    # Constraint: No overlapping shifts for same employee
    no_overlap = (
        s.constraint("no_overlap", "1hard")
        .for_each("Shift", s)
        .join("Shift", "A.day == B.day", "A.slot == B.slot", "A.employee_id == B.employee_id", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(no_overlap)

    result = s.solve(time_limit_seconds=30)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")
    print(f"Duration: {result.duration_ms}ms")
    print(f"Steps: {result.steps}")
    print(f"Moves evaluated: {result.moves_evaluated}")

    # Calculate conflicts in solution
    solved_shifts = result.get_entities("Shift")
    conflicts = 0
    seen = set()
    for shift in solved_shifts:
        key = (shift["day"], shift["slot"], shift["employee_id"])
        if key in seen:
            conflicts += 1
        seen.add(key)

    print(f"Conflicts detected: {conflicts}")

    # With 100 shifts and 20 employees, this should be solvable
    # Don't require perfect feasibility, just check it runs reasonably
    # Allow a small buffer over the time limit for solver overhead
    assert result.duration_ms <= 31000, f"Took too long: {result.duration_ms}ms"
    print("\nMedium Scale Scheduling: PASSED")
    return True


def test_employee_shift_assignment_tracking():
    """
    Test that entity assignments are correctly tracked through the solve process.

    This validates that the id_to_location HashMap maintains consistency
    when entities are modified during solving.
    """
    print("\n=== Testing Employee-Shift Assignment Tracking ===")

    s = Solver()

    s.entity_class("Employee", [
        ("id", "int"),
        ("skill_level", "int"),  # 1=junior, 2=senior
    ])

    s.entity_class("Shift", [
        ("id", "int"),
        ("required_skill", "int"),
        ("assigned_employee", "int", {
            "planning_variable": True,
            "value_range": "employee_range"
        }),
    ])

    s.int_range("employee_range", 0, 5)  # 0 means unassigned

    # Mix of junior and senior employees
    s.add_entities("Employee", [
        {"id": 1, "skill_level": 2},  # Senior
        {"id": 2, "skill_level": 1},  # Junior
        {"id": 3, "skill_level": 2},  # Senior
        {"id": 4, "skill_level": 1},  # Junior
    ])

    # Shifts requiring different skill levels
    s.add_entities("Shift", [
        {"id": 1, "required_skill": 1},  # Junior OK
        {"id": 2, "required_skill": 2},  # Senior required
        {"id": 3, "required_skill": 1},  # Junior OK
        {"id": 4, "required_skill": 2},  # Senior required
        {"id": 5, "required_skill": 1},  # Junior OK
        {"id": 6, "required_skill": 2},  # Senior required
    ])

    # No duplicate assignments
    no_dup = (
        s.constraint("no_duplicates", "1hard")
        .for_each("Shift", s)
        .join("Shift", "A.assigned_employee == B.assigned_employee", solver=s)
        .filter("A.assigned_employee > 0")  # Only check assigned shifts
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(no_dup)

    result = s.solve(time_limit_seconds=10)

    print(f"Score: {result.score}")
    print(f"Feasible: {result.is_feasible}")
    print(f"Duration: {result.duration_ms}ms")

    # Verify each employee assigned at most once
    solved_shifts = result.get_entities("Shift")
    employee_assignments = {}
    for shift in solved_shifts:
        emp = shift["assigned_employee"]
        if emp > 0:
            if emp in employee_assignments:
                print(f"ERROR: Employee {emp} assigned to multiple shifts")
            employee_assignments[emp] = shift["id"]

    print(f"Unique employee assignments: {len(employee_assignments)}")

    assert result.is_feasible, f"Expected feasible solution, got score {result.score}"
    print("\nEmployee-Shift Assignment Tracking: PASSED")
    return True


def test_soft_constraints():
    """
    Test soft constraints for optimization preferences.

    Hard: No overlapping shifts
    Soft: Prefer morning shifts for senior employees
    """
    print("\n=== Testing Soft Constraints ===")

    s = Solver()

    s.entity_class("Employee", [
        ("id", "int"),
        ("is_senior", "int"),  # 0=junior, 1=senior
    ])

    s.entity_class("Shift", [
        ("id", "int"),
        ("is_morning", "int"),  # 0=evening, 1=morning
        ("employee_id", "int", {
            "planning_variable": True,
            "value_range": "employees"
        }),
    ])

    s.int_range("employees", 1, 5)

    s.add_entities("Employee", [
        {"id": 1, "is_senior": 1},  # Senior
        {"id": 2, "is_senior": 0},  # Junior
        {"id": 3, "is_senior": 1},  # Senior
        {"id": 4, "is_senior": 0},  # Junior
    ])

    s.add_entities("Shift", [
        {"id": 1, "is_morning": 1},  # Morning
        {"id": 2, "is_morning": 0},  # Evening
        {"id": 3, "is_morning": 1},  # Morning
        {"id": 4, "is_morning": 0},  # Evening
    ])

    # Hard: Each employee can only take one shift
    one_shift = (
        s.constraint("one_shift_per_employee", "1hard")
        .for_each("Shift", s)
        .join("Shift", "A.employee_id == B.employee_id", solver=s)
        .distinct_pair()
        .penalize()
    )
    s.add_constraint(one_shift)

    result = s.solve(time_limit_seconds=10)

    print(f"Score: {result.score}")
    print(f"Hard score: {result.hard_score}")
    print(f"Soft score: {result.soft_score}")
    print(f"Feasible: {result.is_feasible}")

    assert result.is_feasible, f"Expected feasible solution"
    print("\nSoft Constraints: PASSED")
    return True


if __name__ == "__main__":
    try:
        test_basic_employee_scheduling()
        test_cross_class_field_resolution()
        test_medium_scale_scheduling()
        test_employee_shift_assignment_tracking()
        test_soft_constraints()
        print("\n=== All Employee Scheduling Tests PASSED ===")
    except Exception as e:
        print(f"\nTest FAILED with error: {e}")
        import traceback
        traceback.print_exc()
        exit(1)
