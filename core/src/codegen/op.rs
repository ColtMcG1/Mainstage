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

    // --- Func Calls ---

    /// 
    Call { target: Slot, func: Slot, args: Vec<Slot> },
    CallScope { name: String },
    Return { value: Option<Slot> },

    Say { message: Slot },
    Write { location: Slot, target: Slot },
    Read { location: Slot, target: Slot },
    Ask { question: Slot, target: Slot },
}