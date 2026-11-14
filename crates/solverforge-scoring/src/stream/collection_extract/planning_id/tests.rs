// The planning-id rendering the macros generate: `Display` first, then
// `Debug`, then none, chosen per concrete id type by method resolution.

use super::{
    RenderPlanningId, RenderPlanningIdDebug, RenderPlanningIdDisplay, RenderPlanningIdNone,
};

/// The expression `#[planning_entity]` and `#[problem_fact]` expand to.
macro_rules! render {
    ($id:expr) => {
        (&&&RenderPlanningId(&$id)).render_planning_id()
    };
}

// The field is read only through `Debug`.
#[derive(Debug)]
#[allow(dead_code)]
struct DebugOnly(u32);

struct Neither;

struct Both;

impl std::fmt::Display for Both {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("display")
    }
}

impl std::fmt::Debug for Both {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("debug")
    }
}

#[test]
fn display_ids_render_with_display() {
    assert_eq!(render!(7usize), Some("7".to_string()));
    assert_eq!(render!("anna".to_string()), Some("anna".to_string()));
    assert_eq!(render!(Both), Some("display".to_string()));
}

#[test]
fn ids_without_display_render_with_debug() {
    assert_eq!(render!(DebugOnly(3)), Some("DebugOnly(3)".to_string()));
    assert_eq!(render!(Some(4u8)), Some("Some(4)".to_string()));
}

#[test]
fn ids_without_display_or_debug_render_as_none() {
    assert_eq!(render!(Neither), None);
}
