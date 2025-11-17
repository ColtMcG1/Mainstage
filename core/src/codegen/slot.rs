#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Captured(usize),
    Temp(usize),
    Local(usize),
}

impl Slot {
    pub fn index(&self) -> usize {
        match self {
            Slot::Captured(i) | Slot::Temp(i) | Slot::Local(i) => *i,
        }
    }
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Slot::Captured(i) => write!(f, "captured[{}]", i),
            Slot::Temp(i) => write!(f, "temp[{}]", i),
            Slot::Local(i) => write!(f, "local[{}]", i),
        }
    }
}