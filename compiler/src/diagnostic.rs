//! 利用者向けの診断メッセージを構築する。

use crate::source::{Source, Span};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lex,
    Parse,
    Name,
    Type,
    Codegen,
    Tool,
}

impl DiagnosticKind {
    fn label(self) -> &'static str {
        match self {
            Self::Lex => "字句解析",
            Self::Parse => "構文解析",
            Self::Name => "名前解決",
            Self::Type => "型",
            Self::Codegen => "コード生成",
            Self::Tool => "外部ツール",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Span,
    pub message: String,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(kind: DiagnosticKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
            help: None,
        }
    }
    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.help = Some(message.into());
        self
    }
    pub fn render(&self, source: &Source) -> String {
        let (line, column, text) = source.line_column(self.span.start);
        let width = self.span.end.saturating_sub(self.span.start).max(1);
        let mut result = format!(
            "{}:{}:{}: エラー[{}]: {}\n\n    {}\n    {}{}",
            source.name,
            line,
            column,
            self.kind.label(),
            self.message,
            text,
            " ".repeat(column.saturating_sub(1)),
            "^".repeat(width.min(text.len().saturating_sub(column - 1)).max(1))
        );
        if let Some(help) = &self.help {
            result.push_str(&format!("\n\n補足: {help}"));
        }
        result
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Diagnostic {}

pub type CompileResult<T> = Result<T, Diagnostic>;
