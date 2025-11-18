//! Root analyzer module: orchestrates submodules.
use crate::parser::*;
use crate::semantic::types::InferredType;
use crate::semantic::{SymbolKind, SymbolTable};
use crate::semantic::builtin::Builtins;
use std::collections::{HashMap, HashSet};

mod entrypoint;
mod predeclare;
mod tasks;
mod calls;
mod expressions;
mod diagnostics;
mod assign;
mod util;

pub struct SemanticAnalyzer<'a> {
    ast: AstParser,
    pub symbol_table: SymbolTable<'a>,
    pub entry_point: AstNode<'a>,
    pub(crate) task_returns: HashMap<String, InferredType>,
    pub(crate) scope_members: HashMap<(SymbolKind, String), HashMap<String, InferredType>>,
    init_stack: Vec<HashSet<(SymbolKind, String)>>,

    builtins: Builtins,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(ast: AstParser) -> Result<Self, ()> {
        let mut analyzer = Self {
            entry_point: ast.root().clone(),
            ast: ast.clone(),
            symbol_table: SymbolTable::new(),
            task_returns: Default::default(),
            scope_members: Default::default(),
            init_stack: vec![],
            builtins: Builtins::new(),
        };
        analyzer.predeclare_top_level();
        analyzer.predeclare_scope_members(); // generic member discovery
        analyzer.run()?;
        diagnostics::warn_empty_bodies(analyzer.ast.root());
        Ok(analyzer)
    }

    fn run(&mut self) -> Result<(), ()> {
        self.entry_point = self.detect_entrypoint()?;
        self.task_returns.clear();
        self.collect_task_return_types();
        self.enter_frame();
        self.walk_root_nodes()?;
        self.exit_frame();
        self.symbol_table.warn_unused_symbols();
        self.symbol_table.warn_hot_paths();
        Ok(())
    }

    pub(crate) fn is_builtin(&self, name: &str) -> bool { self.builtins.is(name) }
    pub(crate) fn is_value_builtin(&self, name: &str) -> bool { self.builtins.returns_value(name) }
    pub(crate) fn is_stage_name(&self, name: &str) -> bool { util::is_kind(&self.symbol_table, name, SymbolKind::Stage) }
    pub(crate) fn is_task_name(&self, name: &str) -> bool { util::is_kind(&self.symbol_table, name, SymbolKind::Task) }

    fn walk_root_nodes(&mut self) -> Result<(), ()> {
        for node in &mut self.ast.root.clone().children {
            expressions::analyze_node(self, node)?;
        }
        Ok(())
    }

    fn collect_task_return_types(&mut self) {
        tasks::infer_task_returns(self);
    }

    // Frame management for init tracking
    pub(crate) fn enter_frame(&mut self) {
        self.init_stack.push(HashSet::new());
    }
    pub(crate) fn exit_frame(&mut self) {
        self.init_stack.pop();
    }
    pub(crate) fn mark_scope_initialized(&mut self, kind: SymbolKind, name: &str) {
        if let Some(top) = self.init_stack.last_mut() {
            top.insert((kind, name.to_string()));
        }
    }
    pub(crate) fn is_scope_initialized(&self, kind: SymbolKind, name: &str) -> bool {
        for frame in self.init_stack.iter().rev() {
            if frame.contains(&(kind, name.to_string())) {
                return true;
            }
        }
        false
    }
}