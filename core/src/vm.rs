use crate::ir::value::Value;
use glob::glob;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;

/// Simple runtime VM for MSBC bytecode.
/// Currently implements a minimal interpreter for a subset of ops used by samples:
/// - constants, locals, simple arithmetic and comparisons
/// - jumps and conditional branches
/// - CallLabel (call into a labeled stage) and Call (host functions for Symbol values)
/// - Ret, Halt
///
/// This VM is intentionally small and designed for prototyping.

#[derive(Debug, Clone)]
enum RunValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Symbol(String),
    Array(Vec<RunValue>),
    Object(std::collections::HashMap<String, RunValue>),
    Null,
}

impl From<Value> for RunValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Int(i) => RunValue::Int(i),
            Value::Float(f) => RunValue::Float(f),
            Value::Bool(b) => RunValue::Bool(b),
            Value::Str(s) => RunValue::Str(s),
            Value::Symbol(s) => RunValue::Symbol(s),
            Value::Array(a) => RunValue::Array(a.into_iter().map(From::from).collect()),
            Value::Object(m) => {
                RunValue::Object(m.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
            Value::Null => RunValue::Null,
        }
    }
}

impl RunValue {
    fn as_bool(&self) -> bool {
        match self {
            RunValue::Bool(b) => *b,
            RunValue::Int(i) => *i != 0,
            RunValue::Float(f) => *f != 0.0,
            RunValue::Str(s) => !s.is_empty(),
            RunValue::Symbol(_) => true,
            RunValue::Array(a) => !a.is_empty(),
            RunValue::Object(m) => !m.is_empty(),
            RunValue::Null => false,
        }
    }

    fn to_value(&self) -> Value {
        match self {
            RunValue::Int(i) => Value::Int(*i),
            RunValue::Float(f) => Value::Float(*f),
            RunValue::Bool(b) => Value::Bool(*b),
            RunValue::Str(s) => Value::Str(s.clone()),
            RunValue::Symbol(s) => Value::Symbol(s.clone()),
            RunValue::Array(a) => Value::Array(a.iter().map(|rv| rv.to_value()).collect()),
            RunValue::Object(m) => {
                Value::Object(m.iter().map(|(k, v)| (k.clone(), v.to_value())).collect())
            }
            RunValue::Null => Value::Null,
        }
    }
}

