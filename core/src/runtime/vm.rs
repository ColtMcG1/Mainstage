use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use crate::codegen::{IRProgram, Op, Slot};
use crate::runtime::value::RTValue;

pub type ExecutionResult = Result<(), String>;

struct CallFrame {
    locals: Vec<RTValue>,
    ret_ip: usize,
}

/// Register-IR VM (linear interpreter with frames).
pub struct VmIR<'a> {
    ops: &'a [Op],

    // precomputed
    labels: HashMap<String, usize>,

    // execution
    ip: usize,
    temps: Vec<RTValue>,
    frames: Vec<CallFrame>,
    globals: HashMap<String, RTValue>,

    // scope objects: name -> member map
    objects: HashMap<String, HashMap<String, RTValue>>,
}

impl<'a> VmIR<'a> {
    pub fn new(program: &'a IRProgram) -> Self {
        let ops = &program.ops;

        // Map labels to instruction indices
        let mut labels = HashMap::new();
        for (i, op) in ops.iter().enumerate() {
            if let Op::Label { name } = op {
                labels.insert(name.clone(), i);
            }
        }

        // Preallocate registers from meta (fallback to scan if not present)
        let (max_temp, max_local);
        #[allow(deprecated)]
        {
            max_temp = program.meta.max_temp;
            max_local = program.meta.max_local;
        }

        let temps = vec![RTValue::Null; max_temp];
        let first_frame = CallFrame {
            locals: vec![RTValue::Null; max_local],
            ret_ip: usize::MAX,
        };

        Self {
            ops,
            labels,
            ip: 0,
            temps,
            frames: vec![first_frame],
            globals: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("frame")
    }
    fn current_frame(&self) -> &CallFrame {
        self.frames.last().expect("frame")
    }

    // Register access
    fn get(&self, slot: &Slot) -> RTValue {
        match slot {
            Slot::Temp(i) => self.temps.get(*i).cloned().unwrap_or(RTValue::Null),
            Slot::Local(i) => self.current_frame().locals.get(*i).cloned().unwrap_or(RTValue::Null),
            Slot::Captured(_) => RTValue::Null, // not used yet
        }
    }
    fn set(&mut self, slot: &Slot, val: RTValue) {
        match slot {
            Slot::Temp(i) => {
                if *i >= self.temps.len() {
                    self.temps.resize(*i + 1, RTValue::Null);
                }
                self.temps[*i] = val;
            }
            Slot::Local(i) => {
                if *i >= self.current_frame().locals.len() {
                    let needed = *i + 1;
                    self.current_frame_mut().locals.resize(needed, RTValue::Null);
                }
                self.current_frame_mut().locals[*i] = val;
            }
            Slot::Captured(_) => { /* ignore for now */ }
        }
    }

    // Helpers
    fn truthy(v: &RTValue) -> bool {
        v.as_bool()
    }
    fn ensure_scope_object(&mut self, name: &str) {
        self.objects.entry(name.to_string()).or_default();
        self.globals
            .entry(name.to_string())
            .or_insert(RTValue::Ref { scope: "scope".into(), object: name.to_string() });
    }

    pub fn run(&mut self) -> ExecutionResult {
        while self.ip < self.ops.len() {
            let op = &self.ops[self.ip];
            self.ip += 1;

            match op {
                // Registers / Globals
                Op::LoadConst { target, value } => {
                    self.set(target, RTValue::from(value.clone()));
                }
                Op::LoadLocal { target, source } => {
                    let v = self.get(source);
                    self.set(target, v);
                }
                Op::StoreLocal { source, target } => {
                    let v = self.get(source);
                    self.set(target, v);
                }
                Op::LoadGlobal { target, name } => {
                    let v = self.globals.get(name).cloned().unwrap_or(RTValue::Null);
                    self.set(target, v);
                }
                Op::StoreGlobal { source, name } => {
                    let v = self.get(source);
                    self.globals.insert(name.clone(), v);
                }

                // Arrays
                Op::NewArray { target, size } => {
                    self.set(target, RTValue::Array(vec![RTValue::Null; *size]));
                }
                Op::IGet { target, source, index } => {
                    let arr = self.get(source);
                    let idx = self.get(index).as_int().unwrap_or(0) as usize;
                    let val = match arr {
                        RTValue::Array(a) => a.get(idx).cloned().unwrap_or(RTValue::Null),
                        _ => RTValue::Null,
                    };
                    self.set(target, val);
                }
                Op::ISet { target, index, value } => {
                    let idx = self.get(index).as_int().unwrap_or(0) as usize;
                    let val = self.get(value);
                    let current = self.get(target);
                    match current {
                        RTValue::Array(mut a) => {
                            if idx >= a.len() {
                                a.resize(idx + 1, RTValue::Null);
                            }
                            a[idx] = val;
                            self.set(target, RTValue::Array(a));
                        }
                        _ => return Err("ISet on non-array".into()),
                    }
                }
                Op::Length { target, array } => {
                    let len = match self.get(array) {
                        RTValue::Array(a) => a.len(),
                        _ => 0,
                    };
                    self.set(target, RTValue::Int(len as i64));
                }

                // Members
                Op::MGet { target, source, member } => {
                    let obj = self.get(source);
                    let v = match obj {
                        RTValue::Ref { object, .. } => {
                            self.objects.get(&object)
                                .and_then(|m| m.get(member).cloned())
                                .unwrap_or(RTValue::Null)
                        }
                        _ => RTValue::Null,
                    };
                    self.set(target, v);
                }
                Op::MSet { target, member, value } => {
                    let obj = self.get(target);
                    let val = self.get(value);
                    match obj {
                        RTValue::Ref { object, .. } => {
                            let entry = self.objects.entry(object).or_default();
                            entry.insert(member.clone(), val);
                        }
                        _ => return Err("MSet on non-object".into()),
                    }
                }

                // Arithmetic
                Op::Add { lhs, rhs, target }
                | Op::Sub { lhs, rhs, target }
                | Op::Mul { lhs, rhs, target }
                | Op::Div { lhs, rhs, target } => {
                    let a = self.get(lhs);
                    let b = self.get(rhs);

                    let (af, bf) = (
                        a.as_float().or(a.as_int().map(|i| i as f64)),
                        b.as_float().or(b.as_int().map(|i| i as f64)),
                    );
                    let (af, bf) = match (af, bf) {
                        (Some(x), Some(y)) => (x, y),
                        _ => return Err("Arithmetic: non-numeric operand".into()),
                    };
                    let res = match op {
                        Op::Add { .. } => af + bf,
                        Op::Sub { .. } => af - bf,
                        Op::Mul { .. } => af * bf,
                        Op::Div { .. } => {
                            if bf == 0.0 {
                                return Err("Division by zero".into());
                            }
                            af / bf
                        }
                        _ => unreachable!(),
                    };
                    // Preserve int if both were ints
                    let out = match (a.as_int(), b.as_int()) {
                        (Some(_), Some(_)) if res.fract() == 0.0 => RTValue::Int(res as i64),
                        _ => RTValue::Float(res),
                    };
                    self.set(target, out);
                }

                // Comparisons
                Op::Eq { lhs, rhs, target }
                | Op::Ne { lhs, rhs, target }
                | Op::Lt { lhs, rhs, target }
                | Op::Le { lhs, rhs, target }
                | Op::Gt { lhs, rhs, target }
                | Op::Ge { lhs, rhs, target } => {
                    let a = self.get(lhs);
                    let b = self.get(rhs);
                    let result = match op {
                        Op::Eq { .. } => a == b,
                        Op::Ne { .. } => a != b,
                        Op::Lt { .. } => a < b,
                        Op::Le { .. } => a <= b,
                        Op::Gt { .. } => a > b,
                        Op::Ge { .. } => a >= b,
                        _ => unreachable!(),
                    };
                    self.set(target, RTValue::Bool(result));
                }

                // Control flow
                Op::Label { .. } => { /* no-op */ }
                Op::Jump { target } => {
                    self.ip = *self.labels.get(target).ok_or_else(|| format!("Unknown label {}", target))?;
                }
                Op::BrTrue { condition, target } => {
                    if Self::truthy(&self.get(condition)) {
                        self.ip = *self.labels.get(target).ok_or_else(|| format!("Unknown label {}", target))?;
                    }
                }
                Op::BrFalse { condition, target } => {
                    if !Self::truthy(&self.get(condition)) {
                        self.ip = *self.labels.get(target).ok_or_else(|| format!("Unknown label {}", target))?;
                    }
                }

                // Calls
                Op::CallScope { name } => {
                    // Create scope object if needed and set as a global Ref
                    self.ensure_scope_object(name);

                    let ret_ip = self.ip;
                    let locals_len = self.current_frame().locals.len();
                    self.frames.push(CallFrame {
                        locals: vec![RTValue::Null; locals_len],
                        ret_ip,
                    });

                    let label = format!("scope.{}", name);
                    self.ip = *self.labels.get(&label).ok_or_else(|| format!("Unknown scope label {}", label))?;
                }
                Op::Call { .. } => {
                    return Err("Generic Call not implemented".into());
                }
                Op::Return { value } => {
                    // If return has a value, you can choose to store it in a temp/global if needed
                    let _ret_val = value.as_ref().map(|s| self.get(s));
                    if let Some(frame) = self.frames.pop() {
                        if self.frames.is_empty() {
                            // Exit program on returning from the base frame
                            self.ip = self.ops.len();
                        } else {
                            self.ip = frame.ret_ip;
                        }
                    } else {
                        self.ip = self.ops.len();
                    }
                }

                // Builtins
                Op::Say { message } => {
                    let msg = self.get(message).as_str().unwrap_or_else(|| "null".to_string());
                    println!("{}", msg);
                }
                Op::Ask { question, target } => {
                    let prompt = self.get(question).as_str().unwrap_or_else(|| "".to_string());
                    print!("{}", prompt);
                    let _ = io::stdout().flush();
                    let mut line = String::new();
                    io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
                    let s = line.trim_end_matches(&['\r', '\n'][..]).to_string();
                    self.set(target, RTValue::Str(s));
                }
                Op::Read { location, target } => {
                    let path = self.get(location).as_str().unwrap_or_else(|| "".to_string());
                    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    self.set(target, RTValue::Str(contents));
                }
                Op::Write { location, target: content } => {
                    let path = self.get(location).as_str().unwrap_or_else(|| "".to_string());
                    let data = self.get(content).as_str().unwrap_or_else(|| "".to_string());
                    fs::write(&path, data).map_err(|e| e.to_string())?;
                }

                _ => {
                    return Err(format!("Unsupported IR op: {:?}", op));
                }
            }
        }
        Ok(())
    }

    pub fn take_globals(self) -> HashMap<String, RTValue> {
        self.globals
    }
}