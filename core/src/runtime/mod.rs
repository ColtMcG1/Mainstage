pub mod opcode;
pub mod value;
pub mod vm;

pub use opcode::Op;
pub use value::RTValue;
pub use vm::{ExecutionResult, VmIR};

use std::path::Path;

/// Execute a single bytecode function (first function = entry).
pub fn execute(
    module: &crate::codegen::IRProgram,
    _script_path: &Path,
) -> Result<(), String> {
    //let base = script_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut vm = crate::runtime::vm::VmIR::new(module);
    vm.run()
}
