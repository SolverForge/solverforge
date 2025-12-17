//! Python annotation marker classes for SolverForge.
//!
//! These classes are used with `typing.Annotated` to mark fields with planning metadata,
//! providing a 1:1 compatible API with Timefold's Python bindings.
//!
//! # Example
//!
//! ```python
//! from typing import Annotated, Optional, List
//! from solverforge import (
//!     PlanningId, PlanningVariable, PlanningScore,
//!     ValueRangeProvider, ProblemFactCollectionProperty,
//!     PlanningEntityCollectionProperty,
//! )
//!
//! @planning_entity
//! class Lesson:
//!     id: Annotated[str, PlanningId]
//!     timeslot: Annotated[Optional['Timeslot'], PlanningVariable(value_range_provider_refs=['timeslots'])]
//!
//! @planning_solution
//! class Timetable:
//!     timeslots: Annotated[List[Timeslot], ProblemFactCollectionProperty, ValueRangeProvider(id='timeslots')]
//!     lessons: Annotated[List[Lesson], PlanningEntityCollectionProperty]
//!     score: Annotated[Optional[HardSoftScore], PlanningScore]
//! ```

use pyo3::prelude::*;

/// Marks a field as the planning ID for a planning entity.
///
/// The planning ID uniquely identifies each entity instance and is used by the
/// solver to track entities across moves.
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Lesson:
///     id: Annotated[str, PlanningId]
/// ```
#[pyclass(name = "PlanningId")]
#[derive(Clone, Debug)]
pub struct PyPlanningId;

#[pymethods]
impl PyPlanningId {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "PlanningId()"
    }
}

/// Marks a field as a planning variable that the solver can change.
///
/// A planning variable is a field that the solver will modify to find the optimal solution.
/// It must reference one or more value range providers that define the possible values.
///
/// # Parameters
///
/// - `value_range_provider_refs`: List of IDs referencing ValueRangeProvider annotations
/// - `allows_unassigned`: If true, the variable can be None/unassigned (default: false)
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Lesson:
///     timeslot: Annotated[Optional['Timeslot'], PlanningVariable(value_range_provider_refs=['timeslots'])]
///     room: Annotated[Optional['Room'], PlanningVariable(value_range_provider_refs=['rooms'], allows_unassigned=True)]
/// ```
#[pyclass(name = "PlanningVariable")]
#[derive(Clone, Debug)]
pub struct PyPlanningVariable {
    #[pyo3(get)]
    pub value_range_provider_refs: Vec<String>,
    #[pyo3(get)]
    pub allows_unassigned: bool,
}

#[pymethods]
impl PyPlanningVariable {
    #[new]
    #[pyo3(signature = (value_range_provider_refs=vec![], allows_unassigned=false))]
    fn new(value_range_provider_refs: Vec<String>, allows_unassigned: bool) -> Self {
        Self {
            value_range_provider_refs,
            allows_unassigned,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PlanningVariable(value_range_provider_refs={:?}, allows_unassigned={})",
            self.value_range_provider_refs, self.allows_unassigned
        )
    }
}

/// Marks a field as a planning list variable.
///
/// A planning list variable is a list that the solver can reorder and reassign elements.
/// Used for vehicle routing and similar problems where order matters.
///
/// # Parameters
///
/// - `value_range_provider_refs`: List of IDs referencing ValueRangeProvider annotations
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Vehicle:
///     visits: Annotated[List['Visit'], PlanningListVariable(value_range_provider_refs=['visits'])]
/// ```
#[pyclass(name = "PlanningListVariable")]
#[derive(Clone, Debug)]
pub struct PyPlanningListVariable {
    #[pyo3(get)]
    pub value_range_provider_refs: Vec<String>,
}

#[pymethods]
impl PyPlanningListVariable {
    #[new]
    #[pyo3(signature = (value_range_provider_refs=vec![]))]
    fn new(value_range_provider_refs: Vec<String>) -> Self {
        Self {
            value_range_provider_refs,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PlanningListVariable(value_range_provider_refs={:?})",
            self.value_range_provider_refs
        )
    }
}

