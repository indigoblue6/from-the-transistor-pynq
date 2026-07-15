//! 構文解析と意味解析で共有するAST。

use crate::{source::Span, types::Type};

pub type SymbolId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Global(VarDecl),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub return_type: Type,
    pub parameters: Vec<VarDecl>,
    pub body: Option<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub id: SymbolId,
    pub name: String,
    pub ty: Type,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Empty,
    Expression(Expr),
    Declaration(VarDecl),
    Block(Vec<Stmt>),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    pub ty: Option<Type>,
    pub lvalue: bool,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self {
            kind,
            span,
            ty: None,
            lvalue: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Integer(i32),
    Character(u8),
    StringLiteral(Vec<u8>, Option<usize>),
    Variable {
        name: String,
        symbol: Option<SymbolId>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expression: Box<Expr>,
    },
    Assignment {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    SizeOfType(Type),
    SizeOfExpression(Box<Expr>),
    Postfix {
        increment: bool,
        expression: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    LogicalAnd,
    LogicalOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Plus,
    LogicalNot,
    BitNot,
    Address,
    Dereference,
    PreIncrement,
    PreDecrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}
