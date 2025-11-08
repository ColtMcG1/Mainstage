use crate::codegen::{lower_ast_to_ir, emit_bytecode};
use crate::parser::AstNode;
use crate::codegen::bytecode::BytecodeModule;

/// Entry point to produce bytecode from an AST.
/// Later: thread through reports and semantic info.
pub fn generate_bytecode(root: &AstNode<'_>) -> BytecodeModule {
    let ir = lower_ast_to_ir(root);
    emit_bytecode(&ir)
}