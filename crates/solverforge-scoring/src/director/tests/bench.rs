/* Performance comparison: full recalc vs incremental scoring.

This module demonstrates the performance difference between:
- Full recalculation on every move (O(n) or O(n²) per move)
- Incremental delta scoring (O(affected entities) per move)
*/

#[cfg(test)]
#[path = "benchmarks.rs"]
mod benchmarks;
