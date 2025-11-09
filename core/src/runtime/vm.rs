use crate::runtime::{Op, Value};
use crate::codegen::bytecode::BytecodeModule;

pub type ExecutionResult = Result<(), String>;

#[derive(Debug)]
struct Frame {
    return_func: usize,
    return_ip: usize,
    stack_base: usize,
}

pub struct Vm<'a> {
    module: &'a BytecodeModule,
    func_id: usize,
    ip: usize,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    globals: Vec<Value>,
    const_pool: &'a [crate::codegen::ir::IRConst],
}

impl<'a> Vm<'a> {
    pub fn new(module: &'a BytecodeModule) -> Self {
        // Assume last function is 'main'
        let main_id = module.functions.len().saturating_sub(1);
        Vm {
            module,
            func_id: main_id,
            ip: 0,
            stack: Vec::with_capacity(128),
            frames: Vec::new(),
            globals: Vec::new(),
            const_pool: &module.const_pool,
        }
    }

    fn current_code(&self) -> &[u8] {
        &self.module.functions[self.func_id].code
    }

    pub fn run(&mut self) -> ExecutionResult {
        loop {
            // End-of-function handling
            if self.ip >= self.current_code().len() {
                if let Some(frame) = self.frames.pop() {
                    self.func_id = frame.return_func;
                    self.ip = frame.return_ip;
                    continue;
                } else {
                    break;
                }
            }

            // Fetch opcode byte without keeping a long borrow
            let op_byte = {
                let code = self.current_code();
                code[self.ip]
            };
            let op = Op::from_byte(op_byte)
                .ok_or_else(|| format!("Unknown opcode {:X}", op_byte))?;
            self.ip += 1;

            match op {
                Op::LoadConst => {
                    let idx = self.read_u32();
                    let c = self.const_pool.get(idx as usize)
                        .ok_or_else(|| format!("Const index OOB {}", idx))?;
                    self.stack.push(to_value(c));
                }
                Op::LoadVar => {
                    let gid = self.read_u32() as usize;
                    let v = self.globals.get(gid).cloned().unwrap_or(Value::Null);
                    self.stack.push(v);
                }
                Op::StoreVar | Op::StoreGlobal => {
                    let gid = self.read_u32() as usize;
                    let val = self.pop()?;
                    if gid >= self.globals.len() {
                        self.globals.resize(gid + 1, Value::Null);
                    }
                    self.globals[gid] = val;
                }
                Op::Call => {
                    let fid = self.read_u32() as usize;
                    let argc = self.read_u8() as usize;
                    let stack_base = self.stack.len() - argc;
                    self.frames.push(Frame {
                        return_func: self.func_id,
                        return_ip: self.ip,
                        stack_base,
                    });
                    // mutate after borrow ended
                    self.func_id = fid;
                    self.ip = 0;
                }
                Op::Return => {
                    let ret_val = self.stack.last().cloned();
                    if let Some(frame) = self.frames.pop() {
                        self.stack.truncate(frame.stack_base);
                        self.func_id = frame.return_func;
                        self.ip = frame.return_ip;
                        if let Some(v) = ret_val { self.stack.push(v); }
                    } else {
                        break;
                    }
                }
                Op::Say => {
                    let v = self.pop()?;
                    println!("{}", v.as_str().unwrap_or_else(|| format!("{:?}", v)));
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div => {
                    let b = self.pop()?.as_int().ok_or("Expected int")?;
                    let a = self.pop()?.as_int().ok_or("Expected int")?;
                    let r = match op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => {
                            if b == 0 { return Err("Division by zero".into()); }
                            a / b
                        }
                        _ => unreachable!(),
                    };
                    self.stack.push(Value::Int(r));
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
                Op::NoOp | Op::Write | Op::Read => {}
            }
        }
        Ok(())
    }

    fn read_u32(&mut self) -> u32 {
        let start = self.ip;
        let end = start + 4;
        let val = {
            let code = &self.module.functions[self.func_id].code;
            let bytes = &code[start..end];
            u32::from_le_bytes(bytes.try_into().unwrap())
        }; // borrow ends here
        self.ip = end;
        val
    }

    fn read_u8(&mut self) -> u8 {
        let b = {
            let code = &self.module.functions[self.func_id].code;
            code[self.ip]
        }; // borrow ends here
        self.ip += 1;
        b
    }
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".into())
    }
}

fn to_value(c: &crate::codegen::ir::IRConst) -> Value {
    use crate::codegen::ir::IRConst as K;
    match c {
        K::Int(i) => Value::Int(*i),
        K::Str(s) => Value::Str(s.clone()),
        K::Bool(b) => Value::Bool(*b),
        K::Ident(s) => Value::Identifier(s.clone()),
        K::Command(s) => Value::Command(s.clone()),
        K::Array(items) => Value::Array(items.iter().map(to_value).collect()),
        K::Null => Value::Null,
    }
}