//! ./semantic/analyzer.rs
//!
//! Semantic analyzer for the Mainstage programming language.
//! This module provides the `SemanticAnalyzer` struct and related functionality.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use std::fmt::format;

use crate::parser::*;
use crate::report;
use crate::reports::*;
use crate::scripts::script;
use crate::semantic::symbol::*;

pub struct SemanticAnalyzer<'a> {
    /// The AST to analyze.
    ast: AstParser,
    /// The symbol table for the current scope.
    pub symbol_table: SymbolTable<'a>,
}

impl<'a> SemanticAnalyzer<'a> {
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
        let children = self.ast.root.children.clone();
        for mut node in children {
            self.analyze_node(&mut node)?;
        }
        Ok(())
    }

    /// Analyzes a single AST node.
    /// # Arguments
    /// * `node` - The AST node to analyze.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    fn analyze_node(&mut self, node: &AstNode) -> Result<(), ()> {
        match &node.kind {
            AstType::Workspace { name } => {
                let symbol = Symbol::new_workspace(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(symbol.clone());
            }
            AstType::Project { name } => {
                let symbol = Symbol::new_project(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(symbol.clone());
                self.process_scope(node)?;
            }
            AstType::Stage { name, params } => {
                let symbol = Symbol::new_stage(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(symbol.clone());

                self.symbol_table.enter_scope();

                // Insert parameters as local variables
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        let param_symbol = Symbol::new_variable(
                            param_name.to_string().into(),
                            SymbolType::String, // or infer type if possible
                            SymbolScope::Local,
                        );
                        self.symbol_table.insert(param_symbol);
                    }
                }

                // Now analyze the body
                for child in &node.children {
                    self.analyze_node(child)?;
                }

                self.symbol_table.exit_scope();
            }
            AstType::Task { name, params } => {
                let symbol = Symbol::new_stage(name.to_string().into(), SymbolScope::Local);
                self.symbol_table.insert(symbol.clone());

                self.symbol_table.enter_scope();

                // Insert parameters as local variables
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        let param_symbol = Symbol::new_variable(
                            param_name.to_string().into(),
                            SymbolType::String, // or infer type if possible
                            SymbolScope::Local,
                        );
                        self.symbol_table.insert(param_symbol);
                    }
                }

                // Now analyze the body
                for child in &node.children {
                    self.analyze_node(child)?;
                }

                self.symbol_table.exit_scope();
            }

            AstType::Assignment => self.analyze_assignment(node)?,

            _ => {
                // Handle other AST node types as needed
            }
        }
        Ok(())
    }

    /// Process a scope
    /// # Arguments
    /// * `node` - The AST node representing the scope.
    /// # Returns
    /// * `Ok(())` if the scope is processed successfully.
    fn process_scope(&mut self, node: &AstNode) -> Result<(), ()> {
        self.symbol_table.enter_scope();
        for mut child in &node.children {
            self.analyze_node(&mut child)?;
        }
        self.symbol_table.exit_scope();
        Ok(())
    }

    /// Analyzes an assignment node.
    /// # Arguments
    /// * `node` - The assignment AST node to analyze.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    fn analyze_assignment(&mut self, node: &AstNode) -> Result<(), ()> {
        let lhs = &node.children[0];
        let rhs = &node.children[1];

        if let AstType::Identifier { name } = &lhs.kind {
            let inferred_type = self.infer_type(rhs)?;

            if !self.symbol_table.exists(name) {
                let symbol = Symbol::new_variable(
                    name.to_string().into(),
                    inferred_type,
                    SymbolScope::Local,
                );
                self.symbol_table.insert(symbol);
            } else {
                let existing_type = self.symbol_table.get(name).unwrap()[0].symbol_type();
                if existing_type != &inferred_type {
                    report!(
                        Level::Error,
                        format!(
                            "Type mismatch: cannot assign {:?} to {:?}",
                            inferred_type, existing_type
                        ),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }
            }
        }

        self.analyze_expression(rhs)?;
        Ok(())
    }

    /// Analyzes an expression node.
    /// # Arguments
    /// * `node` - The expression AST node to analyze.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    fn analyze_expression(&mut self, node: &AstNode) -> Result<(), ()> {
        match &node.kind {
            AstType::Identifier { name } => {
                // Check if the identifier exists in the symbol table
                if !self.symbol_table.exists(name) {
                    report!(
                        Level::Error,
                        format!("Undefined identifier: {}", name),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }
            }
            AstType::Array => {
                // Analyze each element in the array literal
                for element in &node.children {
                    self.analyze_expression(element)?;
                }
            }
            AstType::ShellCommand { shell, command } => {
                if shell.is_empty() || command.is_empty() {
                    report!(
                        Level::Error,
                        "Shell command must have both shell and command specified.".into(),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }

                let valid_shells = vec!["bash", "sh", "zsh", "cmd", "pwsh"];
                if valid_shells.iter().all(|&valid| valid != shell) {
                    report!(
                        Level::Error,
                        format!("Invalid shell: {}", shell),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }
            }
            AstType::Boolean { value } => {}
            AstType::String { value } => {
                // Handle string literals
            }
            AstType::Number { value } => {
                // Handle number literals
            }
            AstType::Null => {
                // Handle null literals
            }
            //AstType::BinaryOp { left, right, .. } => {
            //    // Analyze left and right expressions
            //    self.analyze_expression(left);
            //    self.analyze_expression(right);
            //}
            _ => {
                // Handle other expression types as needed
            }
        }
        Ok(())
    }

    /// Infers the type of an AST node.
    /// # Arguments
    /// * `node` - The AST node to infer the type of.
    /// # Returns
    /// * `Ok(SymbolType)` if the type is successfully inferred.
    fn infer_type(&self, node: &AstNode) -> Result<SymbolType, ()> {
        match &node.kind {
            AstType::Boolean { .. } => Ok(SymbolType::Boolean),
            AstType::String { value } => {
                if value.is_empty() {
                    report!(
                        Level::Error,
                        "String literal cannot be empty.".into(),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }

                if value.len() > 1000 {
                    report!(
                        Level::Warning,
                        "String literal exceeds length of 1000 characters. Is this intentional?"
                            .into(),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                }

                Ok(SymbolType::String)
            }
            AstType::ShellCommand { shell, command } => {
                if shell.is_empty() || command.is_empty() {
                    report!(
                        Level::Error,
                        "Shell command must have both shell and command specified.".into(),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }

                let valid_shells = vec!["bash", "sh", "zsh", "cmd", "pwsh"];
                if valid_shells.iter().all(|&valid| valid != shell) {
                    report!(
                        Level::Error,
                        format!("Invalid shell: {}", shell),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    return Err(());
                }

                let dangerous_patterns = vec![
                    "rm -rf /",
                    "rm -rf /*",
                    "rm -rf --no-preserve-root /",
                    ":(){ :|:& };:",
                    "mkfs.ext4 /dev/sda",
                    "dd if=/dev/zero of=/dev/sda",
                    "Delete-Partition -DriveLetter C -Confirm:$false",
                    "Remove-Item -Path C:\\ -Recurse -Force",
                    "format C: /Q /Y",
                    "shutdown /s /f /t 0",
                ];
                for pattern in dangerous_patterns {
                    if command.contains(pattern) {
                        report!(
                            Level::Error,
                            format!("Dangerous command detected: {}", pattern).into(),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                        return Err(());
                    }
                }

                Ok(SymbolType::ShellCommand)
            }
            AstType::Number { .. } => Ok(SymbolType::Float), // Default to Float for numbers
            AstType::Array => {
                let elements = &node.children;
                if elements.is_empty() {
                    return Ok(SymbolType::None); // Empty arrays have no type
                }

                // Infer the type of the first element
                let first_type = self.infer_type(&elements[0])?;

                // Ensure all elements have the same type
                for element in elements.iter().skip(1) {
                    let element_type = self.infer_type(element)?;
                    if element_type != first_type {
                        report!(
                            Level::Error,
                            "Array elements must have the same type.".into(),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                        return Err(());
                    }
                }

                Ok(first_type) // Return the type of the array elements
            }
            AstType::Identifier { name } => {
                if let Some(symbols) = self.symbol_table.get(name) {
                    Ok(symbols[0].symbol_type().clone())
                } else {
                    report!(
                        Level::Error,
                        format!("Undefined identifier: {}", name),
                        Some("SemanticAnalyzer".into()),
                        None,
                        None
                    );
                    Err(())
                }
            }
            //AstType::BinaryOp { left, right, operator } => {
            //    let left_type = self.infer_type(left)?;
            //    let right_type = self.infer_type(right)?;
            //
            //    if left_type == right_type {
            //        match operator {
            //            "+" | "-" | "*" | "/" => Ok(left_type), // Arithmetic operations
            //            "&&" | "||" => {
            //                if left_type == SymbolType::Boolean {
            //                    Ok(SymbolType::Boolean)
            //                } else {
            //                    Err(())
            //                }
            //            }
            //            _ => Err(()),
            //        }
            //    } else {
            //        Err(())
            //    }
            //}
            _ => {
                report!(
                    Level::Error,
                    format!("Unable to infer type for node: {:?}", node.kind).into(),
                    Some("SemanticAnalyzer".into()),
                    None,
                    None
                );
                Err(())
            }
        }
    }
}
