
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Symbol(String),
    Array(Vec<Value>),
    Object(std::collections::HashMap<String, Value>),
    Null,
}