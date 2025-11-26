
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Location {
    /// The file in which the location is found.
    pub file: String,
    /// The line number of the location.
    pub line: usize,
    /// The column number of the location.
    pub column: usize,
}

impl Location {
    /// Creates a new `Location`.
    pub fn new(file: String, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Span {
    /// The starting location of the span.
    pub start: Location,
    /// The ending location of the span.
    pub end: Location,
}

impl Span {
    /// Creates a new `Span` from two `Location`s.
    pub fn new(start: Location, end: Location) -> Self {
        Self { start, end }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}