// Core module for reporting errors and warnings in scripts
// This module provides structures and functions to create detailed reports
// including error messages, spans, and locations within the script.

use console::Style;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

/// Span type enumeration
/// Used to specify the type of span (line or file)
///
/// # Examples
/// ```
/// let span_type = SpanType::Line;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanType {
    /// Span is located relative to the line
    Line,
    /// Span is located relative to the entire file
    File,
}

/// Span of text in the script
/// Represents a contiguous range of text within the script.
/// Used for pinpointing errors or warnings.
///
/// # Examples
/// ```
/// let span = Span { start: 0, end: 5, span_type: SpanType::File };
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Span {
    /// Start index of the span (inclusive)
    pub start: usize,
    /// End index of the span (exclusive)
    pub end: usize,
    /// Type of span (line or file)
    pub span_type: SpanType,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, span_type: SpanType) -> Self {
        Span {
            start,
            end,
            span_type,
        }
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the span contains a specific index
    pub fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }

    /// Merge two spans into one that covers both
    pub fn merge(&self, other: &Span) -> Option<Span> {
        if self.span_type != other.span_type {
            return None;
        }
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        Some(Span {
            start,
            end,
            span_type: self.span_type,
        })
    }

    /// Check if two spans overlap
    pub fn overlaps(&self, other: &Span) -> bool {
        if self.span_type != other.span_type {
            return false;
        }
        self.start < other.end && other.start < self.end
    }

    /// Get a string representation of the span
    pub fn to_string(&self) -> String {
        format!("Span({}, {}, {:?})", self.start, self.end, self.span_type)
    }

    /// ============================================================
    /// Getters for span fields

    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn span_type(&self) -> SpanType {
        self.span_type
    }

    /// ============================================================
    /// Setters for span fields

    pub fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    pub fn set_end(&mut self, end: usize) {
        self.end = end;
    }
    pub fn set_span_type(&mut self, span_type: SpanType) {
        self.span_type = span_type;
    }

    /// ============================================================
    /// Convenience methods for creating spans of specific types

    pub fn file_span(start: usize, end: usize) -> Self {
        Span {
            start,
            end,
            span_type: SpanType::File,
        }
    }
    pub fn line_span(start: usize, end: usize) -> Self {
        Span {
            start,
            end,
            span_type: SpanType::Line,
        }
    }
}

/// Location in the script (line and column)
/// Used for more human-readable error reporting.
///
/// # Examples
/// ```
/// let location = Location {
///     span: Span::new(0, 5),
///     line: 1,
///     column: 1,
///     file: "script.ms".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Span of text in the script
    pub span: Span,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// File name
    pub file: String,
}

impl Location {
    /// Create a new location
    pub fn new(span: Span, line: usize, column: usize, file: &str) -> Self {
        Location {
            span,
            line,
            column,
            file: file.to_string(),
        }
    }

    /// Get a string representation of the location
    pub fn to_string(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }

    /// ============================================================
    /// Getters for location fields

    pub fn span(&self) -> &Span {
        &self.span
    }
    pub fn line(&self) -> usize {
        self.line
    }
    pub fn column(&self) -> usize {
        self.column
    }
    pub fn file(&self) -> &str {
        &self.file
    }

    /// ============================================================
    /// Setters for location fields

    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
    pub fn set_line(&mut self, line: usize) {
        self.line = line;
    }
    pub fn set_column(&mut self, column: usize) {
        self.column = column;
    }
    pub fn set_file(&mut self, file: &str) {
        self.file = file.to_string();
    }

    /// ============================================================
    /// With builder pattern methods for constructing locations

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = line;
        self
    }
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }
    pub fn with_file(mut self, file: &str) -> Self {
        self.file = file.to_string();
        self
    }
}

/// Severity levels for reports
/// Used to categorize the importance of reports.
/// Severity levels for reports
/// Used to categorize the importance of reports.
///
/// # Examples
/// ```
/// let severity = Severity::Error;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Severity::Fatal => "FATAL",
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Info => "INFO",
        };
        write!(f, "{}", s)
    }
}

