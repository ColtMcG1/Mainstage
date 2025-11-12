//! ./semantic/builtins.rs
//!
//! Builtin functions and helpers for arity/type rules.

/// Checks if the given name is a recognized builtin function.
/// # Parameters
/// - `name`: The name of the function to check.
/// # Returns
/// - `true` if the name corresponds to a builtin function, `false` otherwise.
pub fn is_builtin(name: &str) -> bool {
    matches!(name, "say" | "ask" | "read" | "write")
}

/// Checks if the given name is a value-returning builtin.
/// # Parameters
/// - `name`: The name of the builtin function.
/// # Returns
/// - `true` if the builtin returns a value, `false` otherwise.
pub fn is_value_builtin(name: &str) -> bool {
    matches!(name, "ask" | "read")
}

/// Gets the expected arity for a builtin function.
/// # Parameters
/// - `name`: The name of the builtin function.
/// # Returns
/// - `Some(n)` if the expected number of arguments for the builtin function is known.
/// - `None` if the arity is variable or unknown.
pub fn expected_arity(name: &str) -> Option<usize> {
    match name {
        "say" => Some(1),
        "read" => Some(1),
        "write" => Some(2),
        _ => None, // ask is 0 or 1, handle separately
    }
}