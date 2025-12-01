use crate::analyzers::semantic::table::SymbolTable;
use crate::ast::AstNode;
use crate::error::MainstageErrorExt;
use crate::analyzers::semantic::stmt as stmt_mod;

use crate::vm::plugin::PluginDescriptor;
use std::collections::HashMap;

pub struct Analyzer {
    tbl: SymbolTable,
    manifests: Option<HashMap<String, PluginDescriptor>>,
}

impl Analyzer {
    pub fn new_with_manifests(manifests: Option<HashMap<String, PluginDescriptor>>) -> Self {
        Analyzer { tbl: SymbolTable::new(), manifests }
    }

    pub fn analyze(&mut self, node: &mut AstNode) -> Result<(), Box<dyn MainstageErrorExt>> {
        // Delegate to stmt analyzer, keep the table owned by Analyzer.
        stmt_mod::analyze_script_statements(node, &mut self.tbl, self.manifests.as_ref())?;
        Ok(())
    }

    /// Return any diagnostics (warnings/infos) collected during analysis.
    pub fn take_diagnostics(&mut self) -> Vec<Box<dyn MainstageErrorExt>> {
        self.tbl.take_diagnostics()
    }

    /// Get the symbol table after analysis.
    pub fn get_symbol_table(&self) -> &SymbolTable {
        &self.tbl
    }
}