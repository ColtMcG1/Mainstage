use crate::ir::value::Value as IrValue;

#[derive(Debug, Clone)]
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

impl From<IrValue> for Value {
    fn from(v: IrValue) -> Self {
        match v {
            IrValue::Int(i) => Value::Int(i),
            IrValue::Float(f) => Value::Float(f),
            IrValue::Bool(b) => Value::Bool(b),
            IrValue::Str(s) => Value::Str(s),
            IrValue::Symbol(s) => Value::Symbol(s),
            IrValue::Array(a) => Value::Array(a.into_iter().map(From::from).collect()),
            IrValue::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, v.into())).collect()),
            IrValue::Null => Value::Null,
        }
    }
}

impl Value {
    pub(crate) fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Symbol(_) => true,
            Value::Array(a) => !a.is_empty(),
            Value::Object(m) => !m.is_empty(),
            Value::Null => false,
        }
    }

    pub fn to_value(&self) -> IrValue {
        match self {
            Value::Int(i) => IrValue::Int(*i),
            Value::Float(f) => IrValue::Float(*f),
            Value::Bool(b) => IrValue::Bool(*b),
            Value::Str(s) => IrValue::Str(s.clone()),
            Value::Symbol(s) => IrValue::Symbol(s.clone()),
            Value::Array(a) => IrValue::Array(a.iter().map(|rv| rv.to_value()).collect()),
            Value::Object(m) => IrValue::Object(m.iter().map(|(k, v)| (k.clone(), v.to_value())).collect()),
            Value::Null => IrValue::Null,
        }
    }
}