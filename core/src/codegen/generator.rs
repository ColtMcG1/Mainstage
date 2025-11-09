use crate::codegen::{lower_ast_to_ir, emit_bytecode};
use crate::parser::AstNode;
use crate::codegen::bytecode::BytecodeModule;

/// Entry point to produce bytecode from an AST.
/// Later: thread through reports and semantic info.
pub fn generate_bytecode(root: &AstNode<'_>) -> BytecodeModule {
    let ir = lower_ast_to_ir(root);
    emit_bytecode(&ir)
}
// Later when running:
// let bc = generate_bytecode(root);
// let mut vm = Vm::new(&bc);
// vm.run().unwrap();