use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::wasm::expression::Expression;
use crate::wasm::memory::WasmMemoryType;
use indexmap::IndexMap;
use wasm_encoder::{Function, Instruction, ValType};

use super::LayoutCalculator;

/// Tracks local variable allocation during expression compilation.
/// Ensures each expression gets unique local indices without hardcoding offsets.
pub(super) struct LocalAllocator {
    next_local: u32,
}

impl LocalAllocator {
    pub fn new(base: u32) -> Self {
        Self { next_local: base }
    }

    /// Allocate a single local variable, returning its index.
    pub fn alloc(&mut self) -> u32 {
        let local = self.next_local;
        self.next_local += 1;
        local
    }
}

/// Expression compiler that generates WASM instructions from Expression trees.
pub(super) struct ExpressionCompiler<'a> {
    pub layout_calculator: &'a LayoutCalculator,
    pub host_function_indices: &'a IndexMap<String, u32>,
    pub get_string_constant_offset: &'a dyn Fn(&str) -> u32,
}

impl<'a> ExpressionCompiler<'a> {
    /// Check if an expression evaluates to a string type.
    /// Used to determine when string comparison (hstringEquals) is needed.
    fn is_string_expression(expr: &Expression) -> bool {
        matches!(expr, Expression::StringLiteral { .. })
    }

    /// Compile an expression tree into WASM instructions
    ///
    /// Generates WASM code that evaluates the expression and leaves the result on the stack.
    /// The remap_from/remap_to_local parameters handle parameter substitution for loop variables.
    /// The locals allocator tracks local variable usage to avoid collisions.
    pub fn compile_expression(
        &self,
        func: &mut Function,
        expr: &Expression,
        model: &DomainModel,
        remap_from: u32,
        remap_to_local: u32,
        locals: &mut LocalAllocator,
    ) -> SolverForgeResult<()> {
        match expr {
            // ===== Literals =====
            Expression::IntLiteral { value } => {
                if *value >= i32::MIN as i64 && *value <= i32::MAX as i64 {
                    func.instruction(&Instruction::I32Const(*value as i32));
                } else {
                    func.instruction(&Instruction::I64Const(*value));
                    func.instruction(&Instruction::I32WrapI64);
                }
            }
            Expression::Int64Literal { value } => {
                func.instruction(&Instruction::I64Const(*value));
            }
            Expression::BoolLiteral { value } => {
                func.instruction(&Instruction::I32Const(if *value { 1 } else { 0 }));
            }
            Expression::FloatLiteral { value } => {
                func.instruction(&Instruction::F64Const(*value));
            }
            Expression::StringLiteral { value } => {
                let offset = (self.get_string_constant_offset)(value);
                func.instruction(&Instruction::I32Const(offset as i32));
            }
            Expression::Null => {
                func.instruction(&Instruction::I32Const(0));
            }

            // ===== Parameter Access =====
            Expression::Param { index } if *index == remap_from => {
                func.instruction(&Instruction::LocalGet(remap_to_local));
            }
            Expression::Param { index } => {
                func.instruction(&Instruction::LocalGet(*index));
            }

            // ===== Field Access =====
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
                ..
            } => {
                self.compile_expression(func, object, model, remap_from, remap_to_local, locals)?;

                let class = model.classes.get(class_name).ok_or_else(|| {
                    SolverForgeError::WasmGeneration(format!("Class not found: {}", class_name))
                })?;

                let layout = self
                    .layout_calculator
                    .get_layout(&class.name)
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(format!(
                            "Layout not found for class: {}",
                            class.name
                        ))
                    })?;

                let field_layout = layout.field_offsets.get(field_name).ok_or_else(|| {
                    SolverForgeError::WasmGeneration(format!(
                        "Field not found: {}.{}",
                        class_name, field_name
                    ))
                })?;

