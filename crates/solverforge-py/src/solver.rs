//! Synchronous solver for Python API.

use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use solverforge_dynamic::{
    DynamicConstraintSet, DynamicDescriptor, DynamicEntity, DynamicSolution, DynamicValue,
    EntityClassDef, FieldDef, SolveConfig, ValueRangeDef,
};

use crate::constraint_builder::{build_constraint, ConstraintBuilder};
use crate::convert::{parse_field_type, parse_weight, py_to_dynamic};
use crate::solve_result::PySolveResult;

/// The main Solver class for defining and solving constraint problems.
#[pyclass]
pub struct Solver {
    pub(crate) descriptor: DynamicDescriptor,
    entities: Vec<Vec<DynamicEntity>>,
    constraints: Vec<ConstraintBuilder>,
    next_entity_id: i64,
}

#[pymethods]
impl Solver {
    /// Creates a new Solver.
    #[new]
    fn new() -> Self {
        Self {
            descriptor: DynamicDescriptor::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
            next_entity_id: 0,
        }
    }

    /// Define an entity class.
    ///
    /// Fields are specified as a list of tuples:
    /// - ("field_name", "type") for regular fields
    /// - ("field_name", "type", {"planning_variable": True, "value_range": "range_name"}) for planning variables
    #[pyo3(signature = (name, fields))]
    fn entity_class(&mut self, name: &str, fields: &Bound<'_, PyList>) -> PyResult<()> {
        let mut field_defs = Vec::new();

        for field_spec in fields.iter() {
            let tuple = field_spec.downcast::<pyo3::types::PyTuple>()?;
            let field_name: String = tuple.get_item(0)?.extract()?;
            let field_type_str: String = tuple.get_item(1)?.extract()?;
            let field_type = parse_field_type(&field_type_str)?;

            let field_def = if tuple.len() > 2 {
                let options_item = tuple.get_item(2)?;
                let options = options_item.downcast::<PyDict>()?;
                if options.get_item("planning_variable")?.is_some() {
                    let value_range: String = options
                        .get_item("value_range")?
                        .ok_or_else(|| {
                            PyValueError::new_err("Planning variable must have 'value_range'")
                        })?
                        .extract()?;
                    FieldDef::planning_variable(field_name, field_type, value_range)
                } else {
                    FieldDef::new(field_name, field_type)
                }
            } else {
                FieldDef::new(field_name, field_type)
            };

            field_defs.push(field_def);
        }

        self.descriptor
            .add_entity_class(EntityClassDef::new(name, field_defs));
        self.entities.push(Vec::new());

        Ok(())
    }

    /// Define a value range with explicit values.
    fn value_range(&mut self, name: &str, values: &Bound<'_, PyList>) -> PyResult<()> {
        let dynamic_values: PyResult<Vec<_>> = values.iter().map(|v| py_to_dynamic(&v)).collect();
        self.descriptor
            .add_value_range(name, ValueRangeDef::Explicit(dynamic_values?));
        Ok(())
    }

    /// Define an integer range [start, end).
    fn int_range(&mut self, name: &str, start: i64, end: i64) {
        self.descriptor
            .add_value_range(name, ValueRangeDef::int_range(start, end));
    }

    /// Add entities to a class.
    fn add_entities(&mut self, class_name: &str, data: &Bound<'_, PyList>) -> PyResult<()> {
        let class_idx = self
            .descriptor
            .entity_class_index(class_name)
            .ok_or_else(|| {
                PyValueError::new_err(format!("Unknown entity class: {}", class_name))
            })?;

        let class_def = &self.descriptor.entity_classes[class_idx];

        for item in data.iter() {
            let dict = item.downcast::<PyDict>()?;
            let mut fields = Vec::new();

            for field_def in &class_def.fields {
                let value = if let Some(py_val) = dict.get_item(&*field_def.name)? {
                    py_to_dynamic(&py_val)?
                } else {
                    DynamicValue::None
                };
                fields.push(value);
            }

            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.entities[class_idx].push(DynamicEntity::new(id, fields));
        }

        Ok(())
    }

    /// Start defining a constraint.
    fn constraint(&mut self, name: &str, weight: &str) -> PyResult<ConstraintBuilder> {
        let weight = parse_weight(weight)?;
        let builder = ConstraintBuilder {
            name: name.to_string(),
            weight,
            class_idx: None,
            operations: Vec::new(),
        };
        Ok(builder)
    }

    /// Add a completed constraint to the solver.
    fn add_constraint(&mut self, builder: ConstraintBuilder) {
        self.constraints.push(builder);
    }

    /// Solve the problem.
    #[pyo3(signature = (time_limit_seconds = 30))]
    fn solve(&self, time_limit_seconds: u64) -> PyResult<PySolveResult> {
        // Build the solution
        let mut solution = DynamicSolution::new(self.descriptor.clone());
        for (class_idx, entities) in self.entities.iter().enumerate() {
            for entity in entities {
                solution.add_entity(class_idx, entity.clone());
            }
        }

        // Build constraints
        let mut constraint_set = DynamicConstraintSet::new();
        for builder in &self.constraints {
            let constraint = build_constraint(builder, &self.descriptor)?;
            constraint_set.add(constraint);
        }

        // Solve
        let config = SolveConfig::with_time_limit(Duration::from_secs(time_limit_seconds));
        let result = solverforge_dynamic::solve(solution, constraint_set, config);

        Ok(PySolveResult::new(
            format!("{}hard/{}soft", result.score.hard(), result.score.soft()),
            result.score.hard(),
            result.score.soft(),
            result.is_feasible(),
            result.duration.as_millis() as u64,
            result.steps,
            result.moves_evaluated,
            result.solution,
        ))
    }
}
