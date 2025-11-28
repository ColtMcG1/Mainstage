use std::io::Cursor;
use std::collections::HashMap;

#[derive(Debug)]
enum ParsedValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Symbol(String),
    Array(Vec<ParsedValue>),
    Null,
}

impl std::fmt::Display for ParsedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsedValue::Int(i) => write!(f, "Int({})", i),
            ParsedValue::Float(x) => write!(f, "Float({})", x),
            ParsedValue::Bool(b) => write!(f, "Bool({})", b),
            ParsedValue::Str(s) => write!(f, "Str(\"{}\")", s),
            ParsedValue::Symbol(s) => write!(f, "Symbol({})", s),
            ParsedValue::Array(a) => {
                let inner: Vec<String> = a.iter().map(|v| format!("{}", v)).collect();
                write!(f, "Array([{}])", inner.join(", "))
            }
            ParsedValue::Null => write!(f, "Null"),
        }
    }
}

#[derive(Debug)]
enum ParsedOp {
    LConst { dest: u32, val: ParsedValue },
    LLocal { dest: u32, local: u32 },
    SLocal { src: u32, local: u32 },
    Bin { name: &'static str, dest: u32, a: u32, b: u32 },
    Cmp { name: &'static str, dest: u32, a: u32, b: u32 },
    Not { dest: u32, src: u32 },
    Inc { dest: u32 },
    Dec { dest: u32 },
    Label { name: String },
    Jump { target: u32 },
    BrTrue { cond: u32, target: u32 },
    BrFalse { cond: u32, target: u32 },
    Halt,
    AllocClosure { dest: u32 },
    CStore { clo: u32, field: u32, src: u32 },
    CLoad { dest: u32, clo: u32, field: u32 },
    Call { dest: u32, func: u32, args: Vec<u32> },
    CallLabel { dest: u32, label_index: u32, args: Vec<u32> },
    Ret { src: u32 },
}

pub fn disassemble(bytes: &[u8]) -> Result<String, String> {
    let mut cur = Cursor::new(bytes);
    let mut out = String::new();

    use std::io::Read;

    let mut magic = [0u8; 4];
    cur.read_exact(&mut magic).map_err(|e| format!("missing header: {}", e))?;
    out.push_str(&format!("Magic: {}\n", String::from_utf8_lossy(&magic)));
    let version = read_u32(&mut cur)?;
    out.push_str(&format!("Version: {}\n", version));
    let op_count = read_u32(&mut cur)? as usize;
    out.push_str(&format!("Op count: {}\n\n", op_count));

    // First pass: parse all ops into a vector and collect label positions
    let mut ops: Vec<ParsedOp> = Vec::with_capacity(op_count);
    let mut label_map: HashMap<usize, String> = HashMap::new();

    for i in 0..op_count {
        let code = read_u8(&mut cur)?;
        let parsed = match code {
            0x01 => { let dest = read_u32(&mut cur)?; let v = read_parsed_value(&mut cur)?; ParsedOp::LConst { dest, val: v } }
            0x02 => { let dest = read_u32(&mut cur)?; let local = read_u32(&mut cur)?; ParsedOp::LLocal { dest, local } }
            0x03 => { let src = read_u32(&mut cur)?; let local = read_u32(&mut cur)?; ParsedOp::SLocal { src, local } }
            0x10 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Bin { name: "Add", dest, a, b } }
            0x11 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Bin { name: "Sub", dest, a, b } }
            0x12 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Bin { name: "Mul", dest, a, b } }
            0x13 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Bin { name: "Div", dest, a, b } }
            0x14 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Bin { name: "Mod", dest, a, b } }
            0x20 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Eq", dest, a, b } }
            0x21 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Neq", dest, a, b } }
            0x22 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Lt", dest, a, b } }
            0x23 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Lte", dest, a, b } }
            0x24 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Gt", dest, a, b } }
            0x25 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Gte", dest, a, b } }
            0x26 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "And", dest, a, b } }
            0x27 => { let dest = read_u32(&mut cur)?; let a = read_u32(&mut cur)?; let b = read_u32(&mut cur)?; ParsedOp::Cmp { name: "Or", dest, a, b } }
            0x28 => { let dest = read_u32(&mut cur)?; let src = read_u32(&mut cur)?; ParsedOp::Not { dest, src } }
            0x30 => { let dest = read_u32(&mut cur)?; ParsedOp::Inc { dest } }
            0x31 => { let dest = read_u32(&mut cur)?; ParsedOp::Dec { dest } }
            0x40 => { let name = read_string(&mut cur)?; ParsedOp::Label { name } }
            0x41 => { let target = read_u32(&mut cur)?; ParsedOp::Jump { target } }
            0x42 => { let cond = read_u32(&mut cur)?; let target = read_u32(&mut cur)?; ParsedOp::BrTrue { cond, target } }
            0x43 => { let cond = read_u32(&mut cur)?; let target = read_u32(&mut cur)?; ParsedOp::BrFalse { cond, target } }
            0x50 => { ParsedOp::Halt }
            0x60 => { let dest = read_u32(&mut cur)?; ParsedOp::AllocClosure { dest } }
            0x61 => { let clo = read_u32(&mut cur)?; let field = read_u32(&mut cur)?; let src = read_u32(&mut cur)?; ParsedOp::CStore { clo, field, src } }
            0x62 => { let dest = read_u32(&mut cur)?; let clo = read_u32(&mut cur)?; let field = read_u32(&mut cur)?; ParsedOp::CLoad { dest, clo, field } }
            0x70 => {
                let dest = read_u32(&mut cur)?; let func = read_u32(&mut cur)?; let argc = read_u32(&mut cur)?;
                let mut args = Vec::new(); for _ in 0..argc { args.push(read_u32(&mut cur)?); }
                ParsedOp::Call { dest, func, args }
            }
            0x71 => {
                let dest = read_u32(&mut cur)?; let lbl = read_u32(&mut cur)?; let argc = read_u32(&mut cur)?;
                let mut args = Vec::new(); for _ in 0..argc { args.push(read_u32(&mut cur)?); }
                ParsedOp::CallLabel { dest, label_index: lbl, args }
            }
            0x80 => { let src = read_u32(&mut cur)?; ParsedOp::Ret { src } }
            other => return Err(format!("unknown opcode 0x{:02x} at op {}", other, i)),
        };

        // if this op was a Label, record it against this op index
        if let ParsedOp::Label { name } = &parsed {
            label_map.insert(i, name.clone());
        }

        ops.push(parsed);
    }

    // Second pass: render ops, resolving numeric targets to label names when possible
    for (i, op) in ops.iter().enumerate() {
        match op {
            ParsedOp::LConst { dest, val } => out.push_str(&format!("{:04}  LConst r{} <- {}\n", i, dest, val)),
            ParsedOp::LLocal { dest, local } => out.push_str(&format!("{:04}  LLocal r{} <- local[{}]\n", i, dest, local)),
            ParsedOp::SLocal { src, local } => out.push_str(&format!("{:04}  SLocal local[{}] <- r{}\n", i, local, src)),
            ParsedOp::Bin { name, dest, a, b } => out.push_str(&format!("{:04}  {} r{} <- r{} , r{}\n", i, name, dest, a, b)),
            ParsedOp::Cmp { name, dest, a, b } => out.push_str(&format!("{:04}  {} r{} <- r{} , r{}\n", i, name, dest, a, b)),
            ParsedOp::Not { dest, src } => out.push_str(&format!("{:04}  Not r{} <- !r{}\n", i, dest, src)),
            ParsedOp::Inc { dest } => out.push_str(&format!("{:04}  Inc r{} ++\n", i, dest)),
            ParsedOp::Dec { dest } => out.push_str(&format!("{:04}  Dec r{} --\n", i, dest)),
            ParsedOp::Label { name } => out.push_str(&format!("{:04}  Label {}\n", i, name)),
            ParsedOp::Jump { target } => {
                let t = *target as usize;
                if let Some(name) = label_map.get(&t) {
                    out.push_str(&format!("{:04}  Jump {}\n", i, name));
                } else {
                    out.push_str(&format!("{:04}  Jump {}\n", i, target));
                }
            }
            ParsedOp::BrTrue { cond, target } => {
                let t = *target as usize;
                if let Some(name) = label_map.get(&t) {
                    out.push_str(&format!("{:04}  BrTrue r{} -> {}\n", i, cond, name));
                } else {
                    out.push_str(&format!("{:04}  BrTrue r{} -> {}\n", i, cond, target));
                }
            }
            ParsedOp::BrFalse { cond, target } => {
                let t = *target as usize;
                if let Some(name) = label_map.get(&t) {
                    out.push_str(&format!("{:04}  BrFalse r{} -> {}\n", i, cond, name));
                } else {
                    out.push_str(&format!("{:04}  BrFalse r{} -> {}\n", i, cond, target));
                }
            }
            ParsedOp::Halt => out.push_str(&format!("{:04}  Halt\n", i)),
            ParsedOp::AllocClosure { dest } => out.push_str(&format!("{:04}  AllocClosure r{}\n", i, dest)),
            ParsedOp::CStore { clo, field, src } => out.push_str(&format!("{:04}  CStore clo[r{}].{} <- r{}\n", i, clo, field, src)),
            ParsedOp::CLoad { dest, clo, field } => out.push_str(&format!("{:04}  CLoad r{} <- clo[r{}].{}\n", i, dest, clo, field)),
            ParsedOp::Call { dest, func, args } => {
                out.push_str(&format!("{:04}  Call r{} <- r{}(", i, dest, func));
                for (j,a) in args.iter().enumerate() { if j>0 { out.push_str(", "); } out.push_str(&format!("r{}", a)); }
                out.push_str(")\n");
            }
            ParsedOp::CallLabel { dest, label_index, args } => {
                let t = *label_index as usize;
                if let Some(name) = label_map.get(&t) {
                    out.push_str(&format!("{:04}  CallLabel r{} <- {}(", i, dest, name));
                } else {
                    out.push_str(&format!("{:04}  CallLabel r{} <- L{}(", i, dest, label_index));
                }
                for (j,a) in args.iter().enumerate() { if j>0 { out.push_str(", "); } out.push_str(&format!("r{}", a)); }
                out.push_str(")\n");
            }
            ParsedOp::Ret { src } => out.push_str(&format!("{:04}  Ret r{}\n", i, src)),
        }
    }

    Ok(out)
}