/// Optional stable error code for programmatic handling
/// Used to provide a machine-readable identifier for specific error types.
///
/// # Examples
/// ```
/// let code = ErrorCode("E001".to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode(pub u32);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ErrorCode {
    /// Create a new error code
    pub fn new(code: u32) -> Self {
        ErrorCode(code)
    }
    /// Get the code as a string
    pub fn as_str(&self) -> String {
        format!("E_{}", self.0)
    }
}

pub const E_NONE: ErrorCode = ErrorCode(0);
pub const E_IO: ErrorCode = ErrorCode(1);
pub const E_SYNTAX: ErrorCode = ErrorCode(2);
pub const E_RUNTIME: ErrorCode = ErrorCode(3);
pub const E_UNSUPPORTED: ErrorCode = ErrorCode(4);
pub const E_DEPRECATED: ErrorCode = ErrorCode(5);
pub const E_INTERNAL: ErrorCode = ErrorCode(999);

/// Report structure containing message, severity, and location
/// This structure is used to represent a report generated during script parsing.
/// It includes the error message, severity level, and optional location information.
/// Additional fields include an optional error code, suggestion, and tags for categorization.
/// # Examples
/// ```
/// let report = Report::new("An error occurred", Severity::Error, None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub message: String,
    pub severity: Severity,
    pub location: Option<Location>,
    pub code: Option<ErrorCode>,
    pub suggestion: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl Report {
    /// Create a new report
    pub fn new(
        message: &str,
        severity: Severity,
        location: Option<Location>,
        code: Option<ErrorCode>,
        suggestion: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Report {
            message: message.to_string(),
            severity,
            location,
            code,
            suggestion,
            tags,
        }
    }

    pub fn info(
        message: &str,
        location: Option<Location>,
        code: Option<ErrorCode>,
        suggestion: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Report::new(message, Severity::Info, location, code, suggestion, tags)
    }
    pub fn warning(
        message: &str,
        location: Option<Location>,
        code: Option<ErrorCode>,
        suggestion: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Report::new(message, Severity::Warning, location, code, suggestion, tags)
    }
    pub fn error(
        message: &str,
        location: Option<Location>,
        code: Option<ErrorCode>,
        suggestion: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Report::new(message, Severity::Error, location, code, suggestion, tags)
    }
    pub fn fatal(
        message: &str,
        location: Option<Location>,
        code: Option<ErrorCode>,
        suggestion: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Report::new(message, Severity::Fatal, location, code, suggestion, tags)
    }

    // convenience conversion to JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    // convert to a minimal LSP-like diagnostic (map structure)
    pub fn to_lsp_diagnostic(&self) -> serde_json::Value {
        let range = if let Some(loc) = &self.location {
            json!({
                "start": { "line": loc.line.saturating_sub(1), "character": loc.column.saturating_sub(1) },
                "end": { "line": loc.line.saturating_sub(1), "character": loc.column.saturating_sub(1) + loc.span.len() }
            })
        } else {
            json!(null)
        };
        json!({
            "severity": match self.severity {
                Severity::Fatal | Severity::Error => 1,
                Severity::Warning => 2,
                Severity::Info => 3,
            },
            "code": self.code.as_ref().map(|c| c.0.clone()),
            "source": "mainstage",
            "message": self.message,
            "range": range,
        })
    }

    /// Pretty-print with a source snippet and caret under the span (uses console crate styling)
    /// `source` should be the file contents where location refers to.
    pub fn pretty_with_source(&self, source: &str) {
        let header = Style::new().bold().red();
        let sev = match self.severity {
            Severity::Fatal => Style::new().on_red().white().bold(),
            Severity::Error => Style::new().red().bold(),
            Severity::Warning => Style::new().yellow().bold(),
            Severity::Info => Style::new().blue().bold(),
        };

        println!(
            "{} {}",
            sev.apply_to(format!("[{}]", self.severity)),
            header.apply_to(&self.message)
        );

        if let Some(loc) = &self.location {
            let file = &loc.file;
            println!(" --> {}:{}:{}", file, loc.line, loc.column);

            // extract line content (1-based lines)
            if let Some(line_str) = source.lines().nth(loc.line.saturating_sub(1)) {
                // print the source line
                println!(" {:4} | {}", loc.line, line_str);
                // compute caret position (column is 1-based; clamp)
                let col = loc.column.saturating_sub(1);
                let mut caret_line = String::new();
                caret_line.push_str("     | ");
                caret_line.push_str(&" ".repeat(col));
                // highlight caret span length
                let caret_len = loc.span.len().max(1);
                caret_line.push_str(&"^".repeat(caret_len));
                println!("{}", Style::new().green().apply_to(caret_line));
            }
        }

        if let Some(s) = &self.suggestion {
            println!(
                "{}",
                Style::new().cyan().apply_to(format!("Suggestion: {}", s))
            );
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loc = if let Some(l) = &self.location {
            format!(" at {}:{}:{}", l.file, l.line, l.column)
        } else {
            "".to_string()
        };
        if let Some(code) = &self.code {
            write!(f, "[{}]{} ({}): {}", self.severity, loc, code, self.message)
        } else {
            write!(f, "[{}]{}: {}", self.severity, loc, self.message)
        }
    }
}

impl Error for Report {}

/// Collector that aggregates reports, supports dedupe, sorting, counts and exporting
/// This structure collects multiple reports, ensuring no duplicates based on message and code.
/// It provides methods to add reports, check if empty, get counts by severity, and export
///
/// # Examples
/// ```
/// let mut collector = ReportCollector::new();
/// collector.push(Report::new("An error", Severity::Error, None, None, None, None));
/// ```
#[derive(Debug, Clone)]
pub struct ReportCollector {
    pub reports: Vec<Report>,
    pub seen: HashSet<(String, Option<String>)>, // simple dedupe key: (message, code)
}

impl ReportCollector {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            seen: HashSet::new(),
        }
    }

    pub fn push(&mut self, r: Report) {
        let key = (r.message.clone(), r.code.as_ref().map(|c| c.as_str()));
        if !self.seen.contains(&key) {
            self.seen.insert(key);
            self.reports.push(r);
        }
    }

    pub fn extend(&mut self, others: impl IntoIterator<Item = Report>) {
        for r in others {
            self.push(r);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }

    pub fn has_fatal(&self) -> bool {
        self.reports.iter().any(|r| r.severity == Severity::Fatal)
    }

    pub fn has_errors(&self) -> bool {
        self.reports.iter().any(|r| r.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.reports.iter().any(|r| r.severity == Severity::Warning)
    }

    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let mut f = 0;
        let mut e = 0;
        let mut w = 0;
        let mut i = 0;
        for r in &self.reports {
            match r.severity {
                Severity::Fatal => f += 1,
                Severity::Error => e += 1,
                Severity::Warning => w += 1,
                Severity::Info => i += 1,
            }
        }
        (f, e, w, i)
    }

    /// Get an appropriate exit code based on the reports collected
    /// 0 = no issues, 1 = warnings, 2 = errors/fatals
    pub fn exit_code(&self) -> i32 {
        let (f, e, _, _) = self.counts();
        if f > 0 {
            2
        } else if e > 0 {
            1
        } else {
            0
        }
    }

    pub fn print_all_pretty(&self, source_map: &impl Fn(&str) -> Option<&str>) {
        for r in &self.reports {
            if let Some(loc) = &r.location {
                let source = source_map(&loc.file).unwrap_or("");
                r.pretty_with_source(source);
            } else {
                println!("{}", r);
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self.reports)
    }

    pub fn to_lsp_array(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        // group by file
        for r in &self.reports {
            let file = r
                .location
                .as_ref()
                .map(|l| l.file.clone())
                .unwrap_or_else(|| "<unknown>".to_string());
            let entry = map.entry(file).or_insert_with(|| json!([]));
            if let serde_json::Value::Array(arr) = entry {
                arr.push(r.to_lsp_diagnostic());
            }
        }
        serde_json::Value::Object(map)
    }
}
