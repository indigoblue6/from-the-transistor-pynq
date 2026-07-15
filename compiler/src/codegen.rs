//! 型付きASTから既存アセンブラ向けの可読なアセンブリを生成する。

use crate::{
    ast::*,
    diagnostic::{CompileResult, Diagnostic, DiagnosticKind},
    frame::Frame,
    semantic::SemanticInfo,
    types::Type,
};
use std::collections::HashMap;

const RAM_BASE: u32 = 0x0000_4000;
const RAM_END: u32 = 0x0001_0000;
const UART_TX: u32 = 0x8000_0000;
const SIM_EXIT: u32 = 0x8000_1000;

pub fn generate(program: &mut Program, info: &SemanticInfo, debug: bool) -> CompileResult<String> {
    Generator::new(info, debug).run(program)
}

struct Generator<'a> {
    info: &'a SemanticInfo,
    output: String,
    label: usize,
    frame: Option<Frame>,
    return_label: String,
    loops: Vec<(String, String)>,
    globals: HashMap<SymbolId, u32>,
    strings: Vec<(u32, Vec<u8>)>,
    data_cursor: u32,
    debug: bool,
}

impl<'a> Generator<'a> {
    fn new(info: &'a SemanticInfo, debug: bool) -> Self {
        Self {
            info,
            output: String::new(),
            label: 0,
            frame: None,
            return_label: String::new(),
            loops: Vec::new(),
            globals: HashMap::new(),
            strings: Vec::new(),
            data_cursor: RAM_BASE,
            debug,
        }
    }

