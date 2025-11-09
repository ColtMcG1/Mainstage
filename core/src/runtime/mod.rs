pub mod value;
pub mod opcode;
pub mod vm;

pub use value::Value;
pub use opcode::Op;
pub use vm::{Vm, ExecutionResult};

/// Execute a single bytecode function (first function = entry).
pub fn execute(module: &crate::codegen::bytecode::BytecodeModule) -> ExecutionResult {
    let mut vm = Vm::new(
        &module,
    );
    vm.run()
}