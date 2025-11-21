use crate::codegen::slot::Slot;
use crate::codegen::value::OpValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    // --- Stack <-> const/locals/globals ---

    /// Load a constant into a register
    LoadConst  { target: Slot, value: OpValue },
    /// Load a local variable into a register
    LoadLocal  { target: Slot, source: Slot },
    /// Store a register value into a local variable
    StoreLocal { source: Slot, target: Slot },
    /// Load a global variable into a register
    LoadGlobal  { target: Slot, name: String },
    /// Store a register value into a global variable
    StoreGlobal { source: Slot, name: String },

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
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Op::*;
        match self {
            LoadConst { target, value } => write!(f, "LOAD_CONST {:?} <- {:?}", target, value),
            LoadLocal { target, source } => write!(f, "LOAD_LOCAL {:?} <- {:?}", target, source),
            StoreLocal { source, target } => write!(f, "STORE_LOCAL {:?} -> {:?}", source, target),
            LoadGlobal { target, name } => write!(f, "LOAD_GLOBAL {:?} <- {}", target, name),
            StoreGlobal { source, name } => write!(f, "STORE_GLOBAL {:?} -> {}", source, name),
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