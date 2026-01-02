//! Reusable domain model fixtures for testing.
//!
//! These fixtures mirror real-world scheduling domains and provide
//! consistent test data across the workspace.

use solverforge_core::domain::{
    DomainClass, DomainModel, DomainModelBuilder, FieldDescriptor, FieldType, PlanningAnnotation,
    PrimitiveType, ScoreType,
};

// =============================================================================
// TIMETABLING DOMAIN
// =============================================================================

/// A Room that can be assigned to lessons.
pub fn room_class() -> DomainClass {
    DomainClass::new("Room")
        .with_field(FieldDescriptor::new(
            "id",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "name",
            FieldType::Primitive(PrimitiveType::String),
        ))
}

/// A Timeslot representing when a lesson can occur.
pub fn timeslot_class() -> DomainClass {
    DomainClass::new("Timeslot")
        .with_field(FieldDescriptor::new(
            "id",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "dayOfWeek",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "startTime",
            FieldType::Primitive(PrimitiveType::String),
        ))
}

/// A Lesson entity with room and timeslot planning variables.
pub fn lesson_class() -> DomainClass {
    DomainClass::new("Lesson")
        .with_annotation(PlanningAnnotation::PlanningEntity)
        .with_field(
            FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                .with_annotation(PlanningAnnotation::PlanningId),
        )
        .with_field(FieldDescriptor::new(
            "subject",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "teacher",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(
            FieldDescriptor::new("room", FieldType::object("Room"))
                .with_annotation(PlanningAnnotation::planning_variable(vec!["rooms".into()])),
        )
        .with_field(
            FieldDescriptor::new("timeslot", FieldType::object("Timeslot")).with_annotation(
                PlanningAnnotation::planning_variable(vec!["timeslots".into()]),
            ),
        )
}

/// A Timetable solution containing lessons, rooms, and timeslots.
pub fn timetable_solution() -> DomainClass {
    DomainClass::new("Timetable")
        .with_annotation(PlanningAnnotation::PlanningSolution)
        .with_field(
            FieldDescriptor::new("lessons", FieldType::list(FieldType::object("Lesson")))
                .with_annotation(PlanningAnnotation::PlanningEntityCollectionProperty),
        )
        .with_field(
            FieldDescriptor::new("rooms", FieldType::list(FieldType::object("Room")))
                .with_annotation(PlanningAnnotation::value_range_provider_with_id("rooms")),
        )
        .with_field(
            FieldDescriptor::new("timeslots", FieldType::list(FieldType::object("Timeslot")))
                .with_annotation(PlanningAnnotation::value_range_provider_with_id(
                    "timeslots",
                )),
        )
        .with_field(
            FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                .with_annotation(PlanningAnnotation::planning_score()),
        )
}

/// Complete timetabling domain model ready for testing.
pub fn timetabling_model() -> DomainModel {
    DomainModelBuilder::new()
        .add_class(room_class())
        .add_class(timeslot_class())
        .add_class(lesson_class())
        .add_class(timetable_solution())
        .build()
}

// =============================================================================
// EMPLOYEE SCHEDULING DOMAIN
// =============================================================================

/// An Employee that can be assigned to shifts.
pub fn employee_class() -> DomainClass {
    DomainClass::new("Employee")
        .with_field(FieldDescriptor::new(
            "id",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "name",
            FieldType::Primitive(PrimitiveType::String),
        ))
}

/// A Shift entity with employee planning variable.
pub fn shift_class() -> DomainClass {
    DomainClass::new("Shift")
        .with_annotation(PlanningAnnotation::PlanningEntity)
        .with_field(
            FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                .with_annotation(PlanningAnnotation::PlanningId),
        )
        .with_field(FieldDescriptor::new(
            "start",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(FieldDescriptor::new(
            "end",
            FieldType::Primitive(PrimitiveType::String),
        ))
        .with_field(
            FieldDescriptor::new("employee", FieldType::object("Employee")).with_annotation(
                PlanningAnnotation::planning_variable(vec!["employees".into()]),
            ),
        )
}

/// An EmployeeSchedule solution containing shifts and employees.
pub fn employee_schedule_solution() -> DomainClass {
    DomainClass::new("EmployeeSchedule")
        .with_annotation(PlanningAnnotation::PlanningSolution)
        .with_field(
            FieldDescriptor::new("shifts", FieldType::list(FieldType::object("Shift")))
                .with_annotation(PlanningAnnotation::PlanningEntityCollectionProperty),
        )
        .with_field(
            FieldDescriptor::new("employees", FieldType::list(FieldType::object("Employee")))
                .with_annotation(PlanningAnnotation::value_range_provider_with_id(
                    "employees",
                )),
        )
        .with_field(
            FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                .with_annotation(PlanningAnnotation::planning_score()),
        )
}

/// Complete employee scheduling domain model ready for testing.
pub fn employee_scheduling_model() -> DomainModel {
    DomainModelBuilder::new()
        .add_class(employee_class())
        .add_class(shift_class())
        .add_class(employee_schedule_solution())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timetabling_model_validates() {
        let model = timetabling_model();
        assert!(model.validate().is_ok());
    }

    #[test]
    fn employee_scheduling_model_validates() {
        let model = employee_scheduling_model();
        assert!(model.validate().is_ok());
    }
}
