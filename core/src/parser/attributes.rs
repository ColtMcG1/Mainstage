//! ./parser/attributes.rs
//! description: Module for representing attributes in AST nodes.
//! 
//! This module provides functionality to create and manage attributes associated with AST nodes.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025/10/25
//! license: See LICENSE file in repository root.

/// Represents an attribute associated with an AST node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name of the attribute.
    pub name: String,
    /// The value of the attribute.
    pub value: String,
}

impl Attribute {
    /// Creates a new `Attribute`.
    /// # Arguments
    /// * `name` - The name of the attribute.
    /// * `value` - The value of the attribute.
    /// # Examples
    /// ```
    /// let attr = Attribute::new("key".to_string(), "value".to_string());
    /// ```
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    /// Returns a reference to the name of the attribute.
    /// # Returns
    /// * A reference to the name of the attribute.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the value of the attribute.
    /// # Returns
    /// * A reference to the value of the attribute.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns a string representation of the attribute in the format `name="value"`.
    /// # Returns
    /// * A string representation of the attribute.
    pub fn to_string(&self) -> String {
        format!("{}=\"{}\"", self.name, self.value)
    }
}