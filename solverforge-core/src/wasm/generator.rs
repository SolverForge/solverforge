use crate::domain::DomainModel;
use crate::error::{SolverForgeError, SolverForgeResult};
use crate::wasm::expression::Expression;
use crate::wasm::memory::{LayoutCalculator, WasmMemoryType};
use base64::{engine::general_purpose::STANDARD, Engine};
use indexmap::IndexMap;
use wasm_encoder::{
    CodeSection, ConstExpr, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    GlobalType, Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
};

use crate::wasm::host_functions::{HostFunctionRegistry, WasmType};

pub struct WasmModuleBuilder {
    layout_calculator: LayoutCalculator,
    domain_model: Option<DomainModel>,
    predicates: IndexMap<String, PredicateDefinition>,
    function_types: Vec<FunctionSignature>,
    initial_memory_pages: u32,
    max_memory_pages: Option<u32>,
    host_functions: HostFunctionRegistry,
    host_function_indices: IndexMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FunctionSignature {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

#[derive(Debug, Clone)]
pub enum PredicateBody {
    Comparison(Comparison),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct PredicateDefinition {
    pub name: String,
    pub arity: u32,
    pub body: PredicateBody,
    /// Parameter types for the predicate function. If None, defaults to all i32.
    pub param_types: Option<Vec<ValType>>,
}

impl PredicateDefinition {
    pub fn new(name: impl Into<String>, arity: u32, comparison: Comparison) -> Self {
        Self {
            name: name.into(),
            arity,
            body: PredicateBody::Comparison(comparison),
            param_types: None,
        }
    }

    pub fn from_expression(name: impl Into<String>, arity: u32, expression: Expression) -> Self {
        Self {
            name: name.into(),
            arity,
            body: PredicateBody::Expression(expression),
            param_types: None,
        }
    }

    /// Create a predicate from an expression with explicit parameter types.
    /// Useful for functions that take non-i32 parameters (e.g., f32 for loadBalance unfairness).
    pub fn from_expression_with_types(
        name: impl Into<String>,
        param_types: Vec<ValType>,
        expression: Expression,
    ) -> Self {
        Self {
            name: name.into(),
            arity: param_types.len() as u32,
            body: PredicateBody::Expression(expression),
            param_types: Some(param_types),
        }
    }

    pub fn always_true(name: impl Into<String>, arity: u32) -> Self {
        Self::new(name, arity, Comparison::AlwaysTrue)
    }

    pub fn equal(name: impl Into<String>, left: FieldAccess, right: FieldAccess) -> Self {
        Self::new(name, 2, Comparison::Equal(left, right))
    }
}

#[derive(Debug, Clone)]
pub struct FieldAccess {
    pub param_index: u32,
    pub class_name: String,
    pub field_name: String,
}

impl FieldAccess {
    pub fn new(
        param_index: u32,
        class_name: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Self {
        Self {
            param_index,
            class_name: class_name.into(),
            field_name: field_name.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Comparison {
    Equal(FieldAccess, FieldAccess),
    NotEqual(FieldAccess, FieldAccess),
    LessThan(FieldAccess, FieldAccess),
    LessThanOrEqual(FieldAccess, FieldAccess),
    GreaterThan(FieldAccess, FieldAccess),
    GreaterThanOrEqual(FieldAccess, FieldAccess),
    AlwaysTrue,
    AlwaysFalse,
}

impl Default for WasmModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmModuleBuilder {
    pub fn new() -> Self {
        Self {
            layout_calculator: LayoutCalculator::new(),
            domain_model: None,
            predicates: IndexMap::new(),
            function_types: Vec::new(),
            initial_memory_pages: 16,
            max_memory_pages: Some(256),
            host_functions: HostFunctionRegistry::new(),
            host_function_indices: IndexMap::new(),
        }
    }

    pub fn with_host_functions(mut self, registry: HostFunctionRegistry) -> Self {
        self.host_functions = registry;
        self
    }

    pub fn with_domain_model(mut self, model: DomainModel) -> Self {
        for class in model.classes.values() {
            self.layout_calculator.calculate_layout(class);
        }
        self.domain_model = Some(model);
        self
    }

    pub fn with_initial_memory(mut self, pages: u32) -> Self {
        self.initial_memory_pages = pages;
        self
    }

    pub fn with_max_memory(mut self, pages: Option<u32>) -> Self {
        self.max_memory_pages = pages;
        self
    }

    pub fn add_predicate(mut self, predicate: PredicateDefinition) -> Self {
        self.predicates.insert(predicate.name.clone(), predicate);
        self
    }

    pub fn build(mut self) -> SolverForgeResult<Vec<u8>> {
        let model = self
            .domain_model
            .take()
            .ok_or_else(|| SolverForgeError::WasmGeneration("Domain model not set".to_string()))?;

        let mut module = Module::new();

        let mut type_section = TypeSection::new();
        let mut import_section = wasm_encoder::ImportSection::new();
        let mut function_section = FunctionSection::new();
        let mut code_section = CodeSection::new();
        let mut export_section = ExportSection::new();
        let mut memory_section = MemorySection::new();
        let mut global_section = GlobalSection::new();

        memory_section.memory(MemoryType {
            minimum: self.initial_memory_pages as u64,
            maximum: self.max_memory_pages.map(|p| p as u64),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        // Global for bump allocator pointer (starts after first page for safety)
        global_section.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(1024),
        );

        // Generate host function imports
        let mut func_idx: u32 = 0;
        let func_names: Vec<String> = self
            .host_functions
            .function_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for func_name in func_names {
            if let Some(host_func) = self.host_functions.lookup(&func_name).cloned() {
                // Add function type for the import
                let params: Vec<ValType> = host_func
                    .params
                    .iter()
                    .map(|t| wasm_type_to_val_type(*t))
                    .collect();
                let results: Vec<ValType> = if matches!(host_func.return_type, WasmType::Void) {
                    vec![]
                } else {
                    vec![wasm_type_to_val_type(host_func.return_type)]
                };

                let type_idx = self.add_function_type(&mut type_section, params, results);

                // Add import
                import_section.import(
                    "host",
                    &host_func.name,
                    wasm_encoder::EntityType::Function(type_idx),
                );

                // Track the function index for call instructions
                self.host_function_indices
                    .insert(func_name.clone(), func_idx);
                func_idx += 1;
            }
        }

        // alloc(size: i32) -> i32
        let alloc_type_idx =
            self.add_function_type(&mut type_section, vec![ValType::I32], vec![ValType::I32]);
        function_section.function(alloc_type_idx);
        code_section.function(&self.generate_allocator());
        export_section.export("alloc", ExportKind::Func, func_idx);
        func_idx += 1;

        // dealloc(ptr: i32) - Java expects single argument
        let dealloc_type_idx =
            self.add_function_type(&mut type_section, vec![ValType::I32], vec![]);
        function_section.function(dealloc_type_idx);
        code_section.function(&self.generate_deallocator());
        export_section.export("dealloc", ExportKind::Func, func_idx);
        func_idx += 1;

        // Generate getter/setter functions for each class field
        for class in model.classes.values() {
            let layout = self.layout_calculator.calculate_layout(class);

            for field in &class.fields {
                if let Some(field_layout) = layout.field_offsets.get(&field.name) {
                    // Getter
                    let getter_name = format!("get_{}_{}", class.name, field.name);
                    let result_type = wasm_memory_type_to_val_type(field_layout.wasm_type);

                    let getter_type_idx = self.add_function_type(
                        &mut type_section,
                        vec![ValType::I32],
                        vec![result_type],
                    );
                    function_section.function(getter_type_idx);
                    code_section.function(
                        &self.generate_getter(field_layout.offset, field_layout.wasm_type),
                    );
                    export_section.export(&getter_name, ExportKind::Func, func_idx);
                    func_idx += 1;

                    // Setter
                    let setter_name = format!("set_{}_{}", class.name, field.name);
                    let setter_type_idx = self.add_function_type(
                        &mut type_section,
                        vec![ValType::I32, result_type],
                        vec![],
                    );
                    function_section.function(setter_type_idx);
                    code_section.function(
                        &self.generate_setter(field_layout.offset, field_layout.wasm_type),
                    );
                    export_section.export(&setter_name, ExportKind::Func, func_idx);
                    func_idx += 1;
                }
            }
        }

        // Generate predicate functions
        let predicates: Vec<_> = self
            .predicates
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (name, predicate) in &predicates {
            // Use explicit param_types if specified, otherwise default to all i32
            let params: Vec<ValType> = predicate
                .param_types
                .clone()
                .unwrap_or_else(|| (0..predicate.arity).map(|_| ValType::I32).collect());
            let pred_type_idx =
                self.add_function_type(&mut type_section, params, vec![ValType::I32]);
            function_section.function(pred_type_idx);
            code_section.function(&self.generate_predicate(predicate, &model)?);
            export_section.export(name, ExportKind::Func, func_idx);
            func_idx += 1;
        }

        // Solution mapper wrapper functions
        self.generate_solution_mappers(
            &mut type_section,
            &mut function_section,
            &mut code_section,
            &mut export_section,
            &mut func_idx,
        );

        // List accessor functions
        self.generate_list_accessors(
            &mut type_section,
            &mut function_section,
            &mut code_section,
            &mut export_section,
            &mut func_idx,
        );

        export_section.export("memory", ExportKind::Memory, 0);

        module.section(&type_section);
        if !self.host_function_indices.is_empty() {
            module.section(&import_section);
        }
        module.section(&function_section);
        module.section(&memory_section);
        module.section(&global_section);
        module.section(&export_section);
        module.section(&code_section);

        Ok(module.finish())
    }

    pub fn build_base64(self) -> SolverForgeResult<String> {
        let bytes = self.build()?;
        Ok(STANDARD.encode(&bytes))
    }

    fn add_function_type(
        &mut self,
        type_section: &mut TypeSection,
        params: Vec<ValType>,
        results: Vec<ValType>,
    ) -> u32 {
        let sig = FunctionSignature {
            params: params.clone(),
            results: results.clone(),
        };

        for (idx, existing) in self.function_types.iter().enumerate() {
            if *existing == sig {
                return idx as u32;
            }
        }

        let idx = self.function_types.len() as u32;
        type_section.ty().function(params, results);
        self.function_types.push(sig);
        idx
    }

    fn generate_allocator(&self) -> Function {
        // Locals: 0=size (param), 1=result, 2=new_end, 3=current_memory_bytes
        let mut func = Function::new([(3, ValType::I32)]);

        // result = global[0] (current heap pointer)
        func.instruction(&Instruction::GlobalGet(0));
        func.instruction(&Instruction::LocalSet(1));

        // new_end = result + size
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalSet(2));

        // current_memory_bytes = memory.size * 65536
        func.instruction(&Instruction::MemorySize(0));
        func.instruction(&Instruction::I32Const(65536));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::LocalSet(3));

        // if new_end > current_memory_bytes: try to grow
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::LocalGet(3));
        func.instruction(&Instruction::I32GtU);
        func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // Grow by 16 pages (1MB) at a time
        func.instruction(&Instruction::I32Const(16));
        func.instruction(&Instruction::MemoryGrow(0));
        // memory.grow returns -1 on failure
        func.instruction(&Instruction::I32Const(-1));
        func.instruction(&Instruction::I32Eq);
        func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // OOM - trap
        func.instruction(&Instruction::Unreachable);
        func.instruction(&Instruction::End);

        func.instruction(&Instruction::End);

        // Update heap pointer: global[0] = new_end
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::GlobalSet(0));

        // Return result (original heap pointer before allocation)
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::End);
        func
    }

    fn generate_deallocator(&self) -> Function {
        let mut func = Function::new([]);
        // No-op for bump allocator
        func.instruction(&Instruction::End);
        func
    }

    fn generate_getter(&self, offset: u32, wasm_type: WasmMemoryType) -> Function {
        let mut func = Function::new([]);

        func.instruction(&Instruction::LocalGet(0));

        match wasm_type {
            WasmMemoryType::I32 | WasmMemoryType::Pointer => {
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::I64 => {
                func.instruction(&Instruction::I64Load(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::F32 => {
                func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::F64 => {
                func.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::ArrayPointer => {
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
        }

        func.instruction(&Instruction::End);
        func
    }

    fn generate_setter(&self, offset: u32, wasm_type: WasmMemoryType) -> Function {
        let mut func = Function::new([]);

        func.instruction(&Instruction::LocalGet(0));

        match wasm_type {
            WasmMemoryType::I32 | WasmMemoryType::Pointer => {
                func.instruction(&Instruction::LocalGet(1));
                func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::I64 => {
                func.instruction(&Instruction::LocalGet(1));
                func.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::F32 => {
                func.instruction(&Instruction::LocalGet(1));
                func.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::F64 => {
                func.instruction(&Instruction::LocalGet(1));
                func.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
            }
            WasmMemoryType::ArrayPointer => {
                func.instruction(&Instruction::LocalGet(1));
                func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                    offset: offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
        }

        func.instruction(&Instruction::End);
        func
    }

    fn generate_predicate(
        &self,
        predicate: &PredicateDefinition,
        model: &DomainModel,
    ) -> SolverForgeResult<Function> {
        let mut func = Function::new([]);

        match &predicate.body {
            PredicateBody::Comparison(comparison) => match comparison {
                Comparison::AlwaysTrue => {
                    func.instruction(&Instruction::I32Const(1));
                }
                Comparison::AlwaysFalse => {
                    func.instruction(&Instruction::I32Const(0));
                }
                Comparison::Equal(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32Eq);
                }
                Comparison::NotEqual(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32Ne);
                }
                Comparison::LessThan(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32LtS);
                }
                Comparison::LessThanOrEqual(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32LeS);
                }
                Comparison::GreaterThan(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32GtS);
                }
                Comparison::GreaterThanOrEqual(left, right) => {
                    self.generate_field_load(&mut func, left, model)?;
                    self.generate_field_load(&mut func, right, model)?;
                    func.instruction(&Instruction::I32GeS);
                }
            },
            PredicateBody::Expression(expression) => {
                self.compile_expression(&mut func, expression, model)?;
            }
        }

        func.instruction(&Instruction::End);
        Ok(func)
    }

    fn generate_field_load(
        &self,
        func: &mut Function,
        access: &FieldAccess,
        model: &DomainModel,
    ) -> SolverForgeResult<()> {
        let class = model.classes.get(&access.class_name).ok_or_else(|| {
            SolverForgeError::WasmGeneration(format!("Class not found: {}", access.class_name))
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

        let field_layout = layout
            .field_offsets
            .get(&access.field_name)
            .ok_or_else(|| {
                SolverForgeError::WasmGeneration(format!(
                    "Field not found: {}.{}",
                    access.class_name, access.field_name
                ))
            })?;

        func.instruction(&Instruction::LocalGet(access.param_index));

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
                func.instruction(&Instruction::I32WrapI64);
            }
            WasmMemoryType::F32 => {
                func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                    offset: field_layout.offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::I32TruncF32S);
            }
            WasmMemoryType::F64 => {
                func.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
                    offset: field_layout.offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::I32TruncF64S);
            }
            WasmMemoryType::ArrayPointer => {
                func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                    offset: field_layout.offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
        }

        Ok(())
    }

    /// Compile an expression tree into WASM instructions
    ///
    /// Generates WASM code that evaluates the expression and leaves the result
    /// on the stack.
    fn compile_expression(
        &self,
        func: &mut Function,
        expr: &Expression,
        model: &DomainModel,
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
            Expression::BoolLiteral { value } => {
                func.instruction(&Instruction::I32Const(if *value { 1 } else { 0 }));
            }
            Expression::Null => {
                func.instruction(&Instruction::I32Const(0));
            }

            // ===== Parameter Access =====
            Expression::Param { index } => {
                func.instruction(&Instruction::LocalGet(*index));
            }

            // ===== Field Access =====
            Expression::FieldAccess {
                object,
                class_name,
                field_name,
            } => {
                // Compile the object expression to get the pointer
                self.compile_expression(func, object, model)?;

                // Load the field from memory
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
                        func.instruction(&Instruction::I32WrapI64);
                    }
                    WasmMemoryType::F32 => {
                        func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 2,
                            memory_index: 0,
                        }));
                        func.instruction(&Instruction::I32TruncF32S);
                    }
                    WasmMemoryType::F64 => {
                        func.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
                            offset: field_layout.offset as u64,
                            align: 3,
                            memory_index: 0,
                        }));
                        func.instruction(&Instruction::I32TruncF64S);
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
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Eq);
            }
            Expression::Ne { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Ne);
            }
            Expression::Lt { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32LtS);
            }
            Expression::Le { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32LeS);
            }
            Expression::Gt { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32GtS);
            }
            Expression::Ge { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32GeS);
            }

            // ===== Logical Operations =====
            Expression::And { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32And);
            }
            Expression::Or { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Or);
            }
            Expression::Not { operand } => {
                self.compile_expression(func, operand, model)?;
                func.instruction(&Instruction::I32Eqz); // ! in WASM is i32.eqz
            }
            Expression::IsNull { operand } => {
                self.compile_expression(func, operand, model)?;
                func.instruction(&Instruction::I32Eqz); // null check is ptr == 0
            }
            Expression::IsNotNull { operand } => {
                self.compile_expression(func, operand, model)?;
                func.instruction(&Instruction::I32Const(0));
                func.instruction(&Instruction::I32Ne); // not null is ptr != 0
            }

            // ===== Arithmetic Operations =====
            Expression::Add { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Add);
            }
            Expression::Sub { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Sub);
            }
            Expression::Mul { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32Mul);
            }
            Expression::Div { left, right } => {
                self.compile_expression(func, left, model)?;
                self.compile_expression(func, right, model)?;
                func.instruction(&Instruction::I32DivS);
            }

            // ===== List Operations =====
            Expression::ListContains { list, element } => {
                // For now, use a host function to check list containment
                // TODO: Generate inline loop for better performance
                self.compile_expression(func, list, model)?;
                self.compile_expression(func, element, model)?;

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

            // ===== Host Function Calls =====
            Expression::HostCall {
                function_name,
                args,
            } => {
                // Compile all arguments
                for arg in args {
                    self.compile_expression(func, arg, model)?;
                }

                // Get the function index for the imported function
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

                // Generate call instruction
                func.instruction(&Instruction::Call(*func_idx));
            }

            // ===== Conditional =====
            Expression::IfThenElse {
                condition,
                then_branch,
                else_branch,
            } => {
                // Compile condition
                self.compile_expression(func, condition, model)?;

                // WASM if-else structure
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I32,
                )));

                // Compile then branch
                self.compile_expression(func, then_branch, model)?;

                func.instruction(&Instruction::Else);

                // Compile else branch
                self.compile_expression(func, else_branch, model)?;

                func.instruction(&Instruction::End);
            }
        }

        Ok(())
    }

    fn generate_solution_mappers(
        &mut self,
        type_section: &mut TypeSection,
        function_section: &mut FunctionSection,
        code_section: &mut CodeSection,
        export_section: &mut ExportSection,
        func_idx: &mut u32,
    ) {
        // parseSchedule(length: i32, json: i32) -> i32
        // Wrapper that calls hparseSchedule host function
        if let Some(&host_idx) = self.host_function_indices.get("hparseSchedule") {
            let parse_type = self.add_function_type(
                type_section,
                vec![ValType::I32, ValType::I32],
                vec![ValType::I32],
            );
            function_section.function(parse_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // length
            func.instruction(&Instruction::LocalGet(1)); // json ptr
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("parseSchedule", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // scheduleString(schedule: i32) -> i32
        // Wrapper that calls hscheduleString host function
        if let Some(&host_idx) = self.host_function_indices.get("hscheduleString") {
            let string_type =
                self.add_function_type(type_section, vec![ValType::I32], vec![ValType::I32]);
            function_section.function(string_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // schedule ptr
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("scheduleString", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }
    }

    fn generate_list_accessors(
        &mut self,
        type_section: &mut TypeSection,
        function_section: &mut FunctionSection,
        code_section: &mut CodeSection,
        export_section: &mut ExportSection,
        func_idx: &mut u32,
    ) {
        // newList() -> i32 (ptr) - Wrapper that calls hnewList host function
        if let Some(&host_idx) = self.host_function_indices.get("hnewList") {
            let create_type = self.add_function_type(type_section, vec![], vec![ValType::I32]);
            function_section.function(create_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("newList", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // getItem(list: i32, index: i32) -> i32 - Wrapper for hgetItem
        if let Some(&host_idx) = self.host_function_indices.get("hgetItem") {
            let get_type = self.add_function_type(
                type_section,
                vec![ValType::I32, ValType::I32],
                vec![ValType::I32],
            );
            function_section.function(get_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::LocalGet(1)); // index
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("getItem", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // setItem(list: i32, index: i32, value: i32) - Wrapper for hsetItem
        if let Some(&host_idx) = self.host_function_indices.get("hsetItem") {
            let set_type = self.add_function_type(
                type_section,
                vec![ValType::I32, ValType::I32, ValType::I32],
                vec![],
            );
            function_section.function(set_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::LocalGet(1)); // index
            func.instruction(&Instruction::LocalGet(2)); // value
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("setItem", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // size(list: i32) -> i32 - Wrapper for hsize
        if let Some(&host_idx) = self.host_function_indices.get("hsize") {
            let size_type =
                self.add_function_type(type_section, vec![ValType::I32], vec![ValType::I32]);
            function_section.function(size_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("size", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // append(list: i32, value: i32) - Wrapper for happend
        if let Some(&host_idx) = self.host_function_indices.get("happend") {
            let append_type =
                self.add_function_type(type_section, vec![ValType::I32, ValType::I32], vec![]);
            function_section.function(append_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::LocalGet(1)); // value
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("append", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // insert(list: i32, index: i32, value: i32) - Wrapper for hinsert
        if let Some(&host_idx) = self.host_function_indices.get("hinsert") {
            let insert_type = self.add_function_type(
                type_section,
                vec![ValType::I32, ValType::I32, ValType::I32],
                vec![],
            );
            function_section.function(insert_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::LocalGet(1)); // index
            func.instruction(&Instruction::LocalGet(2)); // value
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("insert", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }

        // remove(list: i32, index: i32) - Wrapper for hremove
        if let Some(&host_idx) = self.host_function_indices.get("hremove") {
            let remove_type =
                self.add_function_type(type_section, vec![ValType::I32, ValType::I32], vec![]);
            function_section.function(remove_type);

            let mut func = Function::new(vec![]);
            func.instruction(&Instruction::LocalGet(0)); // list
            func.instruction(&Instruction::LocalGet(1)); // index
            func.instruction(&Instruction::Call(host_idx));
            func.instruction(&Instruction::End);

            code_section.function(&func);
            export_section.export("remove", ExportKind::Func, *func_idx);
            *func_idx += 1;
        }
    }

    // List structure in memory:
    // offset 0: size (i32)
    // offset 4: capacity (i32)
    // offset 8+: elements (i32 each)

    #[allow(dead_code)]
    fn generate_create_list(&self) -> Function {
        let mut func = Function::new([(1, ValType::I32)]);

        // Allocate: 8 bytes header + capacity * 4 bytes for elements
        // header_size + capacity * element_size
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::LocalGet(0)); // capacity
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);

        // Call allocate via bump pointer
        func.instruction(&Instruction::GlobalGet(0));
        func.instruction(&Instruction::LocalTee(1)); // Save result ptr

        // Update bump pointer
        func.instruction(&Instruction::GlobalGet(0));
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::GlobalSet(0));

        // Initialize size to 0
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::I32Const(0));
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Initialize capacity
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        }));

        // Return ptr
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_get_item(&self) -> Function {
        let mut func = Function::new([]);

        // list + 8 + index * 4
        func.instruction(&Instruction::LocalGet(0)); // list ptr
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(1)); // index
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_set_item(&self) -> Function {
        let mut func = Function::new([]);

        // list + 8 + index * 4
        func.instruction(&Instruction::LocalGet(0)); // list ptr
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(1)); // index
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(2)); // value
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_get_size(&self) -> Function {
        let mut func = Function::new([]);

        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_append(&self) -> Function {
        let mut func = Function::new([(1, ValType::I32)]);

        // Get current size
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        func.instruction(&Instruction::LocalSet(2)); // current size

        // Store value at list + 8 + size * 4
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(1)); // value
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Increment size
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::I32Const(1));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_insert(&self) -> Function {
        // Simplified: just set at index (full impl would shift elements)
        let mut func = Function::new([]);

        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(1)); // index
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(2)); // value
        func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_remove(&self) -> Function {
        // Simplified: return item at index, don't shift
        let mut func = Function::new([]);

        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::I32Const(8));
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::I32Const(4));
        func.instruction(&Instruction::I32Mul);
        func.instruction(&Instruction::I32Add);
        func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        func.instruction(&Instruction::End);
        func
    }

    #[allow(dead_code)]
    fn generate_deallocate_list(&self) -> Function {
        let mut func = Function::new([]);
        // No-op for bump allocator
        func.instruction(&Instruction::End);
        func
    }
}

fn wasm_type_to_val_type(wasm_type: WasmType) -> ValType {
    match wasm_type {
        WasmType::I32 | WasmType::Ptr => ValType::I32,
        WasmType::I64 => ValType::I64,
        WasmType::F32 => ValType::F32,
        WasmType::F64 => ValType::F64,
        WasmType::Void => panic!("Void type cannot be converted to ValType"),
    }
}

fn wasm_memory_type_to_val_type(memory_type: WasmMemoryType) -> ValType {
    match memory_type {
        WasmMemoryType::I32 | WasmMemoryType::Pointer | WasmMemoryType::ArrayPointer => {
            ValType::I32
        }
        WasmMemoryType::I64 => ValType::I64,
        WasmMemoryType::F32 => ValType::F32,
        WasmMemoryType::F64 => ValType::F64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        DomainClass, DomainModelBuilder, FieldDescriptor, FieldType, PlanningAnnotation,
        PrimitiveType, ScoreType,
    };

    fn create_test_model() -> DomainModel {
        DomainModelBuilder::new()
            .add_class(
                DomainClass::new("Lesson")
                    .with_annotation(PlanningAnnotation::PlanningEntity)
                    .with_field(FieldDescriptor::new(
                        "id",
                        FieldType::Primitive(PrimitiveType::Int),
                    ))
                    .with_field(FieldDescriptor::new(
                        "roomId",
                        FieldType::Primitive(PrimitiveType::Int),
                    )),
            )
            .add_class(
                DomainClass::new("Timetable")
                    .with_annotation(PlanningAnnotation::PlanningSolution)
                    .with_field(
                        FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                            .with_planning_annotation(PlanningAnnotation::planning_score()),
                    ),
            )
            .build()
    }

    #[test]
    fn test_build_minimal_module() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new().with_domain_model(model);
        let wasm_bytes = builder.build().unwrap();

        assert_eq!(&wasm_bytes[0..4], b"\0asm");
        assert_eq!(&wasm_bytes[4..8], &[1, 0, 0, 0]);
    }

    #[test]
    fn test_predicate_generation() {
        let model = create_test_model();
        let predicate = PredicateDefinition::equal(
            "same_room",
            FieldAccess::new(0, "Lesson", "roomId"),
            FieldAccess::new(1, "Lesson", "roomId"),
        );

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_build_base64() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new().with_domain_model(model);
        let base64 = builder.build_base64().unwrap();

        assert!(base64.starts_with("AGFzbQ")); // Base64 of "\0asm"
    }

    #[test]
    fn test_always_true_predicate() {
        let model = create_test_model();
        let predicate = PredicateDefinition::always_true("always_true", 1);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_missing_domain_model() {
        let builder = WasmModuleBuilder::new();
        let result = builder.build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Domain model not set"));
    }

    #[test]
    fn test_predicate_missing_class() {
        let model = create_test_model();
        let predicate = PredicateDefinition::equal(
            "bad_pred",
            FieldAccess::new(0, "NonExistent", "field"),
            FieldAccess::new(1, "NonExistent", "field"),
        );

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_predicate_missing_field() {
        let model = create_test_model();
        let predicate = PredicateDefinition::equal(
            "bad_pred",
            FieldAccess::new(0, "Lesson", "nonexistent"),
            FieldAccess::new(1, "Lesson", "nonexistent"),
        );

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_configuration() {
        let model = create_test_model();
        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .with_initial_memory(32)
            .with_max_memory(Some(512));

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_comparison_variants() {
        let model = create_test_model();
        let left = FieldAccess::new(0, "Lesson", "id");
        let right = FieldAccess::new(1, "Lesson", "id");

        let comparisons = vec![
            ("eq", Comparison::Equal(left.clone(), right.clone())),
            ("ne", Comparison::NotEqual(left.clone(), right.clone())),
            ("lt", Comparison::LessThan(left.clone(), right.clone())),
            (
                "le",
                Comparison::LessThanOrEqual(left.clone(), right.clone()),
            ),
            ("gt", Comparison::GreaterThan(left.clone(), right.clone())),
            (
                "ge",
                Comparison::GreaterThanOrEqual(left.clone(), right.clone()),
            ),
            ("true", Comparison::AlwaysTrue),
            ("false", Comparison::AlwaysFalse),
        ];

        for (name, comparison) in comparisons {
            let predicate = PredicateDefinition::new(name, 2, comparison);
            let builder = WasmModuleBuilder::new()
                .with_domain_model(model.clone())
                .add_predicate(predicate);

            let result = builder.build();
            assert!(result.is_ok(), "Failed for comparison: {}", name);
        }
    }

    #[test]
    fn test_field_access_constructor() {
        let access = FieldAccess::new(0, "Lesson", "room");
        assert_eq!(access.param_index, 0);
        assert_eq!(access.class_name, "Lesson");
        assert_eq!(access.field_name, "room");
    }

    #[test]
    fn test_expression_based_predicate() {
        use crate::wasm::{Expr, FieldAccessExt};

        let model = create_test_model();

        // Build expression: param(0).roomId == param(1).roomId
        let left = Expr::param(0).get("Lesson", "roomId");
        let right = Expr::param(1).get("Lesson", "roomId");
        let expr = Expr::eq(left, right);

        let predicate = PredicateDefinition::from_expression("same_room_expr", 2, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");
    }

    #[test]
    fn test_expression_with_host_function() {
        use crate::wasm::{Expr, FieldAccessExt, HostFunctionRegistry};

        let model = create_test_model();

        // Build expression that uses host function: hstringEquals(param(0).field, param(1).field)
        // Note: We're using int fields as placeholders since our test model doesn't have string fields
        let left = Expr::param(0).get("Lesson", "id");
        let right = Expr::param(1).get("Lesson", "id");
        let expr = Expr::string_equals(left, right);

        let predicate = PredicateDefinition::from_expression("test_host_call", 2, expr);

        let registry = HostFunctionRegistry::with_standard_functions();

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .with_host_functions(registry)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");

        // Verify the module contains an import section (indicated by section ID 2)
        // WASM sections: 1=Type, 2=Import, 3=Function, 5=Memory, 6=Global, 7=Export, 10=Code
        assert!(wasm_bytes.windows(2).any(|w| w[0] == 2 && w[1] > 0));
    }

    #[test]
    fn test_expression_with_logical_operations() {
        use crate::wasm::{Expr, FieldAccessExt};

        let model = create_test_model();

        // Build: param(0).id > 0 AND param(0).roomId == param(1).roomId
        let id_check = Expr::gt(Expr::param(0).get("Lesson", "id"), Expr::int(0));
        let room_match = Expr::eq(
            Expr::param(0).get("Lesson", "roomId"),
            Expr::param(1).get("Lesson", "roomId"),
        );
        let expr = Expr::and(id_check, room_match);

        let predicate = PredicateDefinition::from_expression("complex_predicate", 2, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
    }

    #[test]
    fn test_expression_with_if_then_else() {
        use crate::wasm::{Expr, FieldAccessExt};

        let model = create_test_model();

        // Build: if param(0).id > 0 { param(0).roomId } else { 0 }
        let expr = Expr::if_then_else(
            Expr::gt(Expr::param(0).get("Lesson", "id"), Expr::int(0)),
            Expr::param(0).get("Lesson", "roomId"),
            Expr::int(0),
        );

        let predicate = PredicateDefinition::from_expression("conditional_pred", 1, expr);

        let builder = WasmModuleBuilder::new()
            .with_domain_model(model)
            .add_predicate(predicate);

        let wasm_bytes = builder.build().unwrap();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], b"\0asm");
    }
}
