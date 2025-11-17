#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    // Stack <-> const/locals/globals
    LoadConst = 0x01,
    LoadLocal = 0x02,
    StoreLocal = 0x03,
    LoadGlobal = 0x04,
    StoreGlobal = 0x05,

    // Object / Collection
    NewArray = 0x06,
    IndexGet = 0x07,
    IndexSet = 0x08,
    MemberGet = 0x09,
    MemberSet = 0x0A,

    // Arithmetic / Compare (binary: pop rhs, lhs; push result)
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,

    Eq = 0x14,
    Ne = 0x15,
    Lt = 0x16,
    Le = 0x17,
    Gt = 0x18,
    Ge = 0x19,

    // Logical
    Not = 0x1A,

    // stack utils (rare; keep minimal)
    Dup = 0x20,
    Swap = 0x21,
    Pop = 0x22,

    // Control Flow
    Jump = 0x30,
    BrTrue = 0x31,
    BrFalse = 0x32,
    Return = 0x50,

    // Func Calls
    Call = 0x40,
    Say = 0x60,
    Write = 0x70,
    Read = 0x71,
    Ask = 0x72,
}

impl Op {
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x01 => Op::LoadConst,
            0x02 => Op::LoadLocal,
            0x03 => Op::StoreLocal,
            0x04 => Op::LoadGlobal,
            0x05 => Op::StoreGlobal,
            0x10 => Op::Add,
            0x11 => Op::Sub,
            0x12 => Op::Mul,
            0x13 => Op::Div,
            0x14 => Op::Eq,
            0x15 => Op::Ne,
            0x16 => Op::Lt,
            0x17 => Op::Le,
            0x18 => Op::Gt,
            0x19 => Op::Ge,
            0x1A => Op::Not,
            0x20 => Op::Dup,
            0x21 => Op::Swap,
            0x22 => Op::Pop,
            0x30 => Op::Jump,
            0x31 => Op::BrTrue,
            0x32 => Op::BrFalse,
            0x50 => Op::Return,
            0x40 => Op::Call,
            0x60 => Op::Say,
            0x70 => Op::Write,
            0x71 => Op::Read,
            0x72 => Op::Ask,
            _ => return None,
        })
    }
}
