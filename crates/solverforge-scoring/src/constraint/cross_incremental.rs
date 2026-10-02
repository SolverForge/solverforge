/* Shared cross-entity join engine module.

The engine owns retained state once per arity; per-arity constraint types
(bi, tri) keep their arity-specific filter, weight, and change-localization
plumbing and delegate row/index bookkeeping here.
*/

mod engine;

pub(crate) use engine::CrossJoinEngine;
