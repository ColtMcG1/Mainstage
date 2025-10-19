//! ./semantic/analyzer.rs
//! 
//! Semantic analyzer for the Mainstage programming language.
//! This module provides the `SemanticAnalyzer` struct and related functionality.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use crate::parser::AstParser;
use crate::semantic::symbol::*;

pub struct SemanticAnalyzer {
    /// The AST to analyze.
    ast: AstParser,
    /// The symbol table for the current scope.
    pub symbol_table: SymbolTable<'static>,
}

impl SemanticAnalyzer {

    /// Creates a new `SemanticAnalyzer` instance.
    /// # Arguments
    /// * `ast` - The AST to analyze.
    /// # Returns
    /// * A new `SemanticAnalyzer` instance.
    pub fn new(ast: AstParser) -> Result<Self, ()> {
        let mut analyzer = Self {
            ast,
            symbol_table: SymbolTable::new(),
        };
        analyzer.analyze()?;
        Ok(analyzer)
    }

    /// Analyzes the AST and populates the symbol table.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    /// * `Err(())` if there is an error during analysis.
    fn analyze(&mut self) -> Result<(), ()> {
        // Perform semantic analysis on the AST.
        // This involves checking for type errors, undeclared variables, etc.
        // For now, we'll just populate the symbol table with basic information.
        self.symbol_table = SymbolTable::new();
        Ok(())
    }
}