                match field_layout.wasm_type {
                    WasmMemoryType::I32 | WasmMemoryType::Pointer => {
                        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    WasmMemoryType::I64 => {
                        func.instruction(&Instruction::I64Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                    WasmMemoryType::F32 => {
                        func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    WasmMemoryType::F64 => {
                        func.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                    WasmMemoryType::ArrayPointer => {
                        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                }
            }

            // ===== Comparisons =====
            Expression::Eq { left, right } => {
                if Self::is_string_expression(left) || Self::is_string_expression(right) {
                    self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                    self.compile_expression(
                        func,
                        right,
                        model,
                        remap_from,
                        remap_to_local,
                        locals,
                    )?;
                    let func_idx =
                        self.host_function_indices
                            .get("hstringEquals")
                            .ok_or_else(|| {
                                SolverForgeError::WasmGeneration(
                                    "hstringEquals host function not registered".to_string(),
                                )
                            })?;
                    func.instruction(&Instruction::Call(*func_idx));
                } else {
                    self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                    self.compile_expression(
                        func,
                        right,
                        model,
                        remap_from,
                        remap_to_local,
                        locals,
                    )?;
                    func.instruction(&Instruction::I32Eq);
                }
            }
            Expression::Ne { left, right } => {
                if Self::is_string_expression(left) || Self::is_string_expression(right) {
                    self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                    self.compile_expression(
                        func,
                        right,
                        model,
                        remap_from,
                        remap_to_local,
                        locals,
                    )?;
                    let func_idx =
                        self.host_function_indices
                            .get("hstringEquals")
                            .ok_or_else(|| {
                                SolverForgeError::WasmGeneration(
                                    "hstringEquals host function not registered".to_string(),
                                )
                            })?;
                    func.instruction(&Instruction::Call(*func_idx));
                    func.instruction(&Instruction::I32Eqz);
                } else {
                    self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                    self.compile_expression(
                        func,
                        right,
                        model,
                        remap_from,
                        remap_to_local,
                        locals,
                    )?;
                    func.instruction(&Instruction::I32Ne);
                }
            }
            Expression::Lt { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32LtS);
            }
            Expression::Le { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32LeS);
            }
            Expression::Gt { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32GtS);
            }
            Expression::Ge { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32GeS);
            }

            // ===== i64 Comparisons =====
            Expression::Eq64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Eq);
            }
            Expression::Ne64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Ne);
            }
            Expression::Lt64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64LtS);
            }
            Expression::Le64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64LeS);
            }
            Expression::Gt64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64GtS);
            }
            Expression::Ge64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64GeS);
            }

            // ===== Logical Operations =====
            Expression::And { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32And);
            }
            Expression::Or { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Or);
            }
            Expression::Not { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Eqz);
            }
            Expression::IsNull { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Eqz);
            }
            Expression::IsNotNull { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Const(0));
                func.instruction(&Instruction::I32Ne);
            }
            Expression::IsNull64 { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Eqz);
            }
            Expression::IsNotNull64 { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::I64Ne);
            }

            // ===== Arithmetic Operations =====
            Expression::Add { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Add);
            }
            Expression::Sub { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Sub);
            }
            Expression::Mul { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32Mul);
            }
            Expression::Div { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32DivS);
            }

            // ===== i64 Arithmetic Operations =====
            Expression::Add64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Add);
            }
            Expression::Sub64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Sub);
            }
            Expression::Mul64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64Mul);
            }
            Expression::Div64 { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64DivS);
            }

            // ===== Float Arithmetic Operations =====
            Expression::FloatAdd { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Add);
            }
            Expression::FloatSub { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Sub);
            }
            Expression::FloatMul { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Mul);
            }
            Expression::FloatDiv { left, right } => {
                self.compile_expression(func, left, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, right, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Div);
            }

            // ===== Math Functions =====
            Expression::Sqrt { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Sqrt);
            }
            Expression::FloatAbs { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Abs);
            }
            Expression::Round { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Nearest);
                func.instruction(&Instruction::I32TruncF64S);
            }
            Expression::Floor { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Floor);
                func.instruction(&Instruction::I32TruncF64S);
            }
            Expression::Ceil { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Ceil);
                func.instruction(&Instruction::I32TruncF64S);
            }
            Expression::Sin { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                let func_idx =
                    self.host_function_indices
                        .get("hsin")
                        .copied()
                        .ok_or_else(|| {
                            SolverForgeError::WasmGeneration(
                                "hsin host function not found".to_string(),
                            )
                        })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Cos { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                let func_idx =
                    self.host_function_indices
                        .get("hcos")
                        .copied()
                        .ok_or_else(|| {
                            SolverForgeError::WasmGeneration(
                                "hcos host function not found".to_string(),
                            )
                        })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Asin { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                let func_idx = self
                    .host_function_indices
                    .get("hasin")
                    .copied()
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(
                            "hasin host function not found".to_string(),
                        )
                    })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Acos { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                let func_idx = self
                    .host_function_indices
                    .get("hacos")
                    .copied()
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(
                            "hacos host function not found".to_string(),
                        )
                    })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Atan { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                let func_idx = self
                    .host_function_indices
                    .get("hatan")
                    .copied()
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(
                            "hatan host function not found".to_string(),
                        )
                    })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Atan2 { y, x } => {
                self.compile_expression(func, y, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, x, model, remap_from, remap_to_local, locals)?;
                let func_idx = self
                    .host_function_indices
                    .get("hatan2")
                    .copied()
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(
                            "hatan2 host function not found".to_string(),
                        )
                    })?;
                func.instruction(&Instruction::Call(func_idx));
            }
            Expression::Radians { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64Const(std::f64::consts::PI / 180.0));
                func.instruction(&Instruction::F64Mul);
            }
            Expression::IntToFloat { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::F64ConvertI32S);
            }
            Expression::FloatToInt { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32TruncF64S);
            }

            // ===== List Operations =====
            Expression::ListContains { list, element } => {
                self.compile_expression(func, list, model, remap_from, remap_to_local, locals)?;
                self.compile_expression(func, element, model, remap_from, remap_to_local, locals)?;

                let func_idx = self
                    .host_function_indices
                    .get("hlistContainsString")
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(
                            "Host function 'hlistContainsString' not found".to_string(),
                        )
                    })?;

                func.instruction(&Instruction::Call(*func_idx));
            }

            Expression::Length { collection } => {
                self.compile_expression(
                    func,
                    collection,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }

            // ===== Host Function Calls =====
            Expression::HostCall {
                function_name,
                args,
            } => {
                for arg in args {
                    self.compile_expression(func, arg, model, remap_from, remap_to_local, locals)?;
                }

                let func_idx = self
                    .host_function_indices
                    .get(function_name)
                    .ok_or_else(|| {
                        SolverForgeError::WasmGeneration(format!(
                            "Host function '{}' not found in registry. Available functions: {:?}",
                            function_name,
                            self.host_function_indices.keys().collect::<Vec<_>>()
                        ))
                    })?;

                func.instruction(&Instruction::Call(*func_idx));
            }

            // ===== Sum Over Collection =====
            Expression::Sum {
                collection,
                item_var_name: _,
                item_param_index,
                item_class_name: _,
                accumulator_expr,
            } => {
                self.compile_expression(
                    func,
                    collection,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                let list_ptr_local = locals.alloc();
                let backing_array_local = locals.alloc();
                let accumulator_local = locals.alloc();
                let counter_local = locals.alloc();
                let element_local = locals.alloc();

                func.instruction(&Instruction::LocalSet(list_ptr_local));

                func.instruction(&Instruction::LocalGet(list_ptr_local));
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 8,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::LocalSet(backing_array_local));

                func.instruction(&Instruction::I32Const(0));
                func.instruction(&Instruction::LocalSet(accumulator_local));

                func.instruction(&Instruction::I32Const(0));
                func.instruction(&Instruction::LocalSet(counter_local));

                func.instruction(&Instruction::Block(wasm_encoder::BlockType::Result(
                    ValType::I32,
                )));

                func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

                func.instruction(&Instruction::LocalGet(counter_local));
                func.instruction(&Instruction::LocalGet(list_ptr_local));
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::I32GeS);

                func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                func.instruction(&Instruction::LocalGet(accumulator_local));
                func.instruction(&Instruction::Br(2));
                func.instruction(&Instruction::End);

                func.instruction(&Instruction::LocalGet(backing_array_local));
                func.instruction(&Instruction::LocalGet(counter_local));
                func.instruction(&Instruction::I32Const(4));
                func.instruction(&Instruction::I32Mul);
                func.instruction(&Instruction::I32Add);
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));

                func.instruction(&Instruction::LocalSet(element_local));

                self.compile_expression(
                    func,
                    accumulator_expr,
                    model,
                    *item_param_index,
                    element_local,
                    locals,
                )?;

                func.instruction(&Instruction::LocalGet(accumulator_local));
                func.instruction(&Instruction::I32Add);
                func.instruction(&Instruction::LocalSet(accumulator_local));

                func.instruction(&Instruction::LocalGet(counter_local));
                func.instruction(&Instruction::I32Const(1));
                func.instruction(&Instruction::I32Add);
                func.instruction(&Instruction::LocalSet(counter_local));

                func.instruction(&Instruction::Br(0));

                func.instruction(&Instruction::End);

                func.instruction(&Instruction::LocalGet(accumulator_local));

                func.instruction(&Instruction::End);
            }

            // ===== Last Element of Collection =====
            Expression::LastElement {
                collection,
                item_class_name: _,
            } => {
                self.compile_expression(
                    func,
                    collection,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                let list_ptr_local = locals.alloc();
                let backing_array_local = locals.alloc();
                let size_local = locals.alloc();

                func.instruction(&Instruction::LocalSet(list_ptr_local));

                func.instruction(&Instruction::LocalGet(list_ptr_local));
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::LocalSet(size_local));

                func.instruction(&Instruction::LocalGet(list_ptr_local));
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 8,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::LocalSet(backing_array_local));

                func.instruction(&Instruction::LocalGet(backing_array_local));
                func.instruction(&Instruction::LocalGet(size_local));
                func.instruction(&Instruction::I32Const(1));
                func.instruction(&Instruction::I32Sub);
                func.instruction(&Instruction::I32Const(4));
                func.instruction(&Instruction::I32Mul);
                func.instruction(&Instruction::I32Add);
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }

            // ===== Conditional =====
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expression(
                    func,
                    condition,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I32,
                )));

                self.compile_expression(
                    func,
                    then_branch,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::Else);

                self.compile_expression(
                    func,
                    else_branch,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::End);
            }

            // ===== Conditional (i64 result) =====
            Expression::IfThenElse64 {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expression(
                    func,
                    condition,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));

                self.compile_expression(
                    func,
                    then_branch,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::Else);

                self.compile_expression(
                    func,
                    else_branch,
                    model,
                    remap_from,
                    remap_to_local,
                    locals,
                )?;

                func.instruction(&Instruction::End);
            }

            // ===== Type Conversions =====
            Expression::I64ToI32 { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I32WrapI64);
            }
            Expression::I32ToI64 { operand } => {
                self.compile_expression(func, operand, model, remap_from, remap_to_local, locals)?;
                func.instruction(&Instruction::I64ExtendI32S);
            }
        }

        Ok(())
    }
}
