//! Python decorators for marking planning entities and solutions.
//!
//! These decorators inspect Python class annotations and build domain metadata
//! for the constraint solver.
//!
//! # Example
//!
//! ```python
//! from typing import Annotated, Optional
//! from solverforge import planning_entity, PlanningId, PlanningVariable
//!
//! @planning_entity
//! class Lesson:
//!     id: Annotated[str, PlanningId]
//!     subject: str
//!     timeslot: Annotated[Optional['Timeslot'], PlanningVariable(value_range_provider_refs=['timeslots'])]
//! ```

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple, PyType};
use solverforge_core::domain::{
    DomainClass, DomainModel, DomainModelBuilder, FieldDescriptor, FieldType, PlanningAnnotation,
    PrimitiveType, ShadowAnnotation,
};

use crate::annotations::{
    PyInverseRelationShadowVariable, PyPlanningEntityCollectionProperty, PyPlanningEntityProperty,
    PyPlanningId, PyPlanningListVariable, PyPlanningPin, PyPlanningScore, PyPlanningVariable,
    PyProblemFactCollectionProperty, PyProblemFactProperty, PyValueRangeProvider,
};

/// Create an Object field type with an unknown class name.
fn object_type() -> FieldType {
    FieldType::Object {
        class_name: "object".to_string(),
    }
}

/// Create a List field type with the given element type.
fn list_type(element_type: FieldType) -> FieldType {
    FieldType::List {
        element_type: Box::new(element_type),
    }
}

/// Build a DomainClass from a Python class by inspecting its annotations.
fn build_domain_class(py: Python<'_>, cls: &Bound<'_, PyType>) -> PyResult<DomainClass> {
    let class_name: String = cls.getattr("__name__")?.extract()?;
    let mut domain_class = DomainClass::new(&class_name);

    // Get raw annotations directly - don't use get_type_hints() as it fails
    // on forward references that aren't yet defined at decoration time
    let annotations: Bound<'_, PyDict> = match cls.getattr("__annotations__") {
        Ok(ann) => ann.cast_into()?,
        Err(_) => return Ok(domain_class), // No annotations, return empty class
    };

    for (field_name, raw_annotation) in annotations.iter() {
        let field_name_str: String = field_name.extract()?;

        // Check if it's an Annotated type
        let (field_type, planning_annotations, shadow_annotations) =
            extract_annotations(py, &raw_annotation)?;

        // Only add fields that have planning annotations
        if !planning_annotations.is_empty() || !shadow_annotations.is_empty() {
            let mut field_desc = FieldDescriptor::new(&field_name_str, field_type);
            for ann in planning_annotations {
                field_desc = field_desc.with_planning_annotation(ann);
            }
            for ann in shadow_annotations {
                field_desc = field_desc.with_shadow_annotation(ann);
            }
            domain_class = domain_class.with_field(field_desc);
        }
    }

    Ok(domain_class)
}