    fn run(mut self, program: &mut Program) -> CompileResult<String> {
        self.layout_data(program)?;
        let main = self.info.functions.get("main").ok_or_else(|| {
            Diagnostic::new(
                DiagnosticKind::Codegen,
                Default::default(),
                "エントリ関数`main`がありません",
            )
        })?;
        if main.return_type != Type::Int || !main.parameters.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticKind::Codegen,
                main.span,
                "mainは`int main()`で定義してください",
            ));
        }
        self.line("; PynqCが生成した独自ISAアセンブリ");
        self.line(".org 0");
        self.emit_start(program)?;
        self.emit_runtime();
        for item in &program.items {
            if let Item::Function(function) = item {
                if function.body.is_some() {
                    self.emit_function(function)?;
                }
            }
        }
        Ok(self.output)
    }

    fn layout_data(&mut self, program: &mut Program) -> CompileResult<()> {
        for item in &program.items {
            if let Item::Global(variable) = item {
                let alignment = variable.ty.alignment() as u32;
                self.data_cursor = align(self.data_cursor, alignment);
                let size = variable.ty.size().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Codegen,
                        variable.span,
                        "サイズのないグローバル型です",
                    )
                })? as u32;
                self.globals.insert(variable.id, self.data_cursor);
                self.data_cursor = self.data_cursor.checked_add(size).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Codegen,
                        variable.span,
                        "グローバル領域がオーバーフローしました",
                    )
                })?;
            }
        }
        for item in &mut program.items {
            self.collect_item_strings(item);
        }
        if self.data_cursor >= RAM_END - 0x1000 {
            return Err(Diagnostic::new(
                DiagnosticKind::Codegen,
                Default::default(),
                "グローバルデータが大きすぎてスタック領域を確保できません",
            ));
        }
        Ok(())
    }

    fn collect_item_strings(&mut self, item: &mut Item) {
        match item {
            Item::Global(variable) => {
                if let Some(initializer) = &mut variable.initializer {
                    self.collect_expr_strings(initializer);
                }
            }
            Item::Function(function) => {
                if let Some(body) = &mut function.body {
                    self.collect_stmt_strings(body);
                }
            }
        }
    }
    fn collect_stmt_strings(&mut self, statement: &mut Stmt) {
        match &mut statement.kind {
            StmtKind::Expression(e) => self.collect_expr_strings(e),
            StmtKind::Declaration(v) => {
                if let Some(e) = &mut v.initializer {
                    self.collect_expr_strings(e);
                }
            }
            StmtKind::Block(items) => {
                for item in items {
                    self.collect_stmt_strings(item);
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_expr_strings(condition);
                self.collect_stmt_strings(then_branch);
                if let Some(branch) = else_branch {
                    self.collect_stmt_strings(branch);
                }
            }
            StmtKind::While { condition, body } => {
                self.collect_expr_strings(condition);
                self.collect_stmt_strings(body);
            }
            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => {
                if let Some(init) = init {
                    self.collect_stmt_strings(init);
                }
                if let Some(e) = condition {
                    self.collect_expr_strings(e);
                }
                if let Some(e) = update {
                    self.collect_expr_strings(e);
                }
                self.collect_stmt_strings(body);
            }
            StmtKind::Return(Some(e)) => self.collect_expr_strings(e),
            _ => {}
        }
    }
    fn collect_expr_strings(&mut self, expression: &mut Expr) {
        match &mut expression.kind {
            ExprKind::StringLiteral(bytes, address) => {
                let found = self
                    .strings
                    .iter()
                    .find(|(_, existing)| existing == bytes)
                    .map(|(address, _)| *address);
                let value = found.unwrap_or_else(|| {
                    let address = self.data_cursor;
                    self.data_cursor += bytes.len() as u32 + 1;
                    self.strings.push((address, bytes.clone()));
                    address
                });
                *address = Some(value as usize);
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect_expr_strings(lhs);
                self.collect_expr_strings(rhs);
            }
            ExprKind::Unary { expression, .. }
            | ExprKind::SizeOfExpression(expression)
            | ExprKind::Postfix { expression, .. } => self.collect_expr_strings(expression),
            ExprKind::Assignment { target, value, .. } => {
                self.collect_expr_strings(target);
                self.collect_expr_strings(value);
            }
            ExprKind::Call { arguments, .. } => {
                for argument in arguments {
                    self.collect_expr_strings(argument);
                }
            }
            ExprKind::Index { base, index } => {
                self.collect_expr_strings(base);
                self.collect_expr_strings(index);
            }
            _ => {}
        }
    }

    fn emit_start(&mut self, program: &Program) -> CompileResult<()> {
        self.line("_start:");
        self.line("    ; スタックをRAM終端へ設定");
        self.line("    li r15, 0x00010000");
        self.line("    ; グローバル領域と文字列を初期化");
        for item in &program.items {
            if let Item::Global(variable) = item {
                self.initialize_global(variable)?;
            }
        }
        for (address, bytes) in self.strings.clone() {
            self.initialize_bytes(address, &bytes, bytes.len() + 1);
        }
        self.line("    call main");
        self.line(&format!("    li r6, 0x{SIM_EXIT:08x}"));
        self.line("    store r1, [r6 + 0]");
        self.line("    halt");
        self.line("");
        Ok(())
    }

    fn initialize_global(&mut self, variable: &VarDecl) -> CompileResult<()> {
        let address = self.globals[&variable.id];
        let size = variable.ty.size().unwrap();
        match (&variable.ty, &variable.initializer) {
            (
                Type::Array { element, .. },
                Some(Expr {
                    kind: ExprKind::StringLiteral(bytes, _),
                    ..
                }),
            ) if **element == Type::Char => self.initialize_bytes(address, bytes, size),
            (_, Some(initializer)) if !matches!(variable.ty, Type::Array { .. }) => {
                let value = match &initializer.kind {
                    ExprKind::Integer(value) => *value as u32,
                    ExprKind::Character(value) => *value as u32,
                    ExprKind::StringLiteral(_, Some(address)) => *address as u32,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Codegen,
                            initializer.span,
                            "未対応のグローバル初期値です",
                        ))
                    }
                };
                self.line(&format!("    li r6, 0x{address:08x}"));
                self.line(&format!("    li r7, 0x{value:08x}"));
                self.line(if variable.ty == Type::Char {
                    "    storeb r7, [r6 + 0]"
                } else {
                    "    store r7, [r6 + 0]"
                });
            }
            _ => self.initialize_bytes(address, &[], size),
        }
        Ok(())
    }

    fn initialize_bytes(&mut self, address: u32, bytes: &[u8], total: usize) {
        self.line(&format!("    li r6, 0x{address:08x}"));
        for index in 0..total {
            let value = bytes.get(index).copied().unwrap_or(0);
            self.line(&format!("    movi r7, {value}"));
            self.line(&format!("    storeb r7, [r6 + {index}]"));
        }
    }

    fn emit_function(&mut self, function: &Function) -> CompileResult<()> {
        let frame = Frame::build(function);
        self.return_label = self.new_label(&format!("{}_return", function.name));
        self.frame = Some(frame.clone());
        self.line(&format!("{}:", function.name));
        self.line("    ; プロローグ: 保存FP/LRと固定フレーム");
        self.line("    addi r15, r15, -8");
        self.line("    store r13, [r15 + 0]");
        self.line("    store r14, [r15 + 4]");
        self.line("    add r13, r15, r0");
        if frame.size != 0 {
            self.line(&format!("    addi r15, r15, -{}", frame.size));
        }
        for (index, parameter) in function.parameters.iter().enumerate() {
            let offset = frame.offsets[&parameter.id];
            self.line(&format!(
                "    {} r{}, [r13 + {offset}]",
                if parameter.ty == Type::Char {
                    "storeb"
                } else {
                    "store"
                },
                index + 2
            ));
        }
        self.emit_stmt(function.body.as_ref().unwrap())?;
        if function.return_type == Type::Void {
            self.line("    movi r1, 0");
        }
        self.line(&format!("{}:", self.return_label));
        self.line("    ; エピローグ");
        self.line("    add r15, r13, r0");
        self.line("    load r13, [r15 + 0]");
        self.line("    load r14, [r15 + 4]");
        self.line("    addi r15, r15, 8");
        self.line("    ret");
        self.line("");
        self.frame = None;
        Ok(())
    }

    fn emit_stmt(&mut self, statement: &Stmt) -> CompileResult<()> {
        if self.debug {
            self.line(&format!(
                "    ; source-span {}..{}",
                statement.span.start, statement.span.end
            ));
        }
        match &statement.kind {
            StmtKind::Empty => {}
            StmtKind::Expression(expression) => self.emit_expr(expression)?,
            StmtKind::Declaration(variable) => self.emit_local_initializer(variable)?,
            StmtKind::Block(items) => {
                for item in items {
                    self.emit_stmt(item)?;
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_label = self.new_label("if_else");
                let end = self.new_label("if_end");
                self.emit_expr(condition)?;
                self.line(&format!("    beq r6, r0, {else_label}"));
                self.emit_stmt(then_branch)?;
                self.line(&format!("    jmp {end}"));
                self.line(&format!("{else_label}:"));
                if let Some(branch) = else_branch {
                    self.emit_stmt(branch)?;
                }
                self.line(&format!("{end}:"));
            }
            StmtKind::While { condition, body } => {
                let begin = self.new_label("while_begin");
                let end = self.new_label("while_end");
                self.loops.push((end.clone(), begin.clone()));
                self.line(&format!("{begin}:"));
                self.emit_expr(condition)?;
                self.line(&format!("    beq r6, r0, {end}"));
                self.emit_stmt(body)?;
                self.line(&format!("    jmp {begin}"));
                self.line(&format!("{end}:"));
                self.loops.pop();
            }
            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => {
                let begin = self.new_label("for_begin");
                let update_label = self.new_label("for_update");
                let end = self.new_label("for_end");
                if let Some(init) = init {
                    self.emit_stmt(init)?;
                }
                self.loops.push((end.clone(), update_label.clone()));
                self.line(&format!("{begin}:"));
                if let Some(condition) = condition {
                    self.emit_expr(condition)?;
                    self.line(&format!("    beq r6, r0, {end}"));
                }
                self.emit_stmt(body)?;
                self.line(&format!("{update_label}:"));
                if let Some(update) = update {
                    self.emit_expr(update)?;
                }
                self.line(&format!("    jmp {begin}"));
                self.line(&format!("{end}:"));
                self.loops.pop();
            }
            StmtKind::Return(expression) => {
                if let Some(expression) = expression {
                    self.emit_expr(expression)?;
                    self.line("    add r1, r6, r0");
                }
                let label = self.return_label.clone();
                self.line(&format!("    jmp {label}"));
            }
            StmtKind::Break => {
                let label = self.loops.last().unwrap().0.clone();
                self.line(&format!("    jmp {label}"));
            }
            StmtKind::Continue => {
                let label = self.loops.last().unwrap().1.clone();
                self.line(&format!("    jmp {label}"));
            }
        }
        Ok(())
    }

    fn emit_local_initializer(&mut self, variable: &VarDecl) -> CompileResult<()> {
        let Some(initializer) = &variable.initializer else {
            return Ok(());
        };
        let offset = self.frame.as_ref().unwrap().offsets[&variable.id];
        if let (Type::Array { element, length }, ExprKind::StringLiteral(bytes, _)) =
            (&variable.ty, &initializer.kind)
        {
            if **element == Type::Char {
                for index in 0..*length {
                    self.line(&format!(
                        "    movi r6, {}",
                        bytes.get(index).copied().unwrap_or(0)
                    ));
                    self.line(&format!("    storeb r6, [r13 + {}]", offset + index as i32));
                }
                return Ok(());
            }
        }
        self.emit_expr(initializer)?;
        self.line(&format!(
            "    {} r6, [r13 + {offset}]",
            if variable.ty == Type::Char {
                "storeb"
            } else {
                "store"
            }
        ));
        Ok(())
    }

    fn emit_expr(&mut self, expression: &Expr) -> CompileResult<()> {
        match &expression.kind {
            ExprKind::Integer(value) => self.line(&format!("    li r6, 0x{:08x}", *value as u32)),
            ExprKind::Character(value) => self.line(&format!("    movi r6, {value}")),
            ExprKind::StringLiteral(_, Some(address)) => {
                self.line(&format!("    li r6, 0x{address:08x}"))
            }
            ExprKind::StringLiteral(_, None) => {
                return Err(self.codegen_error(expression, "文字列アドレスが未配置です"))
            }
            ExprKind::Variable { .. } if matches!(expression.ty, Some(Type::Array { .. })) => {
                self.emit_lvalue(expression)?
            }
            ExprKind::Variable { .. } => {
                self.emit_lvalue(expression)?;
                self.load_from_r6(expression.ty.as_ref().unwrap());
            }
            ExprKind::Unary {
                op,
                expression: inner,
            } => self.emit_unary(*op, inner)?,
            ExprKind::Binary { op, lhs, rhs } => self.emit_binary(*op, lhs, rhs)?,
            ExprKind::Assignment { op, target, value } => {
                self.emit_assignment(*op, target, value)?
            }
            ExprKind::Call { name, arguments } => {
                for argument in arguments {
                    self.emit_expr(argument)?;
                    self.push("r6");
                }
                for index in (0..arguments.len()).rev() {
                    self.pop(&format!("r{}", index + 2));
                }
                self.line(&format!("    call {name}"));
                self.line("    add r6, r1, r0");
            }
            ExprKind::Index { .. } => {
                self.emit_lvalue(expression)?;
                self.load_from_r6(expression.ty.as_ref().unwrap());
            }
            ExprKind::SizeOfType(ty) => self.line(&format!("    movi r6, {}", ty.size().unwrap())),
            ExprKind::SizeOfExpression(inner) => self.line(&format!(
                "    movi r6, {}",
                inner.ty.as_ref().unwrap().size().unwrap()
            )),
            ExprKind::Postfix {
                increment,
                expression: inner,
            } => {
                self.emit_lvalue(inner)?;
                self.push("r6");
                self.load_from_r6(inner.ty.as_ref().unwrap());
                self.line("    add r8, r6, r0");
                self.adjust_scalar("r8", inner.ty.as_ref().unwrap(), *increment);
                self.pop("r7");
                self.store_to("r8", "r7", inner.ty.as_ref().unwrap());
            }
        }
        Ok(())
    }

    fn emit_unary(&mut self, op: UnaryOp, inner: &Expr) -> CompileResult<()> {
        match op {
            UnaryOp::Address => self.emit_lvalue(inner)?,
            UnaryOp::Dereference => {
                self.emit_expr(inner)?;
                self.load_from_r6(inner.ty.as_ref().unwrap().pointed().unwrap());
            }
            UnaryOp::Plus => self.emit_expr(inner)?,
            UnaryOp::Negate => {
                self.emit_expr(inner)?;
                self.line("    sub r6, r0, r6");
            }
            UnaryOp::BitNot => {
                self.emit_expr(inner)?;
                self.line("    li r7, 0xffffffff");
                self.line("    xor r6, r6, r7");
            }
            UnaryOp::LogicalNot => {
                self.emit_expr(inner)?;
                self.boolean_from_branch("beq", "r6", "r0");
            }
            UnaryOp::PreIncrement | UnaryOp::PreDecrement => {
                self.emit_lvalue(inner)?;
                self.push("r6");
                self.load_from_r6(inner.ty.as_ref().unwrap());
                self.adjust_scalar(
                    "r6",
                    inner.ty.as_ref().unwrap(),
                    op == UnaryOp::PreIncrement,
                );
                self.pop("r7");
                self.store_to("r6", "r7", inner.ty.as_ref().unwrap());
            }
        }
        Ok(())
    }

    fn emit_assignment(&mut self, op: AssignOp, target: &Expr, value: &Expr) -> CompileResult<()> {
        self.emit_lvalue(target)?;
        self.push("r6");
        if op == AssignOp::Assign {
            self.emit_expr(value)?;
        } else {
            self.load_from_r6(target.ty.as_ref().unwrap());
            self.push("r6");
            self.emit_expr(value)?;
            self.pop("r7");
            self.emit_binary_registers(
                assign_binary(op),
                target.ty.as_ref().unwrap(),
                value.ty.as_ref().unwrap(),
            )?;
        }
        self.pop("r7");
        self.store_to("r6", "r7", target.ty.as_ref().unwrap());
        Ok(())
    }

    fn emit_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> CompileResult<()> {
        if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
            let short = self.new_label("logical_short");
            let end = self.new_label("logical_end");
            self.emit_expr(lhs)?;
            self.line(&format!(
                "    {} r6, r0, {short}",
                if op == BinaryOp::LogicalAnd {
                    "beq"
                } else {
                    "bne"
                }
            ));
            self.emit_expr(rhs)?;
            self.line(&format!(
                "    {} r6, r0, {short}",
                if op == BinaryOp::LogicalAnd {
                    "beq"
                } else {
                    "bne"
                }
            ));
            self.line(&format!(
                "    movi r6, {}",
                if op == BinaryOp::LogicalAnd { 1 } else { 0 }
            ));
            self.line(&format!("    jmp {end}"));
            self.line(&format!("{short}:"));
            self.line(&format!(
                "    movi r6, {}",
                if op == BinaryOp::LogicalAnd { 0 } else { 1 }
            ));
            self.line(&format!("{end}:"));
            return Ok(());
        }
        self.emit_expr(lhs)?;
        self.push("r6");
        self.emit_expr(rhs)?;
        self.pop("r7");
        self.emit_binary_registers(op, lhs.ty.as_ref().unwrap(), rhs.ty.as_ref().unwrap())
    }

    fn emit_binary_registers(
        &mut self,
        op: BinaryOp,
        lhs_ty: &Type,
        rhs_ty: &Type,
    ) -> CompileResult<()> {
        if op == BinaryOp::Add {
            if matches!(lhs_ty.decay(), Type::Pointer(_)) && rhs_ty.is_integer() {
                self.scale("r6", lhs_ty.decay().pointed().unwrap().size().unwrap());
            } else if lhs_ty.is_integer() && matches!(rhs_ty.decay(), Type::Pointer(_)) {
                self.scale("r7", rhs_ty.decay().pointed().unwrap().size().unwrap());
            }
        } else if op == BinaryOp::Sub
            && matches!(lhs_ty.decay(), Type::Pointer(_))
            && rhs_ty.is_integer()
        {
            self.scale("r6", lhs_ty.decay().pointed().unwrap().size().unwrap());
        }
        match op {
            BinaryOp::Add => self.line("    add r6, r7, r6"),
            BinaryOp::Sub => {
                self.line("    sub r6, r7, r6");
                if matches!(
                    (lhs_ty.decay(), rhs_ty.decay()),
                    (Type::Pointer(_), Type::Pointer(_))
                ) {
                    let size = lhs_ty.decay().pointed().unwrap().size().unwrap();
                    if size == 4 {
                        self.line("    movi r8, 2");
                        self.line("    sar r6, r6, r8");
                    }
                }
            }
            BinaryOp::BitAnd => self.line("    and r6, r7, r6"),
            BinaryOp::BitOr => self.line("    or r6, r7, r6"),
            BinaryOp::BitXor => self.line("    xor r6, r7, r6"),
            BinaryOp::Shl => self.line("    shl r6, r7, r6"),
            BinaryOp::Shr => self.line("    sar r6, r7, r6"),
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                self.line("    add r2, r7, r0");
                self.line("    add r3, r6, r0");
                self.line(&format!(
                    "    call {}",
                    match op {
                        BinaryOp::Mul => "__mulsi3",
                        BinaryOp::Div => "__divsi3",
                        _ => "__modsi3",
                    }
                ));
                self.line("    add r6, r1, r0");
            }
            BinaryOp::Eq => self.boolean_from_branch("beq", "r7", "r6"),
            BinaryOp::Ne => self.boolean_from_branch("bne", "r7", "r6"),
            BinaryOp::Lt => self.boolean_from_branch("blt", "r7", "r6"),
            BinaryOp::Ge => self.boolean_from_branch("bge", "r7", "r6"),
            BinaryOp::Le => self.boolean_from_branch("bge", "r6", "r7"),
            BinaryOp::Gt => self.boolean_from_branch("blt", "r6", "r7"),
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => unreachable!(),
        }
        Ok(())
    }

    fn emit_lvalue(&mut self, expression: &Expr) -> CompileResult<()> {
        match &expression.kind {
            ExprKind::Variable {
                symbol: Some(id), ..
            } => {
                if let Some(address) = self.globals.get(id) {
                    self.line(&format!("    li r6, 0x{address:08x}"));
                } else {
                    let offset = self
                        .frame
                        .as_ref()
                        .and_then(|frame| frame.offsets.get(id))
                        .ok_or_else(|| {
                            self.codegen_error(expression, "ローカル変数のフレーム配置がありません")
                        })?;
                    self.line(&format!("    addi r6, r13, {offset}"));
                }
            }
            ExprKind::Unary {
                op: UnaryOp::Dereference,
                expression,
            } => self.emit_expr(expression)?,
            ExprKind::Index { base, index } => {
                self.emit_expr(base)?;
                self.push("r6");
                self.emit_expr(index)?;
                let size = expression.ty.as_ref().unwrap().size().unwrap();
                self.scale("r6", size);
                self.pop("r7");
                self.line("    add r6, r7, r6");
            }
            _ => {
                return Err(self.codegen_error(expression, "この式は左辺値アドレスを生成できません"))
            }
        }
        Ok(())
    }

    fn load_from_r6(&mut self, ty: &Type) {
        self.line(if *ty == Type::Char {
            "    loadb r6, [r6 + 0]"
        } else {
            "    load r6, [r6 + 0]"
        });
    }
    fn store_to(&mut self, value: &str, address: &str, ty: &Type) {
        self.line(&format!(
            "    {} {value}, [{address} + 0]",
            if *ty == Type::Char { "storeb" } else { "store" }
        ));
    }
    fn adjust_scalar(&mut self, register: &str, ty: &Type, increment: bool) {
        let amount = ty.decay().pointed().and_then(Type::size).unwrap_or(1) as i32
            * if increment { 1 } else { -1 };
        self.line(&format!("    addi {register}, {register}, {amount}"));
    }
    fn scale(&mut self, register: &str, size: usize) {
        if size == 4 {
            self.line("    movi r8, 2");
            self.line(&format!("    shl {register}, {register}, r8"));
        }
    }
    fn push(&mut self, register: &str) {
        self.line("    addi r15, r15, -4");
        self.line(&format!("    store {register}, [r15 + 0]"));
    }
    fn pop(&mut self, register: &str) {
        self.line(&format!("    load {register}, [r15 + 0]"));
        self.line("    addi r15, r15, 4");
    }
    fn boolean_from_branch(&mut self, branch: &str, lhs: &str, rhs: &str) {
        let yes = self.new_label("bool_true");
        let end = self.new_label("bool_end");
        self.line(&format!("    {branch} {lhs}, {rhs}, {yes}"));
        self.line("    movi r6, 0");
        self.line(&format!("    jmp {end}"));
        self.line(&format!("{yes}:"));
        self.line("    movi r6, 1");
        self.line(&format!("{end}:"));
    }
    fn new_label(&mut self, stem: &str) -> String {
        let value = format!(".L_{stem}_{}", self.label);
        self.label += 1;
        value
    }
    fn line(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
    }
    fn codegen_error(&self, expression: &Expr, message: &str) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Codegen, expression.span, message)
    }

    fn emit_runtime(&mut self) {
        self.line("; 最小ランタイム");
        self.line("putchar:");
        self.line(&format!("    li r6, 0x{UART_TX:08x}"));
        self.line("    storeb r2, [r6 + 0]");
        self.line("    movi r1, 0");
        self.line("    ret");
        self.line("");
        self.line("puts:");
        self.line("    add r6, r2, r0");
        self.line(".L_rt_puts_loop:");
        self.line("    loadb r7, [r6 + 0]");
        self.line("    beq r7, r0, .L_rt_puts_end");
        self.line(&format!("    li r8, 0x{UART_TX:08x}"));
        self.line("    storeb r7, [r8 + 0]");
        self.line("    addi r6, r6, 1");
        self.line("    jmp .L_rt_puts_loop");
        self.line(".L_rt_puts_end:");
        self.line("    movi r1, 0");
        self.line("    ret");
        self.line("");
        self.line("strlen:");
        self.line("    movi r1, 0");
        self.line(".L_rt_strlen_loop:");
        self.line("    loadb r6, [r2 + 0]");
        self.line("    beq r6, r0, .L_rt_strlen_end");
        self.line("    addi r1, r1, 1");
        self.line("    addi r2, r2, 1");
        self.line("    jmp .L_rt_strlen_loop");
        self.line(".L_rt_strlen_end:");
        self.line("    ret");
        self.line("");
        self.line("memset:");
        self.line("    add r1, r2, r0");
        self.line("    movi r6, 0");
        self.line(".L_rt_memset_loop:");
        self.line("    bge r6, r4, .L_rt_memset_end");
        self.line("    storeb r3, [r2 + 0]");
        self.line("    addi r2, r2, 1");
        self.line("    addi r6, r6, 1");
        self.line("    jmp .L_rt_memset_loop");
        self.line(".L_rt_memset_end:");
        self.line("    ret");
        self.line("");
        self.line("memcpy:");
        self.line("    add r1, r2, r0");
        self.line("    movi r6, 0");
        self.line(".L_rt_memcpy_loop:");
        self.line("    bge r6, r4, .L_rt_memcpy_end");
        self.line("    loadb r7, [r3 + 0]");
        self.line("    storeb r7, [r2 + 0]");
        self.line("    addi r2, r2, 1");
        self.line("    addi r3, r3, 1");
        self.line("    addi r6, r6, 1");
        self.line("    jmp .L_rt_memcpy_loop");
        self.line(".L_rt_memcpy_end:");
        self.line("    ret");
        self.line("");
        self.emit_mul();
        self.emit_divmod("__divsi3", false);
        self.emit_divmod("__modsi3", true);
        self.emit_print_int();
        self.line("panic:");
        self.line("    addi r15, r15, -4");
        self.line("    store r14, [r15 + 0]");
        self.line("    call puts");
        self.line(&format!("    li r6, 0x{SIM_EXIT:08x}"));
        self.line("    movi r7, 1");
        self.line("    store r7, [r6 + 0]");
        self.line("    halt");
        self.line("");
    }

    fn emit_mul(&mut self) {
        self.line("__mulsi3:");
        self.line("    movi r1, 0");
        self.line("    add r6, r2, r0");
        self.line("    add r7, r3, r0");
        self.line("    movi r8, 1");
        self.line(".L_rt_mul_loop:");
        self.line("    beq r7, r0, .L_rt_mul_end");
        self.line("    and r9, r7, r8");
        self.line("    beq r9, r0, .L_rt_mul_skip");
        self.line("    add r1, r1, r6");
        self.line(".L_rt_mul_skip:");
        self.line("    shl r6, r6, r8");
        self.line("    shr r7, r7, r8");
        self.line("    jmp .L_rt_mul_loop");
        self.line(".L_rt_mul_end:");
        self.line("    ret");
        self.line("");
    }

    fn emit_divmod(&mut self, name: &str, remainder: bool) {
        let prefix = if remainder { "mod" } else { "div" };
        self.line(&format!("{name}:"));
        self.line(&format!("    beq r3, r0, .L_rt_{prefix}_zero"));
        self.line("    add r6, r2, r0");
        self.line("    add r7, r3, r0");
        self.line("    movi r8, 0");
        self.line(&format!("    bge r6, r0, .L_rt_{prefix}_a_pos"));
        self.line("    sub r6, r0, r6");
        self.line("    addi r8, r8, 1");
        self.line(&format!(".L_rt_{prefix}_a_pos:"));
        self.line(&format!("    bge r7, r0, .L_rt_{prefix}_b_pos"));
        self.line("    sub r7, r0, r7");
        if !remainder {
            self.line("    addi r8, r8, 1");
        }
        self.line(&format!(".L_rt_{prefix}_b_pos:"));
        self.line("    movi r1, 0");
        self.line(&format!(".L_rt_{prefix}_loop:"));
        self.line(&format!("    blt r6, r7, .L_rt_{prefix}_end"));
        self.line("    sub r6, r6, r7");
        self.line("    addi r1, r1, 1");
        self.line(&format!("    jmp .L_rt_{prefix}_loop"));
        self.line(&format!(".L_rt_{prefix}_end:"));
        if remainder {
            self.line("    add r1, r6, r0");
            self.line("    and r8, r8, r8");
            self.line(&format!("    beq r8, r0, .L_rt_{prefix}_ret"));
            self.line("    sub r1, r0, r1");
        } else {
            self.line("    movi r9, 1");
            self.line("    and r8, r8, r9");
            self.line(&format!("    beq r8, r0, .L_rt_{prefix}_ret"));
            self.line("    sub r1, r0, r1");
        }
        self.line(&format!(".L_rt_{prefix}_ret:"));
        self.line("    ret");
        self.line(&format!(".L_rt_{prefix}_zero:"));
        self.line(&format!("    li r6, 0x{SIM_EXIT:08x}"));
        self.line("    movi r7, 1");
        self.line("    store r7, [r6 + 0]");
        self.line("    halt");
        self.line("");
    }

    fn emit_print_int(&mut self) {
        self.line("print_int:");
        self.line("    addi r15, r15, -12");
        self.line("    store r13, [r15 + 0]");
        self.line("    store r14, [r15 + 4]");
        self.line("    store r11, [r15 + 8]");
        self.line("    add r13, r15, r0");
        self.line("    add r11, r2, r0");
        self.line("    movi r10, 0");
        self.line("    bne r11, r0, .L_rt_print_nonzero");
        self.line("    movi r2, 48");
        self.line("    call putchar");
        self.line("    jmp .L_rt_print_done");
        self.line(".L_rt_print_nonzero:");
        self.line("    bge r11, r0, .L_rt_print_digits");
        self.line("    movi r2, 45");
        self.line("    call putchar");
        self.line("    sub r11, r0, r11");
        self.line(".L_rt_print_digits:");
        self.line("    add r2, r11, r0");
        self.line("    movi r3, 10");
        self.line("    call __modsi3");
        self.line("    addi r1, r1, 48");
        self.line("    addi r15, r15, -4");
        self.line("    store r1, [r15 + 0]");
        self.line("    addi r10, r10, 1");
        self.line("    add r2, r11, r0");
        self.line("    movi r3, 10");
        self.line("    call __divsi3");
        self.line("    add r11, r1, r0");
        self.line("    bne r11, r0, .L_rt_print_digits");
        self.line(".L_rt_print_output:");
        self.line("    beq r10, r0, .L_rt_print_done");
        self.line("    load r2, [r15 + 0]");
        self.line("    addi r15, r15, 4");
        self.line("    call putchar");
        self.line("    addi r10, r10, -1");
        self.line("    jmp .L_rt_print_output");
        self.line(".L_rt_print_done:");
        self.line("    movi r1, 0");
        self.line("    add r15, r13, r0");
        self.line("    load r13, [r15 + 0]");
        self.line("    load r14, [r15 + 4]");
        self.line("    load r11, [r15 + 8]");
        self.line("    addi r15, r15, 12");
        self.line("    ret");
        self.line("");
    }
}

fn align(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
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
