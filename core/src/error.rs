use std::fmt;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match self {
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        };
        write!(f, "{}", level_str)
    }
}

pub trait MainstageErrorExt {
    fn level(&self) -> Level;
    fn message(&self) -> String;
    fn issuer(&self) -> String;
    fn span(&self) -> Option<crate::location::Span>;
    fn location(&self) -> Option<crate::location::Location>;
}

impl fmt::Debug for dyn MainstageErrorExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format location safely without assuming a Default impl for Location.
        let loc_str = match self.location() {
            Some(loc) => {
                // Location fields are formatted generically to avoid tight coupling.
                format!("{}:{}:{}", loc.file, loc.line, loc.column)
            }
            None => "unknown".to_string(),
        };

        // Span presence only; avoid referencing unknown span internals.
        let span_str = if self.span().is_some() { self.span().unwrap().to_string() } else { "span:none".to_string() };

        write!(
            f,
            "MAINSTAGE | {} | {} | {} | {} | {}",
            self.level(),
            loc_str,
            self.issuer(),
            span_str,
            self.message()
        )
    }
}

impl fmt::Display for dyn MainstageErrorExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Reuse Debug output for a consistent, human-friendly representation.
        write!(f, "{:?}", self)
    }
}