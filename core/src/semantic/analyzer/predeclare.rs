use crate::parser::{AstType, AstNode};
use crate::semantic::{Symbol, SymbolScope, SymbolKind};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::types::InferredType;
use std::collections::HashMap;

impl<'a> SemanticAnalyzer<'a> {
    pub(crate) fn predeclare_top_level(&mut self) {
        let root = self.ast.root().clone();
        for child in &root.children {
            match &child.kind {
                AstType::Workspace { name } =>
                    self.insert_unique(Symbol::new_workspace(name.to_string().into(), SymbolScope::Global)),
                AstType::Project { name } =>
                    self.insert_unique(Symbol::new_project(name.to_string().into(), SymbolScope::Global)),
                AstType::Stage { name, .. } =>
                    self.insert_unique(Symbol::new_stage(name.to_string().into(), SymbolScope::Global)),
                AstType::Task { name, .. } =>
                    self.insert_unique(Symbol::new_task(name.to_string().into(), SymbolScope::Global)),
                _ => {}
            }
        }

        // NEW: bump reference count on the selected entrypoint (attribute first, fallback to first workspace)
        if let Ok(entry) = self.detect_entrypoint() {
            let name = match &entry.kind {
                AstType::Workspace { name }
                | AstType::Project { name }
                | AstType::Stage { name, .. }
                | AstType::Task { name, .. } => name.as_ref(),
                _ => return,
            };
            if let Some(vec) = self.symbol_table.get_mut(name) {
                for s in vec { s.increment_reference_count(); }
            }
        }
    }

    fn insert_unique(&mut self, sym: Symbol<'a>) {
        if !self.symbol_table.exists(sym.name()) {
            let _ = self.symbol_table.insert(sym);
        }
    }

    // Generic: collect members for all scopes
    pub(crate) fn predeclare_scope_members(&mut self) {
        fn collect_members<'b>(node: &AstNode<'b>, acc: &mut HashMap<String, InferredType>) {
            use crate::parser::types::AstType::*;
            match &node.kind {
                Block => {
                    for c in &node.children { collect_members(c, acc); }
                }
                Assignment => {
                    if node.children.len() >= 2 {
                        if let Identifier { name } = &node.children[0].kind {
                            acc.entry(name.to_string()).or_insert(InferredType::Unknown);
                        }
                    }
                }
                _ => {
                    for c in &node.children { collect_members(c, acc); }
                }
            }
        }

        let mut scope_members: HashMap<(SymbolKind, String), HashMap<String, InferredType>> = HashMap::new();
        let root = self.ast.root().clone();
        for child in &root.children {
            match &child.kind {
                AstType::Workspace { name } => {
                    let mut acc = HashMap::new();
                    collect_members(child, &mut acc);
                    scope_members.insert((SymbolKind::Workspace, name.to_string()), acc);
                }
                AstType::Project { name } => {
                    let mut acc = HashMap::new();
                    collect_members(child, &mut acc);
                    scope_members.insert((SymbolKind::Project, name.to_string()), acc);
                }
                AstType::Stage { name, .. } => {
                    let mut acc = HashMap::new();
                    collect_members(child, &mut acc);
                    scope_members.insert((SymbolKind::Stage, name.to_string()), acc);
                }
                AstType::Task { name, .. } => {
                    let mut acc = HashMap::new();
                    collect_members(child, &mut acc);
                    scope_members.insert((SymbolKind::Task, name.to_string()), acc);
                }
                _ => {}
            }
        }
        self.scope_members = scope_members;
    }
}