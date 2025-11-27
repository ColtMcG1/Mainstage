use crate::location;

use super::kind::AstNodeKind;

#[derive(Clone, PartialEq)]
pub struct AstNode {
    id: usize,
    pub kind: AstNodeKind,
    pub location: Option<location::Location>,
    pub span: Option<location::Span>,
}

impl AstNode {

    fn create_id() -> usize {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn new(
        node_type: AstNodeKind,
        location: Option<location::Location>,
        span: Option<location::Span>,
    ) -> Self {
        AstNode {
            id: Self::create_id(),
            kind: node_type,
            location,
            span,
        }
    }

    pub fn with_location(mut self, location: crate::location::Location) -> Self {
        self.location = Some(location);
        self
    }
    pub fn with_span(mut self, span: crate::location::Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
    pub fn get_kind(&self) -> &AstNodeKind {
        &self.kind
    }
    pub fn get_location(&self) -> Option<&crate::location::Location> {
        self.location.as_ref()
    }
    pub fn get_span(&self) -> Option<&crate::location::Span> {
        self.span.as_ref()
    }
}

use std::fmt;

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_indent(f: &mut fmt::Formatter<'_>, s: &str, indent: usize) -> fmt::Result {
            for _ in 0..indent { write!(f, " ")?; }
            write!(f, "{}", s)
        }

        // Header
        writeln!(f, "AstNode {{")?;
        fmt_indent(f, &format!("id: {},\n", self.id), 2)?;
        // Kind with pretty debug (allows readable nested enums/vecs)
        fmt_indent(f, "kind: ", 2)?;
        writeln!(f, "{:#?},", &self.kind)?;

        // Location
        if let Some(loc) = &self.location {
            fmt_indent(
                f,
                &format!("location: {}:{}:{}\n", loc.file, loc.line, loc.column),
                2,
            )?;
        } else {
            fmt_indent(f, "location: None\n", 2)?;
        }

        // Span
        if let Some(span) = &self.span {
            // Adjust to match your Span fields
            fmt_indent(
                f,
                &format!(
                    "span: start={}:{}:{} end={}:{}:{}\n",
                    span.start.file,
                    span.start.line,
                    span.start.column,
                    span.end.file,
                    span.end.line,
                    span.end.column
                ),
                2,
            )?;
        } else {
            fmt_indent(f, "span: None\n", 2)?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Debug for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to Display so both "{}" and "{:?}" are pretty
        write!(f, "{}", self)
    }
}