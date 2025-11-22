use crate::{ir::op::Op, parser::ast};
use crate::ir::lowering::lower_ast_to_ir;
use crate::ir::lowering::context::{analyze_meta, IRMeta};
use crate::ir::optimization::optimize_ops_default;

#[derive(Debug, Clone, PartialEq)]
pub struct IRProgram {
    pub ops: Vec<Op>,
    pub meta: IRMeta,
}

pub fn generate_program_from_ast(root: &ast::AstNode, entry: &str) -> IRProgram {
    let mut ops = lower_ast_to_ir(root, entry).ops;

    // Optimize the rough IR before computing meta
    println!("Ops before optimization : {}", ops.len());
    optimize_ops_default(&mut ops); 
    println!("Ops after optimization : {}", ops.len());

    let meta = analyze_meta(&ops);
    IRProgram { ops, meta }
}