fn coerce_to_f64(a: &RunValue) -> Option<f64> {
    match a {
        RunValue::Int(i) => Some(*i as f64),
        RunValue::Float(f) => Some(*f),
        RunValue::Str(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn numeric_bin(
    a: &RunValue,
    b: &RunValue,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> RunValue {
    match (a, b) {
        (RunValue::Int(x), RunValue::Int(y)) => RunValue::Int(int_op(*x, *y)),
        _ => {
            let ax = coerce_to_f64(a);
            let bx = coerce_to_f64(b);
            if let (Some(x), Some(y)) = (ax, bx) {
                RunValue::Float(float_op(x, y))
            } else {
                RunValue::Null
            }
        }
    }
}

fn numeric_cmp(a: &RunValue, b: &RunValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (RunValue::Int(x), RunValue::Int(y)) => Some(x.cmp(y)),
        _ => {
            let ax = coerce_to_f64(a);
            let bx = coerce_to_f64(b);
            if let (Some(x), Some(y)) = (ax, bx) {
                if x < y {
                    Some(std::cmp::Ordering::Less)
                } else if x > y {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    Some(std::cmp::Ordering::Equal)
                }
            } else {
                None
            }
        }
    }
}

// Parsed runtime op to execute
#[derive(Debug, Clone)]
enum Op {
    LConst {
        dest: usize,
        val: RunValue,
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

struct Frame {
    locals: Vec<RunValue>,
    return_pc: Option<usize>,
    return_reg: Option<usize>,
}

pub fn run_bytecode(bytes: &[u8]) -> Result<(), String> {
    use std::io::Read;
    let mut cur = Cursor::new(bytes);

    // header
    let mut magic = [0u8; 4];
    cur.read_exact(&mut magic)
        .map_err(|e| format!("missing header: {}", e))?;
    if &magic != b"MSBC" {
        return Err("invalid magic".to_string());
    }
    let version = read_u32(&mut cur)?;
    if version != 1 {
        return Err(format!("unsupported version {}", version));
    }

    let op_count = read_u32(&mut cur)? as usize;

    // parse ops and collect label positions
    let mut ops: Vec<Op> = Vec::with_capacity(op_count);
    // map: op_index -> label_name (used for printing)
    let mut label_pos: HashMap<usize, String> = HashMap::new();
    // map: label_name -> op_index (used for resolving CallLabel by name)
    let mut label_by_name: HashMap<String, usize> = HashMap::new();
    for i in 0..op_count {
        let code = read_u8(&mut cur)?;
        match code {
            0x01 => {
                let dest = read_u32(&mut cur)? as usize;
                let val = read_value(&mut cur)?;
                ops.push(Op::LConst { dest, val });
            }
            0x02 => {
                let dest = read_u32(&mut cur)? as usize;
                let local = read_u32(&mut cur)? as usize;
                ops.push(Op::LLocal { dest, local });
            }
            0x03 => {
                let src = read_u32(&mut cur)? as usize;
                let local = read_u32(&mut cur)? as usize;
                ops.push(Op::SLocal { src, local });
            }
            0x10 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Add { dest, a, b });
            }
            0x11 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Sub { dest, a, b });
            }
            0x12 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Mul { dest, a, b });
            }
            0x13 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Div { dest, a, b });
            }
            0x14 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Mod { dest, a, b });
            }
            0x20 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Eq { dest, a, b });
            }
            0x21 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Neq { dest, a, b });
            }
            0x22 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Lt { dest, a, b });
            }
            0x23 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Lte { dest, a, b });
            }
            0x24 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Gt { dest, a, b });
            }
            0x25 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Gte { dest, a, b });
            }
            0x26 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::And { dest, a, b });
            }
            0x27 => {
                let dest = read_u32(&mut cur)? as usize;
                let a = read_u32(&mut cur)? as usize;
                let b = read_u32(&mut cur)? as usize;
                ops.push(Op::Or { dest, a, b });
            }
            0x28 => {
                let dest = read_u32(&mut cur)? as usize;
                let src = read_u32(&mut cur)? as usize;
                ops.push(Op::Not { dest, src });
            }
            0x30 => {
                let dest = read_u32(&mut cur)? as usize;
                ops.push(Op::Inc { dest });
            }
            0x31 => {
                let dest = read_u32(&mut cur)? as usize;
                ops.push(Op::Dec { dest });
            }
            0x40 => {
                let name = read_string(&mut cur)?;
                ops.push(Op::Label);
                label_pos.insert(i, name.clone());
                label_by_name.insert(name.clone(), i);
            }
            0x41 => {
                let target = read_u32(&mut cur)? as usize;
                ops.push(Op::Jump { target });
            }
            0x42 => {
                let cond = read_u32(&mut cur)? as usize;
                let target = read_u32(&mut cur)? as usize;
                ops.push(Op::BrTrue { cond, target });
            }
            0x43 => {
                let cond = read_u32(&mut cur)? as usize;
                let target = read_u32(&mut cur)? as usize;
                ops.push(Op::BrFalse { cond, target });
            }
            0x50 => {
                ops.push(Op::Halt);
            }
            0x60..=0x62 => {
                return Err("closure ops not supported in VM yet".to_string());
            }
            0x70 => {
                let dest = read_u32(&mut cur)? as usize;
                let func = read_u32(&mut cur)? as usize;
                let argc = read_u32(&mut cur)? as usize;
                let mut args = Vec::new();
                for _ in 0..argc {
                    args.push(read_u32(&mut cur)? as usize);
                }
                ops.push(Op::Call { dest, func, args });
            }
            0x71 => {
                let dest = read_u32(&mut cur)? as usize;
                let lbl = read_u32(&mut cur)? as usize;
                let argc = read_u32(&mut cur)? as usize;
                let mut args = Vec::new();
                for _ in 0..argc {
                    args.push(read_u32(&mut cur)? as usize);
                }
                ops.push(Op::CallLabel {
                    dest,
                    label_index: lbl,
                    args,
                });
            }
            0x93 => {
                let dest = read_u32(&mut cur)? as usize;
                let obj = read_u32(&mut cur)? as usize;
                let key = read_u32(&mut cur)? as usize;
                ops.push(Op::GetProp { dest, obj, key });
            }
            0x94 => {
                let obj = read_u32(&mut cur)? as usize;
                let key = read_u32(&mut cur)? as usize;
                let src = read_u32(&mut cur)? as usize;
                ops.push(Op::SetProp { obj, key, src });
            }
            0x90 => {
                let dest = read_u32(&mut cur)? as usize;
                let len = read_u32(&mut cur)? as usize;
                let mut elems = Vec::new();
                for _ in 0..len {
                    elems.push(read_u32(&mut cur)? as usize);
                }
                ops.push(Op::ArrayNew { dest, elems });
            }
            0x95 => {
                let dest = read_u32(&mut cur)? as usize;
                let src = read_u32(&mut cur)? as usize;
                ops.push(Op::LoadGlobal { dest, src });
            }
            0x91 => {
                let dest = read_u32(&mut cur)? as usize;
                let array = read_u32(&mut cur)? as usize;
                let index = read_u32(&mut cur)? as usize;
                ops.push(Op::ArrayGet { dest, array, index });
            }
            0x92 => {
                let array = read_u32(&mut cur)? as usize;
                let index = read_u32(&mut cur)? as usize;
                let src = read_u32(&mut cur)? as usize;
                ops.push(Op::ArraySet { array, index, src });
            }
            0x80 => {
                let src = read_u32(&mut cur)? as usize;
                ops.push(Op::Ret { src });
            }
            other => return Err(format!("unknown opcode 0x{:02x} at op {}", other, i)),
        }
    }

    // runtime state: registers and call frames
    let mut regs: Vec<RunValue> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();

    // create a root frame so top-level `SLocal`/`LLocal` operations work
    frames.push(Frame {
        locals: Vec::new(),
        return_pc: None,
        return_reg: None,
    });

    // helper to ensure register exists
    let ensure_reg = |regs: &mut Vec<RunValue>, idx: usize| {
        if idx >= regs.len() {
            regs.resize_with(idx + 1, || RunValue::Null);
        }
    };
    // start execution at op 0
    let mut pc: usize = 0;
    let mut steps: usize = 0;

    // call-site frame management when invoking CallLabel: we need to set return_pc and return_reg
    while pc < ops.len() {
        steps += 1;
        if steps > 200 {
            return Err("VM step limit exceeded".to_string());
        }
        let op = &ops[pc];
        match op {
            Op::LConst { dest, val } => {
                ensure_reg(&mut regs, *dest);
                regs[*dest] = val.clone();
                pc += 1;
            }
            Op::LLocal { dest, local } => {
                ensure_reg(&mut regs, *dest);
                if let Some(frame) = frames.last() {
                    if *local < frame.locals.len() {
                        regs[*dest] = frame.locals[*local].clone();
                    } else {
                        regs[*dest] = RunValue::Null;
                    }
                } else {
                    regs[*dest] = RunValue::Null;
                }
                pc += 1;
            }
            Op::SLocal { src, local } => {
                ensure_reg(&mut regs, *src);
                if let Some(frame) = frames.last_mut() {
                    if *local >= frame.locals.len() {
                        frame.locals.resize(*local + 1, RunValue::Null);
                    }
                    frame.locals[*local] = regs[*src].clone();
                }
                pc += 1;
            }
            Op::Add { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                regs[*dest] = numeric_bin(&regs[*a], &regs[*b], |x, y| x + y, |x, y| x + y);
                pc += 1;
            }
            Op::Sub { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                regs[*dest] = numeric_bin(&regs[*a], &regs[*b], |x, y| x - y, |x, y| x - y);
                pc += 1;
            }
            Op::Mul { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                regs[*dest] = numeric_bin(&regs[*a], &regs[*b], |x, y| x * y, |x, y| x * y);
                pc += 1;
            }
            Op::Div { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                // division: prefer float if any operand is float or non-divisible
                if let (RunValue::Int(x), RunValue::Int(y)) = (&regs[*a], &regs[*b]) {
                    if *y != 0 && x % y == 0 {
                        regs[*dest] = RunValue::Int(x / y);
                    } else {
                        regs[*dest] = numeric_bin(&regs[*a], &regs[*b], |x, y| x / y, |x, y| x / y);
                    }
                } else {
                    regs[*dest] = numeric_bin(&regs[*a], &regs[*b], |x, y| x / y, |x, y| x / y);
                }
                pc += 1;
            }
            Op::Mod { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                if let (RunValue::Int(x), RunValue::Int(y)) = (&regs[*a], &regs[*b]) {
                    if *y != 0 {
                        regs[*dest] = RunValue::Int(x % y);
                    } else {
                        regs[*dest] = RunValue::Null;
                    }
                } else {
                    regs[*dest] = RunValue::Null;
                }
                pc += 1;
            }
            Op::Eq { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                // numeric-aware equality
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord == std::cmp::Ordering::Equal);
                } else {
                    regs[*dest] = RunValue::Bool(regs[*a].to_value() == regs[*b].to_value());
                }
                pc += 1;
            }
            Op::Neq { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord != std::cmp::Ordering::Equal);
                } else {
                    regs[*dest] = RunValue::Bool(regs[*a].to_value() != regs[*b].to_value());
                }
                pc += 1;
            }
            Op::Lt { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord == std::cmp::Ordering::Less);
                } else {
                    regs[*dest] = RunValue::Bool(false);
                }
                pc += 1;
            }
            Op::Lte { dest, a, b } => {
                ensure_reg(&mut regs, *dest);
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord != std::cmp::Ordering::Greater);
                } else {
                    regs[*dest] = RunValue::Bool(false);
                }
                pc += 1;
            }
            Op::Gt { dest, a, b } => {
                ensure_reg(&mut regs, *dest);
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord == std::cmp::Ordering::Greater);
                } else {
                    regs[*dest] = RunValue::Bool(false);
                }
                pc += 1;
            }
            Op::Gte { dest, a, b } => {
                ensure_reg(&mut regs, *dest);
                if let Some(ord) = numeric_cmp(&regs[*a], &regs[*b]) {
                    regs[*dest] = RunValue::Bool(ord != std::cmp::Ordering::Less);
                } else {
                    regs[*dest] = RunValue::Bool(false);
                }
                pc += 1;
            }
            Op::And { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                let v = regs[*a].as_bool() && regs[*b].as_bool();
                regs[*dest] = RunValue::Bool(v);
                pc += 1;
            }
            Op::Or { dest, a, b } => {
                ensure_reg(&mut regs, *a);
                ensure_reg(&mut regs, *b);
                ensure_reg(&mut regs, *dest);
                let v = regs[*a].as_bool() || regs[*b].as_bool();
                regs[*dest] = RunValue::Bool(v);
                pc += 1;
            }
            Op::Not { dest, src } => {
                ensure_reg(&mut regs, *src);
                ensure_reg(&mut regs, *dest);
                regs[*dest] = RunValue::Bool(!regs[*src].as_bool());
                pc += 1;
            }
            Op::Inc { dest } => {
                ensure_reg(&mut regs, *dest);
                if let RunValue::Int(i) = &mut regs[*dest] {
                    *i += 1
                };
                pc += 1;
            }
            Op::Dec { dest } => {
                ensure_reg(&mut regs, *dest);
                if let RunValue::Int(i) = &mut regs[*dest] {
                    *i -= 1
                };
                pc += 1;
            }
            Op::Label { .. } => {
                pc += 1;
            }
            Op::Jump { target } => {
                pc = *target;
            }
            Op::BrTrue { cond, target } => {
                ensure_reg(&mut regs, *cond);
                if regs[*cond].as_bool() {
                    pc = *target
                } else {
                    pc += 1
                }
            }
            Op::BrFalse { cond, target } => {
                ensure_reg(&mut regs, *cond);
                if !regs[*cond].as_bool() {
                    pc = *target
                } else {
                    pc += 1
                }
            }
            Op::Halt => {
                break;
            }
            Op::Call { dest, func, args } => {
                ensure_reg(&mut regs, *func);
                let func_val = regs[*func].clone();
                // evaluate args
                let mut arg_vals: Vec<RunValue> = Vec::new();
                for &r in args.iter() {
                    ensure_reg(&mut regs, r);
                    arg_vals.push(regs[r].clone());
                }
                // only support Symbol host functions for now
                match func_val {
                    RunValue::Symbol(name) => {
                        let ret = run_host_fn(&name, &arg_vals)?;
                        ensure_reg(&mut regs, *dest);
                        regs[*dest] = ret;
                        pc += 1;
                    }
                    _ => return Err("Call: unsupported non-symbol function value".to_string()),
                }
            }
            Op::CallLabel {
                dest,
                label_index,
                args,
            } => {
                // Save return information and push a new frame
                let return_pc = pc + 1;
                // seed frame locals with args from registers
                let mut f = Frame {
                    locals: Vec::new(),
                    return_pc: Some(return_pc),
                    return_reg: Some(*dest),
                };
                let mut arg_vals: Vec<RunValue> = Vec::new();
                for (i, &areg) in args.iter().enumerate() {
                    ensure_reg(&mut regs, areg);
                    if i >= f.locals.len() {
                        f.locals.resize(i + 1, RunValue::Null);
                    }
                    f.locals[i] = regs[areg].clone();
                    arg_vals.push(regs[areg].clone());
                }
                // CallLabel encodes a label ordinal (L{n}). Resolve to op index.
                let label_name = format!("L{}", label_index);
                let resolved = label_by_name.get(&label_name).copied();
                frames.push(f);
                // jump to the label (the label op is at resolved op index, start after it)
                if let Some(idx) = resolved {
                    pc = idx + 1;
                } else {
                    return Err(format!("CallLabel: unknown label '{}'", label_name));
                }
            }
            Op::ArrayNew { dest, elems } => {
                // build a new array from the values in the specified registers
                let mut items: Vec<RunValue> = Vec::new();
                for &r in elems.iter() {
                    ensure_reg(&mut regs, r);
                    items.push(regs[r].clone());
                }
                ensure_reg(&mut regs, *dest);
                regs[*dest] = RunValue::Array(items);
                pc += 1;
            }
            Op::LoadGlobal { dest, src } => {
                // copy module-level register `src` into function-local dest
                ensure_reg(&mut regs, *src);
                ensure_reg(&mut regs, *dest);
                regs[*dest] = regs[*src].clone();
                pc += 1;
            }
            Op::ArrayGet { dest, array, index } => {
                ensure_reg(&mut regs, *array);
                ensure_reg(&mut regs, *index);
                ensure_reg(&mut regs, *dest);
                // Clone the array and index values so we don't hold multiple borrows
                let arr_val = regs[*array].clone();
                let idx_val = regs[*index].clone();
                match arr_val {
                    RunValue::Array(a) => {
                        if let RunValue::Int(i) = idx_val {
                            let idx = i as isize;
                            if idx >= 0 && (idx as usize) < a.len() {
                                regs[*dest] = a[idx as usize].clone();
                            } else {
                                regs[*dest] = RunValue::Null;
                            }
                        } else {
                            regs[*dest] = RunValue::Null;
                        }
                    }
                    _ => {
                        regs[*dest] = RunValue::Null;
                    }
                }
                pc += 1;
            }
            Op::ArraySet { array, index, src } => {
                ensure_reg(&mut regs, *array);
                ensure_reg(&mut regs, *index);
                ensure_reg(&mut regs, *src);
                // Clone index and src values ahead of the mutable borrow
                let idx_val = regs[*index].clone();
                let src_val = regs[*src].clone();
                // ensure the array register contains an Array, creating one if necessary
                match &mut regs[*array] {
                    RunValue::Array(a) => {
                        if let RunValue::Int(i) = idx_val {
                            let idx = i as usize;
                            if idx >= a.len() {
                                a.resize(idx + 1, RunValue::Null);
                            }
                            a[idx] = src_val;
                        }
                    }
                    other => {
                        // replace with a new array big enough to hold the index
                        if let RunValue::Int(i) = idx_val {
                            let idx = i as usize;
                            let mut a: Vec<RunValue> = Vec::new();
                            a.resize(idx + 1, RunValue::Null);
                            a[idx] = src_val;
                            *other = RunValue::Array(a);
                        }
                    }
                }
                pc += 1;
            }
            Op::GetProp { dest, obj, key } => {
                ensure_reg(&mut regs, *obj);
                ensure_reg(&mut regs, *key);
                ensure_reg(&mut regs, *dest);
                match &regs[*obj] {
                    RunValue::Object(map) => {
                        // key can be Symbol or Str
                        let k = match &regs[*key] {
                            RunValue::Symbol(s) => s.clone(),
                            RunValue::Str(s) => s.clone(),
                            _ => String::new(),
                        };
                        if let Some(v) = map.get(&k) {
                            regs[*dest] = v.clone();
                        } else {
                            regs[*dest] = RunValue::Null;
                        }
                    }
                    RunValue::Array(a) => {
                        // support array.length property
                        match &regs[*key] {
                            RunValue::Symbol(s) | RunValue::Str(s) => {
                                if s == "length" {
                                    regs[*dest] = RunValue::Int(a.len() as i64);
                                } else {
                                    regs[*dest] = RunValue::Null;
                                }
                            }
                            _ => {
                                regs[*dest] = RunValue::Null;
                            }
                        }
                    }
                    RunValue::Str(s) => {
                        // support string.length property
                        match &regs[*key] {
                            RunValue::Symbol(k) | RunValue::Str(k) => {
                                if k == "length" {
                                    regs[*dest] = RunValue::Int(s.chars().count() as i64);
                                } else {
                                    regs[*dest] = RunValue::Null;
                                }
                            }
                            _ => {
                                regs[*dest] = RunValue::Null;
                            }
                        }
                    }
                    _ => {
                        regs[*dest] = RunValue::Null;
                    }
                }
                pc += 1;
            }
            Op::SetProp { obj, key, src } => {
                ensure_reg(&mut regs, *obj);
                ensure_reg(&mut regs, *key);
                ensure_reg(&mut regs, *src);
                // ensure obj is an Object; if not, replace it with a new object
                let key_str = match &regs[*key] {
                    RunValue::Symbol(s) => s.clone(),
                    RunValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let src_val = regs[*src].clone();
                match &mut regs[*obj] {
                    RunValue::Object(map) => {
                        map.insert(key_str, src_val);
                    }
                    other => {
                        // replace non-object with an object that holds the property
                        let mut m = std::collections::HashMap::new();
                        m.insert(key_str, src_val);
                        *other = RunValue::Object(m);
                    }
                }
                pc += 1;
            }
            Op::Ret { src } => {
                ensure_reg(&mut regs, *src);
                // pop current frame
                if let Some(f) = frames.pop() {
                    // if there's a caller, write return value into its return_reg and set pc
                    if let Some(ret_reg) = f.return_reg {
                        ensure_reg(&mut regs, ret_reg);
                        regs[ret_reg] = regs[*src].clone();
                    }
                    if let Some(ret_pc) = f.return_pc {
                        pc = ret_pc;
                    } else {
                        // no return pc -> halt
                        break;
                    }
                } else {
                    // return with no frame -> halt
                    break;
                }
            }
        }
    }

    Ok(())
}

