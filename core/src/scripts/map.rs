//! ./scripts/map.rs
//!
//! Module for handling scripts, including their content, names, and associated mappings.
//! This module provides the `Map` struct and related functionality.
//!
//! author: Colton McGraw <htrtps://github.com/ColtMcG1>
//! date: 2025-10-14

/// Represents a source file with its content and line indices.
/// This struct is useful for mapping byte indices to line and column numbers.
///
/// # Examples
/// ```
/// let map = Map {
///     file_name: "example.rs".into(),
///     content: "fn main() {\n    println!(\"Hello, world!\");\n}".into(),
///     lines: vec![0, 22, 43],
/// };
/// ```
/// The `lines` vector contains the byte indices where each line starts.
#[derive(Debug, Clone)]
pub struct Map {
    /// A vector containing the byte indices where each line starts.
    pub lines: Vec<usize>,
    /// Optional cached lines of the content for quick access. Internal use.
    cached_lines: Option<Vec<String>>,
}

impl Map {
    /// Creates a new `Map` from the given file name and content.
    /// Automatically computes the line start indices.
    ///
    /// # Arguments
    /// * `file_name` - The name of the source file.
    /// * `content` - The content of the source file.
    ///
    /// # Returns
    /// * A new `Map` instance with computed line indices.
    ///
    /// # Examples
    /// ```
    /// let map = Map::new("example.rs".into(), "fn main() {\n    println!(\"Hello, world!\");\n}".into());
    /// assert_eq!(map.lines, vec![0, 12, 43]);
    /// ```
    pub fn new(content: String) -> Self {
        let mut lines = vec![0];
        for (i, c) in content.char_indices() {
            if c == '\n' {
                lines.push(i + 1);
            }
        }
        Self {
            lines,
            cached_lines: None,
        }
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.lines.get(line - 1).cloned()
    }

    pub fn line_end(&self, line: usize, content_len: usize) -> Option<usize> {
        if line == self.lines.len() {
            Some(content_len)
        } else {
            self.lines.get(line).cloned()
        }
    }

    pub fn line_content<'a>(&self, content: &'a str, line: usize) -> Option<&'a str> {
        let start = self.line_start(line)?;
        let end = self.line_end(line, content.len())?;
        content.get(start..end)
    }

    pub fn lines_content<'a>(&self, content: &'a str, start_line: usize, end_line: usize) -> Option<&'a str> {
        let start = self.line_start(start_line)?;
        let end = self.line_end(end_line, content.len())?;
        content.get(start..end)
    }

    pub fn all_lines(&mut self, content: &str) -> &Vec<String> {
        if self.cached_lines.is_none() {
            let mut lines = Vec::new();
            for i in 0..self.lines.len() {
                let start = self.lines[i];
                let end = if i + 1 < self.lines.len() {
                    self.lines[i + 1]
                } else {
                    content.len()
                };
                if let Some(line) = content.get(start..end) {
                    lines.push(line.to_string());
                }
            }
            self.cached_lines = Some(lines);
        }
        self.cached_lines.as_ref().unwrap()
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_lines = None;
    }
}
