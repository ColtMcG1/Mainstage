pub mod opcode;
pub mod value;
pub mod vm;

pub use opcode::Op;
pub use value::Value;
pub use vm::{ExecutionResult, Vm};

use std::path::Path;

/// Execute a single bytecode function (first function = entry).
pub fn execute(
    module: &crate::codegen::bytecode::BytecodeModule,
    script_path: &Path,
) -> Result<(), String> {
    let base = script_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut vm = crate::runtime::vm::Vm::new_with_base(module, base);
    vm.run()
}
