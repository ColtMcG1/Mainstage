
type Register = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IROp {
    LConst { dest: Register, value: Value },
    
    LLocal { dest: Register, local_index: usize },
    SLocal { src: Register, local_index: usize },
    
    Add { dest: Register, src1: Register, src2: Register },
    Sub { dest: Register, src1: Register, src2: Register },
    Mul { dest: Register, src1: Register, src2: Register },
    Div { dest: Register, src1: Register, src2: Register },
    Mod { dest: Register, src1: Register, src2: Register },

    Eq { dest: Register, src1: Register, src2: Register },
    Neq { dest: Register, src1: Register, src2: Register },
    Lt { dest: Register, src1: Register, src2: Register },
    Lte { dest: Register, src1: Register, src2: Register },
    Gt { dest: Register, src1: Register, src2: Register },
    Gte { dest: Register, src1: Register, src2: Register },
    And { dest: Register, src1: Register, src2: Register },
    Or { dest: Register, src1: Register, src2: Register },
    Not { dest: Register, src: Register },

    Inc { dest: Register },
    Dec { dest: Register },

    Label { name: String },
    Jump { target: usize },
    BrTrue { cond: Register, target: usize },
    BrFalse { cond: Register, target: usize },
    Halt,

    Call { dest: Register, func: Register, args: Vec<Register> },
    Ret { src: Register },
}