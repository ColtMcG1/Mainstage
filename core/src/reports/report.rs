use crate::reports::locations;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents the severity level of a report.
/// This enum is useful for categorizing reports based on their importance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// Indicates a critical error that requires immediate attention.
    Critical,
    /// Indicates a recoverable error.
    Error,
    /// Indicates a warning that may not require immediate action.
    Warning,
    /// Indicates an informational message.
    Info,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            Level::Critical => "CRITICAL",
            Level::Error => "ERROR",
            Level::Warning => "WARNING",
            Level::Info => "INFO",
        };
        write!(f, "{}", level_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Report<'a> {
    /// A unique identifier for the report.
    /// This can be used to track and reference specific reports.
    /// - It is generated automatically when the report is created.
    pub id: u128,
    /// The timestamp when the report was generated.
    /// The timestamp when the report was generated.
    /// This is represented as a UTC datetime.
    /// - It is generated automatically when the report is created.
    pub timestamp: DateTime<Utc>,
    /// This helps in categorizing the report for appropriate handling.
    pub level: Level,
    /// The main message of the report.
    pub message: String,
    /// The entity that issued the report.
    /// This could be a module name, function name, or any identifier relevant to the context.
    pub issuer: Option<String>,
    /// The span of code that the report pertains to.
    pub span: Option<locations::Span>,
    /// The location in the source file where the report was generated.
    pub location: Option<locations::Location<'a>>,
}

impl<'a> Report<'a> {
    /// Creates a new `Report` with default values.
    /// This can be used as a base template for creating custom reports.
    pub fn new(
        level: Level,
        message: String,
        issuer: Option<String>,
        span: Option<locations::Span>,
        location: Option<locations::Location<'a>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
            timestamp: Utc::now(),
            level,
            message,
            issuer,
            span,
            location,
        }
    }

    /// Allows overriding the `issuer` field of the `Report`.
    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.issuer = Some(issuer);
        self
    }

    /// Allows overriding the `span` field of the `Report`.
    pub fn with_span(mut self, span: locations::Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Allows overriding the `location` field of the `Report`.
    pub fn with_location(mut self, location: locations::Location<'a>) -> Self {
        self.location = Some(location);
        self
    }

    /// Creates a premade `Report` for a critical error.
    ///
    /// # Arguments
    /// * `message` - The critical error message.
    ///
    /// # Returns
    /// * A `Report` instance with a critical level and the provided message.
    pub fn critical_error(message: &str) -> Self {
        Self::new(Level::Critical, message.to_string(), None, None, None)
    }

    /// Creates a premade `Report` for a generic error.
    ///
    /// # Arguments
    /// * `message` - The error message.
    pub fn error(message: &str) -> Self {
        Self::new(Level::Error, message.to_string(), None, None, None)
    }

    /// Creates a premade `Report` for a warning.
    ///
    /// # Arguments
    /// * `message` - The warning message.
    ///
    /// # Returns
    /// * A `Report` instance with a warning level and the provided message.
    pub fn warning(message: &str) -> Self {
        Self::new(Level::Warning, message.to_string(), None, None, None)
    }

    /// Creates a premade `Report` for informational purposes.
    ///
    /// # Arguments
    /// * `message` - The informational message.
    ///
    /// # Returns
    /// * A `Report` instance with an info level and the provided message.
    pub fn info(message: &str) -> Self {
        Self::new(Level::Info, message.to_string(), None, None, None)
    }
}
