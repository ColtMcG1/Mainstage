use crate::ir::slot::Slot;
use crate::ir::value::OpValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    // --- Stack <-> const/locals/globals ---

    /// Load a constant into a register
    LoadConst  { target: Slot, value: OpValue },
    /// Load a local variable into a register
    LoadLocal  { target: Slot, local: Slot },
    /// Store a register value into a local variable
    StoreLocal { source: Slot, target: Slot },
    /// Load a global variable into a register
    LoadGlobal  { target: Slot, global: String },
    /// Store a register value into a global variable
    StoreGlobal { source: Slot, global: String },

    // --- Object / Collection ---

    /// Create a new array
    NewArray { target: Slot, size: usize },
    /// Get the length of an array or collection
    Length     { target: Slot, array: Slot },
    /// Get an indexed value from an array or collection
    IGet { target: Slot, source: Slot, index: Slot },
    /// Set an indexed value in an array or collection
    ISet { target: Slot, index: Slot, value: Slot },
    /// Get a member value from an object or collection
    MGet { target: Slot, source: Slot, member: String },
    /// Set a member value in an object or collection
    MSet { target: Slot, member: String, value: Slot },

    // Arithmetic / Compare (binary: pop rhs, lhs; push result)

    /// Add two values
    Add { lhs: Slot, rhs: Slot, target: Slot },
    /// Subtract two values
    Sub { lhs: Slot, rhs: Slot, target: Slot },
    /// Multiply two values
    Mul { lhs: Slot, rhs: Slot, target: Slot },
    /// Divide two values
    Div { lhs: Slot, rhs: Slot, target: Slot },

    /// Is lhs equal to rhs?
    Eq { lhs: Slot, rhs: Slot, target: Slot },
    /// Is lhs not equal to rhs?
    Ne { lhs: Slot, rhs: Slot, target: Slot },
    /// Is lhs less than rhs?
    Lt { lhs: Slot, rhs: Slot, target: Slot },
    /// Is lhs less than or equal to rhs?
    Le { lhs: Slot, rhs: Slot, target: Slot },
    /// Is lhs greater than rhs?
    Gt { lhs: Slot, rhs: Slot, target: Slot },
    /// Is lhs greater than or equal to rhs?
    Ge { lhs: Slot, rhs: Slot, target: Slot },

    // --- Logical ---

    /// Is slot false?
    Not { source: Slot, target: Slot },

    // --- Control Flow ---
    Label { name: String },
    Jump { target: String },
    BrTrue { condition: Slot, target: String },
    BrFalse { condition: Slot, target: String },
    Halt,

    // --- Func Calls ---

    /// 
    Call { target: Slot, func: Slot, args: Vec<Slot> },
    Return { value: Option<Slot> },

    Say { message: Slot },
    Write { location: Slot, target: Slot },
    Read { location: Slot, target: Slot },
    Ask { question: Slot, target: Slot },

    Inc { target: Slot }, // target = target + 1
    Dec { target: Slot }, // target = target - 1
}

