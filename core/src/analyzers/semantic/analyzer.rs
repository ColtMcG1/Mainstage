use crate::analyzers::semantic::table::SymbolTable;
use crate::analyzers::semantic::err::SemanticError;
use crate::error::Level;
use crate::ast::{AstNode, AstNodeKind};
use crate::error::MainstageErrorExt;
use crate::analyzers::semantic::stmt as stmt_mod;

pub struct Analyzer {
    tbl: SymbolTable,
    // you can add Vec<Diagnostic> here later
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer { tbl: SymbolTable::new() }
    }

    pub fn analyze(&mut self, node: &mut AstNode) -> Result<(), Box<dyn MainstageErrorExt>> {
        // Delegate to stmt analyzer, keep the table owned by Analyzer.
        stmt_mod::analyze_script_statements(node, &mut self.tbl)
    }
}