/// Marks a field as the planning score for a solution.
///
/// The score field stores the solution's score after constraint evaluation.
/// Only one field per solution class should have this annotation.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Timetable:
///     score: Annotated[Optional[HardSoftScore], PlanningScore]
/// ```
#[pyclass(name = "PlanningScore")]
#[derive(Clone, Debug)]
pub struct PyPlanningScore;

#[pymethods]
impl PyPlanningScore {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "PlanningScore()"
    }
}

/// Marks a collection as providing values for planning variables.
///
/// The solver uses value range providers to know what values can be assigned
/// to planning variables.
///
/// # Parameters
///
/// - `id`: Unique identifier for this value range provider
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Timetable:
///     timeslots: Annotated[List[Timeslot], ProblemFactCollectionProperty, ValueRangeProvider(id='timeslots')]
/// ```
#[pyclass(name = "ValueRangeProvider")]
#[derive(Clone, Debug)]
pub struct PyValueRangeProvider {
    #[pyo3(get)]
    pub id: Option<String>,
}

#[pymethods]
impl PyValueRangeProvider {
    #[new]
    #[pyo3(signature = (id=None))]
    fn new(id: Option<String>) -> Self {
        Self { id }
    }

    fn __repr__(&self) -> String {
        match &self.id {
            Some(id) => format!("ValueRangeProvider(id='{}')", id),
            None => "ValueRangeProvider()".to_string(),
        }
    }
}

/// Marks a field as a single problem fact property.
///
/// Problem facts are immutable input data that constraints can reference.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Schedule:
///     config: Annotated[Config, ProblemFactProperty]
/// ```
#[pyclass(name = "ProblemFactProperty")]
#[derive(Clone, Debug)]
pub struct PyProblemFactProperty;

#[pymethods]
impl PyProblemFactProperty {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "ProblemFactProperty()"
    }
}

/// Marks a field as a collection of problem facts.
///
/// Problem fact collections contain immutable input data that constraints can reference.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Timetable:
///     timeslots: Annotated[List[Timeslot], ProblemFactCollectionProperty]
/// ```
#[pyclass(name = "ProblemFactCollectionProperty")]
#[derive(Clone, Debug)]
pub struct PyProblemFactCollectionProperty;

#[pymethods]
impl PyProblemFactCollectionProperty {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "ProblemFactCollectionProperty()"
    }
}

/// Marks a field as a single planning entity property.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Schedule:
///     main_vehicle: Annotated[Vehicle, PlanningEntityProperty]
/// ```
#[pyclass(name = "PlanningEntityProperty")]
#[derive(Clone, Debug)]
pub struct PyPlanningEntityProperty;

#[pymethods]
impl PyPlanningEntityProperty {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "PlanningEntityProperty()"
    }
}

/// Marks a field as a collection of planning entities.
///
/// Planning entity collections contain entities with planning variables that
/// the solver will optimize.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Timetable:
///     lessons: Annotated[List[Lesson], PlanningEntityCollectionProperty]
/// ```
#[pyclass(name = "PlanningEntityCollectionProperty")]
#[derive(Clone, Debug)]
pub struct PyPlanningEntityCollectionProperty;

#[pymethods]
impl PyPlanningEntityCollectionProperty {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "PlanningEntityCollectionProperty()"
    }
}

/// Marks an entity as pinned (immovable by the solver).
///
/// Pinned entities have their planning variables fixed and won't be changed.
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Lesson:
///     pinned: Annotated[bool, PlanningPin]
/// ```
#[pyclass(name = "PlanningPin")]
#[derive(Clone, Debug)]
pub struct PyPlanningPin;

#[pymethods]
impl PyPlanningPin {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "PlanningPin()"
    }
}

