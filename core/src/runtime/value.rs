#[derive(Debug, Clone)]
pub enum Value {
    Int(f64),
    Bool(bool),
    Str(String),
    Command(String),
    Array(Vec<Value>),
    Identifier(String),
    Ref { scope: String, object: String },
    Null,
}

impl Value {
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Command(c) => !c.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Identifier(id) => !id.is_empty(),
            Value::Ref { scope, object } => !scope.is_empty() && !object.is_empty(),
            Value::Null => false,
        }
    }
    pub fn as_int(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Null => Some(0.0),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<String> {
        match self {
            Value::Bool(b) => Some(if *b { "true".to_string() } else { "false".to_string() }),
            Value::Int(i) => Some(i.to_string()),
            Value::Array(a) => Some(format!("{:?}", a)),
            Value::Command(c) => Some(c.clone()),
            Value::Str(s) => Some(s.clone()),
            Value::Identifier(id) => Some(id.clone()),
            Value::Ref { object, .. } => Some(object.clone()),
            Value::Null => Some("null".to_string()),
        }
    }
}