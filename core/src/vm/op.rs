// Parsed runtime op to execute
#[derive(Debug, Clone)]
pub(crate) enum Op {
    LConst {
        dest: usize,
        val: super::value::Value,
    },
    LLocal {
        dest: usize,
        local: usize,
    },
    SLocal {
        src: usize,
        local: usize,
    },
    Add {
        dest: usize,
        a: usize,
        b: usize,
    },
    Sub {
        dest: usize,
        a: usize,
        b: usize,
    },
    Mul {
        dest: usize,
        a: usize,
        b: usize,
    },
    Div {
        dest: usize,
        a: usize,
        b: usize,
    },
    Mod {
        dest: usize,
        a: usize,
        b: usize,
    },
    Eq {
        dest: usize,
        a: usize,
        b: usize,
    },
    Neq {
        dest: usize,
        a: usize,
        b: usize,
    },
    Lt {
        dest: usize,
        a: usize,
        b: usize,
    },
    Lte {
        dest: usize,
        a: usize,
        b: usize,
    },
    Gt {
        dest: usize,
        a: usize,
        b: usize,
    },
    Gte {
        dest: usize,
        a: usize,
        b: usize,
    },
    And {
        dest: usize,
        a: usize,
        b: usize,
    },
    Or {
        dest: usize,
        a: usize,
        b: usize,
    },
    Not {
        dest: usize,
        src: usize,
    },
    Inc {
        dest: usize,
    },
    Dec {
        dest: usize,
    },
    Label,
    Jump {
        target: usize,
    },
    BrTrue {
        cond: usize,
        target: usize,
    },
    BrFalse {
        cond: usize,
        target: usize,
    },
    Halt,
    Call {
        dest: usize,
        func: usize,
        args: Vec<usize>,
    },
    CallLabel {
        dest: usize,
        label_index: usize,
        args: Vec<usize>,
    },
    ArrayNew {
        dest: usize,
        elems: Vec<usize>,
    },
    LoadGlobal {
        dest: usize,
        src: usize,
    },
    ArrayGet {
        dest: usize,
        array: usize,
        index: usize,
    },
    ArraySet {
        array: usize,
        index: usize,
        src: usize,
    },
    GetProp {
        dest: usize,
        obj: usize,
        key: usize,
    },
    SetProp {
        obj: usize,
        key: usize,
        src: usize,
    },
    Ret {
        src: usize,
    },
}