fn run_host_fn(name: &str, args: &Vec<RunValue>) -> Result<RunValue, String> {
    match name {
        "ask" => {
            use std::io::{self, Write};
            if let Some(RunValue::Str(prompt)) = args.get(0) {
                print!("{}", prompt);
                io::stdout()
                    .flush()
                    .map_err(|e| format!("io error: {}", e))?;
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| format!("io error: {}", e))?;
                let s = input.trim_end().to_string();
                let s_trim = s.trim();
                // Try boolean
                let low = s_trim.to_ascii_lowercase();
                if low == "true" {
                    return Ok(RunValue::Bool(true));
                } else if low == "false" {
                    return Ok(RunValue::Bool(false));
                }
                // Try integer
                if let Ok(i) = s_trim.parse::<i64>() {
                    return Ok(RunValue::Int(i));
                }
                // Try float
                if let Ok(f) = s_trim.parse::<f64>() {
                    return Ok(RunValue::Float(f));
                }
                // Fallback to string
                Ok(RunValue::Str(s))
            } else {
                Ok(RunValue::Str(String::new()))
            }
        }
        "say" => {
            if let Some(a) = args.get(0) {
                match a {
                    RunValue::Str(s) => println!("{}", s),
                    RunValue::Symbol(s) => println!("{}", s),
                    RunValue::Array(arr) => {
                        // Print each array element on its own line if possible
                        for item in arr {
                            match item {
                                RunValue::Str(s) => println!("{}", s),
                                RunValue::Symbol(sym) => println!("{}", sym),
                                other => println!("{:?}", other.to_value()),
                            }
                        }
                    }
                    _ => println!("{:?}", a.to_value()),
                }
            }
            Ok(RunValue::Null)
        }
        "read" => {
            if let Some(RunValue::Str(glob_pat)) = args.get(0) {
                println!("Reading files matching pattern: {}", glob_pat);
                match glob(glob_pat) {
                    Ok(paths) => {
                        let mut out: Vec<RunValue> = Vec::new();
                        for p in paths.flatten() {
                            if let Ok(s) = fs::read_to_string(&p) {
                                out.push(RunValue::Str(s));
                            }
                        }
                        // Return an array (possibly empty) of file contents
                        Ok(RunValue::Array(out))
                    }
                    Err(e) => Err(format!("glob error: {}", e)),
                }
            } else {
                Ok(RunValue::Array(Vec::new()))
            }
        }
        "write" => {
            if let (Some(RunValue::Str(path)), Some(RunValue::Str(content))) =
                (args.get(0), args.get(1))
            {
                match fs::write(path, content) {
                    Ok(_) => Ok(RunValue::Bool(true)),
                    Err(e) => Err(format!("write error: {}", e)),
                }
            } else {
                Err("write: invalid arguments".to_string())
            }
        }
        _ => Err(format!("unknown host function: {}", name)),
    }
}

