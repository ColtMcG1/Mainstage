#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    LoadConst = 0x01,
    LoadVar   = 0x02,
    StoreVar  = 0x03,
    StoreGlobal = 0x04,
    Add       = 0x10,
    Sub       = 0x11,
    Mul       = 0x12,
    Div       = 0x13,
    Concat    = 0x18,
    Jump      = 0x30,
    JumpIfFalse = 0x31,
    Call      = 0x40,
    Return    = 0x50,
    NoOp      = 0xFF,
}

impl Op {
    pub fn from_byte(b: u8) -> Option<Self> {
        use Op::*;
        Some(match b {
            0x01 => LoadConst,
            0x02 => LoadVar,
            0x03 => StoreVar,
            0x04 => StoreGlobal,
            0x10 => Add,
            0x11 => Sub,
            0x12 => Mul,
            0x13 => Div,
            0x18 => Concat,
            0x30 => Jump,
            0x31 => JumpIfFalse,
            0x40 => Call,
            0x50 => Return,
            0xFF => NoOp,
            _ => return None,
        })
    }
}