//! 意味解析で使用するシンボル情報。

use crate::{ast::SymbolId, source::Span, types::Type};

#[derive(Debug, Clone)]
pub struct VariableSymbol {
    pub id: SymbolId,
    pub ty: Type,
    pub span: Span,
    pub global: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub return_type: Type,
    pub parameters: Vec<Type>,
    pub span: Span,
    pub defined: bool,
    pub builtin: bool,
}
