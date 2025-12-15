//! Core traits for domain model types in SolverForge.
//!
//! These traits define the interface for planning entities and solutions
//! that can be solved by the constraint solver. They are typically implemented
//! via derive macros from the `solverforge-derive` crate.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_derive::{PlanningEntity, PlanningSolution};
//! use solverforge_core::{PlanningEntity, PlanningSolution, HardSoftScore};
//!
//! #[derive(PlanningEntity)]
//! struct Lesson {
//!     #[planning_id]
//!     id: String,
//!     #[planning_variable(value_range_provider = "rooms")]
//!     room: Option<Room>,
//! }
//!
//! #[derive(PlanningSolution)]
//! #[constraint_provider = "define_constraints"]
//! struct Timetable {
//!     #[problem_fact_collection]
//!     #[value_range_provider(id = "rooms")]
//!     rooms: Vec<Room>,
//!     #[planning_entity_collection]
//!     lessons: Vec<Lesson>,
//!     #[planning_score]
//!     score: Option<HardSoftScore>,
//! }
//! ```

use crate::constraints::ConstraintSet;
use crate::domain::{DomainClass, DomainModel};
use crate::score::Score;
use crate::value::Value;
use crate::SolverForgeResult;

/// Marker trait for types that can be used as planning entities.
///
/// A planning entity is an object that can be changed during solving.
/// It contains one or more planning variables that the solver assigns
/// values to from their respective value ranges.
///
/// # Requirements
///
/// - Must have a unique planning ID field
/// - Must have at least one planning variable
/// - Must be serializable to/from `Value`
///
/// # Derive Macro
///
/// This trait is typically implemented via `#[derive(PlanningEntity)]`:
///
/// ```ignore
/// #[derive(PlanningEntity)]
/// struct Lesson {
///     #[planning_id]
///     id: String,
///     subject: String,
///     #[planning_variable(value_range_provider = "timeslots")]
///     timeslot: Option<Timeslot>,
/// }
/// ```
pub trait PlanningEntity: Send + Sync + Clone {
    /// Returns the domain class descriptor for this entity type.
    ///
    /// The domain class contains metadata about the entity's fields,
    /// annotations, and planning variables.
    fn domain_class() -> DomainClass;

    /// Returns the planning ID for this instance.
    ///
    /// The planning ID uniquely identifies this entity instance and is
    /// used for tracking during solving and for entity matching.
    fn planning_id(&self) -> Value;

    /// Serializes this entity to a language-agnostic `Value`.
    ///
    /// The resulting `Value::Object` contains all fields of the entity.
    fn to_value(&self) -> Value;

    /// Deserializes an entity from a `Value`.
    ///
    /// Returns an error if the value cannot be converted to this entity type.
    fn from_value(value: &Value) -> SolverForgeResult<Self>
    where
        Self: Sized;
}

/// Marker trait for types that can be used as planning solutions.
///
/// A planning solution contains:
/// - Problem facts: immutable data that constraints can reference
/// - Planning entities: objects that can be changed during solving
/// - A score: represents the quality of the solution
///
/// # Requirements
///
/// - Must have at least one planning entity collection
/// - Must have a score field
/// - Must provide a constraint provider function
///
/// # Derive Macro
///
/// This trait is typically implemented via `#[derive(PlanningSolution)]`:
///
/// ```ignore
/// #[derive(PlanningSolution)]
/// #[constraint_provider = "define_constraints"]
/// struct Timetable {
///     #[problem_fact_collection]
///     #[value_range_provider(id = "timeslots")]
///     timeslots: Vec<Timeslot>,
///     #[planning_entity_collection]
///     lessons: Vec<Lesson>,
///     #[planning_score]
///     score: Option<HardSoftScore>,
/// }
/// ```
pub trait PlanningSolution: Send + Sync + Clone {
    /// The score type used by this solution.
    type Score: Score;

    /// Returns the complete domain model for this solution.
    ///
    /// The domain model includes all entity classes, problem fact classes,
    /// and the solution class itself with all their fields and annotations.
    fn domain_model() -> DomainModel;

    /// Returns the constraint set for this solution.
    ///
    /// The constraint set contains all constraints that will be evaluated
    /// during solving to calculate the solution's score.
    fn constraints() -> ConstraintSet;

    /// Returns the current score of this solution, if set.
    fn score(&self) -> Option<Self::Score>;

    /// Sets the score of this solution.
    fn set_score(&mut self, score: Self::Score);

