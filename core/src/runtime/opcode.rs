#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    LoadConst = 0x01,
    LoadVar = 0x02,
    StoreVar = 0x03,
    StoreGlobal = 0x04,
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,
    Concat = 0x18,
    Jump = 0x30,
    JumpIfFalse = 0x31,
    Call = 0x40,
    Return = 0x50,
    Say = 0x60,
    Write = 0x70,
    Read = 0x71,
    Ask = 0x72,
    LoadMemberDyn = 0x80,
    NoOp = 0xFF,
}

impl Op {
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x01 => Op::LoadConst,
            0x02 => Op::LoadVar,
            0x03 => Op::StoreVar,
            0x04 => Op::StoreGlobal,
            0x10 => Op::Add,
            0x11 => Op::Sub,
            0x12 => Op::Mul,
            0x13 => Op::Div,
            0x18 => Op::Concat,
            0x30 => Op::Jump,
            0x31 => Op::JumpIfFalse,
            0x40 => Op::Call,
            0x50 => Op::Return,
            0x60 => Op::Say,
            0x70 => Op::Write,
            0x71 => Op::Read,
            0x72 => Op::Ask,
            0x80 => Op::LoadMemberDyn,
            0xFF => Op::NoOp,
            _ => return None,
        })
    }
}
