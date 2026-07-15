//! 関数ごとのローカル変数配置を計算する。

use crate::ast::{Function, Stmt, StmtKind, SymbolId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Frame {
    pub offsets: HashMap<SymbolId, i32>,
    pub size: usize,
}

impl Frame {
    pub fn build(function: &Function) -> Self {
        let mut frame = Self {
            offsets: HashMap::new(),
            size: 0,
        };
        for parameter in &function.parameters {
            frame.allocate(
                parameter.id,
                parameter.ty.size().unwrap_or(4),
                parameter.ty.alignment(),
            );
        }
        if let Some(body) = &function.body {
            frame.walk(body);
        }
        frame.size = align(frame.size, 4);
        frame
    }
    fn walk(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Declaration(variable) => self.allocate(
                variable.id,
                variable.ty.size().unwrap_or(4),
                variable.ty.alignment(),
            ),
            StmtKind::Block(items) => {
                for item in items {
                    self.walk(item);
                }
            }
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.walk(then_branch);
                if let Some(branch) = else_branch {
                    self.walk(branch);
                }
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } => self.walk(body),
            _ => {}
        }
    }
    fn allocate(&mut self, id: SymbolId, size: usize, alignment: usize) {
        if self.offsets.contains_key(&id) {
            return;
        }
        self.size = align(self.size, alignment) + size;
        self.offsets.insert(id, -(self.size as i32));
    }
}

fn align(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
