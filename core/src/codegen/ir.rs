use std::collections::HashMap;
use crate::reports::locations::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum IRConst {
    Int(i64),
    Str(String),
    Bool(bool),
    Array(Vec<IRConst>),
    Ident(String),
    Command(String),
    Null,
}

pub type Label = u32;

#[derive(Debug, Clone)]
pub enum IROpKind {
    LoadConst(u32),      // const pool index
    LoadVar(u32),        // symbol id
    StoreVar(u32),       // symbol id
    StoreGlobal(u32),    // symbol id
    Add, Sub, Mul, Div,
    Concat,
    Jump(Label),
    JumpIfFalse(Label),
    Call(u32, u8),       // function id, argc
    Return,
    Say, 
    Read, 
    Write,
    NoOp,
}

#[derive(Debug, Clone)]
pub struct IROp {
    pub kind: IROpKind,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: Label,
    pub ops: Vec<IROp>,
    pub next: Vec<Label>,
}

#[derive(Debug, Clone)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<u32>,      // symbol ids
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug)]
pub struct ModuleIR {
    pub globals: Vec<String>,
    pub const_pool: Vec<IRConst>,
    pub functions: Vec<IRFunction>,
    pub func_index: HashMap<String, u32>,       // full name
    pub plain_index: HashMap<String, u32>,      // plain (stage/task) name
}

impl ModuleIR {
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
            const_pool: Vec::new(),
            functions: Vec::new(),
            func_index: HashMap::new(),
            plain_index: HashMap::new(),
        }
    }

    pub fn intern_const(&mut self, c: IRConst) -> u32 {
        if let Some((i, _)) = self.const_pool.iter().enumerate().find(|(_, v)| **v == c) {
            i as u32
        } else {
            let idx = self.const_pool.len() as u32;
            self.const_pool.push(c);
            idx
        }
    }

    pub fn intern_global(&mut self, name: String) -> u32 {
        if let Some((i, _)) = self.globals.iter().enumerate().find(|(_, v)| **v == name) {
            i as u32
        } else {
            let idx = self.globals.len() as u32;
            self.globals.push(name);
            idx
        }
    }

    pub fn add_function(&mut self, f: IRFunction) -> u32 {
        let id = self.functions.len() as u32;
        self.func_index.insert(f.name.clone(), id);
        if let Some(rest) = f.name.strip_prefix("stage:init:") {
            self.plain_index.insert(rest.to_string(), id);
        } else if let Some(rest) = f.name.strip_prefix("task:init:") {
            self.plain_index.insert(rest.to_string(), id);
        }
        self.functions.push(f);
        id
    }

    pub fn get_plain_func(&self, name: &str) -> Option<u32> {
        self.plain_index.get(name).copied()
    }
}