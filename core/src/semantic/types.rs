//! ./semantic/types.rs
//!
//! InferredType, SymbolType, and related conversions.

/// The type of the symbol.
/// Indicates the data type of the symbol, such as integer, string, boolean, array, etc.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolType {
    /// No type.
    None,
    /// An integer type.
    Integer,
    /// A string type.
    String,
    /// An array type.
    Array,
    /// A shell command type.
    ShellCommand,
    /// A boolean type.
    Boolean,
    /// A void type (not used for values).
    Void,
}

// Internal inference lattice used during analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredType {
    Int,
    Bool,
    Str,
    Array,
    Unit,
    Unknown,
}

impl InferredType {
    pub fn to_symbol_type(self) -> SymbolType {
        match self {
            InferredType::Int => SymbolType::Integer,
            InferredType::Bool => SymbolType::Boolean,
            InferredType::Str => SymbolType::String,
            InferredType::Array => SymbolType::Array,
            InferredType::Unit => SymbolType::None,
            InferredType::Unknown => SymbolType::None,
        }
    }
}