//! ./semantic/analyzer.rs
//!
//! Semantic analyzer for the Mainstage programming language.

use crate::parser::*;
use crate::report;
use crate::reports::*;
use crate::semantic::{Symbol, SymbolKind, SymbolScope, SymbolTable};
use crate::semantic::types::{InferredType, SymbolType};
use crate::semantic::inference as infer;
use std::collections::{HashMap, HashSet};

pub struct SemanticAnalyzer<'a> {
    ast: AstParser,
    pub symbol_table: SymbolTable<'a>,
    pub entry_point: AstNode<'a>,
    pub(crate) task_returns: HashMap<String, InferredType>,
    builtin_funcs: HashSet<&'static str>,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(ast: AstParser) -> Result<Self, ()> {
        let mut analyzer = Self {
            ast: ast.clone(),
            symbol_table: SymbolTable::new(),
            entry_point: ast.root().clone(),
            task_returns: HashMap::new(),
            builtin_funcs: ["say", "ask", "read", "write"].into_iter().collect(),
        };
        analyzer.analyze()?;
        Ok(analyzer)
    }

    pub(crate) fn is_builtin(&self, name: &str) -> bool {
        self.builtin_funcs.contains(name)
    }
    pub(crate) fn is_value_builtin(&self, name: &str) -> bool {
        matches!(name, "read" | "ask")
    }

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
                    if has_entrypoint_attr(node) { entrypoint_ids.push(node.clone()); }
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
            _ => for c in &node.children { self.infer_task_return_types(c); }
        }
    }

    fn infer_return_type_in_task(&self, task_node: &AstNode<'_>) -> Option<InferredType> {
        let mut acc: Option<InferredType> = None;
        self.walk_returns(task_node, &mut |expr| {
            let t = infer::infer_expr_type(self, expr);
            acc = Some(match acc { None => t, Some(prev) => infer::unify(prev, t) });
        });
        acc
    }

    fn walk_returns<F: FnMut(&AstNode<'_>)>(&self, node: &AstNode<'_>, f: &mut F) {
        match &node.kind {
            AstType::Return => { if let Some(expr) = node.children.get(0) { f(expr); } }
            _ => for c in &node.children { self.walk_returns(c, f); }
        }
    }

    // -------- Expression / Call handling --------
    pub(crate) fn is_stage_name(&self, name: &str) -> bool {
        if let Some(syms) = self.symbol_table.get(name) {
            syms.iter().any(|s| s.kind() == &SymbolKind::Stage)
        } else { false }
    }
    pub(crate) fn is_task_name(&self, name: &str) -> bool {
        if let Some(syms) = self.symbol_table.get(name) {
            syms.iter().any(|s| s.kind() == &SymbolKind::Task)
        } else { false }
    }

    fn infer_type(&self, node: &AstNode) -> Result<SymbolType, ()> {
        infer::infer_type(self, node)
    }

    fn analyze_expression(&mut self, node: &AstNode<'a>) -> Result<(), ()> {
        match &node.kind {
            AstType::CallExpression { .. } => self.analyze_call(node, true),
            AstType::MemberAccess { target, member } => {
                // If target is a stage call, allow it (no “cannot be used” error)
                if let AstType::CallExpression { target , .. } = &target.kind {
                    if let AstType::Identifier { name: stage_name } = &target.kind {
                        if self.is_stage_name(stage_name) {
                            // Mark stage referenced
                            if let Some(vec) = self.symbol_table.get_mut(stage_name) {
                                for s in vec {
                                    if s.kind() == &SymbolKind::Stage {
                                        s.increment_reference_count();
                                    }
                                }
                            }
                            // Analyze call args
                            if let AstType::CallExpression { arguments, .. } = &target.kind {
                                for a in arguments {
                                    self.analyze_expression(a)?;
                                }
                            }
                        }
                    }
                } else {
                    // Normal target expression
                    self.analyze_expression(target)?;
                }

                // Member identifier reference (variable)
                if let AstType::Identifier { name: field } = &member.kind {
                    if let Some(vec) = self.symbol_table.get_mut(field) {
                        for s in vec {
                            if s.kind() == &SymbolKind::Variable {
                                s.increment_reference_count();
                            }
                        }
                    }
                }
                Ok(())
            }
            AstType::Identifier { name } => {
                if crate::semantic::symbol::RESERVED_WORKSPACE_MEMBERS.contains(&name.as_ref())
                    || crate::semantic::symbol::RESERVED_PROJECT_MEMBERS.contains(&name.as_ref())
                    || crate::semantic::symbol::RESERVED_STAGE_MEMBERS.contains(&name.as_ref())
                    || crate::semantic::symbol::RESERVED_TASK_MEMBERS.contains(&name.as_ref())
                {
                    if let Some(vec) = self.symbol_table.get_mut(name) {
                        for s in vec { s.increment_reference_count(); }
                    }
                    return Ok(());
                }

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
            AstType::Index { target, index } => {
                // Analyze both parts
                self.analyze_expression(target)?;
                self.analyze_expression(index)?;
                Ok(())
            }
            AstType::BinaryOp { left, right, .. } => {
                // Analyze both operands
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
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
                    report!(
                        Level::Error,
                        "String literal cannot be empty.".into(),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
                Ok(())
            }
            AstType::Number { value } => {
                if !value.is_finite() {
                    report!(
                        Level::Error,
                        format!("Invalid number: {}", value).into(),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                }
                Ok(())
            }
            AstType::Boolean { .. } | AstType::Null | AstType::ShellCommand { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    // Insert / process a top-level or nested node
    fn analyze_node(&mut self, node: &mut AstNode<'a>) -> Result<(), ()> {
        match &node.kind {
            AstType::Workspace { name } => {
                let sym = Symbol::new_workspace(name.to_string().into(), SymbolScope::Global);
                self.symbol_table.insert(sym)?;
                for m in crate::semantic::symbol::RESERVED_WORKSPACE_MEMBERS {
                    self.insert_reserved_member(m);
                }
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
                        let rhs = &node.children[1];

                        // If assigning to a reserved member, do NOT attempt to insert a new symbol.
                        let is_reserved = crate::semantic::symbol::RESERVED_WORKSPACE_MEMBERS.contains(&name.as_ref())
                            || crate::semantic::symbol::RESERVED_PROJECT_MEMBERS.contains(&name.as_ref())
                            || crate::semantic::symbol::RESERVED_STAGE_MEMBERS.contains(&name.as_ref())
                            || crate::semantic::symbol::RESERVED_TASK_MEMBERS.contains(&name.as_ref());

                        if is_reserved {
                            // Analyze RHS expression only.
                            self.analyze_expression(rhs)?;
                            // Optionally update type of existing reserved symbol if still None.
                            let inferred = self.infer_type(rhs).unwrap_or(SymbolType::None);
                            if let Some(vec) = self.symbol_table.get_mut(name) {
                                for s in vec {
                                    if s.symbol_type() == &SymbolType::None {
                                        s.set_symbol_type(inferred.clone());
                                    }
                                    s.increment_reference_count(); // write counts as use
                                }
                            }
                        } else {
                            let ty = self.infer_type(rhs).unwrap_or(SymbolType::None);
                            let var_sym = Symbol::new_variable(name.to_string().into(), ty, SymbolScope::Global);
                            // Ignore duplicate error silently (do not emit duplicate for re-assignment).
                            let _ = self.symbol_table.insert(var_sym);
                            self.analyze_expression(rhs)?;
                            // Mark variable referenced (write)
                            if let Some(vec) = self.symbol_table.get_mut(name) {
                                for s in vec {
                                    if s.kind() == &SymbolKind::Variable {
                                        s.increment_reference_count();
                                    }
                                }
                            }
                        }
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
        let (target, arguments) = match &node.kind {
            AstType::CallExpression { target, arguments } => (target, arguments),
            _ => return Ok(()),
        };

        let name = match &target.kind {
            AstType::Identifier { name } => name.as_ref(),
            _ => return Ok(()),
        };

        // Builtins (say, ask, read, write...)
        if self.is_builtin(name) {
            // Arity checks (keep existing ones)
            if name == "say" && arguments.len() != 1 {
                report!(
                    Level::Error,
                    "say expects exactly 1 argument".into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            if name == "ask" && arguments.len() > 1 {
                report!(
                    Level::Error,
                    "ask expects 0 or 1 argument".into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            if name == "read" && arguments.len() != 1 {
                report!(
                    Level::Error,
                    "read expects exactly 1 argument".into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            if name == "write" && arguments.len() != 2 {
                report!(
                    Level::Error,
                    "write expects exactly 2 arguments".into(),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            for a in arguments {
                self.analyze_expression(a)?;
            }

            // NEW: warn if value-returning builtin used as bare statement
            if !in_expression && self.is_value_builtin(name) {
                report!(
                    Level::Warning,
                    format!("Return value of builtin '{}' is discarded (not assigned).", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
            }
            return Ok(());
        }

        // Stage call: only valid as statement; no return value to discard
        if self.is_stage_name(name) {
            if in_expression {
                report!(
                    Level::Error,
                    format!("Stage '{}' returns no value; cannot be used in expression.", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            // reference counting
            if let Some(vec) = self.symbol_table.get_mut(name) {
                for s in vec {
                    if s.kind() == &SymbolKind::Stage {
                        s.increment_reference_count();
                    }
                }
            }
            for a in arguments {
                self.analyze_expression(a)?;
            }
            return Ok(());
        }

        // Task call
        if self.is_task_name(name) {
            // Mark task referenced
            if let Some(vec) = self.symbol_table.get_mut(name) {
                for s in vec {
                    if s.kind() == &SymbolKind::Task {
                        s.increment_reference_count();
                    }
                }
            }

            // Analyze arguments
            for a in arguments {
                self.analyze_expression(a)?;
            }

            // Determine return type
            let ret_ty = self.task_returns.get(name);
            let returns_value = matches!(ret_ty, Some(InferredType::Str)
                                      | Some(InferredType::Int)
                                      | Some(InferredType::Bool)
                                      | Some(InferredType::Array)
                                      | Some(InferredType::Unit));

            // If used as bare statement and returns a value, warn
            if !in_expression && returns_value {
                report!(
                    Level::Warning,
                    format!("Return value of task '{}' is discarded (not assigned).", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
            }

            return Ok(());
        }

        // Unknown callable
        report!(
            Level::Error,
            format!("Unknown callable '{}'", name),
            Some("SemanticAnalyzer".into()),
            node.span.clone(),
            node.location.clone()
        );
        Err(())
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

    // Mark projects referenced by workspace.projects = [ ... ]
    fn mark_workspace_members(&mut self, workspace_node: &AstNode<'a>) {
        for child in &workspace_node.children {
            if let AstType::Assignment = child.kind {
                if child.children.len() >= 2 {
                    if let AstType::Identifier { name: lhs } = &child.children[0].kind {
                        if lhs == "projects" {
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

    fn insert_reserved_member(&mut self, name: &str) {
        if !self.symbol_table.exists(name) {
            let sym = Symbol::new_variable(name.to_string().into(), SymbolType::Array, SymbolScope::Global)
                .with_reserved(); // ensure you have a builder or set a flag; if not, remove this line.
            let _ = self.symbol_table.insert(sym);
        }
        if let Some(vec) = self.symbol_table.get_mut(name) {
            for s in vec { s.increment_reference_count(); }
        }
    }
}
