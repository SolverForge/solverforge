/* Deterministic fast hashing for engine-internal index keys.

Moved to `solverforge_core::fast_hash` so host-facing crates share the
one definition; this module is a private alias.
*/

pub(crate) use solverforge_core::fast_hash::{FastMap, FastSet};