/// Extract field type and annotations from a potentially Annotated type.
fn extract_annotations(
    py: Python<'_>,
    annotation: &Bound<'_, PyAny>,
) -> PyResult<(FieldType, Vec<PlanningAnnotation>, Vec<ShadowAnnotation>)> {
    let typing = py.import("typing")?;
    let get_origin = typing.getattr("get_origin")?;
    let get_args = typing.getattr("get_args")?;
    let annotated_type = typing.getattr("Annotated")?;

    // Check if this is an Annotated type
    let origin = get_origin.call1((annotation,))?;
    if origin.is(&annotated_type) {
        // Get the args: (base_type, *annotations)
        let args: Bound<'_, PyTuple> = get_args.call1((annotation,))?.cast_into()?;
        if args.len() < 2 {
            return Ok((object_type(), vec![], vec![]));
        }

        let base_type = args.get_item(0)?;
        let field_type = python_type_to_field_type(py, &base_type)?;

        let mut planning_annotations = Vec::new();
        let mut shadow_annotations = Vec::new();

        // Process annotation markers (args[1:])
        for i in 1..args.len() {
            let marker = args.get_item(i)?;

            // Check each annotation type
            if marker.is_instance_of::<PyPlanningId>() {
                planning_annotations.push(PlanningAnnotation::PlanningId);
            } else if let Ok(pv) = marker.cast::<PyPlanningVariable>() {
                let pv_ref = pv.borrow();
                if pv_ref.allows_unassigned {
                    planning_annotations.push(PlanningAnnotation::planning_variable_unassigned(
                        pv_ref.value_range_provider_refs.clone(),
                    ));
                } else {
                    planning_annotations.push(PlanningAnnotation::planning_variable(
                        pv_ref.value_range_provider_refs.clone(),
                    ));
                }
            } else if let Ok(plv) = marker.cast::<PyPlanningListVariable>() {
                let plv_ref = plv.borrow();
                planning_annotations.push(PlanningAnnotation::planning_list_variable(
                    plv_ref.value_range_provider_refs.clone(),
                ));
            } else if marker.is_instance_of::<PyPlanningScore>() {
                planning_annotations.push(PlanningAnnotation::PlanningScore {
                    bendable_hard_levels: None,
                    bendable_soft_levels: None,
                });
            } else if let Ok(vrp) = marker.cast::<PyValueRangeProvider>() {
                let vrp_ref = vrp.borrow();
                planning_annotations.push(PlanningAnnotation::ValueRangeProvider {
                    id: vrp_ref.id.clone(),
                });
            } else if marker.is_instance_of::<PyProblemFactProperty>() {
                planning_annotations.push(PlanningAnnotation::ProblemFactProperty);
            } else if marker.is_instance_of::<PyProblemFactCollectionProperty>() {
                planning_annotations.push(PlanningAnnotation::ProblemFactCollectionProperty);
            } else if marker.is_instance_of::<PyPlanningEntityProperty>() {
                planning_annotations.push(PlanningAnnotation::PlanningEntityProperty);
            } else if marker.is_instance_of::<PyPlanningEntityCollectionProperty>() {
                planning_annotations.push(PlanningAnnotation::PlanningEntityCollectionProperty);
            } else if marker.is_instance_of::<PyPlanningPin>() {
                planning_annotations.push(PlanningAnnotation::PlanningPin);
            } else if let Ok(shadow) = marker.cast::<PyInverseRelationShadowVariable>() {
                let shadow_ref = shadow.borrow();
                shadow_annotations.push(ShadowAnnotation::InverseRelationShadowVariable {
                    source_variable_name: shadow_ref.source_variable_name.clone(),
                });
            }
        }

        Ok((field_type, planning_annotations, shadow_annotations))
    } else {
        // Not an Annotated type, just extract the field type
        let field_type = python_type_to_field_type(py, annotation)?;
        Ok((field_type, vec![], vec![]))
    }
}

/// Convert a Python type to a FieldType.
fn python_type_to_field_type(py: Python<'_>, type_hint: &Bound<'_, PyAny>) -> PyResult<FieldType> {
    let typing = py.import("typing")?;
    let get_origin = typing.getattr("get_origin")?;
    let get_args = typing.getattr("get_args")?;

    // Check for None/NoneType
    if type_hint.is_none() {
        return Ok(object_type());
    }

    // Handle string type hints (forward references)
    if type_hint.is_instance_of::<PyString>() {
        let type_name: String = type_hint.extract()?;
        return Ok(FieldType::Object {
            class_name: type_name,
        });
    }

    // Check origin for generic types
    let origin = get_origin.call1((type_hint,))?;

    // Handle Optional[T] which is Union[T, None]
    let union_type = typing.getattr("Union")?;
    if origin.is(&union_type) {
        let args: Bound<'_, PyTuple> = get_args.call1((type_hint,))?.cast_into()?;
        // Filter out NoneType
        let none_type_repr = "<class 'NoneType'>";
        for i in 0..args.len() {
            let arg = args.get_item(i)?;
            let arg_repr = arg.repr()?;
            if !arg.is_none() && arg_repr.to_cow()? != none_type_repr {
                return python_type_to_field_type(py, &arg);
            }
        }
        return Ok(object_type());
    }

    // Handle List[T]
    if !origin.is_none() {
        let origin_name = origin.repr()?.to_string();
        if origin_name.contains("list") {
            let args: Bound<'_, PyTuple> = get_args.call1((type_hint,))?.cast_into()?;
            if !args.is_empty() {
                let element_type = python_type_to_field_type(py, &args.get_item(0)?)?;
                return Ok(list_type(element_type));
            }
            return Ok(list_type(object_type()));
        }
    }

    // Check for built-in types
    let builtins = py.import("builtins")?;

    // Try to match against built-in types
    if let Ok(str_type) = builtins.getattr("str") {
        if type_hint.is(&str_type) {
            return Ok(FieldType::Primitive(PrimitiveType::String));
        }
    }
    if let Ok(int_type) = builtins.getattr("int") {
        if type_hint.is(&int_type) {
            return Ok(FieldType::Primitive(PrimitiveType::Long));
        }
    }
    if let Ok(float_type) = builtins.getattr("float") {
        if type_hint.is(&float_type) {
            return Ok(FieldType::Primitive(PrimitiveType::Double));
        }
    }
    if let Ok(bool_type) = builtins.getattr("bool") {
        if type_hint.is(&bool_type) {
            return Ok(FieldType::Primitive(PrimitiveType::Bool));
        }
    }

    // Try to extract class name for complex types
    if let Ok(name) = type_hint.getattr("__name__") {
        if let Ok(class_name) = name.extract::<String>() {
            return Ok(FieldType::Object { class_name });
        }
    }

    // Default to Object for unknown types
    Ok(object_type())
}