impl Op {
    pub fn is_terminator(&self) -> bool {
        matches!(self,
            Op::Jump { .. }
            | Op::BrTrue { .. }
            | Op::BrFalse { .. }
            | Op::Halt
            | Op::Return { .. }
        )
    }
    pub fn defines_slot(&self) -> Option<Slot> {
        match self {
            Op::LoadConst { target, .. }
            | Op::LoadLocal { target, .. }
            | Op::LoadGlobal { target, .. }
            | Op::Add { target, .. }
            | Op::Sub { target, .. }
            | Op::Mul { target, .. }
            | Op::Div { target, .. }
            | Op::Eq  { target, .. }
            | Op::Ne  { target, .. }
            | Op::Lt  { target, .. }
            | Op::Le  { target, .. }
            | Op::Gt  { target, .. }
            | Op::Ge  { target, .. }
            | Op::Length { target, .. }
            | Op::IGet { target, .. }
            | Op::Call { target, .. }
            | Op::NewArray { target, .. }
            | Op::MGet { target, .. }
            | Op::Not { target, .. } => Some(*target),
            _ => None,
        }
    }
    pub fn is_pure(&self) -> bool {
        matches!(self,
            Op::LoadConst { .. }
            | Op::Add { .. } | Op::Sub { .. } | Op::Mul { .. } | Op::Div { .. }
            | Op::Eq { .. } | Op::Ne { .. } | Op::Lt { .. } | Op::Le { .. } | Op::Gt { .. } | Op::Ge { .. }
            | Op::Length { .. } | Op::IGet { .. } | Op::Not { .. }
            | Op::LoadLocal { .. } | Op::LoadGlobal { .. }
        )
    }
    pub fn each_used_slot<F: FnMut(Slot)>(&self, mut f: F) {
        match self {
            Op::Add { lhs, rhs, .. }
            | Op::Sub { lhs, rhs, .. }
            | Op::Mul { lhs, rhs, .. }
            | Op::Div { lhs, rhs, .. }
            | Op::Eq  { lhs, rhs, .. }
            | Op::Ne  { lhs, rhs, .. }
            | Op::Lt  { lhs, rhs, .. }
            | Op::Le  { lhs, rhs, .. }
            | Op::Gt  { lhs, rhs, .. }
            | Op::Ge  { lhs, rhs, .. } => { f(*lhs); f(*rhs); }
            Op::Length { array, .. } => f(*array),
            Op::IGet { source, index, .. } => { f(*source); f(*index); }
            Op::StoreLocal { source, .. } => f(*source),
            Op::StoreGlobal { source, .. } => f(*source),
            Op::Call { func, args, .. } => {
                f(*func);
                for a in args { f(*a); }
            }
            Op::Say { message: value } => f(*value),
            Op::BrTrue { condition, .. } | Op::BrFalse { condition, .. } => f(*condition),
            Op::Inc { target } | Op::Dec { target } => f(*target),
            Op::Return { value } => { if let Some(v)=value { f(*v); } }
            Op::MGet { target, source, .. } => { f(*target); f(*source); }
            Op::MSet { target, value, .. } => { f(*target); f(*value); }
            Op::ISet { target, index, value } => { f(*target); f(*index); f(*value); }
            _ => {}
        }
    }