// helpers for reading bytecode values (copied from bytecode emitter format)
fn read_u8(cur: &mut Cursor<&[u8]>) -> Result<u8, String> {
    use std::io::Read;
    let mut b = [0u8; 1];
    cur.read_exact(&mut b)
        .map_err(|e| format!("unexpected eof: {}", e))?;
    Ok(b[0])
}
fn read_u32(cur: &mut Cursor<&[u8]>) -> Result<u32, String> {
    use std::io::Read;
    let mut b = [0u8; 4];
    cur.read_exact(&mut b)
        .map_err(|e| format!("unexpected eof: {}", e))?;
    Ok(u32::from_le_bytes(b))
}
fn read_u64(cur: &mut Cursor<&[u8]>) -> Result<u64, String> {
    use std::io::Read;
    let mut b = [0u8; 8];
    cur.read_exact(&mut b)
        .map_err(|e| format!("unexpected eof: {}", e))?;
    Ok(u64::from_le_bytes(b))
}
fn read_string(cur: &mut Cursor<&[u8]>) -> Result<String, String> {
    let len = read_u32(cur)? as usize;
    let mut buf = vec![0u8; len];
    use std::io::Read;
    cur.read_exact(&mut buf)
        .map_err(|e| format!("unexpected eof reading string: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("invalid utf8: {}", e))
}

fn read_value(cur: &mut Cursor<&[u8]>) -> Result<RunValue, String> {
    use std::io::Read;
    let mut tag = [0u8; 1];
    cur.read_exact(&mut tag)
        .map_err(|e| format!("eof reading value tag: {}", e))?;
    match tag[0] {
        0x01 => {
            let v = read_u64(cur)? as i64;
            Ok(RunValue::Int(v))
        }
        0x02 => {
            let bits = read_u64(cur)?;
            Ok(RunValue::Float(f64::from_bits(bits)))
        }
        0x03 => {
            let mut b = [0u8; 1];
            cur.read_exact(&mut b)
                .map_err(|e| format!("eof bool: {}", e))?;
            Ok(RunValue::Bool(b[0] != 0))
        }
        0x04 => {
            let s = read_string(cur)?;
            Ok(RunValue::Str(s))
        }
        0x05 => {
            let s = read_string(cur)?;
            Ok(RunValue::Symbol(s))
        }
        0x06 => {
            let len = read_u32(cur)? as usize;
            let mut items = Vec::new();
            for _ in 0..len {
                items.push(read_value(cur)?);
            }
            Ok(RunValue::Array(items))
        }
        0x08 => {
            let len = read_u32(cur)? as usize;
            let mut map = std::collections::HashMap::new();
            for _ in 0..len {
                let key = read_string(cur)?;
                let val = read_value(cur)?;
                map.insert(key, val);
            }
            Ok(RunValue::Object(map))
        }
        0x07 => Ok(RunValue::Null),
        other => Err(format!("unknown value tag 0x{:02x}", other)),
    }
}
