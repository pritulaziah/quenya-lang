//! Shared helpers for declaration and control-flow checks.

use crate::ast::{Expr, ExprKind, Stmt, StmtKind};

pub(super) fn is_reserved_type_name(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "notessë" | "quetta" | "nanwa" | "er" | "técma"
    )
}

// A block definitely returns when one of its statements definitely returns.
pub(super) fn block_returns(body: &[Stmt]) -> bool {
    body.iter().any(stmt_returns)
}

// A statement definitely returns if it is `entul`, a returning block, or an
// `if` whose both branches return. This is conservative: loops are ignored.
pub(super) fn stmt_returns(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::Let { value, .. } => expr_returns(value),
        StmtKind::Assign { value, .. } => expr_returns(value),
        StmtKind::IndexAssign {
            target,
            index,
            value,
        } => expr_returns(target) || expr_returns(index) || expr_returns(value),
        StmtKind::FieldAssign { target, value, .. } => expr_returns(target) || expr_returns(value),
        StmtKind::Print(expr) => expr_returns(expr),
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_returns(condition)
                || match else_branch {
                    Some(else_branch) => block_returns(then_branch) && stmt_returns(else_branch),
                    None => false,
                }
        }
        StmtKind::While { condition, .. } => expr_returns(condition),
        StmtKind::For { iterable, .. } => expr_returns(iterable),
        StmtKind::Expression(expr) => expr_returns(expr),
        StmtKind::Block(body) => block_returns(body),
        _ => false,
    }
}

pub(super) fn expr_returns(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Block { statements, tail } => {
            block_returns(statements) || tail.as_deref().map(expr_returns).unwrap_or(false)
        }
        ExprKind::Match { value, arms } => {
            expr_returns(value)
                || (!arms.is_empty() && arms.iter().all(|arm| expr_returns(&arm.body)))
        }
        _ => false,
    }
}
