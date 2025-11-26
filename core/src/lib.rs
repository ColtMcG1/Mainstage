pub mod ast;
pub mod error;
pub mod location;
pub mod script;

pub use ast::RulesParser;
pub use error::{Level, MainstageErrorExt};
pub use location::{Location, Span};
pub use script::Script;

pub fn generate_error_report<E: MainstageErrorExt>(error: &E) -> String {
    let level = error.level();
    let location = match error.location() {
        Some(loc) => loc.to_string(),
        None => "unknown location".to_string(),
    };
    let message = error.message();

    format!("MAINSTAGE | {} | {} | {}", level, location, message)
}

pub fn analyze_ast(ast: &str) -> Result<String, Box<dyn MainstageErrorExt>> {
    Ok(format!("Analysis({})", ast))
}

pub fn generate_ir_from_ast(
    ast: &str,
    analysis: &str,
) -> Result<String, Box<dyn MainstageErrorExt>> {
    // Placeholder implementation
    Ok(format!("IR({} + {})", ast, analysis))
}

pub fn optimize_ir(ir: &str) -> Result<String, Box<dyn MainstageErrorExt>> {
    Ok(format!("Optimized({})", ir))
}

pub fn run_ir_in_vm(_ir: &str) -> Result<String, Box<dyn MainstageErrorExt>> {
    Ok(format!("IR"))
}

pub fn compile_source_to_ir(source: &Script) -> Result<String, Box<dyn MainstageErrorExt>> {
    let _ast = ast::generate_ast_from_source(source)?;
    let analysis = analyze_ast("")?;
    let ir = generate_ir_from_ast("", &analysis)?;
    let optimized_ir = optimize_ir(&ir)?;
    Ok(optimized_ir)
}