fn read_u8(cur: &mut Cursor<&[u8]>) -> Result<u8, String> {
    use std::io::Read;
    let mut b = [0u8;1];
    cur.read_exact(&mut b).map_err(|e| format!("unexpected eof: {}", e))?;
    Ok(b[0])
}

fn read_u32(cur: &mut Cursor<&[u8]>) -> Result<u32, String> {
    use std::io::Read;
    let mut b = [0u8;4];
    cur.read_exact(&mut b).map_err(|e| format!("unexpected eof: {}", e))?;
    Ok(u32::from_le_bytes(b))
}

fn read_string(cur: &mut Cursor<&[u8]>) -> Result<String, String> {
    let len = read_u32(cur)? as usize;
    let mut buf = vec![0u8; len];
    use std::io::Read;
    cur.read_exact(&mut buf).map_err(|e| format!("unexpected eof reading string: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("invalid utf8: {}", e))
}

fn read_parsed_value(cur: &mut Cursor<&[u8]>) -> Result<ParsedValue, String> {
    use std::io::Read;
    let mut tag = [0u8;1];
    cur.read_exact(&mut tag).map_err(|e| format!("eof reading value tag: {}", e))?;
    match tag[0] {
        0x01 => { let mut ib=[0u8;8]; cur.read_exact(&mut ib).map_err(|e| format!("eof int: {}", e))?; Ok(ParsedValue::Int(u64::from_le_bytes(ib) as i64)) }
        0x02 => { let mut fb=[0u8;8]; cur.read_exact(&mut fb).map_err(|e| format!("eof float: {}", e))?; Ok(ParsedValue::Float(f64::from_bits(u64::from_le_bytes(fb)))) }
        0x03 => { let mut vb=[0u8;1]; cur.read_exact(&mut vb).map_err(|e| format!("eof bool: {}", e))?; Ok(ParsedValue::Bool(vb[0]!=0)) }
        0x04 => { let s = read_string(cur)?; Ok(ParsedValue::Str(s)) }
        0x05 => { let s = read_string(cur)?; Ok(ParsedValue::Symbol(s)) }
        0x06 => { let len = read_u32(cur)?; let mut items = Vec::new(); for _ in 0..len { items.push(read_parsed_value(cur)?); } Ok(ParsedValue::Array(items)) }
        0x07 => Ok(ParsedValue::Null),
        other => Err(format!("unknown value tag 0x{:02x}", other)),
    }
}

