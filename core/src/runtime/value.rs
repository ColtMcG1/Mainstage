#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Command(String),
    Array(Vec<Value>),
    Identifier(String),
    Null,
}

impl Value {
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Str(s) => !s.is_empty(),
            Value::Command(c) => !c.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Identifier(id) => !id.is_empty(),
            Value::Null => false,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::Null => Some(0),
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
            Value::Null => Some("null".to_string()),
        }
    }
}