/// The @planning_entity decorator marks a class as a planning entity.
///
/// A planning entity has one or more planning variables that the solver
/// will modify to find an optimal solution.
///
/// # Example
///
/// ```python
/// @planning_entity
/// class Lesson:
///     id: Annotated[str, PlanningId]
///     timeslot: Annotated[Optional['Timeslot'], PlanningVariable(value_range_provider_refs=['timeslots'])]
/// ```
#[pyfunction]
pub fn planning_entity(py: Python<'_>, cls: &Bound<'_, PyType>) -> PyResult<Py<PyType>> {
    // Build domain class from annotations
    let mut domain_class = build_domain_class(py, cls)?;

    // Add PlanningEntity annotation to the class
    domain_class = domain_class.with_annotation(PlanningAnnotation::PlanningEntity);

    // Serialize to JSON and store on the class
    let json = serde_json::to_string(&domain_class)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    cls.setattr("__solverforge_domain_class__", json)?;

    // Return the class unchanged (decorator pattern)
    Ok(cls.clone().unbind())
}

/// The @planning_solution decorator marks a class as a planning solution.
///
/// A planning solution contains the problem data (problem facts), planning entities,
/// and a score field that represents the solution quality.
///
/// # Example
///
/// ```python
/// @planning_solution
/// class Timetable:
///     timeslots: Annotated[List[Timeslot], ProblemFactCollectionProperty, ValueRangeProvider(id='timeslots')]
///     lessons: Annotated[List[Lesson], PlanningEntityCollectionProperty]
///     score: Annotated[Optional[HardSoftScore], PlanningScore]
/// ```
#[pyfunction]
pub fn planning_solution(py: Python<'_>, cls: &Bound<'_, PyType>) -> PyResult<Py<PyType>> {
    // Build domain class from annotations
    let mut domain_class = build_domain_class(py, cls)?;

    // Add PlanningSolution annotation to the class
    domain_class = domain_class.with_annotation(PlanningAnnotation::PlanningSolution);

    // Serialize to JSON and store on the class
    let json = serde_json::to_string(&domain_class)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    cls.setattr("__solverforge_domain_class__", json)?;

    // Return the class unchanged (decorator pattern)
    Ok(cls.clone().unbind())
}

/// PyDomainClass wraps a DomainClass for Python access.
#[pyclass(name = "DomainClass")]
#[derive(Clone, Debug)]
pub struct PyDomainClass {
    inner: DomainClass,
}

#[pymethods]
impl PyDomainClass {
    /// Get the class name.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the number of fields.
    fn field_count(&self) -> usize {
        self.inner.fields.len()
    }

    /// Get field names.
    fn field_names(&self) -> Vec<String> {
        self.inner.fields.iter().map(|f| f.name.clone()).collect()
    }

    /// Check if this is a planning entity.
    fn is_planning_entity(&self) -> bool {
        self.inner.is_planning_entity()
    }

    /// Check if this is a planning solution.
    fn is_planning_solution(&self) -> bool {
        self.inner.is_planning_solution()
    }

    /// Get the JSON representation.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DomainClass(name='{}', fields={})",
            self.inner.name,
            self.inner.fields.len()
        )
    }
}

impl PyDomainClass {
    pub fn from_rust(inner: DomainClass) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> DomainClass {
        self.inner.clone()
    }
}

/// PyDomainModel wraps a DomainModel for Python access.
#[pyclass(name = "DomainModel")]
#[derive(Clone, Debug)]
pub struct PyDomainModel {
    inner: DomainModel,
}

#[pymethods]
impl PyDomainModel {
    /// Create a new empty domain model.
    #[new]
    fn new() -> Self {
        Self {
            inner: DomainModel::new(),
        }
    }

    /// Get the solution class name.
    #[getter]
    fn solution_class(&self) -> Option<String> {
        self.inner.solution_class.clone()
    }

    /// Get entity class names.
    #[getter]
    fn entity_classes(&self) -> Vec<String> {
        self.inner.entity_classes.clone()
    }

