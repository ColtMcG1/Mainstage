pub mod value;
pub mod opcode;
pub mod vm;

pub use value::Value;
pub use opcode::Op;
pub use vm::{Vm, ExecutionResult};

/// Execute a single bytecode function (first function = entry).
pub fn execute(bytecode: &crate::codegen::bytecode::BytecodeModule) -> ExecutionResult {
    let entry = bytecode.functions.first()
        .ok_or_else(|| "No entry function".to_string())?;

    let mut vm = Vm::new(
        &entry.code,
        &bytecode.const_pool,
        bytecode.functions.len(),
        bytecode.const_pool.len(),
    );

    vm.run()
}