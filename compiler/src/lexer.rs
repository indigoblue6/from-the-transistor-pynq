//! UTF-8ソースをPynqCトークン列へ変換する手書きLexer。

use crate::{
    diagnostic::{CompileResult, Diagnostic, DiagnosticKind},
    source::Span,
    token::{Token, TokenKind},
};

pub fn lex(text: &str) -> CompileResult<Vec<Token>> {
    Lexer { text, position: 0 }.run()
}

struct Lexer<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    fn run(mut self) -> CompileResult<Vec<Token>> {
        let mut tokens = Vec::new();
        while self.position < self.text.len() {
            self.skip_space_and_comments()?;
            if self.position >= self.text.len() {
                break;
            }
            tokens.push(self.next_token()?);
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.position, self.position),
        });
        Ok(tokens)
    }

    fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }
    fn starts(&self, value: &str) -> bool {
        self.bytes()[self.position..].starts_with(value.as_bytes())
    }

    fn skip_space_and_comments(&mut self) -> CompileResult<()> {
        loop {
            while self.position < self.text.len()
                && self.bytes()[self.position].is_ascii_whitespace()
            {
                self.position += 1;
            }
            if self.starts("//") {
                self.position += 2;
                while self.position < self.text.len() && self.bytes()[self.position] != b'\n' {
                    self.position += 1;
                }
            } else if self.starts("/*") {
                let start = self.position;
                self.position += 2;
                while self.position + 1 < self.text.len()
                    && !(self.bytes()[self.position] == b'*'
                        && self.bytes()[self.position + 1] == b'/')
                {
                    self.position += 1;
                }
                if self.position + 1 >= self.text.len() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Lex,
                        Span::new(start, start + 2),
                        "ブロックコメントが閉じられていません",
                    ));
                }
                self.position += 2;
            } else {
                return Ok(());
            }
        }
    }

    fn next_token(&mut self) -> CompileResult<Token> {
        let start = self.position;
        let byte = self.bytes()[self.position];
        if byte.is_ascii_alphabetic() || byte == b'_' {
            self.position += 1;
            while self.position < self.text.len()
                && (self.bytes()[self.position].is_ascii_alphanumeric()
                    || self.bytes()[self.position] == b'_')
            {
                self.position += 1;
            }
            let word = &self.text[start..self.position];
            let kind = match word {
                "int" => TokenKind::Int,
                "char" => TokenKind::Char,
                "void" => TokenKind::Void,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "for" => TokenKind::For,
                "return" => TokenKind::Return,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "sizeof" => TokenKind::Sizeof,
                _ => TokenKind::Ident(word.into()),
            };
            return Ok(Token {
                kind,
                span: Span::new(start, self.position),
            });
        }
        if byte.is_ascii_digit() {
            self.position += 1;
            let hex = byte == b'0'
                && self.position < self.text.len()
                && matches!(self.bytes()[self.position], b'x' | b'X');
            if hex {
                self.position += 1;
            }
            let digits = self.position;
            while self.position < self.text.len()
                && if hex {
                    self.bytes()[self.position].is_ascii_hexdigit()
                } else {
                    self.bytes()[self.position].is_ascii_digit()
                }
            {
                self.position += 1;
            }
            if hex && self.position == digits {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lex,
                    Span::new(start, self.position),
                    "16進整数に数字がありません",
                ));
            }
            let raw = if hex {
                &self.text[digits..self.position]
            } else {
                &self.text[start..self.position]
            };
            let value = u32::from_str_radix(raw, if hex { 16 } else { 10 }).map_err(|_| {
                Diagnostic::new(
                    DiagnosticKind::Lex,
                    Span::new(start, self.position),
                    "整数が32 bitに収まりません",
                )
            })?;
            return Ok(Token {
                kind: TokenKind::Integer(value as i32),
                span: Span::new(start, self.position),
            });
        }
        if byte == b'\'' {
            self.position += 1;
            let value = self.escaped_byte(b'\'', start)?;
            if self.position >= self.text.len() || self.bytes()[self.position] != b'\'' {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lex,
                    Span::new(start, self.position),
                    "文字リテラルが閉じられていません",
                ));
            }
            self.position += 1;
            return Ok(Token {
                kind: TokenKind::Character(value),
                span: Span::new(start, self.position),
            });
        }
        if byte == b'"' {
            self.position += 1;
            let mut value = Vec::new();
            while self.position < self.text.len() && self.bytes()[self.position] != b'"' {
                if self.bytes()[self.position] == b'\n' {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Lex,
                        Span::new(start, self.position),
                        "文字列リテラルが閉じられていません",
                    ));
                }
                value.push(self.escaped_byte(b'"', start)?);
            }
            if self.position >= self.text.len() {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lex,
                    Span::new(start, self.position),
                    "文字列リテラルが閉じられていません",
                ));
            }
            self.position += 1;
            return Ok(Token {
                kind: TokenKind::StringLiteral(value),
                span: Span::new(start, self.position),
            });
        }
        let operators: &[(&str, TokenKind)] = &[
            ("<<=", TokenKind::ShiftLeftEqual),
            (">>=", TokenKind::ShiftRightEqual),
            ("++", TokenKind::PlusPlus),
            ("--", TokenKind::MinusMinus),
            ("==", TokenKind::EqualEqual),
            ("!=", TokenKind::BangEqual),
            ("<=", TokenKind::LessEqual),
            (">=", TokenKind::GreaterEqual),
            ("<<", TokenKind::ShiftLeft),
            (">>", TokenKind::ShiftRight),
            ("&&", TokenKind::AmpAmp),
            ("||", TokenKind::PipePipe),
            ("+=", TokenKind::PlusEqual),
            ("-=", TokenKind::MinusEqual),
            ("*=", TokenKind::StarEqual),
            ("/=", TokenKind::SlashEqual),
            ("%=", TokenKind::PercentEqual),
            ("&=", TokenKind::AmpEqual),
            ("|=", TokenKind::PipeEqual),
            ("^=", TokenKind::CaretEqual),
        ];
        for (text, kind) in operators {
            if self.starts(text) {
                self.position += text.len();
                return Ok(Token {
                    kind: kind.clone(),
                    span: Span::new(start, self.position),
                });
            }
        }
        self.position += 1;
        let kind = match byte {
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'&' => TokenKind::Amp,
            b'|' => TokenKind::Pipe,
            b'^' => TokenKind::Caret,
            b'~' => TokenKind::Tilde,
            b'!' => TokenKind::Bang,
            b'=' => TokenKind::Equal,
            b'<' => TokenKind::Less,
            b'>' => TokenKind::Greater,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b';' => TokenKind::Semicolon,
            b',' => TokenKind::Comma,
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lex,
                    Span::new(start, self.position),
                    format!("不正な文字 `{}` です", byte as char),
                ))
            }
        };
        Ok(Token {
            kind,
            span: Span::new(start, self.position),
        })
    }

    fn escaped_byte(&mut self, terminator: u8, start: usize) -> CompileResult<u8> {
        if self.position >= self.text.len() {
            return Err(Diagnostic::new(
                DiagnosticKind::Lex,
                Span::new(start, self.position),
                "リテラルが閉じられていません",
            ));
        }
        let byte = self.bytes()[self.position];
        self.position += 1;
        if byte != b'\\' {
            return Ok(byte);
        }
        if self.position >= self.text.len() {
            return Err(Diagnostic::new(
                DiagnosticKind::Lex,
                Span::new(start, self.position),
                "エスケープシーケンスが途中で終わっています",
            ));
        }
        let escaped = self.bytes()[self.position];
        self.position += 1;
        match escaped {
            b'n' => Ok(b'\n'),
            b'r' => Ok(b'\r'),
            b't' => Ok(b'\t'),
            b'0' => Ok(0),
            b'\\' => Ok(b'\\'),
            b'\'' => Ok(b'\''),
            b'"' => Ok(b'"'),
            _ => Err(Diagnostic::new(
                DiagnosticKind::Lex,
                Span::new(self.position - 2, self.position),
                format!("未対応のエスケープ `\\{}` です", escaped as char),
            )
            .help(format!(
                "`\\{}`をそのまま書くにはバックスラッシュを追加してください",
                terminator as char
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn キーワード数値文字列コメントを解析する() {
        let tokens = lex("int x=0x2a; // 行\n char *s=\"A\\n\"; /* 塊 */").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Integer(42)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::StringLiteral(vec![b'A', b'\n'])));
    }

    #[test]
    fn 未終了リテラルとコメントを拒否する() {
        assert!(lex("\"abc").unwrap_err().message.contains("文字列"));
        assert!(lex("/* abc").unwrap_err().message.contains("コメント"));
    }
}