    // Map/replace used slots in-place. The closure must return the replacement slot.
    pub fn map_used_slots<F: FnMut(Slot) -> Slot>(&mut self, mut f: F) {
        match self {
            Op::Add { lhs, rhs, .. }
            | Op::Sub { lhs, rhs, .. }
            | Op::Mul { lhs, rhs, .. }
            | Op::Div { lhs, rhs, .. }
            | Op::Eq  { lhs, rhs, .. }
            | Op::Ne  { lhs, rhs, .. }
            | Op::Lt  { lhs, rhs, .. }
            | Op::Le  { lhs, rhs, .. }
            | Op::Gt  { lhs, rhs, .. }
            | Op::Ge  { lhs, rhs, .. } => {
                *lhs = f(*lhs);
                *rhs = f(*rhs);
            }

            Op::Length { array, .. } => {
                *array = f(*array);
            }

            Op::IGet { source, index, .. } => {
                *source = f(*source);
                *index = f(*index);
            }

            Op::ISet { target, index, value } => {
                *target = f(*target);
                *index = f(*index);
                *value = f(*value);
            }

            Op::MGet { target, source, .. } => {
                *target = f(*target);
                *source = f(*source);
            }
            Op::MSet { target, member: _, value } => {
                *target = f(*target);
                *value = f(*value);
            }

            Op::StoreLocal { source, .. } | Op::StoreGlobal { source, .. } => {
                *source = f(*source);
            }

            Op::Call { func, args, .. } => {
                *func = f(*func);
                for a in args.iter_mut() { *a = f(*a); }
            }

            Op::Say { message } => {
                *message = f(*message);
            }
            Op::Ask { question, target: _ } => {
                *question = f(*question);
            }
            Op::Read { location, target: _ } => {
                *location = f(*location);
            }
            Op::Write { location, target: _ } => {
                *location = f(*location);
            }

            Op::BrTrue { condition, .. } | Op::BrFalse { condition, .. } => {
                *condition = f(*condition);
            }

            Op::Inc { target } | Op::Dec { target } => {
                *target = f(*target);
            }

            Op::Return { value } => {
                if let Some(v) = value { *v = f(*v); }
            }

            _ => {}
        }
    }
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Op::*;
        match self {
            LoadConst { target, value } => write!(f, "LOAD_CONST {:?} <- {:?}", target, value),
            LoadLocal { target, local: source } => write!(f, "LOAD_LOCAL {:?} <- {:?}", target, source),
            StoreLocal { source, target } => write!(f, "STORE_LOCAL {:?} -> {:?}", source, target),
            LoadGlobal { target, global: name } => write!(f, "LOAD_GLOBAL {:?} <- {}", target, name),
            StoreGlobal { source, global: name } => write!(f, "STORE_GLOBAL {:?} -> {}", source, name),
            NewArray { target, size } => write!(f, "NEW_ARRAY {:?} size {}", target, size),
            Length { target, array } => write!(f, "LENGTH {:?} <- {:?}", target, array),
            IGet { target, source, index } => write!(f, "IGET {:?} <- {:?}[{:?}]", target, source, index),
            ISet { target, index, value } => write!(f, "ISET {:?}[{:?}] <- {:?}", target, index, value),
            MGet { target, source, member } => write!(f, "MGET {:?} <- {:?}.{}", target, source, member),
            MSet { target, member, value } => write!(f, "MSET {:?}.{} <- {:?}", target, member, value),
            Add { lhs, rhs, target } => write!(f, "ADD {:?} <- {:?} + {:?}", target, lhs, rhs),
            Sub { lhs, rhs, target } => write!(f, "SUB {:?} <- {:?} - {:?}", target, lhs, rhs),
            Mul { lhs, rhs, target } => write!(f, "MUL {:?} <- {:?} * {:?}", target, lhs, rhs),
            Div { lhs, rhs, target } => write!(f, "DIV {:?} <- {:?} / {:?}", target, lhs, rhs),
            Eq { lhs, rhs, target } => write!(f, "EQ {:?} <- {:?} == {:?}", target, lhs, rhs),
            Ne { lhs, rhs, target } => write!(f, "NE {:?} <- {:?} != {:?}", target, lhs, rhs),
            Lt { lhs, rhs, target } => write!(f, "LT {:?} <- {:?} < {:?}", target, lhs, rhs),
            Le { lhs, rhs, target } => write!(f, "LE {:?} <- {:?} <= {:?}", target, lhs, rhs),
            Gt { lhs, rhs, target } => write!(f, "GT {:?} <- {:?} > {:?}", target, lhs, rhs),
            Ge { lhs, rhs, target } => write!(f, "GE {:?} <- {:?} >= {:?}", target, lhs, rhs),
            Not { source, target } => write!(f, "NOT {:?} <- {:?}", target, source),
            Label { name } => write!(f, "LABEL {}", name),
            Jump { target } => write!(f, "JUMP {}", target),
            BrTrue { condition, target } => write!(f, "BR_TRUE {:?} -> {}", condition, target),
            BrFalse { condition, target } => write!(f, "BR_FALSE {:?} -> {}", condition, target),
            Halt => write!(f, "HALT"),
            Call { target, func, args } => write!(f, "CALL {:?} <- {:?}({:?})", target, func, args),
            Return { value } => match value {
                Some(v) => write!(f, "RETURN {:?}", v),
                None => write!(f, "RETURN"),
            },
            Say { message } => write!(f, "SAY {:?}", message),
            Write { location, target } => write!(f, "WRITE {:?} -> {:?}", target, location),
            Read { location, target } => write!(f, "READ {:?} <- {:?}", target, location),
            Ask { question, target } => write!(f, "ASK {:?} -> {:?}", question, target),
            Inc { target } => write!(f, "INC {:?}", target),
            Dec { target } => write!(f, "DEC {:?}", target),
        }
    }
}