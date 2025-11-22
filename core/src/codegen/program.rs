use crate::{codegen::op::Op, parser::ast};
use crate::codegen::lowering::lower_ast_to_ir;
use crate::codegen::lowering::context::{analyze_meta, IRMeta};

#[derive(Debug, Clone, PartialEq)]
pub struct IRProgram {
    pub ops: Vec<Op>,
    pub meta: IRMeta,
}

pub fn generate_program_from_ast(root: &ast::AstNode, entry: Option<&str>) -> IRProgram {
    let ops = lower_ast_to_ir(root, entry).ops;
    let meta = analyze_meta(&ops);
    IRProgram { ops, meta }
}