    /// Serializes this solution to JSON.
    fn to_json(&self) -> SolverForgeResult<String>;

    /// Deserializes a solution from JSON.
    fn from_json(json: &str) -> SolverForgeResult<Self>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Constraint;
    use crate::domain::{FieldDescriptor, FieldType, PlanningAnnotation, PrimitiveType, ScoreType};
    use crate::{HardSoftScore, SolverForgeError};
    use std::collections::HashMap;

    // Test entity: Room (problem fact, no planning variables)
    #[derive(Clone, Debug, PartialEq)]
    struct Room {
        id: String,
        name: String,
    }

    // Test entity: Lesson (planning entity with planning variable)
    #[derive(Clone, Debug, PartialEq)]
    struct Lesson {
        id: String,
        subject: String,
        room: Option<String>, // References Room.id
    }

    impl PlanningEntity for Lesson {
        fn domain_class() -> DomainClass {
            DomainClass::new("Lesson")
                .with_annotation(PlanningAnnotation::PlanningEntity)
                .with_field(
                    FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                        .with_planning_annotation(PlanningAnnotation::PlanningId),
                )
                .with_field(FieldDescriptor::new(
                    "subject",
                    FieldType::Primitive(PrimitiveType::String),
                ))
                .with_field(
                    FieldDescriptor::new("room", FieldType::object("Room"))
                        .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                            "rooms".to_string(),
                        ])),
                )
        }

        fn planning_id(&self) -> Value {
            Value::String(self.id.clone())
        }

        fn to_value(&self) -> Value {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::String(self.id.clone()));
            map.insert("subject".to_string(), Value::String(self.subject.clone()));
            map.insert(
                "room".to_string(),
                self.room.clone().map(Value::String).unwrap_or(Value::Null),
            );
            Value::Object(map)
        }

        fn from_value(value: &Value) -> SolverForgeResult<Self> {
            match value {
                Value::Object(map) => {
                    let id = map
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| SolverForgeError::Serialization("Missing id".to_string()))?
                        .to_string();
                    let subject = map
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SolverForgeError::Serialization("Missing subject".to_string())
                        })?
                        .to_string();
                    let room = map.get("room").and_then(|v| v.as_str()).map(String::from);
                    Ok(Lesson { id, subject, room })
                }
                _ => Err(SolverForgeError::Serialization(
                    "Expected object".to_string(),
                )),
            }
        }
    }

    // Test solution: Timetable
    #[derive(Clone, Debug)]
    struct Timetable {
        rooms: Vec<Room>,
        lessons: Vec<Lesson>,
        score: Option<HardSoftScore>,
    }

    impl PlanningSolution for Timetable {
        type Score = HardSoftScore;

        fn domain_model() -> DomainModel {
            DomainModel::builder()
                .add_class(Lesson::domain_class())
                .add_class(DomainClass::new("Room").with_field(FieldDescriptor::new(
                    "id",
                    FieldType::Primitive(PrimitiveType::String),
                )))
                .add_class(
                    DomainClass::new("Timetable")
                        .with_annotation(PlanningAnnotation::PlanningSolution)
                        .with_field(
                            FieldDescriptor::new(
                                "rooms",
                                FieldType::list(FieldType::object("Room")),
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::ProblemFactCollectionProperty,
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::value_range_provider("rooms"),
                            ),
                        )
                        .with_field(
                            FieldDescriptor::new(
                                "lessons",
                                FieldType::list(FieldType::object("Lesson")),
                            )
                            .with_planning_annotation(
                                PlanningAnnotation::PlanningEntityCollectionProperty,
                            ),
                        )
                        .with_field(
                            FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                                .with_planning_annotation(PlanningAnnotation::planning_score()),
                        ),
                )
                .build()
        }

        fn constraints() -> ConstraintSet {
            // Simple constraint set for testing
            ConstraintSet::new().with_constraint(Constraint::new("Test constraint"))
        }

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Self::Score) {
            self.score = Some(score);
        }

        fn to_json(&self) -> SolverForgeResult<String> {
            // Simplified JSON serialization for testing
            let mut map = HashMap::new();

            let rooms: Vec<Value> = self
                .rooms
                .iter()
                .map(|r| {
                    let mut m = HashMap::new();
                    m.insert("id".to_string(), Value::String(r.id.clone()));
                    m.insert("name".to_string(), Value::String(r.name.clone()));
                    Value::Object(m)
                })
                .collect();
            map.insert("rooms".to_string(), Value::Array(rooms));

            let lessons: Vec<Value> = self.lessons.iter().map(|l| l.to_value()).collect();
            map.insert("lessons".to_string(), Value::Array(lessons));

            if let Some(score) = &self.score {
                map.insert("score".to_string(), Value::String(format!("{}", score)));
            }

            serde_json::to_string(&Value::Object(map))
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))
        }

        fn from_json(json: &str) -> SolverForgeResult<Self> {
            let value: Value = serde_json::from_str(json)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

            match value {
                Value::Object(map) => {
                    let rooms = match map.get("rooms") {
                        Some(Value::Array(arr)) => arr
                            .iter()
                            .map(|v| match v {
                                Value::Object(m) => {
                                    let id = m
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = m
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    Room { id, name }
                                }
                                _ => Room {
                                    id: String::new(),
                                    name: String::new(),
                                },
                            })
                            .collect(),
                        _ => Vec::new(),
                    };

                    let lessons = match map.get("lessons") {
                        Some(Value::Array(arr)) => arr
                            .iter()
                            .filter_map(|v| Lesson::from_value(v).ok())
                            .collect(),
                        _ => Vec::new(),
                    };

                    Ok(Timetable {
                        rooms,
                        lessons,
                        score: None,
                    })
                }
                _ => Err(SolverForgeError::Serialization(
                    "Expected object".to_string(),
                )),
            }
        }
    }

    #[test]
    fn test_planning_entity_domain_class() {
        let class = Lesson::domain_class();
        assert_eq!(class.name, "Lesson");
        assert!(class.is_planning_entity());
        assert!(class.get_planning_id_field().is_some());
        assert_eq!(class.get_planning_variables().count(), 1);
    }

    #[test]
    fn test_planning_entity_planning_id() {
        let lesson = Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            room: Some("R1".to_string()),
        };
        assert_eq!(lesson.planning_id(), Value::String("L1".to_string()));
    }

    #[test]
    fn test_planning_entity_to_value() {
        let lesson = Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            room: Some("R1".to_string()),
        };
        let value = lesson.to_value();

        match value {
            Value::Object(map) => {
                assert_eq!(map.get("id"), Some(&Value::String("L1".to_string())));
                assert_eq!(map.get("subject"), Some(&Value::String("Math".to_string())));
                assert_eq!(map.get("room"), Some(&Value::String("R1".to_string())));
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_planning_entity_from_value() {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::String("L1".to_string()));
        map.insert("subject".to_string(), Value::String("Math".to_string()));
        map.insert("room".to_string(), Value::String("R1".to_string()));

        let lesson = Lesson::from_value(&Value::Object(map)).unwrap();
        assert_eq!(lesson.id, "L1");
        assert_eq!(lesson.subject, "Math");
        assert_eq!(lesson.room, Some("R1".to_string()));
    }

    #[test]
    fn test_planning_entity_roundtrip() {
        let original = Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            room: Some("R1".to_string()),
        };

        let value = original.to_value();
        let restored = Lesson::from_value(&value).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_planning_solution_domain_model() {
        let model = Timetable::domain_model();

        assert!(model.get_solution_class().is_some());
        assert_eq!(model.get_solution_class().unwrap().name, "Timetable");
        assert!(model.get_class("Lesson").is_some());
        assert!(model.get_class("Room").is_some());
    }

    #[test]
    fn test_planning_solution_constraints() {
        let constraints = Timetable::constraints();
        assert!(!constraints.is_empty());
    }

    #[test]
    fn test_planning_solution_score() {
        let mut timetable = Timetable {
            rooms: vec![],
            lessons: vec![],
            score: None,
        };

        assert!(timetable.score().is_none());

        timetable.set_score(HardSoftScore::of(-1, -5));
        assert_eq!(timetable.score(), Some(HardSoftScore::of(-1, -5)));
    }

    #[test]
    fn test_planning_solution_json_roundtrip() {
        let timetable = Timetable {
            rooms: vec![Room {
                id: "R1".to_string(),
                name: "Room 1".to_string(),
            }],
            lessons: vec![Lesson {
                id: "L1".to_string(),
                subject: "Math".to_string(),
                room: Some("R1".to_string()),
            }],
            score: None,
        };

        let json = timetable.to_json().unwrap();
        let restored = Timetable::from_json(&json).unwrap();

        assert_eq!(restored.rooms.len(), 1);
        assert_eq!(restored.lessons.len(), 1);
        assert_eq!(restored.lessons[0].id, "L1");
    }
}
