use crate::runtime::{Op, Value};
use crate::codegen::bytecode::BytecodeModule;
use std::path::{Path, PathBuf};

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
    base_dir: PathBuf, // NEW: script directory
}

impl<'a> Vm<'a> {
    pub fn new_with_base(module: &'a BytecodeModule, base_dir: impl Into<PathBuf>) -> Self {
        let main_id = module.functions.len().saturating_sub(1);
        Vm {
            module,
            func_id: main_id,
            ip: 0,
            stack: Vec::with_capacity(128),
            frames: Vec::new(),
            globals: Vec::new(),
            const_pool: &module.const_pool,
            base_dir: base_dir.into(),
        }
    }

    // Backwards-compatible constructor (uses current_dir)
    pub fn new(module: &'a BytecodeModule) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new_with_base(module, cwd)
    }

    fn resolve_path(&self, raw: &str) -> PathBuf {
        let p = Path::new(raw);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.base_dir.join(p)
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
                Op::Read => {
                    // Stack: ... [path]
                    let path_v = self.pop()?;
                    let path = path_v.as_str().ok_or_else(|| "read: path must be a string".to_string())?;
                    let full = self.resolve_path(&path);
                    let contents = std::fs::read_to_string(&full)
                        .map_err(|e| format!("read: {}: {}", full.display(), e))?;
                    self.stack.push(Value::Str(contents));
                }
                Op::Write => {
                    // Stack: ... [path, data] (pushed in that order), pop in reverse
                    let data_v = self.pop()?;
                    let path_v = self.pop()?;
                    let path = path_v.as_str().ok_or_else(|| "write: path must be a string".to_string())?;
                    let data = data_v.as_str().ok_or_else(|| "write: data must be a string".to_string())?;
                    let full = self.resolve_path(&path);
                    if let Some(parent) = full.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| format!("write: {}: {}", parent.display(), e))?;
                    }
                    std::fs::write(&full, data.as_bytes())
                        .map_err(|e| format!("write: {}: {}", full.display(), e))?;
                    // unit: no push
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
                Op::Ask => {
                    let argc = self.read_u8() as usize;
                    if argc > 1 { return Err("ask: invalid argc".into()); }
                    use std::io::{self, Write};
                    if argc == 1 {
                        let prompt_val = self.pop()?;
                        if let Some(p) = prompt_val.as_str() {
                            print!("{}", p);
                            io::stdout().flush().map_err(|e| e.to_string())?;
                        }
                    }
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
                    let trimmed = input.trim_end().to_string();
                    self.stack.push(Value::Str(trimmed));
                }
                Op::LoadMemberDyn => {
                    let field_ci = self.read_u32() as usize;
                    let field = self.const_pool[field_ci].as_str().ok_or_else(|| "LoadMemberDyn: field must be string const".to_string())?;
                    let obj_v = self.pop()?;
                    let obj = obj_v.as_str().ok_or_else(|| "LoadMemberDyn: object must be Identifier or String".to_string())?;
                    if let Some(gidx) = self.resolve_member_dyn(&obj, field) {
                        let v = self.globals.get(gidx).cloned().unwrap_or(Value::Null);
                        self.stack.push(v);
                    } else {
                        println!("Warning: LoadMemberDyn: member not found: {}.{}", obj, field);
                        self.stack.push(Value::Null);
                    }
                }
                Op::NoOp => {}
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

    fn resolve_member_dyn(&self, obj: &str, field: &str) -> Option<usize> {
        for scope in ["project", "stage", "workspace", "task"] {
            let key = format!("{scope}:{obj}.{field}");
            if let Some(&idx) = self.module.name_to_global.get(&key) {
                return Some(idx as usize);
            }
        }
        None
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