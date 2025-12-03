pub mod lower;
pub mod opt;
pub mod bytecode;
pub mod op;
pub mod value;
pub mod module;

pub use self::bytecode::emit_bytecode;
use self::module::IrModule;
use self::lower::lower_script_objects;

pub fn lower_ast_to_ir(
    ast: &crate::ast::AstNode,
    _entrypoint: &str,
    optimize: bool,
    analysis: Option<&crate::analyzers::output::AnalyzerOutput>,
) -> IrModule {
    let mut ir_mod = IrModule::new();
    lower_script_objects(ast, &mut ir_mod, analysis);
    if optimize {
        println!("Ops before optimization: {}", ir_mod.len());
        opt::optimize(&mut ir_mod);
        println!("Ops after optimization: {}", ir_mod.len());
    }
    ir_mod
}