/// Marks a field as an inverse relation shadow variable.
///
/// Shadow variables are automatically updated by the solver based on genuine planning variables.
/// An inverse relation shadow variable tracks the inverse of a planning list variable.
///
/// # Parameters
///
/// - `source_variable_name`: Name of the planning variable this shadows
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Visit:
///     vehicle: Annotated[Optional['Vehicle'], InverseRelationShadowVariable(source_variable_name='visits')]
/// ```
#[pyclass(name = "InverseRelationShadowVariable")]
#[derive(Clone, Debug)]
pub struct PyInverseRelationShadowVariable {
    #[pyo3(get)]
    pub source_variable_name: String,
}

#[pymethods]
impl PyInverseRelationShadowVariable {
    #[new]
    fn new(source_variable_name: String) -> Self {
        Self {
            source_variable_name,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "InverseRelationShadowVariable(source_variable_name='{}')",
            self.source_variable_name
        )
    }
}

/// Marks a field as a previous element shadow variable.
///
/// Automatically tracks the previous element in a planning list variable.
/// Used with PlanningListVariable to access the element before this one.
///
/// # Parameters
///
/// - `source_variable_name`: Name of the planning list variable this shadows
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Visit:
///     previous_visit: Annotated[Optional['Visit'], PreviousElementShadowVariable(source_variable_name='visits')]
/// ```
#[pyclass(name = "PreviousElementShadowVariable")]
#[derive(Clone, Debug)]
pub struct PyPreviousElementShadowVariable {
    #[pyo3(get)]
    pub source_variable_name: String,
}

#[pymethods]
impl PyPreviousElementShadowVariable {
    #[new]
    fn new(source_variable_name: String) -> Self {
        Self {
            source_variable_name,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PreviousElementShadowVariable(source_variable_name='{}')",
            self.source_variable_name
        )
    }
}

/// Marks a field as a next element shadow variable.
///
/// Automatically tracks the next element in a planning list variable.
/// Used with PlanningListVariable to access the element after this one.
///
/// # Parameters
///
/// - `source_variable_name`: Name of the planning list variable this shadows
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Visit:
///     next_visit: Annotated[Optional['Visit'], NextElementShadowVariable(source_variable_name='visits')]
/// ```
#[pyclass(name = "NextElementShadowVariable")]
#[derive(Clone, Debug)]
pub struct PyNextElementShadowVariable {
    #[pyo3(get)]
    pub source_variable_name: String,
}

#[pymethods]
impl PyNextElementShadowVariable {
    #[new]
    fn new(source_variable_name: String) -> Self {
        Self {
            source_variable_name,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "NextElementShadowVariable(source_variable_name='{}')",
            self.source_variable_name
        )
    }
}

/// Marks a field as a cascading update shadow variable.
///
/// A shadow variable that is updated based on other shadow variables or planning variables,
/// with updates cascading through a chain of elements.
///
/// # Parameters
///
/// - `target_method_name`: Name of the method to call for computing the shadow value
/// - `source_variable_name`: Optional name of the source variable that triggers updates
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Visit:
///     arrival_time: Annotated[datetime, CascadingUpdateShadowVariable(
///         target_method_name='calculate_arrival_time',
///         source_variable_name='visits'
///     )]
/// ```
#[pyclass(name = "CascadingUpdateShadowVariable")]
#[derive(Clone, Debug)]
pub struct PyCascadingUpdateShadowVariable {
    #[pyo3(get)]
    pub target_method_name: String,
    #[pyo3(get)]
    pub source_variable_name: Option<String>,
}

#[pymethods]
impl PyCascadingUpdateShadowVariable {
    #[new]
    #[pyo3(signature = (target_method_name, source_variable_name=None))]
    fn new(target_method_name: String, source_variable_name: Option<String>) -> Self {
        Self {
            target_method_name,
            source_variable_name,
        }
    }

    fn __repr__(&self) -> String {
        match &self.source_variable_name {
            Some(src) => format!(
                "CascadingUpdateShadowVariable(target_method_name='{}', source_variable_name='{}')",
                self.target_method_name, src
            ),
            None => format!(
                "CascadingUpdateShadowVariable(target_method_name='{}')",
                self.target_method_name
            ),
        }
    }
}

