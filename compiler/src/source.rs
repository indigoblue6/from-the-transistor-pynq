//! ソース本文と位置情報を管理する。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn join(self, other: Span) -> Self {
        Self::new(self.start, other.end)
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub text: String,
}

impl Source {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }

    pub fn line_column(&self, offset: usize) -> (usize, usize, &str) {
        let offset = offset.min(self.text.len());
        let before = &self.text[..offset];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let line_end = self.text[offset..]
            .find('\n')
            .map_or(self.text.len(), |n| offset + n);
        let column = self.text[line_start..offset].chars().count() + 1;
        (line, column, &self.text[line_start..line_end])
    }
}
