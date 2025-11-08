use crate::runtime::{Op, Value};

pub type ExecutionResult = Result<(), String>;

#[derive(Debug)]
struct Frame {
    return_ip: usize,
    #[allow(dead_code)]
    stack_base: usize,
}

pub struct Vm<'a> {
    code: &'a [u8],
    ip: usize,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    globals: Vec<Value>,
    const_pool: &'a [crate::codegen::ir::IRConst],
}

impl<'a> Vm<'a> {
    pub fn new(code: &'a [u8],
               const_pool: &'a [crate::codegen::ir::IRConst],
               _func_count: usize,
               _const_count: usize) -> Self {
        Vm {
            code,
            ip: 0,
            stack: Vec::with_capacity(128),
            frames: Vec::new(),
            globals: Vec::new(),
            const_pool,
        }
    }

    pub fn run(&mut self) -> ExecutionResult {
        while self.ip < self.code.len() {
            let op = Op::from_byte(self.code[self.ip])
                .ok_or_else(|| format!("Unknown opcode {:X}", self.code[self.ip]))?;
            self.ip += 1;

            match op {
                Op::LoadConst => {
                    let idx = self.read_u32();
                    let c = self.const_pool.get(idx as usize)
                        .ok_or_else(|| format!("Const index OOB {}", idx))?;
                    let v = to_value(c);
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let gid = self.read_u32() as usize;
                    let val = self.pop()?;
                    if gid >= self.globals.len() {
                        self.globals.resize(gid + 1, Value::Null);
                    }
                    self.globals[gid] = val;
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div => {
                    let rb = self.pop()?.as_int().ok_or("Expected int")?;
                    let ra = self.pop()?.as_int().ok_or("Expected int")?;
                    let res = match op {
                        Op::Add => ra + rb,
                        Op::Sub => ra - rb,
                        Op::Mul => ra * rb,
                        Op::Div => {
                            if rb == 0 { return Err("Division by zero".into()); }
                            ra / rb
                        }
                        _ => unreachable!(),
                    };
                    self.stack.push(Value::Int(res));
                }
                Op::Concat => {
                    let b = self.pop()?.as_str().ok_or("Expected string")?;
                    let a = self.pop()?.as_str().ok_or("Expected string")?;
                    self.stack.push(Value::Str(a + &b));
                }
                Op::Jump => {
                    let target = self.read_u32() as usize;
                    self.ip = target;
                }
                Op::JumpIfFalse => {
                    let target = self.read_u32() as usize;
                    let cond = self.pop()?.as_bool();
                    if !cond { self.ip = target; }
                }
                Op::Call => {
                    // Placeholder: no multi-function dispatch yet
                    let _fid = self.read_u32();
                    let _argc = self.read_u8();
                }
                Op::Return => {
                    if let Some(frame) = self.frames.pop() {
                        self.ip = frame.return_ip;
                    } else {
                        break;
                    }
                }
                Op::NoOp | Op::LoadVar | Op::StoreVar => {
                    // Not implemented yet
                }
            }
        }
        Ok(())
    }

    fn read_u32(&mut self) -> u32 {
        let end = self.ip + 4;
        let bytes = &self.code[self.ip..end];
        self.ip = end;
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_u8(&mut self) -> u8 {
        let b = self.code[self.ip];
        self.ip += 1;
        b
    }

    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".into())
    }
}

// Convert IR constants into runtime values recursively (supports nested arrays)
fn to_value(c: &crate::codegen::ir::IRConst) -> Value {
    use crate::codegen::ir::IRConst as K;
    match c {
        K::Int(i) => Value::Int(*i),
        K::Str(s) => Value::Str(s.clone()),
        K::Bool(b) => Value::Bool(*b),
        K::Ident(s) => Value::Identifier(s.clone()),
        K::Command(s) => Value::Command(s.clone()),
        K::Array(items) => {
            let vals = items.iter().map(to_value).collect::<Vec<_>>();
            Value::Array(vals)
        }
        K::Null => Value::Null,
    }
}