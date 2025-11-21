use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use crate::codegen::{IRProgram, Op, Slot};
use crate::runtime::value::RTValue;

pub type ExecutionResult = Result<(), String>;

struct CallFrame {
    locals: Vec<RTValue>,
    ret_ip: usize,
    ret_target: Option<Slot>,
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
    // helper to grow arrays
    fn ensure_len(v: &mut Vec<RTValue>, n: usize) {
        if v.len() < n {
            v.resize(n, RTValue::Null);
        }
    }

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
            ret_target: None,
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
            Slot::Local(i) => self
                .current_frame()
                .locals
                .get(*i)
                .cloned()
                .unwrap_or(RTValue::Null),
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
                    self.current_frame_mut()
                        .locals
                        .resize(needed, RTValue::Null);
                }
                self.current_frame_mut().locals[*i] = val;
            }
            Slot::Captured(_) => { /* ignore for now */ }
        }
    }
    fn get_mut(&mut self, slot: &Slot) -> &mut RTValue {
        match slot {
            Slot::Temp(i) => {
                if *i >= self.temps.len() {
                    self.temps.resize(*i + 1, RTValue::Null);
                }
                &mut self.temps[*i]
            }
            Slot::Local(i) => {
                let frame = self.current_frame_mut();
                if *i >= frame.locals.len() {
                    frame.locals.resize(*i + 1, RTValue::Null);
                }
                &mut frame.locals[*i]
            }
            Slot::Captured(_) => panic!("Captured not implemented"),
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
            .or_insert(RTValue::Ref {
                scope: "scope".into(),
                object: name.to_string(),
            });
    }
    fn func_name_from(&self, v: &RTValue) -> Option<String> {
        match v {
            RTValue::Str(s) => Some(s.clone()),
            RTValue::Identifier(s) => Some(s.clone()),
            // If you have a function/object ref variant, support it here:
            RTValue::Ref { object, .. } => Some(object.clone()),
            _ => None,
        }
    }

    fn parse_input(s: &str) -> RTValue {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return RTValue::Null;
        }
        // bool
        match trimmed.to_ascii_lowercase().as_str() {
            "true" => return RTValue::Bool(true),
            "false" => return RTValue::Bool(false),
            _ => {}
        }
        // int
        if let Ok(i) = trimmed.parse::<i64>() {
            return RTValue::Int(i);
        }
        // float
        if let Ok(f) = trimmed.parse::<f64>() {
            return RTValue::Float(f);
        }
        // future: arrays / objects (basic comma split)
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let mut items = Vec::new();
            for part in inner.split(',') {
                let v = Self::parse_input(part);
                items.push(v);
            }
            return RTValue::Array(items);
        }
        RTValue::Str(trimmed.to_string())
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
                    let sz = *size as usize;
                    self.set(target, RTValue::Array(vec![RTValue::Null; sz]));
                }
                Op::ISet {
                    target,
                    index,
                    value,
                } => {
                    let idx = match self.get(index) {
                        RTValue::Int(i) if i >= 0 => i as usize,
                        _ => return Err("ISet: non-integer index".into()),
                    };
                    let val = self.get(value);
                    match self.get_mut(target) {
                        RTValue::Array(a) => {
                            Self::ensure_len(a, idx + 1);
                            a[idx] = val;
                        }
                        _ => return Err("ISet: target is not an array".into()),
                    }
                }
                Op::IGet {
                    target,
                    source,
                    index,
                } => {
                    let idx = match self.get(index) {
                        RTValue::Int(i) if i >= 0 => i as usize,
                        _ => return Err("IGet: non-integer index".into()),
                    };
                    let out = match self.get(source) {
                        RTValue::Array(ref a) => a.get(idx).cloned().unwrap_or(RTValue::Null),
                        _ => RTValue::Null,
                    };
                    self.set(target, out);
                }
                Op::Length { target, array } => {
                    let len = match self.get(array) {
                        RTValue::Array(a) => a.len() as i64,
                        RTValue::Str(s) => s.chars().count() as i64, // or s.len() for byte length
                        _ => 0,
                    };
                    self.set(target, RTValue::Int(len));
                }

                // Members
                Op::MGet {
                    target,
                    source,
                    member,
                } => {
                    let obj = self.get(source);
                    let v = match obj {
                        RTValue::Ref { object, .. } => self
                            .objects
                            .get(&object)
                            .and_then(|m| m.get(member).cloned())
                            .unwrap_or(RTValue::Null),
                        _ => RTValue::Null,
                    };
                    self.set(target, v);
                }
                Op::MSet {
                    target,
                    member,
                    value,
                } => {
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

                    // String concatenation for Add
                    if matches!(op, Op::Add { .. }) {
                        if let (Some(sa), Some(sb)) = (a.as_str(), b.as_str()) {
                            self.set(target, RTValue::Str(format!("{}{}", sa, sb)));
                            continue;
                        }
                        // One side string: coerce other
                        if let Some(sa) = a.as_str() {
                            self.set(target, RTValue::Str(format!("{}{}", sa, b.to_string())));
                            continue;
                        }
                        if let Some(sb) = b.as_str() {
                            self.set(target, RTValue::Str(format!("{}{}", a.to_string(), sb)));
                            continue;
                        }
                    }

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
                    self.ip = *self
                        .labels
                        .get(target)
                        .ok_or_else(|| format!("Unknown label {}", target))?;
                }
                Op::BrTrue { condition, target } => {
                    if Self::truthy(&self.get(condition)) {
                        self.ip = *self
                            .labels
                            .get(target)
                            .ok_or_else(|| format!("Unknown label {}", target))?;
                    }
                }
                Op::BrFalse { condition, target } => {
                    if !Self::truthy(&self.get(condition)) {
                        self.ip = *self
                            .labels
                            .get(target)
                            .ok_or_else(|| format!("Unknown label {}", target))?;
                    }
                }
                Op::Halt => {
                    self.ip = self.ops.len();
                }

                // Calls
                Op::Call { target, func, args } => {
                    let func_val = self.get(func);
                    let Some(func_name) = self.func_name_from(&func_val) else {
                        return Err("Call on non-function".into());
                    };

                    // Prepare new frame
                    let ret_ip = self.ip;
                    let locals_len = self.current_frame().locals.len();
                    let mut new_frame = CallFrame {
                        locals: vec![RTValue::Null; locals_len],
                        ret_ip,
                        ret_target: Some(*target),
                    };

                    // Pass arguments into callee locals[0..N]
                    for (i, arg_slot) in args.iter().enumerate() {
                        if i < new_frame.locals.len() {
                            new_frame.locals[i] = self.get(arg_slot);
                        }
                    }
                    self.frames.push(new_frame);

                    // Resolve destination label. Prefer function label; fall back to scope label.
                    if let Some(&addr) = self.labels.get(&format!("func.{}", func_name)) {
                        self.ip = addr;
                    } else if let Some(&addr) = self.labels.get(&format!("scope.{}", func_name)) {
                        // Scope call: ensure object exists before entering
                        self.ensure_scope_object(&func_name);
                        self.ip = addr;
                    } else {
                        return Err(format!("Unknown function/scope '{}'", func_name));
                    }
                }
                Op::Return { value } => {
                    let ret_val = value.as_ref().map(|s| self.get(s)).unwrap_or(RTValue::Null);
                    if let Some(frame) = self.frames.pop() {
                        if let Some(slot) = frame.ret_target {
                            // store in caller
                            self.set(&slot, ret_val);
                        }
                        if self.frames.is_empty() {
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
                    let msg = self
                        .get(message)
                        .as_str()
                        .unwrap_or_else(|| "null".to_string());
                    println!("{}", msg);
                }
                Op::Ask { question, target } => {
                    let q = self.get(question);
                    if let RTValue::Str(prompt) = q {
                        print!("{} ", prompt);
                    } else {
                        print!("? ");
                    }
                    io::stdout().flush().ok();
                    let mut buf = String::new();
                    if io::stdin().read_line(&mut buf).is_ok() {
                        let val = Self::parse_input(&buf);
                        self.set(target, val);
                    } else {
                        self.set(target, RTValue::Null);
                    }
                }
                Op::Read { location, target } => {
                    let path = self
                        .get(location)
                        .as_str()
                        .unwrap_or_else(|| "".to_string());
                    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    self.set(target, RTValue::Str(contents));
                }
                Op::Write {
                    location,
                    target: content,
                } => {
                    let path = self
                        .get(location)
                        .as_str()
                        .unwrap_or_else(|| "".to_string());
                    let data = self.get(content).as_str().unwrap_or_else(|| "".to_string());
                    fs::write(&path, data).map_err(|e| e.to_string())?;
                }
                Op::Inc { target } => {
                    let v = self.get(target);
                    match v {
                        RTValue::Int(i) => self.set(target, RTValue::Int(i + 1)),
                        _ => return Err("Inc: target is not an integer".into()),
                    }
                }
                Op::Dec { target } => {
                    let v = self.get(target);
                    match v {
                        RTValue::Int(i) => self.set(target, RTValue::Int(i - 1)),
                        _ => return Err("Dec: target is not an integer".into()),
                    }
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
