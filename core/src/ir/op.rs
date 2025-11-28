use super::value::Value;

type Register = usize;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    CallLabel { dest: Register, label_index: usize, args: Vec<Register> },
    Ret { src: Register },
}

impl std::fmt::Display for IROp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IROp::LConst { dest, value } => write!(f, "LConst r{} <- {:?}", dest, value),
            IROp::LLocal { dest, local_index } => write!(f, "LLocal r{} <- local[{}]", dest, local_index),
            IROp::SLocal { src, local_index } => write!(f, "SLocal local[{}] <- r{}", local_index, src),
            IROp::Add { dest, src1, src2 } => write!(f, "Add r{} <- r{} + r{}", dest, src1, src2),
            IROp::Sub { dest, src1, src2 } => write!(f, "Sub r{} <- r{} - r{}", dest, src1, src2),
            IROp::Mul { dest, src1, src2 } => write!(f, "Mul r{} <- r{} * r{}", dest, src1, src2),
            IROp::Div { dest, src1, src2 } => write!(f, "Div r{} <- r{} / r{}", dest, src1, src2),
            IROp::Mod { dest, src1, src2 } => write!(f, "Mod r{} <- r{} % r{}", dest, src1, src2),
            IROp::Eq { dest, src1, src2 } => write!(f, "Eq r{} <- r{} == r{}", dest, src1, src2),
            IROp::Neq { dest, src1, src2 } => write!(f, "Neq r{} <- r{} != r{}", dest, src1, src2),
            IROp::Lt { dest, src1, src2 } => write!(f, "Lt r{} <- r{} < r{}", dest, src1, src2),
            IROp::Lte { dest, src1, src2 } => write!(f, "Lte r{} <- r{} <= r{}", dest, src1, src2),
            IROp::Gt { dest, src1, src2 } => write!(f, "Gt r{} <- r{} > r{}", dest, src1, src2),
            IROp::Gte { dest, src1, src2 } => write!(f, "Gte r{} <- r{} >= r{}", dest, src1, src2),
            IROp::And { dest, src1, src2 } => write!(f, "And r{} <- r{} && r{}", dest, src1, src2),
            IROp::Or { dest, src1, src2 } => write!(f, "Or r{} <- r{} || r{}", dest, src1, src2),
            IROp::Not { dest, src } => write!(f, "Not r{} <- !r{}", dest, src),
            IROp::Inc { dest } => write!(f, "Inc r{} ++", dest),
            IROp::Dec { dest } => write!(f, "Dec r{} --", dest),
            IROp::Label { name } => write!(f, "Label {}", name),
            IROp::Jump { target } => write!(f, "Jump {}", target),
            IROp::BrTrue { cond, target } => write!(f, "BrTrue r{} -> {}", cond, target),
            IROp::BrFalse { cond, target } => write!(f, "BrFalse r{} -> {}", cond, target),
            IROp::Halt => write!(f, "Halt"),
            IROp::Call { dest, func, args } => {
                write!(f, "Call r{} <- r{}(", dest, func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "r{}", arg)?;
                }
                write!(f, ")")
            }
            IROp::CallLabel { dest, label_index, args } => {
                write!(f, "CallLabel r{} <- L{}(", dest, label_index)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "r{}", arg)?;
                }
                write!(f, ")")
            }
            IROp::Ret { src } => write!(f, "Ret r{}", src),
        }
    }
}