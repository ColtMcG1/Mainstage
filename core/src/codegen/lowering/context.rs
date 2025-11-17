use crate::codegen::op::Op;
use crate::codegen::slot::Slot;
use crate::parser::ast::AstNode;
use crate::parser::types::AstType;
use std::collections::{HashMap, HashSet};

pub struct LowerCtx {
    pub ops: Vec<Op>,
    pub(crate) frames: Vec<HashMap<String, Slot>>,
    pub(crate) next_local: usize,
    pub(crate) next_temp: usize,
    pub(crate) pending_scopes: Vec<(String, AstNode<'static>)>,
    pub(crate) scope_names: HashSet<String>,

    pub(crate) current_scope: Option<String>,
    pub(crate) initialized_members: HashMap<String, HashSet<String>>,

    // NEW: first-reference init tracking + entry
    pub(crate) called_scopes: HashSet<String>,
    pub(crate) entry_workspace: Option<String>,
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
            entry_workspace: None,
        }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }
    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }
    pub fn is_root(&self) -> bool {
        self.frames.len() == 1
    }

    pub fn ensure_local(&mut self, name: &str) -> Slot {
        let frame = self.frames.last_mut().unwrap();
        if let Some(s) = frame.get(name) {
            return *s;
        }
        let slot = Slot::Local(self.next_local);
        self.next_local += 1;
        frame.insert(name.to_string(), slot);
        slot
    }
    pub fn lookup_local(&self, name: &str) -> Option<Slot> {
        for f in self.frames.iter().rev() {
            if let Some(s) = f.get(name) {
                return Some(*s);
            }
        }
        None
    }
    pub fn temp(&mut self) -> Slot {
        let t = Slot::Temp(self.next_temp);
        self.next_temp += 1;
        t
    }
    pub fn emit(&mut self, op: Op) {
        self.ops.push(op);
    }

    pub fn has_called_scope(&self, name: &str) -> bool { self.called_scopes.contains(name) }
    pub fn note_scope_call(&mut self, name: &str) { self.called_scopes.insert(name.to_string()); }

    pub fn record_scope(&mut self, node: &AstNode) {
        let name = match &node.kind {
            AstType::Workspace { name } => {
                if self.entry_workspace.is_none() {
                    self.entry_workspace = Some(name.as_ref().to_string());
                }
                name.as_ref()
            }
            AstType::Project { name }
            | AstType::Stage { name, .. }
            | AstType::Task { name, .. } => name.as_ref(),
            _ => return,
        };
        if self.scope_names.insert(name.to_string()) {
            let owned: AstNode<'static> = unsafe { std::mem::transmute::<AstNode<'_>, AstNode<'static>>(node.clone()) };
            self.pending_scopes.push((name.to_string(), owned));
        }
    }

    pub fn note_member_init(&mut self, container: &str, member: &str) {
        self.initialized_members
            .entry(container.to_string())
            .or_default()
            .insert(member.to_string());
    }

    pub fn is_member_initialized(&self, container: &str, member: &str) -> bool {
        self.initialized_members
            .get(container)
            .map(|set| set.contains(member))
            .unwrap_or(false)
    }

    pub fn emit_scope_regions<F>(&mut self, mut lower_body: F)
    where
        F: FnMut(&mut LowerCtx, &AstNode),
    {
        let pending_scopes: Vec<_> = self.pending_scopes.drain(..).collect();
        for (name, node) in pending_scopes {
            // Save outer state
            let saved_scope = self.current_scope.clone();
            let saved_inits = std::mem::take(&mut self.initialized_members);
            let saved_called = std::mem::take(&mut self.called_scopes);

            self.current_scope = Some(name.clone());
            self.initialized_members = HashMap::new();
            self.called_scopes = HashSet::new();

            self.emit(Op::Label { name: format!("scope.{}", name) });
            self.push_frame();
            for child in &node.children {
                lower_body(self, child);
            }
            self.pop_frame();
            self.emit(Op::Return { value: None });

            // Restore outer state
            self.current_scope = saved_scope;
            self.initialized_members = saved_inits;
            self.called_scopes = saved_called;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRMeta {
    pub max_temp: usize,
    pub max_local: usize,
}

pub fn analyze_meta(ops: &[Op]) -> IRMeta {
    let mut max_temp = 0;
    let mut max_local = 0;
    let mut visit = |s: &Slot| {
        match s {
            Slot::Temp(i) => max_temp = max_temp.max(*i),
            Slot::Local(i) => max_local = max_local.max(*i),
            Slot::Captured(_) => {}
        }
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
            Label { .. } | Jump { .. } | CallScope { .. } => {}
        }
    }
    IRMeta { max_temp: max_temp + 1, max_local: max_local + 1 }
}
