//! ./scripts/script.rs
//! 
//! Module for managing scripts and their associated metadata.
//! This module provides the `Script` struct and related functionality.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

use std::fmt;
use crate::reports::locations;

use crate::scripts::map;

/// Represents a script with its name, content, and associated `Map`.
/// This struct is useful for managing source files and their metadata.
/// 
/// # Examples
/// ```
/// let script = Script::new("example.js".into(), "console.log('Hello, world!');".into());
/// ```
#[derive(Clone)]
pub struct Script {
    /// The name of the script.
    pub name: String,
    /// Path of the script file.
    pub path: std::path::PathBuf,
    /// The content of the script.
    pub content: String,
    /// The associated `Map` for the script.
    pub map: map::Map,
}

impl Script {
    // --- Constructors ---

    /// Creates a new `Script` instance with the given name and content.
    /// 
    /// # Arguments
    /// * `name` - The name of the script.
    /// * `content` - The content of the script.
    pub fn new(name: String, path: std::path::PathBuf, content: String) -> Self {
        let map = map::Map::new(content.clone());
        Self { name, path, content, map }
    }

    /// Creates a `Script` instance from a file path.
    /// 
    /// # Arguments
    /// * `path` - The path to the script file.
    /// 
    /// # Returns
    /// * `Ok(Script)` if the file is successfully read.
    /// * `Err(std::io::Error)` if there is an error reading the file.
    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let name = path.to_string_lossy().to_string();
        Ok(Self::new(name, path.to_path_buf(), content))
    }

    /// Creates a `Script` instance from a `PathBuf`.
    /// 
    /// # Arguments
    /// * `path` - The `PathBuf` to the script file.
    /// 
    /// # Returns
    /// * `Ok(Script)` if the file is successfully read.
    /// * `Err(std::io::Error)` if there is an error reading the file.
    pub fn from_path_buf(path: std::path::PathBuf) -> std::io::Result<Self> {
        Self::from_path(&path)
    }

    /// Creates a `Script` instance from a name and content as string slices.
    /// 
    /// # Arguments
    /// * `name` - The name of the script.
    /// * `content` - The content of the script.
    pub fn from_str(name: &str, content: &str) -> Self {
        Self::new(name.to_string(), std::path::PathBuf::from(name), content.to_string())
    }

    /// Creates a `Script` instance from a name and content as `String`.
    /// 
    /// # Arguments
    /// * `name` - The name of the script.
    /// * `content` - The content of the script.
    pub fn from_string(name: String, content: String) -> Self {
        Self::new(name.clone(), std::path::PathBuf::from(name), content)
    }

    /// Creates a `Script` instance from a file and assigns a custom name.
    /// 
    /// # Arguments
    /// * `name` - The custom name for the script.
    /// * `path` - The path to the script file.
    /// 
    /// # Returns
    /// * `Ok(Script)` if the file is successfully read.
    /// * `Err(std::io::Error)` if there is an error reading the file.
    pub fn from_file(name: &str, path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::new(name.to_string(), path.to_path_buf(), content))
    }

    // --- Setters/Modifiers ---

    /// Updates the content of the script and regenerates the associated `Map`.
    /// 
    /// # Arguments
    /// * `content` - The new content for the script.
    /// 
    /// # Returns
    /// * The updated `Script` instance.
    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self.map = map::Map::new(self.content.clone());
        self
    }

    /// Updates the name of the script.
    /// 
    /// # Arguments
    /// * `name` - The new name for the script.
    /// 
    /// # Returns
    /// * The updated `Script` instance.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// Updates the path of the script.
    ///
    /// # Arguments
    /// * `path` - The new path for the script.
    ///
    /// # Returns
    /// * The updated `Script` instance.
    pub fn with_path(mut self, path: std::path::PathBuf) -> Self {
        self.path = path;
        self
    }

    // --- Getters ---

    /// Returns the name of the script.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path of the script.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Returns the content of the script.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns a reference to the associated `Map`.
    pub fn map(&self) -> &map::Map {
        &self.map
    }

    /// Returns the length of the script content.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Checks if the script content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns the number of lines in the script content.
    pub fn lines(&self) -> usize {
        self.map.lines.len()
    }

    // --- Utility Functions ---

    /// Converts a byte index in the script content to a `Location`.
    /// 
    /// # Arguments
    /// * `byte_index` - The byte index to convert.
    /// 
    /// # Returns
    /// * `Some(Location)` if the byte index is valid.
    /// * `None` if the byte index is invalid.
    pub fn location<'a>(&'a self, byte_index: usize) -> Option<locations::Location<'a>> {
        if byte_index > self.content.len() {
            return None;
        }
        let line = match self.map.lines.binary_search(&byte_index) {
            Ok(line) => line + 1,
            Err(line) => line,
        };
        let line_start = self.map.lines.get(line - 1).cloned().unwrap_or(0);
        let column = byte_index - line_start + 1;
        Some(locations::Location::new(line, column, self.name.clone()))
    }

    /// Converts a `Span` to a pair of `Location`s representing the start and end.
    /// 
    /// # Arguments
    /// * `span` - The `Span` to convert.
    /// 
    /// # Returns
    /// * `Some((Location, Location))` if the span is valid.
    /// * `None` if the span is invalid.
    pub fn span_location<'a>(&'a self, span: locations::Span) -> Option<(locations::Location<'a>, locations::Location<'a>)> {
        let start = self.location(span.start)?;
        let end = self.location(span.end)?;
        Some((start, end))
    }

    /// Returns the byte index of the start of a specific line.
    /// 
    /// # Arguments
    /// * `line` - The line number (1-based).
    /// 
    /// # Returns
    /// * `Some(usize)` if the line exists.
    /// * `None` if the line does not exist.
    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.map.lines.get(line - 1).cloned()
    }

    /// Returns the byte index of the end of a specific line.
    /// 
    /// # Arguments
    /// * `line` - The line number (1-based).
    /// 
    /// # Returns
    /// * `Some(usize)` if the line exists.
    /// * `None` if the line does not exist.
    pub fn line_end(&self, line: usize) -> Option<usize> {
        if line == self.map.lines.len() {
            Some(self.content.len())
        } else {
            self.map.lines.get(line).cloned()
        }
    }

    /// Returns the content of a specific line.
    /// 
    /// # Arguments
    /// * `line` - The line number (1-based).
    /// 
    /// # Returns
    /// * `Some(&str)` if the line exists.
    /// * `None` if the line does not exist.
    pub fn line_content(&self, line: usize) -> Option<&str> {
        self.map.line_content(&self.content, line)
    }

    /// Returns the content of a range of lines.
    /// 
    /// # Arguments
    /// * `start_line` - The starting line number (1-based).
    /// * `end_line` - The ending line number (1-based).
    /// 
    /// # Returns
    /// * `Some(&str)` if the range is valid.
    /// * `None` if the range is invalid.
    pub fn lines_content(&self, start_line: usize, end_line: usize) -> Option<&str> {
        self.map.lines_content(&self.content, start_line, end_line)
    }

    /// Returns all lines of the script content as a vector of strings.
    pub fn all_lines(&mut self) -> Vec<&str> {
        self.map.all_lines(&self.content).iter().map(String::as_str).collect()
    }

    // --- Miscellaneous ---

    /// Prints the script content with line numbers.
    pub fn print(&mut self) {
        println!("Script: {}", self.name);
        for (i, line) in self.all_lines().iter().enumerate() {
            println!("{:4} | {}", i + 1, line);
        }
    }
}

/// Implements the `Debug` trait for `Script` for easier debugging and logging.
impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script")
            .field("name", &self.name)
            .field("content", &self.content)
            .finish()
    }
}