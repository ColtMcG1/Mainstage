pub mod lower;
pub mod opt;
pub mod bytecode;
pub mod op;
pub mod value;

use self::lower::IrModule;
pub use self::bytecode::emit_bytecode;

pub fn lower_ast_to_ir(ast: &crate::ast::AstNode, entrypoint: &str, optimize: bool) -> IrModule {
    let mut ir_mod = IrModule::new();
    ir_mod.lower_from_ast(ast, entrypoint);
    if optimize {
        opt::optimize(&mut ir_mod);
    }
    ir_mod
}