//! 再帰下降と優先順位上昇法を組み合わせたPynqC Parser。

use crate::{
    ast::*,
    diagnostic::{CompileResult, Diagnostic, DiagnosticKind},
    source::Span,
    token::{Token, TokenKind},
    types::Type,
};
use std::mem::discriminant;

pub fn parse(tokens: Vec<Token>) -> CompileResult<Program> {
    Parser {
        tokens,
        position: 0,
        next_symbol: 1,
    }
    .program()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    next_symbol: usize,
}

impl Parser {
    fn program(mut self) -> CompileResult<Program> {
        let mut items = Vec::new();
        while !self.at(&TokenKind::Eof) {
            items.push(self.item()?);
        }
        Ok(Program { items })
    }

    fn item(&mut self) -> CompileResult<Item> {
        let start = self.current().span;
        let base = self.base_type()?;
        let (name, ty, name_span) = self.declarator(base, true)?;
        if self.eat(&TokenKind::LParen).is_some() {
            if matches!(ty, Type::Array { .. }) {
                return self.error(name_span, "関数の戻り値を配列にはできません");
            }
            let mut parameters = Vec::new();
            if !self.at(&TokenKind::RParen) {
                if self.at(&TokenKind::Void) && self.peek_at(1, &TokenKind::RParen) {
                    self.bump();
                } else {
                    loop {
                        if parameters.len() == 4 {
                            return self.error(self.current().span, "関数引数は4個までです");
                        }
                        let parameter_base = self.base_type()?;
                        let (parameter_name, mut parameter_ty, span) =
                            self.declarator(parameter_base, false)?;
                        if let Type::Array { element, .. } = parameter_ty {
                            parameter_ty = Type::Pointer(element);
                        }
                        if parameter_ty == Type::Void {
                            return self.error(span, "void型の引数は宣言できません");
                        }
                        parameters.push(VarDecl {
                            id: self.symbol(),
                            name: parameter_name,
                            ty: parameter_ty,
                            initializer: None,
                            span,
                        });
                        if self.eat(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
            }
            self.expect(&TokenKind::RParen, "関数引数の後に`)`が必要です")?;
            if self.eat(&TokenKind::Semicolon).is_some() {
                return Ok(Item::Function(Function {
                    name,
                    return_type: ty,
                    parameters,
                    body: None,
                    span: start.join(self.previous().span),
                }));
            }
            let body = self.statement()?;
            if !matches!(body.kind, StmtKind::Block(_)) {
                return self.error(body.span, "関数本体にはブロックが必要です");
            }
            Ok(Item::Function(Function {
                name,
                return_type: ty,
                parameters,
                span: start.join(body.span),
                body: Some(body),
            }))
        } else {
            if ty == Type::Void {
                return self.error(name_span, "void型の変数は宣言できません");
            }
            let initializer = if self.eat(&TokenKind::Equal).is_some() {
                Some(self.expression()?)
            } else {
                None
            };
            let end = self
                .expect(
                    &TokenKind::Semicolon,
                    "グローバル変数宣言の後に`;`が必要です",
                )?
                .span;
            Ok(Item::Global(VarDecl {
                id: self.symbol(),
                name,
                ty,
                initializer,
                span: start.join(end),
            }))
        }
    }

    fn base_type(&mut self) -> CompileResult<Type> {
        let span = self.current().span;
        if self.eat(&TokenKind::Int).is_some() {
            Ok(Type::Int)
        } else if self.eat(&TokenKind::Char).is_some() {
            Ok(Type::Char)
        } else if self.eat(&TokenKind::Void).is_some() {
            Ok(Type::Void)
        } else {
            self.error(span, "型名（int、char、void）が必要です")
        }
    }

    fn declarator(
        &mut self,
        mut ty: Type,
        allow_unsized_array: bool,
    ) -> CompileResult<(String, Type, Span)> {
        while self.eat(&TokenKind::Star).is_some() {
            ty = Type::Pointer(Box::new(ty));
        }
        let token = self.bump().clone();
        let name = if let TokenKind::Ident(name) = token.kind {
            name
        } else {
            return self.error(token.span, "識別子が必要です");
        };
        if self.eat(&TokenKind::LBracket).is_some() {
            let length = if self.at(&TokenKind::RBracket) && allow_unsized_array {
                0
            } else {
                let token = self.bump().clone();
                match token.kind {
                    TokenKind::Integer(value) if value > 0 => value as usize,
                    _ => return self.error(token.span, "配列長には正の整数定数が必要です"),
                }
            };
            self.expect(&TokenKind::RBracket, "配列長の後に`]`が必要です")?;
            ty = Type::Array {
                element: Box::new(ty),
                length,
            };
        }
        Ok((name, ty, token.span))
    }

    fn statement(&mut self) -> CompileResult<Stmt> {
        let start = self.current().span;
        if self.eat(&TokenKind::Semicolon).is_some() {
            return Ok(Stmt {
                kind: StmtKind::Empty,
                span: start,
            });
        }
        if self.eat(&TokenKind::LBrace).is_some() {
            let mut statements = Vec::new();
            while !self.at(&TokenKind::RBrace) {
                if self.at(&TokenKind::Eof) {
                    return self.error(start, "ブロックが閉じられていません");
                }
                statements.push(self.statement()?);
            }
            let end = self.bump().span;
            return Ok(Stmt {
                kind: StmtKind::Block(statements),
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::If).is_some() {
            self.expect(&TokenKind::LParen, "ifの後に`(`が必要です")?;
            let condition = self.expression()?;
            self.expect(&TokenKind::RParen, "if条件の後に`)`が必要です")?;
            let then_branch = Box::new(self.statement()?);
            let else_branch = if self.eat(&TokenKind::Else).is_some() {
                Some(Box::new(self.statement()?))
            } else {
                None
            };
            let end = else_branch
                .as_ref()
                .map_or(then_branch.span, |branch| branch.span);
            return Ok(Stmt {
                kind: StmtKind::If {
                    condition,
                    then_branch,
                    else_branch,
                },
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::While).is_some() {
            self.expect(&TokenKind::LParen, "whileの後に`(`が必要です")?;
            let condition = self.expression()?;
            self.expect(&TokenKind::RParen, "while条件の後に`)`が必要です")?;
            let body = Box::new(self.statement()?);
            let end = body.span;
            return Ok(Stmt {
                kind: StmtKind::While { condition, body },
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::For).is_some() {
            self.expect(&TokenKind::LParen, "forの後に`(`が必要です")?;
            let init = if self.eat(&TokenKind::Semicolon).is_some() {
                None
            } else if self.is_type() {
                Some(Box::new(self.declaration_statement()?))
            } else {
                let expression = self.expression()?;
                let end = self
                    .expect(&TokenKind::Semicolon, "for初期化の後に`;`が必要です")?
                    .span;
                Some(Box::new(Stmt {
                    span: expression.span.join(end),
                    kind: StmtKind::Expression(expression),
                }))
            };
            let condition = if self.eat(&TokenKind::Semicolon).is_some() {
                None
            } else {
                let value = self.expression()?;
                self.expect(&TokenKind::Semicolon, "for条件の後に`;`が必要です")?;
                Some(value)
            };
            let update = if self.at(&TokenKind::RParen) {
                None
            } else {
                Some(self.expression()?)
            };
            self.expect(&TokenKind::RParen, "for更新式の後に`)`が必要です")?;
            let body = Box::new(self.statement()?);
            let end = body.span;
            return Ok(Stmt {
                kind: StmtKind::For {
                    init,
                    condition,
                    update,
                    body,
                },
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::Return).is_some() {
            let expression = if self.at(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.expression()?)
            };
            let end = self
                .expect(&TokenKind::Semicolon, "returnの後に`;`が必要です")?
                .span;
            return Ok(Stmt {
                kind: StmtKind::Return(expression),
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::Break).is_some() {
            let end = self
                .expect(&TokenKind::Semicolon, "breakの後に`;`が必要です")?
                .span;
            return Ok(Stmt {
                kind: StmtKind::Break,
                span: start.join(end),
            });
        }
        if self.eat(&TokenKind::Continue).is_some() {
            let end = self
                .expect(&TokenKind::Semicolon, "continueの後に`;`が必要です")?
                .span;
            return Ok(Stmt {
                kind: StmtKind::Continue,
                span: start.join(end),
            });
        }
        if self.is_type() {
            return self.declaration_statement();
        }
        let expression = self.expression()?;
        let end = self
            .expect(&TokenKind::Semicolon, "式の後に`;`が必要です")?
            .span;
        Ok(Stmt {
            span: expression.span.join(end),
            kind: StmtKind::Expression(expression),
        })
    }

    fn declaration_statement(&mut self) -> CompileResult<Stmt> {
        let start = self.current().span;
        let base = self.base_type()?;
        let (name, ty, name_span) = self.declarator(base, true)?;
        if ty == Type::Void {
            return self.error(name_span, "void型の変数は宣言できません");
        }
        let initializer = if self.eat(&TokenKind::Equal).is_some() {
            Some(self.expression()?)
        } else {
            None
        };
        let end = self
            .expect(&TokenKind::Semicolon, "変数宣言の後に`;`が必要です")?
            .span;
        Ok(Stmt {
            span: start.join(end),
            kind: StmtKind::Declaration(VarDecl {
                id: self.symbol(),
                name,
                ty,
                initializer,
                span: start.join(end),
            }),
        })
    }

    fn expression(&mut self) -> CompileResult<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> CompileResult<Expr> {
        let target = self.binary(1)?;
        let op = match &self.current().kind {
            TokenKind::Equal => Some(AssignOp::Assign),
            TokenKind::PlusEqual => Some(AssignOp::Add),
            TokenKind::MinusEqual => Some(AssignOp::Sub),
            TokenKind::StarEqual => Some(AssignOp::Mul),
            TokenKind::SlashEqual => Some(AssignOp::Div),
            TokenKind::PercentEqual => Some(AssignOp::Mod),
            TokenKind::AmpEqual => Some(AssignOp::BitAnd),
            TokenKind::PipeEqual => Some(AssignOp::BitOr),
            TokenKind::CaretEqual => Some(AssignOp::BitXor),
            TokenKind::ShiftLeftEqual => Some(AssignOp::Shl),
            TokenKind::ShiftRightEqual => Some(AssignOp::Shr),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let value = self.assignment()?;
            let span = target.span.join(value.span);
            Ok(Expr::new(
                ExprKind::Assignment {
                    op,
                    target: Box::new(target),
                    value: Box::new(value),
                },
                span,
            ))
        } else {
            Ok(target)
        }
    }

    fn binary(&mut self, minimum: u8) -> CompileResult<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let Some((precedence, op)) = self.binary_op() else {
                break;
            };
            if precedence < minimum {
                break;
            }
            self.bump();
            let rhs = self.binary(precedence + 1)?;
            let span = lhs.span.join(rhs.span);
            lhs = Expr::new(
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Ok(lhs)
    }

    fn binary_op(&self) -> Option<(u8, BinaryOp)> {
        Some(match self.current().kind {
            TokenKind::PipePipe => (1, BinaryOp::LogicalOr),
            TokenKind::AmpAmp => (2, BinaryOp::LogicalAnd),
            TokenKind::Pipe => (3, BinaryOp::BitOr),
            TokenKind::Caret => (4, BinaryOp::BitXor),
            TokenKind::Amp => (5, BinaryOp::BitAnd),
            TokenKind::EqualEqual => (6, BinaryOp::Eq),
            TokenKind::BangEqual => (6, BinaryOp::Ne),
            TokenKind::Less => (7, BinaryOp::Lt),
            TokenKind::LessEqual => (7, BinaryOp::Le),
            TokenKind::Greater => (7, BinaryOp::Gt),
            TokenKind::GreaterEqual => (7, BinaryOp::Ge),
            TokenKind::ShiftLeft => (8, BinaryOp::Shl),
            TokenKind::ShiftRight => (8, BinaryOp::Shr),
            TokenKind::Plus => (9, BinaryOp::Add),
            TokenKind::Minus => (9, BinaryOp::Sub),
            TokenKind::Star => (10, BinaryOp::Mul),
            TokenKind::Slash => (10, BinaryOp::Div),
            TokenKind::Percent => (10, BinaryOp::Mod),
            _ => return None,
        })
    }

    fn unary(&mut self) -> CompileResult<Expr> {
        let start = self.current().span;
        let op = match self.current().kind {
            TokenKind::Minus => Some(UnaryOp::Negate),
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Bang => Some(UnaryOp::LogicalNot),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::Amp => Some(UnaryOp::Address),
            TokenKind::Star => Some(UnaryOp::Dereference),
            TokenKind::PlusPlus => Some(UnaryOp::PreIncrement),
            TokenKind::MinusMinus => Some(UnaryOp::PreDecrement),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let expression = self.unary()?;
            let span = start.join(expression.span);
            return Ok(Expr::new(
                ExprKind::Unary {
                    op,
                    expression: Box::new(expression),
                },
                span,
            ));
        }
        if self.eat(&TokenKind::Sizeof).is_some() {
            if self.eat(&TokenKind::LParen).is_some() {
                if self.is_type() {
                    let mut ty = self.base_type()?;
                    while self.eat(&TokenKind::Star).is_some() {
                        ty = Type::Pointer(Box::new(ty));
                    }
                    let end = self
                        .expect(&TokenKind::RParen, "sizeof型名の後に`)`が必要です")?
                        .span;
                    return Ok(Expr::new(ExprKind::SizeOfType(ty), start.join(end)));
                }
                let expression = self.expression()?;
                let end = self
                    .expect(&TokenKind::RParen, "sizeof式の後に`)`が必要です")?
                    .span;
                return Ok(Expr::new(
                    ExprKind::SizeOfExpression(Box::new(expression)),
                    start.join(end),
                ));
            }
            let expression = self.unary()?;
            let span = start.join(expression.span);
            return Ok(Expr::new(
                ExprKind::SizeOfExpression(Box::new(expression)),
                span,
            ));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> CompileResult<Expr> {
        let mut expression = self.primary()?;
        loop {
            if self.eat(&TokenKind::LBracket).is_some() {
                let index = self.expression()?;
                let end = self
                    .expect(&TokenKind::RBracket, "添字の後に`]`が必要です")?
                    .span;
                let span = expression.span.join(end);
                expression = Expr::new(
                    ExprKind::Index {
                        base: Box::new(expression),
                        index: Box::new(index),
                    },
                    span,
                );
            } else if self.eat(&TokenKind::LParen).is_some() {
                let name = if let ExprKind::Variable { name, .. } = expression.kind {
                    name
                } else {
                    return self.error(expression.span, "関数ポインタ呼出しは未対応です");
                };
                let mut arguments = Vec::new();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        arguments.push(self.expression()?);
                        if self.eat(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                let end = self
                    .expect(&TokenKind::RParen, "関数引数の後に`)`が必要です")?
                    .span;
                expression = Expr::new(
                    ExprKind::Call { name, arguments },
                    expression.span.join(end),
                );
            } else if self.eat(&TokenKind::PlusPlus).is_some()
                || self.eat(&TokenKind::MinusMinus).is_some()
            {
                let increment = matches!(self.previous().kind, TokenKind::PlusPlus);
                let span = expression.span.join(self.previous().span);
                expression = Expr::new(
                    ExprKind::Postfix {
                        increment,
                        expression: Box::new(expression),
                    },
                    span,
                );
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn primary(&mut self) -> CompileResult<Expr> {
        let token = self.bump().clone();
        match token.kind {
            TokenKind::Integer(value) => Ok(Expr::new(ExprKind::Integer(value), token.span)),
            TokenKind::Character(value) => Ok(Expr::new(ExprKind::Character(value), token.span)),
            TokenKind::StringLiteral(value) => {
                Ok(Expr::new(ExprKind::StringLiteral(value, None), token.span))
            }
            TokenKind::Ident(name) => Ok(Expr::new(
                ExprKind::Variable { name, symbol: None },
                token.span,
            )),
            TokenKind::LParen => {
                let expression = self.expression()?;
                self.expect(&TokenKind::RParen, "式の後に`)`が必要です")?;
                Ok(expression)
            }
            _ => self.error(
                token.span,
                format!("式が必要ですが{}が見つかりました", token.kind.name()),
            ),
        }
    }

    fn symbol(&mut self) -> usize {
        let id = self.next_symbol;
        self.next_symbol += 1;
        id
    }
    fn is_type(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Int | TokenKind::Char | TokenKind::Void
        )
    }
    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }
    fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }
    fn peek_at(&self, offset: usize, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.position + offset)
            .is_some_and(|token| discriminant(&token.kind) == discriminant(kind))
    }
    fn at(&self, kind: &TokenKind) -> bool {
        discriminant(&self.current().kind) == discriminant(kind)
    }
    fn eat(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump().clone())
        } else {
            None
        }
    }
    fn bump(&mut self) -> &Token {
        let index = self.position;
        self.position = (self.position + 1).min(self.tokens.len() - 1);
        &self.tokens[index]
    }
    fn expect(&mut self, kind: &TokenKind, message: &str) -> CompileResult<Token> {
        if self.at(kind) {
            Ok(self.bump().clone())
        } else {
            self.error(self.current().span, message)
        }
    }
    fn error<T>(&self, span: Span, message: impl Into<String>) -> CompileResult<T> {
        Err(Diagnostic::new(DiagnosticKind::Parse, span, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    fn parse_text(text: &str) -> CompileResult<Program> {
        parse(lex(text)?)
    }

    #[test]
    fn 優先順位と右結合代入を解析する() {
        let program = parse_text("int main(){int a; int b; a=b=1+2*3; return a;}").unwrap();
        let Item::Function(function) = &program.items[0] else {
            panic!()
        };
        let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
            panic!()
        };
        let StmtKind::Expression(expr) = &statements[2].kind else {
            panic!()
        };
        assert!(matches!(expr.kind, ExprKind::Assignment { .. }));
    }

    #[test]
    fn 関数配列制御文を解析する() {
        parse_text("int add(int a,int b); int main(){int x[3]; for(int i=0;i<3;i++){x[i]=i;} if(x[0]) return add(x[1],2); else return 0;}").unwrap();
    }
}