    /// Get the number of classes in the model.
    fn class_count(&self) -> usize {
        self.inner.classes.len()
    }

    /// Get all class names.
    fn class_names(&self) -> Vec<String> {
        self.inner.classes.keys().cloned().collect()
    }

    /// Get a domain class by name.
    fn get_class(&self, name: &str) -> Option<PyDomainClass> {
        self.inner
            .get_class(name)
            .cloned()
            .map(PyDomainClass::from_rust)
    }

    /// Get the JSON representation.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DomainModel(solution='{}', entities={:?}, classes={})",
            self.inner.solution_class.as_deref().unwrap_or("None"),
            self.inner.entity_classes,
            self.inner.classes.len()
        )
    }
}

impl PyDomainModel {
    pub fn from_rust(inner: DomainModel) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> DomainModel {
        self.inner.clone()
    }
}

/// Build a DomainModel from a solution class and its referenced entity classes.
///
/// This function collects domain metadata from decorated classes and builds
/// a complete domain model for the solver.
#[pyfunction]
pub fn build_domain_model(
    solution_cls: &Bound<'_, PyType>,
    entity_classes: Vec<Bound<'_, PyType>>,
) -> PyResult<PyDomainModel> {
    let mut builder = DomainModelBuilder::new();

    // Add solution class
    let solution_json: String = solution_cls
        .getattr("__solverforge_domain_class__")?
        .extract()?;
    let solution_class: DomainClass = serde_json::from_str(&solution_json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    builder = builder.add_class(solution_class);

    // Add entity classes
    for entity_cls in entity_classes {
        let entity_json: String = entity_cls
            .getattr("__solverforge_domain_class__")?
            .extract()?;
        let entity_class: DomainClass = serde_json::from_str(&entity_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        builder = builder.add_class(entity_class);
    }

    Ok(PyDomainModel::from_rust(builder.build()))
}

/// Get the domain class metadata from a decorated class.
#[pyfunction]
pub fn get_domain_class(cls: &Bound<'_, PyType>) -> PyResult<Option<PyDomainClass>> {
    match cls.getattr("__solverforge_domain_class__") {
        Ok(json) => {
            let json_str: String = json.extract()?;
            let domain_class: DomainClass = serde_json::from_str(&json_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Ok(Some(PyDomainClass::from_rust(domain_class)))
        }
        Err(_) => Ok(None),
    }
}

/// Wrapper class for constraint provider functions.
///
/// This stores the decorated function and provides access to the constraints.
#[pyclass(name = "ConstraintProvider")]
pub struct PyConstraintProvider {
    /// The constraint provider function
    func: Py<PyAny>,
    /// The function name
    name: String,
}

impl Clone for PyConstraintProvider {
    fn clone(&self) -> Self {
        Python::attach(|py| Self {
            func: self.func.clone_ref(py),
            name: self.name.clone(),
        })
    }
}

impl std::fmt::Debug for PyConstraintProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstraintProvider")
            .field("name", &self.name)
            .finish()
    }
}

#[pymethods]
impl PyConstraintProvider {
    /// Get the constraint provider name.
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the underlying function.
    fn get_function(&self) -> Py<PyAny> {
        Python::attach(|py| self.func.clone_ref(py))
    }

    fn __repr__(&self) -> String {
        format!("ConstraintProvider(name='{}')", self.name)
    }

    fn __call__(
        &self,
        py: Python<'_>,
        factory: &crate::stream::PyConstraintFactory,
    ) -> PyResult<Py<PyAny>> {
        let factory_obj = factory.clone().into_pyobject(py)?;
        self.func.call1(py, (factory_obj,))
    }
}

impl PyConstraintProvider {
    /// Create a new constraint provider from a function.
    pub fn new(func: Py<PyAny>, name: String) -> Self {
        Self { func, name }
    }

    /// Get the provider name (Rust API).
    pub fn provider_name(&self) -> &str {
        &self.name
    }

    /// Get the inner function.
    pub fn func(&self) -> &Py<PyAny> {
        &self.func
    }

    /// Get constraints from this provider (Rust API).
    pub fn get_constraints(&self, py: Python<'_>) -> PyResult<Vec<crate::stream::PyConstraint>> {
        use crate::stream::PyConstraintFactory;

        let factory = PyConstraintFactory::create();
        let factory_obj = factory.into_pyobject(py)?;

        let result = self.func.call1(py, (factory_obj,))?;

        // The result should be a list of constraints
        let constraints: Vec<crate::stream::PyConstraint> = result.bind(py).extract()?;
        Ok(constraints)
    }
}

/// The @constraint_provider decorator marks a function as a constraint provider.
///
/// A constraint provider is a function that takes a ConstraintFactory and returns
/// a list of constraints that define the rules for the solver.
///
/// # Example
///
/// ```python
/// @constraint_provider
/// def define_constraints(factory: ConstraintFactory):
///     return [
///         factory.for_each(Lesson)
///             .filter(lambda lesson: lesson.room is None)
///             .penalize(HardSoftScore.ONE_HARD)
///             .as_constraint("Room required"),
///     ]
/// ```
#[pyfunction]
pub fn constraint_provider(py: Python<'_>, func: Py<PyAny>) -> PyResult<PyConstraintProvider> {
    // Get the function name
    let name: String = func
        .bind(py)
        .getattr("__name__")
        .and_then(|n| n.extract())
        .unwrap_or_else(|_| "constraint_provider".to_string());

    // Verify it's callable
    if !func.bind(py).is_callable() {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "constraint_provider decorator must be applied to a callable",
        ));
    }

    Ok(PyConstraintProvider::new(func, name))
}

