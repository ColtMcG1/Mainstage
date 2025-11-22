//! Root analyzer module: orchestrates submodules.
use crate::parser::*;
use crate::semantic::types::InferredType;
use crate::semantic::{SymbolKind, SymbolTable};
use crate::semantic::builtin::*;
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
    init_stack: Vec<HashSet<(SymbolKind, String)>>
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

    pub(crate) fn is_builtin_method(&self, name: &str) -> bool {
        BUILTIN_METHODS.contains_key(name)
    }
    pub(crate) fn get_builtin_method(&self, name: &str) -> Option<&BuiltinMethod> {
        BUILTIN_METHODS.get(name)
    }
    pub(crate) fn is_builtin_function(&self, name: &str) -> bool {
        BUILTIN_FUNCS.contains_key(name)
    }
    pub(crate) fn get_builtin_function(&self, name: &str) -> Option<&BuiltinFunc> {
        BUILTIN_FUNCS.get(name)
    }

    pub(crate) fn is_stage_name(&self, name: &str) -> bool {
        self.symbol_table.contains_stage(name)
    }
    pub(crate) fn is_task_name(&self, name: &str) -> bool {
        self.symbol_table.contains_task(name)
    }

    fn walk_root_nodes(&mut self) -> Result<(), ()> {
        for node in &mut self.ast.root.clone().children {
            expressions::analyze_node(self, node)?;
        }
        Ok(())
    }

    fn collect_task_return_types(&mut self) {
        tasks::infer_task_returns(self);
        // NEW: infer stage returns (simple scan)
        self.task_returns.extend(self.infer_stage_returns());
    }

    fn infer_stage_returns(&self) -> HashMap<String, InferredType> {
        let mut map = HashMap::new();
        let root = self.ast.root();
        fn scan<'b>(node: &AstNode<'b>, current: &mut Option<String>, out: &mut HashMap<String, InferredType>) {
            match &node.kind {
                AstType::Stage { name, .. } => {
                    *current = Some(name.as_ref().to_string());
                }
                AstType::Return => {
                    if let Some(scope) = current.as_ref() {
                        let ty = if node.children.get(0).is_some() {
                            InferredType::Dynamic
                        } else {
                            InferredType::Unit
                        };
                        out.entry(scope.clone()).or_insert(ty);
                    }
                }
                _ => {}
            }
            for c in &node.children {
                let mut inner = current.clone();
                scan(c, &mut inner, out);
            }
        }
        scan(root, &mut None, &mut map);
        map
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