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
use std::collections::HashMap;
use std::collections::HashSet;

// Simple internal inference enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredType {
    Int,
    Bool,
    Str,
    Array,
    Unit,
    Unknown,
}

impl InferredType {
    fn to_symbol_type(self) -> SymbolType {
        match self {
            InferredType::Int => SymbolType::Integer,
            InferredType::Bool => SymbolType::Boolean,
            InferredType::Str => SymbolType::String,
            InferredType::Array => SymbolType::Array,
            InferredType::Unit => SymbolType::None,
            InferredType::Unknown => SymbolType::None,
        }
    }
}

pub struct SemanticAnalyzer<'a> {
    ast: AstParser,
    pub symbol_table: SymbolTable<'a>,
    pub entry_point: AstNode<'a>,
    task_returns: HashMap<String, InferredType>,
    builtin_funcs: HashSet<&'static str>,
}

// --- Analyze ---
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
            task_returns: HashMap::new(),
            builtin_funcs: ["say", "read", "write"].into_iter().collect(),
        };
        analyzer.analyze()?;
        // If we reach here, analysis was successful
        Ok(analyzer)
    }

    fn is_builtin(&self, name: &str) -> bool {
        self.builtin_funcs.contains(name)
    }

    /// Analyzes the AST and populates the symbol table.
    /// # Returns
    /// * `Ok(())` if the analysis is successful.
    /// * `Err(())` if there is an error during analysis.
    fn analyze(&mut self) -> Result<(), ()> {
        let root_clone = self.ast.root.clone();
        let mut children = root_clone.children.clone();

        if children.is_empty() {
            report!(Level::Error, "The AST is empty. Nothing to analyze.".into(), Some("SemanticAnalyzer".into()), None, None);
            return Err(());
        }

        let mut entrypoint_ids = Vec::new();
        let mut workspace_id: Option<AstNode<'a>> = None;

        fn has_entrypoint_attr(node: &AstNode) -> bool {
            node.attributes.iter().any(|attr| attr.name == "entrypoint")
        }

        for node in &children {
            match &node.kind {
                AstType::Workspace { .. } => workspace_id = Some(node.clone()),
                AstType::Project { .. } | AstType::Stage { .. } => {
                    if has_entrypoint_attr(node) {
                        entrypoint_ids.push(node.clone());
                    }
                }
                _ => {}
            }
        }

        if entrypoint_ids.len() > 1 {
            report!(Level::Critical, format!("Multiple entrypoints specified: {:?}. Only one entrypoint is allowed.", entrypoint_ids), Some("SemanticAnalyzer".into()), None, None);
            return Err(());
        } else if entrypoint_ids.len() == 1 {
            self.entry_point = entrypoint_ids[0].clone();
        } else if let Some(ws_id) = workspace_id.clone() {
            self.entry_point = ws_id;
        } else {
            report!(Level::Critical, "No entrypoint or workspace found in the AST.".into(), Some("SemanticAnalyzer".into()), None, None);
            return Err(());
        }

        self.task_returns.clear();
        self.infer_task_return_types(&root_clone);

        for node in &mut children {
            self.analyze_node(node)?;
        }

        let has_project_entrypoint = !entrypoint_ids.is_empty();
        if let Some(ws_node) = workspace_id {
            if !has_project_entrypoint {
                self.mark_symbol(&ws_node, |sym| sym.kind() == &SymbolKind::Workspace);
            }
            self.mark_workspace_members(&ws_node);
        }

        self.symbol_table.warn_unused_symbols();
        self.symbol_table.warn_hot_paths();
        Ok(())
    }

    // -------- Task return inference --------
    fn infer_task_return_types(&mut self, node: &AstNode<'_>) {
        match &node.kind {
            AstType::Task { name, .. } => {
                let ty = self.infer_return_type_in_task(node).unwrap_or(InferredType::Unit);
                self.task_returns.insert(name.to_string(), ty);
            }
            _ => {
                for c in &node.children {
                    self.infer_task_return_types(c);
                }
            }
        }
    }

    fn infer_return_type_in_task(&self, task_node: &AstNode<'_>) -> Option<InferredType> {
        let mut acc: Option<InferredType> = None;
        self.walk_returns(task_node, &mut |expr| {
            let t = self.infer_expr_type(expr);
            acc = Some(match acc {
                None => t,
                Some(prev) => Self::unify(prev, t),
            });
        });
        acc
    }

    fn walk_returns<F: FnMut(&AstNode<'_>)>(&self, node: &AstNode<'_>, f: &mut F) {
        match &node.kind {
            AstType::Return => {
                if let Some(expr) = node.children.get(0) {
                    f(expr);
                } else {
                    // return; with no value => Unit
                }
            }
            _ => {
                for c in &node.children {
                    self.walk_returns(c, f);
                }
            }
        }
    }

    fn unify(a: InferredType, b: InferredType) -> InferredType {
        use InferredType::*;
        match (a, b) {
            (Unknown, t) | (t, Unknown) => t,
            (Unit, t) | (t, Unit) => t,
            (Int, Int) => Int,
            (Bool, Bool) => Bool,
            (Str, Str) => Str,
            (Array, Array) => Array,
            _ => Unknown,
        }
    }

    // -------- Expression / Call handling --------
    fn is_stage_name(&self, name: &str) -> bool {
        if let Some(syms) = self.symbol_table.get(name) {
            syms.iter().any(|s| s.kind() == &SymbolKind::Stage)
        } else {
            false
        }
    }
    fn is_task_name(&self, name: &str) -> bool {
        if let Some(syms) = self.symbol_table.get(name) {
            syms.iter().any(|s| s.kind() == &SymbolKind::Task)
        } else {
            false
        }
    }

    fn infer_expr_type(&self, node: &AstNode<'_>) -> InferredType {
        match &node.kind {
            AstType::Number { .. } => InferredType::Int,
            AstType::Boolean { .. } => InferredType::Bool,
            AstType::String { .. } => InferredType::Str,
            AstType::Array => InferredType::Array,
            AstType::Null => InferredType::Unit,
            AstType::CallExpression { .. } => self.infer_call_expr_type(node).unwrap_or(InferredType::Unknown),
            AstType::Identifier { name } => {
                if let Some(syms) = self.symbol_table.get(name) {
                    match syms[0].symbol_type() {
                        SymbolType::Integer => InferredType::Int,
                        SymbolType::Boolean => InferredType::Bool,
                        SymbolType::String => InferredType::Str,
                        SymbolType::Array => InferredType::Array,
                        SymbolType::None => InferredType::Unit,
                        _ => InferredType::Unknown,
                    }
                } else {
                    InferredType::Unknown
                }
            }
            _ => InferredType::Unknown,
        }
    }

    fn infer_call_expr_type(&self, node: &AstNode<'_>) -> Option<InferredType> {
        let (callee, _args) = match &node.kind {
            AstType::CallExpression { callee, args } => (callee, args),
            _ => return None,
        };
        let name = match &callee.kind {
            AstType::Identifier { name } => name.as_ref(),
            _ => return Some(InferredType::Unknown),
        };
        if self.is_builtin(name) {
            return Some(InferredType::Unit);
        }
        if self.is_stage_name(name) {
            return Some(InferredType::Unit);
        }
        if self.is_task_name(name) {
            if let Some(rt) = self.task_returns.get(name) {
                return Some(*rt);
            }
        }
        None
    }

    fn analyze_expression(&mut self, node: &AstNode<'a>) -> Result<(), ()> {
        match &node.kind {
            AstType::CallExpression { .. } => self.analyze_call(node, true),
            AstType::Identifier { name } => {
                if !self.symbol_table.exists(name) {
                    report!(
                        Level::Error,
                        format!("Undefined identifier: {}", name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                } else if let Some(symbols) = self.symbol_table.get_mut(name) {
                    symbols[0].increment_reference_count();
                }
                Ok(())
            }
            AstType::Array => {
                for e in &node.children {
                    self.analyze_expression(e)?;
                }
                Ok(())
            }
            AstType::Return => {
                if let Some(expr) = node.children.get(0) {
                    self.analyze_expression(expr)?;
                }
                Ok(())
            }
            AstType::String { value } => {
                if value.is_empty() {
                    report!(Level::Error, "String literal cannot be empty.".into(),
                        Some("SemanticAnalyzer".into()), node.span.clone(), node.location.clone());
                    return Err(());
                }
                Ok(())
            }
            AstType::Number { value } => {
                if !value.is_finite() {
                    report!(Level::Error, format!("Invalid number: {}", value).into(),
                        Some("SemanticAnalyzer".into()), node.span.clone(), node.location.clone());
                    return Err(());
                }
                Ok(())
            }
            AstType::Boolean { .. } | AstType::Null | AstType::ShellCommand { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    fn infer_type(&self, node: &AstNode) -> Result<SymbolType, ()> {
        match &node.kind {
            AstType::CallExpression { .. } => {
                let it = self.infer_call_expr_type(node).unwrap_or(InferredType::Unknown);
                match it {
                    InferredType::Unknown | InferredType::Unit => {
                        // Defer error: treat as None (procedural) so assignment still parses.
                        Ok(SymbolType::None)
                    }
                    _ => Ok(it.to_symbol_type()),
                }
            }
            AstType::Return => {
                if let Some(expr) = node.children.get(0) {
                    self.infer_type(expr)
                } else {
                    Ok(SymbolType::None)
                }
            }
            // keep existing branches
            AstType::Number { .. } => Ok(SymbolType::Integer),
            AstType::Boolean { .. } => Ok(SymbolType::Boolean),
            AstType::String { .. } => Ok(SymbolType::String),
            AstType::Array => {
                // unchanged array element uniformity check
                if node.children.is_empty() {
                    Ok(SymbolType::Array)
                } else {
                    let first = self.infer_type(&node.children[0])?;
                    for e in node.children.iter().skip(1) {
                        let t = self.infer_type(e)?;
                        if t != first {
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
                    Ok(SymbolType::Array)
                }
            }
            AstType::Identifier { name } => {
                if let Some(syms) = self.symbol_table.get(name) {
                    Ok(syms[0].symbol_type().clone())
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
            AstType::Null => Ok(SymbolType::None),
            _ => {
                // existing fallback
                report!(
                    Level::Error,
                    format!("Unable to infer type for node: {:?}", node.kind),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                Err(())
            }
        }
    }

    // Insert / process a top-level or nested node
    fn analyze_node(&mut self, node: &mut AstNode<'a>) -> Result<(), ()> {
        match &node.kind {
            AstType::Workspace { name } => {
                let sym = Symbol::new_workspace(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(sym)?;
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
            AstType::Project { name } => {
                let sym = Symbol::new_project(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(sym)?;
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
            AstType::Stage { name, .. } => {
                let sym = Symbol::new_stage(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(sym)?;
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
            AstType::Task { name, .. } => {
                let sym = Symbol::new_task(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(sym)?;
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
            AstType::Assignment => {
                if node.children.len() >= 2 {
                    if let AstType::Identifier { name } = &node.children[0].kind {
                        let ty = self.infer_type(&node.children[1]).unwrap_or(SymbolType::None);
                        let var_sym = Symbol::new_variable(name.to_string().into(), ty, SymbolScope::Global);
                        let _ = self.symbol_table.insert(var_sym);
                        // RHS expression => expression context
                        self.analyze_expression(&node.children[1])?;
                    }
                }
            }
            AstType::CallExpression { .. } => {
                // Statement context
                self.analyze_call(node, false)?;
            }
            AstType::Return => {
                if let Some(expr) = node.children.get(0) {
                    self.analyze_expression(expr)?;
                }
            }
            AstType::Identifier { .. }
            | AstType::String { .. }
            | AstType::Number { .. }
            | AstType::Boolean { .. }
            | AstType::Array
            | AstType::Null
            | AstType::ShellCommand { .. } => {
                self.analyze_expression(node)?;
            }
            AstType::Include { .. } | AstType::Import { .. } => {
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
            _ => {
                for c in &mut node.children {
                    self.analyze_node(c)?;
                }
            }
        }
        Ok(())
    }

    // New helper: in_expression determines whether stage usage is illegal
    fn analyze_call(&mut self, node: &AstNode<'a>, in_expression: bool) -> Result<(), ()> {
        let (callee, args) = match &node.kind {
            AstType::CallExpression { callee, args } => (callee, args),
            _ => return Ok(()),
        };
        let name = match &callee.kind {
            AstType::Identifier { name } => name.as_ref(),
            _ => {
                report!(Level::Error, "Callee must be identifier".into(),
                    Some("SemanticAnalyzer".into()), node.span.clone(), node.location.clone());
                return Err(());
            }
        };

        // Builtins
        if self.is_builtin(name) {
            // Basic arity check for say(x)
            if name == "say" && args.len() != 1 {
                report!(
                    Level::Error,
                    "say expects exactly 1 argument".into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            for a in args { self.analyze_expression(a)?; }
            return Ok(());
        }

        // Stage: allowed only as statement (side-effect), not as value
        if self.is_stage_name(name) {
            if in_expression {
                report!(
                    Level::Error,
                    format!("Stage '{}' does not return a value; cannot be used in expression.", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
        } else if self.is_task_name(name) {
            // Task: ensure return type inferred (already done); OK in both contexts
            if let Some(rt) = self.task_returns.get(name) {
                if in_expression && matches!(rt, InferredType::Unit | InferredType::Unknown) {
                    report!(
                        Level::Error,
                        format!("Task '{}' has no usable return value.", name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
            }
        } else if !self.symbol_table.exists(name) {
            report!(
                Level::Error,
                format!("Unknown callable '{}'", name),
                Some("SemanticAnalyzer".into()),
                node.span.clone(),
                node.location.clone()
            );
            return Err(());
        }

        // Analyze args (always expression context)
        for a in args {
            self.analyze_expression(a)?;
        }
        Ok(())
    }

    // Mark symbols of a given node that satisfy predicate
    fn mark_symbol<F>(&mut self, node: &AstNode<'a>, pred: F)
    where
        F: Fn(&Symbol<'a>) -> bool,
    {
        let name_opt = match &node.kind {
            AstType::Workspace { name }
            | AstType::Project { name }
            | AstType::Stage { name, .. }
            | AstType::Task { name, .. } => Some(name.as_ref()),
            _ => None,
        };
        if let Some(name) = name_opt {
            if let Some(vec) = self.symbol_table.get_mut(name) {
                for s in vec {
                    if pred(s) {
                        s.increment_reference_count();
                    }
                }
            }
        }
    }

    // Mark projects referenced by workspace.members = [ ... ]
    fn mark_workspace_members(&mut self, workspace_node: &AstNode<'a>) {
        for child in &workspace_node.children {
            // Expect Assignment with key 'members'
            if let AstType::Assignment = child.kind {
                if child.children.len() >= 2 {
                    if let AstType::Identifier { name: lhs } = &child.children[0].kind {
                        if lhs == "members" {
                            self.collect_member_projects(&child.children[1]);
                        }
                    }
                }
            }
        }
    }

    fn collect_member_projects(&mut self, node: &AstNode<'a>) {
        match &node.kind {
            AstType::Identifier { name } => {
                if let Some(vec) = self.symbol_table.get_mut(name) {
                    for s in vec {
                        if s.kind() == &SymbolKind::Project {
                            s.increment_reference_count();
                        }
                    }
                }
            }
            _ => {
                for c in &node.children {
                    self.collect_member_projects(c);
                }
            }
        }
    }
}
