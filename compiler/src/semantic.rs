//! 名前解決と最小型検査を行い、ASTへ型とシンボルIDを付加する。

use crate::{
    ast::*,
    diagnostic::{CompileResult, Diagnostic, DiagnosticKind},
    source::Span,
    symbol::{FunctionSymbol, VariableSymbol},
    types::Type,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SemanticInfo {
    pub functions: HashMap<String, FunctionSymbol>,
    pub globals: HashMap<String, VariableSymbol>,
}

pub fn analyze(program: &mut Program) -> CompileResult<SemanticInfo> {
    Analyzer::new().run(program)
}

struct Analyzer {
    functions: HashMap<String, FunctionSymbol>,
    globals: HashMap<String, VariableSymbol>,
    scopes: Vec<HashMap<String, VariableSymbol>>,
    return_type: Type,
    loop_depth: usize,
}

impl Analyzer {
    fn new() -> Self {
        let mut this = Self {
            functions: HashMap::new(),
            globals: HashMap::new(),
            scopes: Vec::new(),
            return_type: Type::Void,
            loop_depth: 0,
        };
        let int = Type::Int;
        let char_ptr = Type::Pointer(Box::new(Type::Char));
        this.builtin("putchar", Type::Void, vec![Type::Char]);
        this.builtin("puts", Type::Void, vec![char_ptr.clone()]);
        this.builtin("print_int", Type::Void, vec![int.clone()]);
        this.builtin("panic", Type::Void, vec![char_ptr.clone()]);
        this.builtin("strlen", int.clone(), vec![char_ptr.clone()]);
        this.builtin(
            "memset",
            char_ptr.clone(),
            vec![char_ptr.clone(), int.clone(), int.clone()],
        );
        this.builtin(
            "memcpy",
            char_ptr.clone(),
            vec![char_ptr.clone(), char_ptr.clone(), int.clone()],
        );
        this.builtin("__mulsi3", int.clone(), vec![int.clone(), int.clone()]);
        this.builtin("__divsi3", int.clone(), vec![int.clone(), int.clone()]);
        this.builtin("__modsi3", int.clone(), vec![int.clone(), int.clone()]);
        for name in [
            "__csr_read_status",
            "__csr_read_epc",
            "__csr_read_cause",
            "__csr_read_badaddr",
            "__csr_read_timer_count",
            "__uart_rx_status",
            "__uart_rx_read",
        ] {
            this.builtin(name, int.clone(), vec![]);
        }
        for name in [
            "__csr_write_status",
            "__csr_write_epc",
            "__csr_write_tvec",
            "__csr_write_timer_compare",
            "__csr_write_interrupt_enable",
            "__csr_write_user_base",
            "__csr_write_user_limit",
            "__csr_write_timer_control",
            "__uart_rx_control",
        ] {
            this.builtin(name, Type::Void, vec![int.clone()]);
        }
        this.builtin("__wfi", Type::Void, vec![]);
        this.builtin(
            "__syscall4",
            int.clone(),
            vec![int.clone(), int.clone(), int.clone(), int],
        );
        this
    }

    fn builtin(&mut self, name: &str, return_type: Type, parameters: Vec<Type>) {
        self.functions.insert(
            name.into(),
            FunctionSymbol {
                return_type,
                parameters,
                span: Span::default(),
                defined: true,
                builtin: true,
            },
        );
    }

    fn run(mut self, program: &mut Program) -> CompileResult<SemanticInfo> {
        for item in &mut program.items {
            match item {
                Item::Global(variable) => {
                    self.infer_array_length(variable)?;
                    if self.functions.contains_key(&variable.name)
                        || self.globals.contains_key(&variable.name)
                    {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Name,
                            variable.span,
                            format!("グローバル名 `{}` が重複しています", variable.name),
                        ));
                    }
                    self.globals.insert(
                        variable.name.clone(),
                        VariableSymbol {
                            id: variable.id,
                            ty: variable.ty.clone(),
                            span: variable.span,
                            global: true,
                        },
                    );
                }
                Item::Function(function) => {
                    if self.globals.contains_key(&function.name) {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Name,
                            function.span,
                            format!("`{}` は変数として定義済みです", function.name),
                        ));
                    }
                    let parameters: Vec<_> = function
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty.clone())
                        .collect();
                    if let Some(previous) = self.functions.get_mut(&function.name) {
                        if previous.builtin {
                            return Err(Diagnostic::new(
                                DiagnosticKind::Name,
                                function.span,
                                format!("組み込み関数 `{}` は再定義できません", function.name),
                            ));
                        }
                        if previous.return_type != function.return_type
                            || previous.parameters != parameters
                        {
                            return Err(Diagnostic::new(
                                DiagnosticKind::Type,
                                function.span,
                                format!("関数 `{}` の宣言と型が一致しません", function.name),
                            ));
                        }
                        if previous.defined && function.body.is_some() {
                            return Err(Diagnostic::new(
                                DiagnosticKind::Name,
                                function.span,
                                format!("関数 `{}` が重複定義されています", function.name),
                            ));
                        }
                        previous.defined |= function.body.is_some();
                    } else {
                        self.functions.insert(
                            function.name.clone(),
                            FunctionSymbol {
                                return_type: function.return_type.clone(),
                                parameters,
                                span: function.span,
                                defined: function.body.is_some(),
                                builtin: false,
                            },
                        );
                    }
                }
            }
        }
        for item in &mut program.items {
            match item {
                Item::Global(variable) => self.global_initializer(variable)?,
                Item::Function(function) if function.body.is_some() => self.function(function)?,
                _ => {}
            }
        }
        for (name, function) in &self.functions {
            if !function.builtin && !function.defined {
                return Err(Diagnostic::new(
                    DiagnosticKind::Name,
                    function.span,
                    format!("関数 `{name}` は宣言されていますが定義がありません"),
                ));
            }
        }
        Ok(SemanticInfo {
            functions: self.functions,
            globals: self.globals,
        })
    }

    fn infer_array_length(&self, variable: &mut VarDecl) -> CompileResult<()> {
        if let Type::Array { element, length } = &mut variable.ty {
            if *length == 0 {
                if **element == Type::Char {
                    if let Some(Expr {
                        kind: ExprKind::StringLiteral(bytes, _),
                        ..
                    }) = &variable.initializer
                    {
                        *length = bytes.len() + 1;
                        return Ok(());
                    }
                }
                return Err(Diagnostic::new(
                    DiagnosticKind::Type,
                    variable.span,
                    "要素数を省略した配列には文字列初期化が必要です",
                ));
            }
        }
        Ok(())
    }

    fn global_initializer(&mut self, variable: &mut VarDecl) -> CompileResult<()> {
        let Some(initializer) = &mut variable.initializer else {
            return Ok(());
        };
        let valid_constant = matches!(
            initializer.kind,
            ExprKind::Integer(_) | ExprKind::Character(_) | ExprKind::StringLiteral(_, _)
        );
        if !valid_constant {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                initializer.span,
                "グローバル初期値には整数・文字・文字列定数だけを使用できます",
            ));
        }
        let source = self.expression(initializer)?;
        if matches!(variable.ty, Type::Array { .. }) {
            if !matches!(initializer.kind, ExprKind::StringLiteral(_, _))
                || !matches!(variable.ty, Type::Array { ref element, .. } if **element == Type::Char)
            {
                return Err(Diagnostic::new(
                    DiagnosticKind::Type,
                    initializer.span,
                    "配列の初期化はchar配列への文字列だけに対応しています",
                ));
            }
        } else if !variable.ty.assignable_from(&source) {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                initializer.span,
                format!("型 `{source}` を `{}` へ初期化できません", variable.ty),
            ));
        }
        Ok(())
    }

    fn function(&mut self, function: &mut Function) -> CompileResult<()> {
        self.return_type = function.return_type.clone();
        self.scopes.clear();
        self.scopes.push(HashMap::new());
        for parameter in &function.parameters {
            self.define_local(parameter)?;
        }
        let body = function.body.as_mut().expect("呼出し側で本体を確認済み");
        let always_returns = if let StmtKind::Block(statements) = &mut body.kind {
            let mut returned = false;
            for statement in statements {
                returned |= self.statement(statement)?;
            }
            returned
        } else {
            self.statement(body)?
        };
        if function.return_type != Type::Void && !always_returns {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                function.span,
                format!(
                    "非void関数 `{}` に明白なreturn不足があります",
                    function.name
                ),
            ));
        }
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> CompileResult<bool> {
        match &mut statement.kind {
            StmtKind::Empty => Ok(false),
            StmtKind::Expression(expression) => {
                self.expression(expression)?;
                Ok(false)
            }
            StmtKind::Declaration(variable) => {
                self.infer_array_length(variable)?;
                if let Some(initializer) = &mut variable.initializer {
                    let source = self.expression(initializer)?;
                    if matches!(variable.ty, Type::Array { .. }) {
                        if !matches!(initializer.kind, ExprKind::StringLiteral(_, _))
                            || !matches!(variable.ty, Type::Array { ref element, .. } if **element == Type::Char)
                        {
                            return Err(Diagnostic::new(
                                DiagnosticKind::Type,
                                initializer.span,
                                "ローカル配列初期化はchar配列への文字列だけに対応しています",
                            ));
                        }
                    } else if !variable.ty.assignable_from(&source) {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Type,
                            initializer.span,
                            format!("型 `{source}` を `{}` へ初期化できません", variable.ty),
                        ));
                    }
                }
                self.define_local(variable)?;
                Ok(false)
            }
            StmtKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                let mut returned = false;
                for child in statements {
                    returned |= self.statement(child)?;
                }
                self.scopes.pop();
                Ok(returned)
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.condition(condition)?;
                let then_return = self.statement(then_branch)?;
                let else_return = if let Some(branch) = else_branch {
                    self.statement(branch)?
                } else {
                    false
                };
                Ok(then_return && else_return)
            }
            StmtKind::While { condition, body } => {
                self.condition(condition)?;
                self.loop_depth += 1;
                self.statement(body)?;
                self.loop_depth -= 1;
                Ok(false)
            }
            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => {
                self.scopes.push(HashMap::new());
                if let Some(init) = init {
                    self.statement(init)?;
                }
                if let Some(condition) = condition {
                    self.condition(condition)?;
                }
                if let Some(update) = update {
                    self.expression(update)?;
                }
                self.loop_depth += 1;
                self.statement(body)?;
                self.loop_depth -= 1;
                self.scopes.pop();
                Ok(false)
            }
            StmtKind::Return(expression) => {
                let return_type = self.return_type.clone();
                match (&return_type, expression) {
                    (Type::Void, None) => {}
                    (Type::Void, Some(value)) => {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Type,
                            value.span,
                            "void関数から値を返せません",
                        ))
                    }
                    (_, None) => {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Type,
                            statement.span,
                            format!("`{}`関数には戻り値が必要です", self.return_type),
                        ))
                    }
                    (expected, Some(value)) => {
                        let actual = self.expression(value)?;
                        if !expected.assignable_from(&actual) {
                            return Err(Diagnostic::new(
                                DiagnosticKind::Type,
                                value.span,
                                format!(
                                    "戻り値型 `{actual}` は期待する `{expected}` と一致しません"
                                ),
                            ));
                        }
                    }
                }
                Ok(true)
            }
            StmtKind::Break => {
                if self.loop_depth == 0 {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        statement.span,
                        "breakはループ内でだけ使用できます",
                    ))
                } else {
                    Ok(false)
                }
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        statement.span,
                        "continueはループ内でだけ使用できます",
                    ))
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn condition(&mut self, expression: &mut Expr) -> CompileResult<()> {
        let ty = self.expression(expression)?.decay();
        if !ty.is_scalar() {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                expression.span,
                format!("条件式にはスカラー型が必要ですが`{ty}`です"),
            ));
        }
        Ok(())
    }

    fn expression(&mut self, expression: &mut Expr) -> CompileResult<Type> {
        let (ty, lvalue) = match &mut expression.kind {
            ExprKind::Integer(_) => (Type::Int, false),
            ExprKind::Character(_) => (Type::Char, false),
            ExprKind::StringLiteral(bytes, _) => (
                Type::Array {
                    element: Box::new(Type::Char),
                    length: bytes.len() + 1,
                },
                false,
            ),
            ExprKind::Variable { name, symbol } => {
                let variable = self.resolve_variable(name).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Name,
                        expression.span,
                        format!("未定義変数 `{name}` です"),
                    )
                    .help(format!("`{name}`を使用前に宣言してください"))
                })?;
                *symbol = Some(variable.id);
                (variable.ty, true)
            }
            ExprKind::Unary {
                op,
                expression: inner,
            } => {
                let inner_ty = self.expression(inner)?;
                self.unary_type(*op, inner, inner_ty)?
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let lhs_ty = self.expression(lhs)?;
                let rhs_ty = self.expression(rhs)?;
                (self.binary_type(*op, lhs, rhs, lhs_ty, rhs_ty)?, false)
            }
            ExprKind::Assignment { op, target, value } => {
                let target_ty = self.expression(target)?;
                let value_ty = self.expression(value)?;
                if !target.lvalue {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        target.span,
                        "代入先は左辺値でなければなりません",
                    ));
                }
                if matches!(target_ty, Type::Array { .. }) {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        target.span,
                        "配列全体へ代入できません",
                    ));
                }
                if *op == AssignOp::Assign {
                    if !target_ty.assignable_from(&value_ty) {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Type,
                            value.span,
                            format!("型 `{value_ty}` を `{target_ty}` へ代入できません"),
                        ));
                    }
                } else {
                    let binary = assign_binary(*op);
                    self.binary_type(binary, target, value, target_ty.clone(), value_ty)?;
                }
                (target_ty, false)
            }
            ExprKind::Call { name, arguments } => {
                let function = self.functions.get(name).cloned().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Name,
                        expression.span,
                        format!("未定義関数 `{name}` です"),
                    )
                })?;
                if arguments.len() != function.parameters.len() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        format!(
                            "関数 `{name}` の引数は{}個必要ですが{}個です",
                            function.parameters.len(),
                            arguments.len()
                        ),
                    ));
                }
                for (argument, expected) in arguments.iter_mut().zip(&function.parameters) {
                    let actual = self.expression(argument)?;
                    if !expected.assignable_from(&actual) {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Type,
                            argument.span,
                            format!("引数型 `{actual}` は期待する `{expected}` と一致しません"),
                        ));
                    }
                }
                (function.return_type, false)
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.expression(base)?.decay();
                let index_ty = self.expression(index)?;
                if !index_ty.is_integer() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        index.span,
                        "配列添字には整数が必要です",
                    ));
                }
                let element = base_ty.pointed().cloned().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Type,
                        base.span,
                        "添字演算の対象は配列またはポインタでなければなりません",
                    )
                })?;
                (element, true)
            }
            ExprKind::SizeOfType(ty) => {
                if ty.size().is_none() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        format!("`{ty}`のサイズは定義されていません"),
                    ));
                }
                (Type::Int, false)
            }
            ExprKind::SizeOfExpression(inner) => {
                let ty = self.expression(inner)?;
                if ty.size().is_none() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        inner.span,
                        format!("`{ty}`のサイズは定義されていません"),
                    ));
                }
                (Type::Int, false)
            }
            ExprKind::Postfix {
                expression: inner, ..
            } => {
                let ty = self.expression(inner)?;
                if !inner.lvalue || !ty.is_scalar() {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        inner.span,
                        "インクリメント対象にはスカラー左辺値が必要です",
                    ));
                }
                (ty, false)
            }
        };
        if ty == Type::Void && !matches!(expression.kind, ExprKind::Call { .. }) {
            return Err(Diagnostic::new(
                DiagnosticKind::Type,
                expression.span,
                "void値を式で使用できません",
            ));
        }
        expression.ty = Some(ty.clone());
        expression.lvalue = lvalue;
        Ok(ty)
    }

    fn unary_type(&self, op: UnaryOp, expression: &Expr, ty: Type) -> CompileResult<(Type, bool)> {
        match op {
            UnaryOp::Address => {
                if expression.lvalue {
                    Ok((Type::Pointer(Box::new(ty)), false))
                } else {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        "アドレス取得には左辺値が必要です",
                    ))
                }
            }
            UnaryOp::Dereference => ty
                .decay()
                .pointed()
                .cloned()
                .map(|inner| (inner, true))
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        "非ポインタ型を間接参照できません",
                    )
                }),
            UnaryOp::PreIncrement | UnaryOp::PreDecrement => {
                if expression.lvalue && ty.is_scalar() {
                    Ok((ty, false))
                } else {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        "インクリメント対象にはスカラー左辺値が必要です",
                    ))
                }
            }
            UnaryOp::Negate | UnaryOp::Plus | UnaryOp::BitNot => {
                if ty.is_integer() {
                    Ok((Type::Int, false))
                } else {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        "この単項演算子には整数が必要です",
                    ))
                }
            }
            UnaryOp::LogicalNot => {
                if ty.decay().is_scalar() {
                    Ok((Type::Int, false))
                } else {
                    Err(Diagnostic::new(
                        DiagnosticKind::Type,
                        expression.span,
                        "論理否定にはスカラー値が必要です",
                    ))
                }
            }
        }
    }

    fn binary_type(
        &self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        lhs_ty: Type,
        rhs_ty: Type,
    ) -> CompileResult<Type> {
        let a = lhs_ty.decay();
        let b = rhs_ty.decay();
        let error = || {
            Diagnostic::new(
                DiagnosticKind::Type,
                lhs.span.join(rhs.span),
                format!("演算子`{op:?}`を型`{a}`と`{b}`へ適用できません"),
            )
        };
        match op {
            BinaryOp::Add => match (&a, &b) {
                (Type::Pointer(_), t) if t.is_integer() => Ok(a),
                (t, Type::Pointer(_)) if t.is_integer() => Ok(b),
                (x, y) if x.is_integer() && y.is_integer() => Ok(Type::Int),
                _ => Err(error()),
            },
            BinaryOp::Sub => match (&a, &b) {
                (Type::Pointer(x), Type::Pointer(y)) if x == y => Ok(Type::Int),
                (Type::Pointer(_), t) if t.is_integer() => Ok(a),
                (x, y) if x.is_integer() && y.is_integer() => Ok(Type::Int),
                _ => Err(error()),
            },
            BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                if a.is_integer() && b.is_integer() {
                    Ok(Type::Int)
                } else {
                    Err(error())
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                if (a.is_integer() && b.is_integer())
                    || matches!((&a, &b), (Type::Pointer(x), Type::Pointer(y)) if x == y)
                {
                    Ok(Type::Int)
                } else {
                    Err(error())
                }
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                if (a.is_integer() && b.is_integer())
                    || matches!((&a, &b), (Type::Pointer(x), Type::Pointer(y)) if x == y)
                {
                    Ok(Type::Int)
                } else {
                    Err(error())
                }
            }
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                if a.is_scalar() && b.is_scalar() {
                    Ok(Type::Int)
                } else {
                    Err(error())
                }
            }
        }
    }

    fn define_local(&mut self, variable: &VarDecl) -> CompileResult<()> {
        let scope = self
            .scopes
            .last_mut()
            .expect("関数解析中はスコープが存在する");
        if scope.contains_key(&variable.name) {
            return Err(Diagnostic::new(
                DiagnosticKind::Name,
                variable.span,
                format!(
                    "同一スコープで変数 `{}` が重複定義されています",
                    variable.name
                ),
            ));
        }
        scope.insert(
            variable.name.clone(),
            VariableSymbol {
                id: variable.id,
                ty: variable.ty.clone(),
                span: variable.span,
                global: false,
            },
        );
        Ok(())
    }

    fn resolve_variable(&self, name: &str) -> Option<VariableSymbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
            .or_else(|| self.globals.get(name).cloned())
    }
}

