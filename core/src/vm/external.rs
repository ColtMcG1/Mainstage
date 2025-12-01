use crate::vm::plugin::Plugin;
use crate::vm::value::Value;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct ExternalPlugin {
    name: String,
    exe: PathBuf,
}

impl ExternalPlugin {
    pub fn new(name: String, exe: PathBuf) -> Self {
        Self { name, exe }
    }

    fn value_to_json(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            Value::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Symbol(s) => serde_json::Value::String(s.clone()),
            Value::Array(a) => serde_json::Value::Array(a.iter().map(Self::value_to_json).collect()),
            Value::Object(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m.iter() {
                    map.insert(k.clone(), Self::value_to_json(v));
                }
                serde_json::Value::Object(map)
            }
            Value::Null => serde_json::Value::Null,
        }
    }

    fn json_to_value(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    Value::Int(n.as_i64().unwrap_or(0))
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(a) => Value::Array(a.iter().map(Self::json_to_value).collect()),
            serde_json::Value::Object(o) => {
                let mut map = std::collections::HashMap::new();
                for (k, v) in o.iter() {
                    map.insert(k.clone(), Self::json_to_value(v));
                }
                Value::Object(map)
            }
        }
    }
}

#[async_trait]
impl Plugin for ExternalPlugin {
    fn name(&self) -> &str { &self.name }

    async fn call(&self, func: &str, args: Vec<Value>) -> Result<Value, String> {
        // Prepare request JSON: { "func": "<func>", "args": [ ... ] }
        let req = serde_json::json!({
            "func": func,
            "args": args.iter().map(Self::value_to_json).collect::<Vec<_>>()
        });

        let mut cmd = Command::new(&self.exe);
        cmd.arg("call");
        cmd.arg(func);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| format!("spawn plugin '{}': {}", self.exe.display(), e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let body = serde_json::to_vec(&req).map_err(|e| format!("serialize req: {}", e))?;
            use std::io::Write;
            stdin.write_all(&body).map_err(|e| format!("write stdin: {}", e))?;
            // close stdin so child sees EOF
            drop(stdin);
        }

        let output = child.wait_with_output().map_err(|e| format!("wait plugin: {}", e))?;
        if !output.status.success() {
            return Err(format!("plugin '{}' exit code: {}", self.name, output.status));
        }

        let out = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&out).map_err(|e| format!("parse plugin output: {} \noutput: {}", e, out))?;
        Ok(Self::json_to_value(&json))
    }
}
