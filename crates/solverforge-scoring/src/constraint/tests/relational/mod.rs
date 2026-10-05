/* Relational join test subtree: independent-key join verification.

Fixtures, an independent nested-loop oracle, and per-phase regression
suites live here. The oracle never calls production indexes or traversal;
production results must agree with it on scores, row multisets, and counts.
*/

mod chained;
mod compiled_conditions;
mod complement_producer;
mod condition_mutations;
mod conditions;
mod evaluation;
mod existence_producer;
mod filtered_operators;
mod fixtures;
mod flatten_producer;
mod fluent_chain;
mod group_producer;
mod handle_map;
mod identity;
mod independent_keys;
mod indexes;
mod merged_operators;
mod operator_terminal;
mod operator_tree;
mod oracle;
mod owned_projection;
mod primitives;
mod sharing;
mod terminal;
mod triple_terminal;
mod updates;
