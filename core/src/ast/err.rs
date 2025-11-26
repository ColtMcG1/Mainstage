use crate::error::{Level, MainstageErrorExt};
use crate::location::{Location, Span};

#[derive(Debug, Clone)]
pub struct EmptyScriptError {
    level: Level,
    message: String,
    issuer: String,
    location: Option<Location>,
    span: Option<Span>,
}

impl EmptyScriptError {
    /// Default constructor for an empty-script error.
    pub fn new(issuer: String, location: Option<Location>, span: Option<Span>) -> Self {
        EmptyScriptError {
            level: Level::Error,
            message: "The provided script is empty.".to_string(),
            issuer,
            location,
            span,
        }
    }

    /// More explicit constructor when you need to set level/message.
    pub fn with(level: Level, message: String, issuer: String, location: Option<Location>, span: Option<Span>) -> Self {
        EmptyScriptError {
            level,
            message,
            issuer,
            location,
            span,
        }
    }
}

impl std::fmt::Display for EmptyScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "{} (at {}:{}:{})", self.message, loc.file, loc.line, loc.column)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for EmptyScriptError {}

impl MainstageErrorExt for EmptyScriptError {
    fn level(&self) -> Level {
        self.level
    }

    fn message(&self) -> String {
        self.message.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn span(&self) -> Option<Span> {
        self.span.clone()
    }

    fn location(&self) -> Option<Location> {
        self.location.clone()
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxError {
    level: Level,
    message: String,
    issuer: String,
    location: Option<Location>,
    span: Option<Span>,
}

impl SyntaxError {
    pub fn new(issuer: String, location: Option<Location>, span: Option<Span>) -> Self {
        SyntaxError {
            level: Level::Error,
            message: "There was a syntax error in the script.".to_string(),
            issuer: issuer.to_string(),
            location,
            span,
        }
    }

    pub fn with(level: Level, message: String, issuer: String, location: Option<Location>, span: Option<Span>) -> Self {
        SyntaxError {
            level,
            message,
            issuer,
            location,
            span,
        }
    }
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "{} (at {}:{}:{})", self.message, loc.file, loc.line, loc.column)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for SyntaxError {}

impl MainstageErrorExt for SyntaxError {
    fn level(&self) -> Level {
        self.level
    }

    fn message(&self) -> String {
        self.message.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn span(&self) -> Option<Span> {
        self.span.clone()
    }

    fn location(&self) -> Option<Location> {
        self.location.clone()
    }
}