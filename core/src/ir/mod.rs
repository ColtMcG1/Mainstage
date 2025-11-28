pub mod irmod;
pub mod op;
pub mod value;

use super::ir::irmod::IrModule;

pub fn lower_ast_to_ir(ast: &crate::ast::AstNode, entrypoint: &str) -> IrModule {
    let mut ir_mod = IrModule::new();
    ir_mod.lower_from_ast(ast, entrypoint);
    ir_mod
}