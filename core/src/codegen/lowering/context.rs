use crate::codegen::op::Op;
use crate::codegen::slot::Slot;
use std::collections::{HashMap, HashSet};
use crate::parser::{AstNode, AstType};

#[derive(Debug)]
pub struct LowerCtx {
    pub ops: Vec<Op>,
    pub(crate) frames: Vec<HashMap<String, Slot>>,
    pub(crate) next_local: usize,
    pub(crate) next_temp: usize,

    // scopes
    pub(crate) pending_scopes: Vec<(String, AstNode<'static>)>,
    pub(crate) scope_names: HashSet<String>,
    pub(crate) current_scope: Option<String>,
    pub(crate) initialized_members: HashMap<String, HashSet<String>>,
    pub(crate) called_scopes: HashSet<String>,
    pub(crate) param_names: HashMap<String, Vec<String>>,
    pub(crate) param_slots: HashMap<String, Vec<Slot>>,

    pub(crate) entry: String,
    prev_next_local: Vec<usize>,
}

impl LowerCtx {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            frames: vec![HashMap::new()],
            next_local: 0,
            next_temp: 0,
            pending_scopes: Vec::new(),
            scope_names: HashSet::new(),
            current_scope: None,
            initialized_members: HashMap::new(),
            called_scopes: HashSet::new(),
            param_names: HashMap::new(),
            param_slots: HashMap::new(),
            entry: String::new(),
            prev_next_local: Vec::new(),
        }
    }

    pub fn emit(&mut self, op: Op) { self.ops.push(op); }
    pub fn temp(&mut self) -> Slot { let s = Slot::Temp(self.next_temp); self.next_temp += 1; s }

    pub fn ensure_local(&mut self, name: &str) -> Slot {
        if let Some(s) = self.lookup_local(name) { return s; }
        let idx = self.next_local;
        self.next_local += 1;
        let slot = Slot::Local(idx);
        self.frames.last_mut().unwrap().insert(name.to_string(), slot);
        slot
    }

    pub fn lookup_local(&self, name: &str) -> Option<Slot> {
        for f in self.frames.iter().rev() {
            if let Some(s) = f.get(name) { return Some(*s); }
        }
        None
    }

    // Unified frame push (optionally reset locals)
    pub fn push_frame(&mut self, reset_locals: bool) {
        if reset_locals {
            self.prev_next_local.push(self.next_local);
            self.next_local = 0;
        }
        self.frames.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        let _ = self.frames.pop();
        if let Some(prev) = self.prev_next_local.pop() {
            self.next_local = prev;
        }
    }

    pub fn has_called_scope(&self, name: &str) -> bool { self.called_scopes.contains(name) }
    pub fn note_scope_call(&mut self, name: &str) { self.called_scopes.insert(name.to_string()); }

    pub fn is_member_initialized(&self, scope: &str, member: &str) -> bool {
        self.initialized_members
            .get(scope)
            .map_or(false, |s| s.contains(member))
    }

    pub fn note_member_init(&mut self, scope: &str, member: &str) {
        self.initialized_members
            .entry(scope.to_string())
            .or_default()
            .insert(member.to_string());
    }

    // Scope insertion helper
    fn insert_scope(&mut self, name: &str, params: Option<&[AstNode]>, node: &AstNode) {
        if self.scope_names.insert(name.to_string()) {
            if let Some(pv) = params {
                let collected = pv.iter().filter_map(|p| {
                    if let AstType::Identifier { name } = &p.kind {
                        Some(name.as_ref().to_string())
                    } else { None }
                }).collect::<Vec<_>>();
                self.param_names.insert(name.to_string(), collected);
            }
            self.pending_scopes.push((name.to_string(), node.clone().into_lifetime()));
        }
    }

    pub fn record_scope(&mut self, node: &AstNode) {
        match &node.kind {
            AstType::Workspace { name } => self.insert_scope(name.as_ref(), None, node),
            AstType::Project { name } => self.insert_scope(name.as_ref(), None, node),
            AstType::Stage { name, params, .. } => self.insert_scope(name.as_ref(), Some(params), node),
            AstType::Task { name, params, .. } => self.insert_scope(name.as_ref(), Some(params), node),
            _ => {}
        }
    }

    // Allocate param slots at region entry
    pub fn allocate_param_slots(&mut self, scope_name: &str) {
        if self.param_slots.contains_key(scope_name) { return; }
        let mut slots = Vec::new();
        if let Some(names) = self.param_names.get(scope_name) {
            let names: Vec<String> = names.iter().cloned().collect();
            for pname in names {
                let s = self.ensure_local(&pname);
                slots.push(s);
            }
        }
        self.param_slots.insert(scope_name.to_string(), slots);
    }

    fn begin_scope_region(&mut self, name: &str) {
        self.current_scope = Some(name.to_string());
        self.initialized_members.clear();
        self.called_scopes.clear();
        self.emit(Op::Label { name: format!("func.{}", name) });
        self.emit(Op::Label { name: format!("scope.{}", name) });
        self.push_frame(true);
        self.allocate_param_slots(name);
    }

    fn end_scope_region(&mut self, name: &str) {
        // Optional implicit return of 'out' if initialized:
        let ret_slot = if self.is_member_initialized(name, "out") {
            self.lookup_local("out")
        } else {
            None
        };
        self.pop_frame();
        self.emit(Op::Return { value: ret_slot });
        self.current_scope = None;
    }

    pub fn emit_scope_regions<F>(&mut self, mut lower_stmt_entry: F)
    where
        F: FnMut(&mut LowerCtx, &AstNode),
    {
        for (name, node) in std::mem::take(&mut self.pending_scopes) {
            self.begin_scope_region(&name);
            lower_stmt_entry(self, &node);
            self.end_scope_region(&name);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRMeta { pub max_temp: usize, pub max_local: usize }

pub fn analyze_meta(ops: &[Op]) -> IRMeta {
    let mut max_temp = 0;
    let mut max_local = 0;
    let mut visit = |s: &Slot| match s {
        Slot::Temp(i) => max_temp = max_temp.max(*i),
        Slot::Local(i) => max_local = max_local.max(*i),
        Slot::Captured(_) => {}
    };
    for op in ops {
        use Op::*;
        match op {
            LoadConst { target, .. } => visit(target),
            LoadLocal { target, source } => { visit(target); visit(source); }
            StoreLocal { source, target } => { visit(source); visit(target); }
            LoadGlobal { target, .. } => visit(target),
            StoreGlobal { source, .. } => visit(source),
            NewArray { target, .. } => visit(target),
            ISet { target, index, value } => { visit(target); visit(index); visit(value); }
            IGet { target, source, index } => { visit(target); visit(source); visit(index); }
            Length { target, array } => { visit(target); visit(array); }
            MGet { target, source, .. } => { visit(target); visit(source); }
            MSet { target, value, .. } => { visit(target); visit(value); }
            Add { lhs, rhs, target }
            | Sub { lhs, rhs, target }
            | Mul { lhs, rhs, target }
            | Div { lhs, rhs, target }
            | Eq { lhs, rhs, target }
            | Ne { lhs, rhs, target }
            | Lt { lhs, rhs, target }
            | Le { lhs, rhs, target }
            | Gt { lhs, rhs, target }
            | Ge { lhs, rhs, target } => { visit(lhs); visit(rhs); visit(target); }
            Not { source, target } => { visit(source); visit(target); }
            Say { message } => visit(message),
            Ask { question, target } => { visit(question); visit(target); }
            Read { location, target } => { visit(location); visit(target); }
            Write { location, target } => { visit(location); visit(target); }
            Call { target, func, args } => { visit(target); visit(func); for a in args { visit(a); } }
            BrTrue { condition, .. } | BrFalse { condition, .. } => visit(condition),
            Return { value } => { if let Some(v) = value { visit(v); } }
            Label { .. } | Jump { .. } | Halt => {}
            Inc { target } | Dec { target } => visit(target),
        }
    }
    IRMeta { max_temp: max_temp + 1, max_local: max_local + 1 }
}
