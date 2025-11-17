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
    /// A floating-point type.
    Float,
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

/// Internal inference lattice used during analysis. Not directly exposed.
/// Represents the inferred type of an expression or symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredType {
    /// An integer type. A whole number.
    Int,
    /// A floating-point type. A number with a fractional part.
    Float,
    /// A boolean type. A true/false value.
    Bool,
    /// A string type. A string of Unicode characters.
    Str,
    /// An array type. An array of values. All elements are of the same type.
    Array,
    /// A unit type. Represents a value that carries no information.
    Unit,
    /// An unknown type. The type could not be inferred.
    Unknown,
}

impl InferredType {
    /// Converts an `InferredType` to a `SymbolType`.
    /// # Returns
    /// - The corresponding `SymbolType`.
    pub fn to_symbol_type(self) -> SymbolType {
        match self {
            InferredType::Int => SymbolType::Integer,
            InferredType::Float => SymbolType::Float,
            InferredType::Bool => SymbolType::Boolean,
            InferredType::Str => SymbolType::String,
            InferredType::Array => SymbolType::Array,
            InferredType::Unit => SymbolType::None,
            InferredType::Unknown => SymbolType::None,
        }
    }
}