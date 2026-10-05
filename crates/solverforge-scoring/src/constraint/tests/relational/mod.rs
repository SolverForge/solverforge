/* Relational join test subtree: independent-key join verification.

Fixtures, an independent nested-loop oracle, and per-phase regression
suites live here. The oracle never calls production indexes or traversal;
production results must agree with it on scores, row multisets, and counts.
*/

mod fixtures;
mod identity;
mod independent_keys;
mod oracle;
mod primitives;
mod terminal;
mod updates;
