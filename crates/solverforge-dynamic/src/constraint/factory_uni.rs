//! Factory function for building unary (single entity) constraints.
//!
//! Zero-fallback policy: if JIT compilation fails, we panic in debug and
//! log a critical error + panic in release. No interpreter fallback.

use std::sync::Arc;

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;

use super::closures_extract::make_extractor;
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::jit;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Enforce zero-fallback: JIT compilation must succeed.
fn require_jit<T>(result: Result<T, jit::JitError>, context: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let msg = format!(
                "CRITICAL: JIT compilation failed for {context}: {e}. \
                 Zero-fallback policy: all constraint expressions must be JIT-compilable."
            );
            eprintln!("{msg}");
            panic!("{msg}");
        }
    }
}

/// Builds a unary constraint (single entity class, no joins).
///
/// All closures are JIT-compiled. If compilation fails, the solver panics.
///
/// # Expression Context
///
/// Both filter and weight expressions are evaluated in a single-entity context:
/// - `Field { param_idx: 0, field_idx }` accesses fields from the entity
/// - Arithmetic, comparisons, and logical operations work as expected
pub fn build_uni_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    filter_expr: Expr,
    weight_expr: Expr,
    _descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    let extractor = make_extractor(class_idx);

    // --- Filter: JIT ---
    let jit_filter = Arc::new(require_jit(jit::compile_1(&filter_expr), "uni filter"));
    let ci_filter = class_idx;
    let filter = Box::new(
        move |solution: &DynamicSolution, entity: &DynamicEntity| -> bool {
            let loc = solution.get_entity_location(entity.id);
            let Some((_, entity_idx)) = loc else {
                return false;
            };
            let ptr = solution.flat_entity_ptr(ci_filter, entity_idx);
            jit_filter.call_1(ptr) != 0
        },
    ) as Box<dyn Fn(&DynamicSolution, &DynamicEntity) -> bool + Send + Sync>;

    // --- Weight: JIT ---
    let jit_weight = Arc::new(require_jit(jit::compile_1(&weight_expr), "uni weight"));
    let hard = is_hard;
    let weight = Box::new(move |entity: &DynamicEntity| -> HardSoftScore {
        // For uni-weight, the entity fields are converted to flat i64 inline.
        // This is called once per matching entity per insert/retract — acceptable cost.
        // We cannot access the solution here (weight signature is Fn(&A) -> Sc),
        // so we build a tiny stack buffer from the entity fields.
        let flat: Vec<i64> = entity.fields().iter().map(|f| f.to_flat_i64()).collect();
        let w = jit_weight.call_1(flat.as_ptr());
        if hard {
            HardSoftScore::of_hard(w)
        } else {
            HardSoftScore::of_soft(w)
        }
    }) as Box<dyn Fn(&DynamicEntity) -> HardSoftScore + Send + Sync>;

    Box::new(IncrementalUniConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        filter,
        weight,
        is_hard,
    ))
}
