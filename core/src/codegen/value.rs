use crate::parser::{AstType};
use crate::parser::ast::AstNode;

#[derive(Debug, Clone, PartialEq)]
pub enum OpValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<OpValue>),
    Unit,
    Null,
}

impl OpValue {
    /// Returns true if the value is a scalar (int, bool, str, unit, null)
    pub fn is_scalar(&self) -> bool {
        matches!(self, OpValue::Int(_) | OpValue::Bool(_) | OpValue::Str(_) | OpValue::Unit | OpValue::Null)
    }

    /// Returns the truthiness of the value
    pub fn is_truthy(&self) -> bool {
        match self {
            OpValue::Bool(b) => *b,
            OpValue::Int(i) => *i != 0,
            OpValue::Float(f) => *f != 0.0,
            OpValue::Str(s) => !s.is_empty(),
            OpValue::Array(a) => !a.is_empty(),
            OpValue::Unit => false,
            OpValue::Null => false,
        }
    }
}

impl std::fmt::Display for OpValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpValue::Int(i) => write!(f, "{i}"),
            OpValue::Float(fl) => write!(f, "{fl}"),
            OpValue::Bool(b) => write!(f, "{b}"),
            OpValue::Str(s) => write!(f, "\"{s}\""),
            OpValue::Array(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            OpValue::Unit => write!(f, "unit"),
            OpValue::Null => write!(f, "null"),
        }
    }
}

impl From<i64> for OpValue { fn from(v: i64) -> Self { OpValue::Int(v) } }
impl From<bool> for OpValue { fn from(v: bool) -> Self { OpValue::Bool(v) } }
impl From<String> for OpValue { fn from(v: String) -> Self { OpValue::Str(v) } }
impl From<&str> for OpValue { fn from(v: &str) -> Self { OpValue::Str(v.to_string()) } }

impl TryFrom<&AstType<'_>> for OpValue {
    type Error = ();
    fn try_from(ast: &AstType<'_>) -> Result<Self, ()> {
        match ast {
            AstType::Integer { value } => Ok(OpValue::Int(*value)),
            AstType::Float { value } => Ok(OpValue::Float(*value)),
            AstType::Bool { value } => Ok(OpValue::Bool(*value)),
            AstType::Str { value } => Ok(OpValue::Str(value.to_string())),
            AstType::Null => Ok(OpValue::Null),
            AstType::Array => Ok(OpValue::Array(Vec::new())), // caller fills children
            _ => Err(()),
        }
    }
}

pub fn literal_from_node(node: &AstNode) -> Option<OpValue> {
    match &node.kind {
        AstType::Array => {
            let mut elems = Vec::with_capacity(node.children.len());
            for child in &node.children {
                if let Some(v) = literal_from_node(child) {
                    elems.push(v);
                } else {
                    return None; // non-const element breaks array constness
                }
            }
            Some(OpValue::Array(elems))
        }
        AstType::Integer { value } => Some(OpValue::Int(*value)),
        AstType::Float { value } => Some(OpValue::Float(*value)),
        AstType::Bool { value } => Some(OpValue::Bool(*value)),
        AstType::Str { value } => Some(OpValue::Str(value.to_string())),
        AstType::Null => Some(OpValue::Null),
        _ => None,
    }
}