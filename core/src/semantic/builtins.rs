//! ./semantic/builtins.rs
//!
//! Builtin functions and helpers for arity/type rules.

pub fn is_builtin(name: &str) -> bool {
    matches!(name, "say" | "ask" | "read" | "write")
}

pub fn is_value_builtin(name: &str) -> bool {
    matches!(name, "ask" | "read")
}

// Optional: arity helpers if you prefer centralizing these rules
pub fn expected_arity(name: &str) -> Option<usize> {
    match name {
        "say" => Some(1),
        "read" => Some(1),
        "write" => Some(2),
        _ => None, // ask is 0 or 1, handle separately
    }
}