/// Register decorator functions with the Python module.
pub fn register_decorators(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(planning_entity, m)?)?;
    m.add_function(wrap_pyfunction!(planning_solution, m)?)?;
    m.add_function(wrap_pyfunction!(constraint_provider, m)?)?;
    m.add_function(wrap_pyfunction!(get_domain_class, m)?)?;
    m.add_function(wrap_pyfunction!(build_domain_model, m)?)?;
    m.add_class::<PyDomainClass>()?;
    m.add_class::<PyDomainModel>()?;
    m.add_class::<PyConstraintProvider>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    fn init_python() {
        pyo3::prepare_freethreaded_python();
    }

    #[test]
    fn test_domain_class_wrapper() {
        let dc = DomainClass::new("TestEntity")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(
                FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                    .with_planning_annotation(PlanningAnnotation::PlanningId),
            );

        let py_dc = PyDomainClass::from_rust(dc);
        assert_eq!(py_dc.name(), "TestEntity");
        assert_eq!(py_dc.field_count(), 1);
        assert!(py_dc.is_planning_entity());
        assert!(!py_dc.is_planning_solution());
    }

    #[test]
    fn test_domain_class_to_json() {
        let dc = DomainClass::new("Lesson").with_annotation(PlanningAnnotation::PlanningEntity);

        let py_dc = PyDomainClass::from_rust(dc);
        let json = py_dc.to_json().unwrap();
        assert!(json.contains("Lesson"));
        assert!(json.contains("PlanningEntity"));
    }

    #[test]
    fn test_constraint_provider_creation() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"def define_constraints(factory): return []",
                None,
                Some(&locals),
            )
            .unwrap();
            let func = locals.get_item("define_constraints").unwrap().unwrap();

            let provider = constraint_provider(py, func.unbind()).unwrap();
            assert_eq!(provider.name(), "define_constraints");
        });
    }

    #[test]
    fn test_constraint_provider_repr() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"def my_provider(factory): return []", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("my_provider").unwrap().unwrap();

            let provider = constraint_provider(py, func.unbind()).unwrap();
            assert_eq!(
                provider.__repr__(),
                "ConstraintProvider(name='my_provider')"
            );
        });
    }

    #[test]
    fn test_constraint_provider_get_function() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"def test_func(factory): return []", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("test_func").unwrap().unwrap();
            let func_py = func.unbind();

            let provider = constraint_provider(py, func_py.clone_ref(py)).unwrap();
            let retrieved = provider.get_function();

            // Verify we can call the retrieved function
            assert!(retrieved.bind(py).is_callable());
        });
    }

    #[test]
    fn test_constraint_provider_rejects_non_callable() {
        init_python();
        Python::attach(|py| {
            // Create a non-callable object (an integer)
            let not_callable = 42_i32.into_pyobject(py).unwrap();

            let result = constraint_provider(py, not_callable.unbind().into_any());
            assert!(result.is_err());
            let err = result.unwrap_err();
            let err_str = err.to_string();
            assert!(err_str.contains("callable"));
        });
    }

    #[test]
    fn test_constraint_provider_with_lambda() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda factory: []", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let provider = constraint_provider(py, func.unbind()).unwrap();
            // Lambda name is "<lambda>"
            assert_eq!(provider.name(), "<lambda>");
        });
    }

    #[test]
    fn test_constraint_provider_new() {
        init_python();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"def my_func(factory): pass", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("my_func").unwrap().unwrap();

            let provider = PyConstraintProvider::new(func.unbind(), "custom_name".to_string());
            assert_eq!(provider.name(), "custom_name");
        });
    }
}
