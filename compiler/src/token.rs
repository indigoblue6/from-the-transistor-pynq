//! PynqCのトークン定義。

use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Integer(i32),
    Character(u8),
    StringLiteral(Vec<u8>),
    Int,
    Char,
    Void,
    If,
    Else,
    While,
    For,
    Return,
    Break,
    Continue,
    Sizeof,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    AmpAmp,
    PipePipe,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,
    AmpEqual,
    PipeEqual,
    CaretEqual,
    ShiftLeftEqual,
    ShiftRightEqual,
    PlusPlus,
    MinusMinus,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Eof,
}

impl TokenKind {
    pub fn name(&self) -> String {
        match self {
            Self::Ident(s) => format!("識別子 `{s}`"),
            Self::Integer(n) => format!("整数 `{n}`"),
            Self::Character(_) => "文字".into(),
            Self::StringLiteral(_) => "文字列".into(),
            Self::Eof => "入力末尾".into(),
            other => format!("`{other:?}`"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
