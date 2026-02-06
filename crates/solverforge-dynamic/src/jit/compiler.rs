//! Cranelift JIT compiler for Expr trees.
//!
//! Compiles constraint expressions into native function pointers operating on
//! flat `*const i64` entity buffers. Each entity field occupies one i64 slot.
//!
//! # Memory Layout
//!
//! Entity fields are laid out as contiguous i64 slots:
//! ```text
//! offset 0:  field_0 as i64
//! offset 8:  field_1 as i64
//! ...
//! ```
//!
//! `DynamicValue::None` is encoded as `i64::MIN` (see `crate::NONE_SENTINEL`).

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::expr::Expr;
use crate::NONE_SENTINEL;

/// A JIT-compiled function. Owns the Cranelift module (code memory) and holds
/// the raw function pointer. The caller transmutes to the appropriate signature
/// based on the arity used at compile time.
pub struct JitFn {
    _module: JITModule,
    ptr: *const u8,
    arity: u8,
}

// SAFETY: JITModule owns the code memory. ptr is valid for the module's lifetime.
unsafe impl Send for JitFn {}
unsafe impl Sync for JitFn {}

impl JitFn {
    /// Call as `fn(*const i64) -> i64` (uni key/weight).
    #[inline]
    pub fn call_1(&self, a: *const i64) -> i64 {
        debug_assert_eq!(self.arity, 1);
        let f: unsafe fn(*const i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { f(a) }
    }

    /// Call as `fn(*const i64, *const i64) -> i64` (bi key/weight/filter).
    /// For filters, interpret nonzero as true.
    #[inline]
    pub fn call_2(&self, a: *const i64, b: *const i64) -> i64 {
        debug_assert_eq!(self.arity, 2);
        let f: unsafe fn(*const i64, *const i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { f(a, b) }
    }

    pub fn arity(&self) -> u8 {
        self.arity
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JitError {
    #[error("Cranelift codegen: {0}")]
    Codegen(String),
    #[error("Cranelift module: {0}")]
    Module(#[from] cranelift_module::ModuleError),
    #[error("Unsupported expr for JIT: {0}")]
    Unsupported(String),
    #[error("Settings: {0}")]
    Settings(#[from] settings::SetError),
}

/// Compile an expression with 1 entity pointer param, returning i64.
pub fn compile_1(expr: &Expr) -> Result<JitFn, JitError> {
    let (module, ptr) = compile_function(expr, 1)?;
    Ok(JitFn {
        _module: module,
        ptr,
        arity: 1,
    })
}

/// Compile an expression with 2 entity pointer params, returning i64.
pub fn compile_2(expr: &Expr) -> Result<JitFn, JitError> {
    let (module, ptr) = compile_function(expr, 2)?;
    Ok(JitFn {
        _module: module,
        ptr,
        arity: 2,
    })
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

fn make_jit_module() -> Result<JITModule, JitError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false")?;
    flag_builder.set("is_pic", "false")?;
    let isa_builder = cranelift_native::builder().map_err(|e| JitError::Codegen(e.to_string()))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| JitError::Codegen(e.to_string()))?;
    let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    Ok(JITModule::new(builder))
}

fn compile_function(expr: &Expr, arity: u8) -> Result<(JITModule, *const u8), JitError> {
    let mut module = make_jit_module()?;
    let ptr_type = module.target_config().pointer_type();

    let mut sig = Signature::new(CallConv::SystemV);
    for _ in 0..arity {
        sig.params.push(AbiParam::new(ptr_type));
    }
    sig.returns.push(AbiParam::new(I64));

    let func_id = module.declare_function("jit_expr", Linkage::Local, &sig)?;
    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);
    let mut func_ctx = FunctionBuilderContext::new();

    {
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let params: Vec<_> = builder.block_params(entry).to_vec();
        let result = emit_expr(&mut builder, expr, &params, arity)?;
        builder.ins().return_(&[result]);
        builder.finalize();
    }

    let mut ctx = Context::for_function(func);
    module
        .define_function(func_id, &mut ctx)
        .map_err(|e| JitError::Codegen(e.to_string()))?;
    module.clear_context(&mut ctx);
    module
        .finalize_definitions()
        .map_err(|e| JitError::Codegen(e.to_string()))?;

    let ptr = module.get_finalized_function(func_id);
    Ok((module, ptr))
}

/// Emit Cranelift IR for an expression. All values are i64 internally.
/// Booleans are 0 or 1 as i64.
fn emit_expr(
    builder: &mut FunctionBuilder,
    expr: &Expr,
    params: &[cranelift_codegen::ir::Value],
    arity: u8,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    use Expr::*;
    match expr {
        Literal(crate::solution::DynamicValue::I64(n)) => Ok(builder.ins().iconst(I64, *n)),
        Literal(crate::solution::DynamicValue::Bool(b)) => Ok(builder.ins().iconst(I64, *b as i64)),
        Literal(crate::solution::DynamicValue::None) => {
            Ok(builder.ins().iconst(I64, NONE_SENTINEL))
        }

        Field {
            param_idx,
            field_idx,
        } => {
            if *param_idx as u8 >= arity {
                return Err(JitError::Unsupported(format!(
                    "param_idx={} but arity={}",
                    param_idx, arity
                )));
            }
            let ptr = params[*param_idx];
            let offset = (*field_idx as i32) * 8;
            Ok(builder
                .ins()
                .load(I64, cranelift_codegen::ir::MemFlags::trusted(), ptr, offset))
        }

        Eq(l, r) => icmp_op(builder, IntCC::Equal, l, r, params, arity),
        Ne(l, r) => icmp_op(builder, IntCC::NotEqual, l, r, params, arity),
        Lt(l, r) => icmp_op(builder, IntCC::SignedLessThan, l, r, params, arity),
        Le(l, r) => icmp_op(builder, IntCC::SignedLessThanOrEqual, l, r, params, arity),
        Gt(l, r) => icmp_op(builder, IntCC::SignedGreaterThan, l, r, params, arity),
        Ge(l, r) => icmp_op(
            builder,
            IntCC::SignedGreaterThanOrEqual,
            l,
            r,
            params,
            arity,
        ),

        And(l, r) => bin_op(builder, |b, a, c| b.ins().band(a, c), l, r, params, arity),
        Or(l, r) => bin_op(builder, |b, a, c| b.ins().bor(a, c), l, r, params, arity),
        Not(inner) => {
            let v = emit_expr(builder, inner, params, arity)?;
            let one = builder.ins().iconst(I64, 1);
            Ok(builder.ins().bxor(v, one))
        }

        Add(l, r) => bin_op(builder, |b, a, c| b.ins().iadd(a, c), l, r, params, arity),
        Sub(l, r) => bin_op(builder, |b, a, c| b.ins().isub(a, c), l, r, params, arity),
        Mul(l, r) => bin_op(builder, |b, a, c| b.ins().imul(a, c), l, r, params, arity),
        Neg(inner) => {
            let v = emit_expr(builder, inner, params, arity)?;
            Ok(builder.ins().ineg(v))
        }
        Abs(inner) => {
            let v = emit_expr(builder, inner, params, arity)?;
            let neg = builder.ins().ineg(v);
            let zero = builder.ins().iconst(I64, 0);
            let is_neg = builder.ins().icmp(IntCC::SignedLessThan, v, zero);
            Ok(builder.ins().select(is_neg, neg, v))
        }

        Min(l, r) => select_op(builder, IntCC::SignedLessThan, l, r, params, arity),
        Max(l, r) => select_op(builder, IntCC::SignedGreaterThan, l, r, params, arity),

        IsNone(inner) => {
            let v = emit_expr(builder, inner, params, arity)?;
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::Equal, v, sentinel);
            Ok(builder.ins().sextend(I64, cmp))
        }
        IsNotNone(inner) => {
            let v = emit_expr(builder, inner, params, arity)?;
            let sentinel = builder.ins().iconst(I64, NONE_SENTINEL);
            let cmp = builder.ins().icmp(IntCC::NotEqual, v, sentinel);
            Ok(builder.ins().sextend(I64, cmp))
        }

        Overlaps {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, params, arity)?;
            let e1 = emit_expr(builder, end1, params, arity)?;
            let s2 = emit_expr(builder, start2, params, arity)?;
            let e2 = emit_expr(builder, end2, params, arity)?;
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            let cmp = builder
                .ins()
                .icmp(IntCC::SignedLessThan, max_start, min_end);
            Ok(builder.ins().sextend(I64, cmp))
        }

        OverlapMinutes {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = emit_expr(builder, start1, params, arity)?;
            let e1 = emit_expr(builder, end1, params, arity)?;
            let s2 = emit_expr(builder, start2, params, arity)?;
            let e2 = emit_expr(builder, end2, params, arity)?;
            let cmp_s = builder.ins().icmp(IntCC::SignedGreaterThan, s1, s2);
            let max_start = builder.ins().select(cmp_s, s1, s2);
            let cmp_e = builder.ins().icmp(IntCC::SignedLessThan, e1, e2);
            let min_end = builder.ins().select(cmp_e, e1, e2);
            let diff = builder.ins().isub(min_end, max_start);
            let zero = builder.ins().iconst(I64, 0);
            let is_pos = builder.ins().icmp(IntCC::SignedGreaterThan, diff, zero);
            let clamped = builder.ins().select(is_pos, diff, zero);
            let ms_per_min = builder.ins().iconst(I64, 60_000);
            Ok(builder.ins().sdiv(clamped, ms_per_min))
        }

        If {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = emit_expr(builder, cond, params, arity)?;
            let t = emit_expr(builder, then_expr, params, arity)?;
            let e = emit_expr(builder, else_expr, params, arity)?;
            let zero = builder.ins().iconst(I64, 0);
            let is_true = builder.ins().icmp(IntCC::NotEqual, c, zero);
            Ok(builder.ins().select(is_true, t, e))
        }

        Literal(_) => Err(JitError::Unsupported("non-i64/bool/None literal".into())),
        Param(_) => Err(JitError::Unsupported("Param".into())),
        Div(..) => Err(JitError::Unsupported("Div".into())),
        Mod(..) => Err(JitError::Unsupported("Mod".into())),
        Contains(..) => Err(JitError::Unsupported("Contains".into())),
        RefField { .. } => Err(JitError::Unsupported("RefField".into())),
        SetContains { .. } => Err(JitError::Unsupported("SetContains".into())),
        DateOf(_) => Err(JitError::Unsupported("DateOf".into())),
        OverlapsDate { .. } => Err(JitError::Unsupported("OverlapsDate".into())),
        OverlapDateMinutes { .. } => Err(JitError::Unsupported("OverlapDateMinutes".into())),
        FlattenedValue => Err(JitError::Unsupported("FlattenedValue".into())),
    }
}

// Helpers to reduce match-arm boilerplate

fn icmp_op(
    builder: &mut FunctionBuilder,
    cc: IntCC,
    left: &Expr,
    right: &Expr,
    params: &[cranelift_codegen::ir::Value],
    arity: u8,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    let l = emit_expr(builder, left, params, arity)?;
    let r = emit_expr(builder, right, params, arity)?;
    let cmp = builder.ins().icmp(cc, l, r);
    Ok(builder.ins().sextend(I64, cmp))
}

fn bin_op(
    builder: &mut FunctionBuilder,
    op: impl FnOnce(
        &mut FunctionBuilder,
        cranelift_codegen::ir::Value,
        cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value,
    left: &Expr,
    right: &Expr,
    params: &[cranelift_codegen::ir::Value],
    arity: u8,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    let l = emit_expr(builder, left, params, arity)?;
    let r = emit_expr(builder, right, params, arity)?;
    Ok(op(builder, l, r))
}

fn select_op(
    builder: &mut FunctionBuilder,
    cc: IntCC,
    left: &Expr,
    right: &Expr,
    params: &[cranelift_codegen::ir::Value],
    arity: u8,
) -> Result<cranelift_codegen::ir::Value, JitError> {
    let l = emit_expr(builder, left, params, arity)?;
    let r = emit_expr(builder, right, params, arity)?;
    let cmp = builder.ins().icmp(cc, l, r);
    Ok(builder.ins().select(cmp, l, r))
}
