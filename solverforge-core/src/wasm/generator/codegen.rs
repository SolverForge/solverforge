use crate::wasm::memory::WasmMemoryType;
use indexmap::IndexMap;
use wasm_encoder::{Function, Instruction, ValType};

/// Generate the bump allocator function.
pub(super) fn generate_allocator() -> Function {
    let mut func = Function::new([(3, ValType::I32)]);

    func.instruction(&Instruction::GlobalGet(0));
    func.instruction(&Instruction::LocalSet(1));

    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(2));

    func.instruction(&Instruction::MemorySize(0));
    func.instruction(&Instruction::I32Const(65536));
    func.instruction(&Instruction::I32Mul);
    func.instruction(&Instruction::LocalSet(3));

    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::I32GtU);
    func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::MemoryGrow(0));
    func.instruction(&Instruction::I32Const(-1));
    func.instruction(&Instruction::I32Eq);
    func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::Unreachable);
    func.instruction(&Instruction::End);

    func.instruction(&Instruction::End);

    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::GlobalSet(0));

    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::End);
    func
}

/// Generate the deallocator function (no-op for bump allocator).
pub(super) fn generate_deallocator() -> Function {
    let mut func = Function::new([]);
    func.instruction(&Instruction::End);
    func
}

/// Generate a getter function for a field.
pub(super) fn generate_getter(offset: u32, wasm_type: WasmMemoryType) -> Function {
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

/// Generate a setter function for a field.
pub(super) fn generate_setter(offset: u32, wasm_type: WasmMemoryType) -> Function {
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

/// Describes a generated function to be added to the module.
pub(super) struct GeneratedFunction {
    pub name: String,
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
    pub body: Function,
}

/// Generate solution mapper wrapper functions.
pub(super) fn generate_solution_mappers(
    host_function_indices: &IndexMap<String, u32>,
) -> Vec<GeneratedFunction> {
    let mut functions = Vec::new();

    if let Some(&host_idx) = host_function_indices.get("hparseSchedule") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "parseSchedule".to_string(),
            params: vec![ValType::I32, ValType::I32],
            results: vec![ValType::I32],
            body: func,
        });
    }

    if let Some(&host_idx) = host_function_indices.get("hscheduleString") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "scheduleString".to_string(),
            params: vec![ValType::I32],
            results: vec![ValType::I32],
            body: func,
        });
    }

    functions
}

/// Generate list accessor wrapper functions.
pub(super) fn generate_list_accessors(
    host_function_indices: &IndexMap<String, u32>,
) -> Vec<GeneratedFunction> {
    let mut functions = Vec::new();

    // newList() -> i32
    if let Some(&host_idx) = host_function_indices.get("hnewList") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "newList".to_string(),
            params: vec![],
            results: vec![ValType::I32],
            body: func,
        });
    }

    // getItem(list: i32, index: i32) -> i32
    if let Some(&host_idx) = host_function_indices.get("hgetItem") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "getItem".to_string(),
            params: vec![ValType::I32, ValType::I32],
            results: vec![ValType::I32],
            body: func,
        });
    }

    // setItem(list: i32, index: i32, value: i32)
    if let Some(&host_idx) = host_function_indices.get("hsetItem") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "setItem".to_string(),
            params: vec![ValType::I32, ValType::I32, ValType::I32],
            results: vec![],
            body: func,
        });
    }

    // size(list: i32) -> i32
    if let Some(&host_idx) = host_function_indices.get("hsize") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "size".to_string(),
            params: vec![ValType::I32],
            results: vec![ValType::I32],
            body: func,
        });
    }

    // append(list: i32, value: i32)
    if let Some(&host_idx) = host_function_indices.get("happend") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "append".to_string(),
            params: vec![ValType::I32, ValType::I32],
            results: vec![],
            body: func,
        });
    }

    // insert(list: i32, index: i32, value: i32)
    if let Some(&host_idx) = host_function_indices.get("hinsert") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "insert".to_string(),
            params: vec![ValType::I32, ValType::I32, ValType::I32],
            results: vec![],
            body: func,
        });
    }

    // remove(list: i32, index: i32)
    if let Some(&host_idx) = host_function_indices.get("hremove") {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::LocalGet(1));
        func.instruction(&Instruction::Call(host_idx));
        func.instruction(&Instruction::End);

        functions.push(GeneratedFunction {
            name: "remove".to_string(),
            params: vec![ValType::I32, ValType::I32],
            results: vec![],
            body: func,
        });
    }

    functions
}
