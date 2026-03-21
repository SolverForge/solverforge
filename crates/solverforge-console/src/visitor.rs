// Event visitor that collects structured fields from tracing events.

use tracing::field::{Field, Visit};

#[derive(Default)]
pub(crate) struct EventVisitor {
    pub(crate) event: Option<String>,
    pub(crate) phase: Option<String>,
    pub(crate) phase_index: Option<u64>,
    pub(crate) steps: Option<u64>,
    pub(crate) speed: Option<u64>,
    pub(crate) score: Option<String>,
    pub(crate) step: Option<u64>,
    pub(crate) entity: Option<u64>,
    pub(crate) accepted: Option<bool>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) entity_count: Option<u64>,
    pub(crate) value_count: Option<u64>,
    pub(crate) constraint_count: Option<u64>,
    pub(crate) time_limit_secs: Option<u64>,
    pub(crate) feasible: Option<bool>,
    pub(crate) moves_speed: Option<u64>,
    pub(crate) calc_speed: Option<u64>,
    pub(crate) acceptance_rate: Option<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        match field.name() {
            "event" => self.event = Some(s.trim_matches('"').to_string()),
            "phase" => self.phase = Some(s.trim_matches('"').to_string()),
            "score" => self.score = Some(s.trim_matches('"').to_string()),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "phase_index" => self.phase_index = Some(value),
            "steps" => self.steps = Some(value),
            "speed" => self.speed = Some(value),
            "step" => self.step = Some(value),
            // TRACE step events emit `move_index`; keep `entity` as a legacy alias.
            "entity" | "move_index" => self.entity = Some(value),
            "duration_ms" => self.duration_ms = Some(value),
            "entity_count" => self.entity_count = Some(value),
            // List solves emit `element_count`; keep `value_count` for legacy/basic solves.
            "value_count" | "element_count" => self.value_count = Some(value),
            "constraint_count" => self.constraint_count = Some(value),
            "time_limit_secs" => self.time_limit_secs = Some(value),
            "moves_speed" => self.moves_speed = Some(value),
            "calc_speed" => self.calc_speed = Some(value),
            _ => {}
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_u64(field, value as u64);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        match field.name() {
            "accepted" => self.accepted = Some(value),
            "feasible" => self.feasible = Some(value),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.event = Some(value.to_string()),
            "phase" => self.phase = Some(value.to_string()),
            "score" => self.score = Some(value.to_string()),
            "acceptance_rate" => self.acceptance_rate = Some(value.to_string()),
            _ => {}
        }
    }
}