/// Marks a field as requiring deep planning cloning.
///
/// Not needed for `planning_solution` or `planning_entity` attributes because those
/// are automatically deep cloned. Use this for problem facts or other fields that
/// need to be cloned when creating working solutions.
///
/// When applied to a list or dict, the container is cloned, but elements are only
/// cloned if they are of a type that needs planning cloning.
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Employee:
///     work_day_to_hours: Annotated[dict[date, int], ShadowVariable(...), DeepPlanningClone]
/// ```
#[pyclass(name = "DeepPlanningClone")]
#[derive(Clone, Debug)]
pub struct PyDeepPlanningClone;

#[pymethods]
impl PyDeepPlanningClone {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "DeepPlanningClone()"
    }
}

/// Decorator to mark a problem fact class as requiring deep planning cloning.
///
/// Not needed for a `planning_solution` or `planning_entity` because those are
/// automatically deep cloned. Use this for problem fact classes that need to be
/// cloned when creating working solutions.
///
/// To annotate an attribute instead of a class, use `DeepPlanningClone`.
///
/// # Example
///
/// ```python
/// @deep_planning_clone
/// class WorkSchedule:
///     shifts: list[Shift]
///     preferences: dict[str, int]
/// ```
#[pyfunction]
#[pyo3(name = "deep_planning_clone")]
pub fn py_deep_planning_clone(_py: Python<'_>, cls: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    // Mark the class as requiring deep cloning
    cls.setattr("__solverforge_deep_clone__", true)?;

    // Return the class unchanged
    Ok(cls.clone().unbind())
}