fn assign_binary(op: AssignOp) -> BinaryOp {
    match op {
        AssignOp::Add => BinaryOp::Add,
        AssignOp::Sub => BinaryOp::Sub,
        AssignOp::Mul => BinaryOp::Mul,
        AssignOp::Div => BinaryOp::Div,
        AssignOp::Mod => BinaryOp::Mod,
        AssignOp::BitAnd => BinaryOp::BitAnd,
        AssignOp::BitOr => BinaryOp::BitOr,
        AssignOp::BitXor => BinaryOp::BitXor,
        AssignOp::Shl => BinaryOp::Shl,
        AssignOp::Shr => BinaryOp::Shr,
        AssignOp::Assign => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::lex, parser::parse};
    fn check(text: &str) -> CompileResult<()> {
        let mut program = parse(lex(text)?)?;
        analyze(&mut program).map(|_| ())
    }

    #[test]
    fn スコープ関数ポインタ配列を検査する() {
        check("int add(int a,int b){return a+b;} int main(){int x[2]; int *p=x; p[0]=add(1,2); {int p=4;} return p[0];}").unwrap();
    }
    #[test]
    fn 代表的な不正プログラムを拒否する() {
        assert!(check("int main(){return missing;}")
            .unwrap_err()
            .message
            .contains("未定義"));
        assert!(check("int main(){int x; int x; return 0;}")
            .unwrap_err()
            .message
            .contains("重複"));
        assert!(check("int main(){*1=2; return 0;}")
            .unwrap_err()
            .message
            .contains("非ポインタ"));
        assert!(check("int main(){break; return 0;}")
            .unwrap_err()
            .message
            .contains("ループ"));
    }
}
