//! ./reports/locations.rs
//! 
//! This module defines structures and functions for representing and manipulating locations in source files.
//! It includes definitions for spans and locations, which are essential for reporting errors and warnings in source code.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-14

use std::borrow::Cow;

/// Represents a span in a source file with start and end byte indices.
/// The `start` is inclusive and the `end` is exclusive.
/// For example, a span with `start = 0` and `end = 5` covers the first five bytes of the file.
/// This struct is useful for pinpointing exact locations in source code for reporting errors or warnings.
/// # Examples
/// ```
/// let span = Span { start: 0, end: 5 };
/// assert_eq!(span.start, 0);
/// assert_eq!(span.end, 5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Creates a new `Span` from the given start and end byte indices.
    /// # Arguments
    /// * `start` - The starting byte index (inclusive).
    /// * `end` - The ending byte index (exclusive).
    /// # Returns
    /// * A new `Span` instance.
    /// # Panics
    /// * Panics if `end` is less than `start`.
    pub fn new(start: usize, end: usize) -> Self {
        assert!(end >= start, "end must be greater than or equal to start");
        Self { start, end }
    }

    /// Checks if the given byte index is within the span.
    /// # Arguments
    /// * `index` - The byte index to check.
    /// # Returns
    /// * `true` if the index is within the span, `false` otherwise.
    pub fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }

    /// Returns the length of the span.
    /// # Returns
    /// * The length of the span.
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

/// Provides a default implementation for the `Span`.
/// The default span has both `start` and `end` set to `0`.
/// # Examples
/// ```
/// let span = Span::default();
/// assert_eq!(span.start, 0);
/// assert_eq!(span.end, 0);
/// ```
impl Default for Span {
    fn default() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Implements ordering for `Span` based on the `start` index.
/// # Examples
/// ```
/// let span1 = Span::new(0, 5);
/// let span2 = Span::new(5, 10);
/// assert!(span1 < span2);
/// ```
impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.start.cmp(&other.start))
    }
}

/// Implements total ordering for `Span` based on the `start` index.
/// # Examples
/// ```
/// let span1 = Span::new(0, 5);
/// let span2 = Span::new(5, 10);
/// assert!(span1 < span2);
/// ```
impl Ord for Span {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

/// Implements the `Display` trait for `Span` to provide a human-readable representation.
/// # Examples
/// ``` 
/// let span = Span::new(0, 5);
/// assert_eq!(format!("{}", span), "0-5");
/// ```
impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

/// Represents a specific location in a source file.
/// This includes the file name, line number, and column number.
/// This struct is useful for reporting the exact location of errors or warnings in source code.
/// # Examples
/// ```
/// let location = Location::new(1, 1, "example.rs".to_string());
/// assert_eq!(format!("{}", location), "example.rs:1:1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location<'a> {
    /// The name of the source file.
    pub file: Cow<'a, str>,
    /// The line number in the source file (1-based).
    pub line: usize,
    /// The column number in the source file (1-based).
    pub column: usize,
}

impl<'a> Location<'a> {
    /// Creates a new `Location` with the given line, column, and file name.
    /// # Arguments
    /// * `line` - The line number in the source file (1-based).
    /// * `column` - The column number in the source file (1-based).
    /// * `file` - The name of the source file.
    /// # Returns
    /// * A new `Location` instance.
    pub fn new<S: Into<Cow<'a, str>>>(line: usize, column: usize, file: S) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
    
    /// Creates a new `Location` with the given line and column, and an empty file name.
    /// # Arguments
    /// * `line` - The line number in the source file (1-based).
    /// * `column` - The column number in the source file (1-based).
    pub fn from_line_and_column(line: usize, column: usize) -> Self {
        Self {
            file: Cow::Borrowed(""),
            line,
            column,
        }
    }

    /// Sets the file name for the `Location`.
    /// # Arguments
    /// * `file` - The name of the source file.
    pub fn with_file<'b>(self, file: Cow<'b, str>) -> Location<'b> {
        Location {
            file,
            line: self.line,
            column: self.column,
        }
    }

    pub fn into_owned(self) -> Location<'static> {
        Location {
            file: Cow::Owned(self.file.into_owned()),
            line: self.line,
            column: self.column,
        }
    }
}

/// Provides a default implementation for the `Location`.
/// The default location has an empty file name and both line and column set to `1`.
/// # Examples
/// ```
/// let location = Location::default();
/// assert_eq!(location.file, "");
/// assert_eq!(location.line, 1);
/// assert_eq!(location.column, 1);
/// ```
impl Default for Location<'_> {
    fn default() -> Self {
        Self {
            file: Cow::Borrowed(""),
            line: 1,
            column: 1,
        }
    }
}

/// Implements the `Display` trait for `Location` to provide a human-readable representation.
/// # Examples
/// ```
/// let location = Location::new(1, 1, "example.rs".to_string());
/// assert_eq!(format!("{}", location), "example.rs:1:1");
/// ```
impl std::fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}