/// Register annotation classes with the Python module.
pub fn register_annotations(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPlanningId>()?;
    m.add_class::<PyPlanningVariable>()?;
    m.add_class::<PyPlanningListVariable>()?;
    m.add_class::<PyPlanningScore>()?;
    m.add_class::<PyValueRangeProvider>()?;
    m.add_class::<PyProblemFactProperty>()?;
    m.add_class::<PyProblemFactCollectionProperty>()?;
    m.add_class::<PyPlanningEntityProperty>()?;
    m.add_class::<PyPlanningEntityCollectionProperty>()?;
    m.add_class::<PyPlanningPin>()?;
    m.add_class::<PyInverseRelationShadowVariable>()?;
    m.add_class::<PyPreviousElementShadowVariable>()?;
    m.add_class::<PyNextElementShadowVariable>()?;
    m.add_class::<PyCascadingUpdateShadowVariable>()?;
    m.add_class::<PyDeepPlanningClone>()?;
    m.add_function(wrap_pyfunction!(py_deep_planning_clone, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_id() {
        let id = PyPlanningId::new();
        assert_eq!(id.__repr__(), "PlanningId()");
    }

    #[test]
    fn test_planning_variable() {
        let var = PyPlanningVariable::new(vec!["timeslots".to_string()], false);
        assert_eq!(var.value_range_provider_refs, vec!["timeslots"]);
        assert!(!var.allows_unassigned);
        assert!(var.__repr__().contains("timeslots"));
    }

    #[test]
    fn test_planning_variable_allows_unassigned() {
        let var = PyPlanningVariable::new(vec!["rooms".to_string()], true);
        assert!(var.allows_unassigned);
    }

    #[test]
    fn test_planning_list_variable() {
        let var = PyPlanningListVariable::new(vec!["visits".to_string()]);
        assert_eq!(var.value_range_provider_refs, vec!["visits"]);
    }

    #[test]
    fn test_planning_score() {
        let score = PyPlanningScore::new();
        assert_eq!(score.__repr__(), "PlanningScore()");
    }

    #[test]
    fn test_value_range_provider_with_id() {
        let vrp = PyValueRangeProvider::new(Some("timeslots".to_string()));
        assert_eq!(vrp.id, Some("timeslots".to_string()));
        assert!(vrp.__repr__().contains("timeslots"));
    }

    #[test]
    fn test_value_range_provider_without_id() {
        let vrp = PyValueRangeProvider::new(None);
        assert_eq!(vrp.id, None);
        assert_eq!(vrp.__repr__(), "ValueRangeProvider()");
    }

    #[test]
    fn test_problem_fact_property() {
        let prop = PyProblemFactProperty::new();
        assert_eq!(prop.__repr__(), "ProblemFactProperty()");
    }

    #[test]
    fn test_problem_fact_collection_property() {
        let prop = PyProblemFactCollectionProperty::new();
        assert_eq!(prop.__repr__(), "ProblemFactCollectionProperty()");
    }

    #[test]
    fn test_planning_entity_property() {
        let prop = PyPlanningEntityProperty::new();
        assert_eq!(prop.__repr__(), "PlanningEntityProperty()");
    }

    #[test]
    fn test_planning_entity_collection_property() {
        let prop = PyPlanningEntityCollectionProperty::new();
        assert_eq!(prop.__repr__(), "PlanningEntityCollectionProperty()");
    }

    #[test]
    fn test_planning_pin() {
        let pin = PyPlanningPin::new();
        assert_eq!(pin.__repr__(), "PlanningPin()");
    }

    #[test]
    fn test_inverse_relation_shadow_variable() {
        let shadow = PyInverseRelationShadowVariable::new("visits".to_string());
        assert_eq!(shadow.source_variable_name, "visits");
        assert!(shadow.__repr__().contains("visits"));
    }

    #[test]
    fn test_previous_element_shadow_variable() {
        let shadow = PyPreviousElementShadowVariable::new("visits".to_string());
        assert_eq!(shadow.source_variable_name, "visits");
        assert!(shadow.__repr__().contains("visits"));
        assert!(shadow.__repr__().contains("PreviousElementShadowVariable"));
    }

    #[test]
    fn test_next_element_shadow_variable() {
        let shadow = PyNextElementShadowVariable::new("visits".to_string());
        assert_eq!(shadow.source_variable_name, "visits");
        assert!(shadow.__repr__().contains("visits"));
        assert!(shadow.__repr__().contains("NextElementShadowVariable"));
    }

    #[test]
    fn test_cascading_update_shadow_variable() {
        let shadow = PyCascadingUpdateShadowVariable::new(
            "calculate_arrival".to_string(),
            Some("visits".to_string()),
        );
        assert_eq!(shadow.target_method_name, "calculate_arrival");
        assert_eq!(shadow.source_variable_name, Some("visits".to_string()));
        assert!(shadow.__repr__().contains("calculate_arrival"));
        assert!(shadow.__repr__().contains("visits"));
    }

    #[test]
    fn test_cascading_update_shadow_variable_no_source() {
        let shadow = PyCascadingUpdateShadowVariable::new("update_time".to_string(), None);
        assert_eq!(shadow.target_method_name, "update_time");
        assert_eq!(shadow.source_variable_name, None);
        assert!(shadow.__repr__().contains("update_time"));
        assert!(!shadow.__repr__().contains("source_variable_name"));
    }

    #[test]
    fn test_deep_planning_clone() {
        let clone = PyDeepPlanningClone::new();
        assert_eq!(clone.__repr__(), "DeepPlanningClone()");
    }

    #[test]
    fn test_deep_planning_clone_decorator() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            use pyo3::types::PyDict;
            let locals = PyDict::new(py);
            py.run(c"class WorkSchedule:\n    pass", None, Some(&locals))
                .unwrap();
            let cls = locals.get_item("WorkSchedule").unwrap().unwrap();

            // Apply the decorator
            let result = py_deep_planning_clone(py, &cls).unwrap();

            // Check that the marker was set
            let marker = result.getattr(py, "__solverforge_deep_clone__").unwrap();
            assert!(marker.extract::<bool>(py).unwrap());
        });
    }
}
