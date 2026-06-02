/// A byte-offset span in source code, used for error reporting.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type, schemars::JsonSchema,
)]
pub struct Span {
    /// Byte offset of the start (inclusive).
    pub start: usize,
    /// Byte offset of the end (exclusive).
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a span that covers both `self` and `other`.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
