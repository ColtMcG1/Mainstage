use crate::ir::OpValue;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum RTValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Command(String),
    Array(Vec<RTValue>),
    Identifier(String),
    Ref { scope: String, object: String },
    Null,
}

impl RTValue {
    pub fn as_bool(&self) -> bool {
        match self {
            RTValue::Bool(b) => *b,
            RTValue::Int(i) => *i != 0,
            RTValue::Float(f) => *f != 0.0,
            RTValue::Str(s) => !s.is_empty(),
            RTValue::Command(c) => !c.is_empty(),
            RTValue::Array(a) => !a.is_empty(),
            RTValue::Identifier(id) => !id.is_empty(),
            RTValue::Ref { scope, object } => !scope.is_empty() && !object.is_empty(),
            RTValue::Null => false,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            RTValue::Int(i) => Some(*i),
            RTValue::Float(f) => Some(*f as i64),
            RTValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            RTValue::Null => Some(0),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self {
            RTValue::Float(f) => Some(*f),
            RTValue::Int(i) => Some(*i as f64),
            RTValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            RTValue::Null => Some(0.0),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[RTValue]> {
        match self {
            RTValue::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<String> {
        match self {
            RTValue::Bool(b) => Some(if *b { "true".to_string() } else { "false".to_string() }),
            RTValue::Int(i) => Some(i.to_string()),
            RTValue::Float(f) => Some(f.to_string()),
            RTValue::Array(a) => Some(format!("{:?}", a)),
            RTValue::Command(c) => Some(c.clone()),
            RTValue::Str(s) => Some(s.clone()),
            RTValue::Identifier(id) => Some(id.clone()),
            RTValue::Ref { object, .. } => Some(object.clone()),
            RTValue::Null => Some("null".to_string()),
        }
    }
}

impl From<OpValue> for crate::runtime::value::RTValue {
    fn from(v: OpValue) -> Self {
        use crate::runtime::value::RTValue::*;
        match v {
            OpValue::Int(i)   => Int(i),
            OpValue::Float(f) => Float(f),
            OpValue::Bool(b)  => Bool(b),
            OpValue::Str(s)   => Str(s),
            OpValue::Array(items) => {
                Array(items.into_iter().map(|ov| crate::runtime::value::RTValue::from(ov)).collect())
            }
            OpValue::Unit => Null,
            OpValue::Null => Null,
        }
    }
}

impl std::fmt::Display for RTValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RTValue::Int(i) => write!(f, "{}", i),
            RTValue::Float(fl) => write!(f, "{}", fl),
            RTValue::Bool(b) => write!(f, "{}", b),
            RTValue::Str(s) => write!(f, "{}", s),
            RTValue::Command(c) => write!(f, "Command({})", c),
            RTValue::Array(a) => {
                let items: Vec<String> = a.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            RTValue::Identifier(id) => write!(f, "Identifier({})", id),
            RTValue::Ref { scope, object } => write!(f, "Ref({}::{})", scope, object),
            RTValue::Null => write!(f, "null"),
        }
    }
}