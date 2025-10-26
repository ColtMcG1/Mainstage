//! ./semantic/analyzer.rs
//!
//! Semantic analyzer for the Mainstage programming language.
//! This module provides the `SemanticAnalyzer` struct and related functionality.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use crate::parser::*;
use crate::report;
use crate::reports::*;
use crate::semantic::symbol::*;

pub struct SemanticAnalyzer<'a> {
    /// The AST to analyze.
    ast: AstParser,
    /// The symbol table for the current scope.
    pub symbol_table: SymbolTable<'a>,
    /// Entry point for analysis. Node id of the entry point stage/task.
    pub entry_point: AstNode<'a>,
}

impl<'a> SemanticAnalyzer<'a> {
    /// Creates a new `SemanticAnalyzer` instance.
    /// # Arguments
    /// * `ast` - The AST to analyze.
    /// # Returns
    /// * A new `SemanticAnalyzer` instance.
    pub fn new(ast: AstParser) -> Result<Self, ()> {
        let mut analyzer = Self {
            ast: ast.clone(),
            symbol_table: SymbolTable::new(),
            entry_point: ast.root().clone(),
        };
        analyzer.analyze()?;
        // If we reach here, analysis was successful
        Ok(analyzer)
    }

    /// Analyzes the AST and populates the symbol table.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    /// * `Err(())` if there is an error during analysis.
    fn analyze(&mut self) -> Result<(), ()> {
        let children = self.ast.root.children.clone();

        if children.is_empty() {
            report!(
                Level::Error,
                "The AST is empty. Nothing to analyze.".into(),
                Some("SemanticAnalyzer".into()),
                None,
                None
            );
            return Err(());
        }

        let mut entrypoint_ids = Vec::new();
        let mut workspace_id = None;

        fn has_entrypoint_attr(node: &AstNode) -> bool {
            node.attributes.iter().any(|attr| attr.name == "entrypoint")
        }

        for node in &children {
            match &node.kind {
                AstType::Workspace { name: _ } => {
                    workspace_id = Some(node.clone());
                }
                AstType::Project { .. } | AstType::Stage { .. } => {
                    if has_entrypoint_attr(node) {
                        entrypoint_ids.push(node.clone());
                    }
                }
                _ => {}
            }
        }

        if entrypoint_ids.len() > 1 {
            report!(
                Level::Critical,
                format!(
                    "Multiple entrypoints specified: {:?}. Only one entrypoint is allowed.",
                    entrypoint_ids
                ),
                Some("SemanticAnalyzer".into()),
                None,
                None
            );
            return Err(());
        } else if entrypoint_ids.len() == 1 {
            self.entry_point = entrypoint_ids[0].clone();
        } else if let Some(ws_id) = workspace_id {
            self.entry_point = ws_id;
        } else {
            report!(
                Level::Critical,
                "No entrypoint or workspace found in the AST.".into(),
                Some("SemanticAnalyzer".into()),
                None,
                None
            );
            return Err(());
        }

        for mut node in children {
            self.analyze_node(&mut node)?;
        }
        self.symbol_table.warn_unused_symbols();
        self.symbol_table.warn_hot_paths();

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
                self.symbol_table.insert(symbol.clone())?;
            }
            AstType::Project { name } => {
                let symbol = Symbol::new_project(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(symbol.clone())?;
                self.process_scope(node)?;
            }
            AstType::Stage { name, params } => {
                let symbol = Symbol::new_stage(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(symbol.clone())?;

                self.symbol_table.enter_scope();

                // Insert parameters as local variables
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        let param_symbol = Symbol::new_variable(
                            param_name.to_string().into(),
                            SymbolType::String, // or infer type if possible
                            SymbolScope::Local,
                        );

                        if self.symbol_table.exists(param_name) {
                            report!(
                                Level::Error,
                                format!("Parameter name '{}' already exists in scope.", param_name),
                                Some("SemanticAnalyzer".into()),
                                node.span.clone(),
                                node.location.clone()
                            );
                            return Err(());
                        } else {
                            self.symbol_table.insert(param_symbol)?;
                        }
                    }
                }

                // Now analyze the body
                for child in &node.children {
                    self.analyze_node(child)?;
                }

                // Warn if the stage body is empty
                if self
                    .symbol_table
                    .get_current_scope_symbols()
                    .ok_or(())?
                    .len()
                    == 0
                {
                    report!(
                        Level::Warning,
                        format!("Stage '{}' has an empty body.", name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                }

                // Check for unused parameters
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        if let Some(symbols) = self.symbol_table.get(param_name) {
                            let symbol = &symbols[0];
                            if symbol.reference_count() == 1 {
                                report!(
                                    Level::Warning,
                                    format!(
                                        "Parameter '{}' is declared but never used in stage '{}'.",
                                        param_name, name
                                    ),
                                    Some("SemanticAnalyzer".into()),
                                    node.span.clone(),
                                    node.location.clone()
                                );
                            }
                        }
                    }
                }

                // Exit the stage scope
                self.symbol_table.exit_scope();
            }
            AstType::Task { name, params } => {
                let symbol = Symbol::new_stage(name.to_string().into(), SymbolScope::Local);
                self.symbol_table.insert(symbol.clone())?;

                self.symbol_table.enter_scope();

                // Insert parameters as local variables
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        let param_symbol = Symbol::new_variable(
                            param_name.to_string().into(),
                            SymbolType::String, // or infer type if possible
                            SymbolScope::Local,
                        );

                        if self.symbol_table.exists(param_name) {
                            report!(
                                Level::Error,
                                format!("Parameter name '{}' already exists in scope.", param_name),
                                Some("SemanticAnalyzer".into()),
                                node.span.clone(),
                                node.location.clone()
                            );
                            return Err(());
                        } else {
                            self.symbol_table.insert(param_symbol)?;
                        }
                    }
                }

                // Now analyze the body
                for child in &node.children {
                    self.analyze_node(child)?;
                }

                // Warn if the task body is empty
                if self
                    .symbol_table
                    .get_current_scope_symbols()
                    .ok_or(())?
                    .len()
                    == 0
                {
                    report!(
                        Level::Warning,
                        format!("Task '{}' has an empty body.", name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                }

                // Check for unused parameters
                for param in params {
                    if let AstType::Identifier { name: param_name } = &param.kind {
                        if let Some(symbols) = self.symbol_table.get(param_name) {
                            let symbol = &symbols[0];
                            if symbol.reference_count() == 1 {
                                report!(
                                    Level::Warning,
                                    format!(
                                        "Parameter '{}' is declared but never used in task '{}'.",
                                        param_name, name
                                    ),
                                    Some("SemanticAnalyzer".into()),
                                    node.span.clone(),
                                    node.location.clone()
                                );
                            }
                        }
                    }
                }

                // Exit the task scope
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
        if node.children.len() != 2 {
            report!(
                Level::Error,
                "Invalid assignment node: must have exactly two children.".into(),
                Some("SemanticAnalyzer".into()),
                node.span.clone(),
                node.location.clone()
            );
            return Err(());
        }

        let lhs = &node.children[0];
        let rhs = &node.children[1];

        // Ensure LHS is an identifier
        if let AstType::Identifier { name } = &lhs.kind {
            let inferred_type = self.infer_type(rhs)?;

            if !self.symbol_table.exists(name) {
                let symbol = Symbol::new_variable(
                    name.to_string().into(),
                    inferred_type,
                    SymbolScope::Local,
                );
                self.symbol_table.insert(symbol)?;
            } else {
                let mut existing_symbol = self.symbol_table.get_mut(name).unwrap()[0].clone();
                let existing_type = existing_symbol.symbol_type();
                if existing_type != &inferred_type {
                    report!(
                        Level::Error,
                        format!(
                            "Type mismatch: cannot assign {:?} to {:?}",
                            inferred_type, existing_type
                        ),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                } else {
                    existing_symbol.increment_reference_count();
                }
            }
        } else {
            report!(
                Level::Critical,
                "Left-hand side of assignment must be an identifier.".into(),
                Some("SemanticAnalyzer".into()),
                node.span.clone(),
                node.location.clone()
            );
            return Err(());
        }

        self.analyze_expression(rhs)?;

        // If we reach here, the assignment is valid
        // Next check if the assignment is recursive (i.e., assigning a variable to itself)
        if let AstType::Identifier { name: rhs_name } = &rhs.kind {
            if let AstType::Identifier { name: lhs_name } = &lhs.kind {
                if rhs_name == lhs_name {
                    report!(
                        Level::Warning,
                        format!("Recursive assignment detected for variable: {}", lhs_name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                }
            }
        }

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
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                } else {
                    // Increment reference count for the symbol
                    if let Some(symbols) = self.symbol_table.get_mut(name) {
                        symbols[0].increment_reference_count();
                    }
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
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }

                let valid_shells = vec!["bash", "sh", "zsh", "cmd", "pwsh"];
                if valid_shells.iter().all(|&valid| valid != shell) {
                    report!(
                        Level::Error,
                        format!("Invalid shell: {}", shell),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
            }
            AstType::Boolean { value } => {
                if *value != true && *value != false {
                    report!(
                        Level::Error,
                        format!("Invalid boolean literal: {}", value),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
            }
            AstType::String { value } => {
                if value.is_empty() {
                    report!(
                        Level::Error,
                        "String literal cannot be empty.".into(),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
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
            }
            AstType::Number { value } => {
                if !value.is_finite() {
                    report!(
                        Level::Error,
                        format!("Invalid number literal: {}", value),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
            }
            AstType::Null => {
                // Null type is valid, no further analysis needed
            }
            //AstType::BinaryOp { left, right, .. } => {
            //    // Analyze left and right expressions
            //    self.analyze_expression(left);
            //    self.analyze_expression(right);
            //}
            _ => {
                report!(
                    Level::Error,
                    format!("Unsupported expression type: {:?}", node.kind).into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
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
                        node.span.clone(),
                        node.location.clone()
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
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }

                let valid_shells = vec!["bash", "sh", "zsh", "cmd", "pwsh"];
                if valid_shells.iter().all(|&valid| valid != shell) {
                    report!(
                        Level::Error,
                        format!("Invalid shell: {}", shell),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
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
                            node.span.clone(),
                            node.location.clone()
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
                            node.span.clone(),
                            node.location.clone()
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
                        node.span.clone(),
                        node.location.clone()
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
                    node.span.clone(),
                    node.location.clone()
                );
                Err(())
            }
        }
    }
}
