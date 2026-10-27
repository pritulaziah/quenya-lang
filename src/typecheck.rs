//! The static type checker.
//!
//! Rust checks types before a program runs. This pass does the same for
//! Quenya: it walks the AST after parsing and before interpretation.
//!
//! The checker infers local variable types and checks function signatures,
//! calls, operators, lists, conditions, and returns. It is deliberately
//! small: there are no generics, traits, or ownership yet. The builtins
//! `not` and `napan` are special-cased until the language gets traits.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, Type, UnaryOp};
use crate::error::Error;

/// A type used during inference. `Var` is a type variable.
#[derive(Debug, Clone, PartialEq, Eq)]
enum InferType {
    Int,
    Str,
    Bool,
    Unit,
    List(Box<InferType>),
    Function(Vec<InferType>, Box<InferType>),
    Builtin(BuiltinKind),
    Var(usize),
}

impl InferType {
    // Convert a type written in source code into an inference type.
    fn from_ast(ty: &Type) -> Self {
        match ty {
            Type::Int => InferType::Int,
            Type::Str => InferType::Str,
            Type::Bool => InferType::Bool,
            Type::Unit => InferType::Unit,
            Type::List(inner) => InferType::List(Box::new(InferType::from_ast(inner))),
            Type::Function(params, ret) => InferType::Function(
                params.iter().map(InferType::from_ast).collect(),
                Box::new(InferType::from_ast(ret)),
            ),
        }
    }
}

impl fmt::Display for InferType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferType::Int => write!(f, "Int"),
            InferType::Str => write!(f, "Str"),
            InferType::Bool => write!(f, "Bool"),
            InferType::Unit => write!(f, "Unit"),
            InferType::List(inner) => write!(f, "List<{inner}>"),
            InferType::Function(params, ret) => {
                write!(f, "maita(")?;
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {ret}")
            }
            InferType::Builtin(builtin) => write!(f, "<builtin {}>", builtin.name()),
            InferType::Var(_) => write!(f, "_"),
        }
    }
}

/// A builtin with a special type rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinKind {
    Len,
    Push,
}

impl BuiltinKind {
    fn name(self) -> &'static str {
        match self {
            BuiltinKind::Len => "not",
            BuiltinKind::Push => "napan",
        }
    }

    fn arity(self) -> usize {
        match self {
            BuiltinKind::Len => 1,
            BuiltinKind::Push => 2,
        }
    }
}

/// The type checker keeps scopes, type variables, and the current return type.
#[derive(Clone)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, InferType>>,
    substitutions: Vec<Option<InferType>>,
    current_return: Option<InferType>,
    loop_depth: usize,
    /// Type variables from empty lists, with the line of the `[`.
    pending: Vec<(usize, usize)>,
}

impl TypeChecker {
    /// Create a checker with the builtin functions in the global scope.
    pub fn new() -> Self {
        let mut global = HashMap::new();
        global.insert("not".to_string(), InferType::Builtin(BuiltinKind::Len));
        global.insert("napan".to_string(), InferType::Builtin(BuiltinKind::Push));
        Self {
            scopes: vec![global],
            substitutions: Vec::new(),
            current_return: None,
            loop_depth: 0,
            pending: Vec::new(),
        }
    }

    /// Check a program. On error, restore the checker so a REPL can continue.
    pub fn check_program(&mut self, program: &[Stmt]) -> Result<(), Error> {
        let checkpoint = self.clone();
        let result = self.check_program_inner(program);
        if result.is_err() {
            *self = checkpoint;
        }
        result
    }

    fn check_program_inner(&mut self, program: &[Stmt]) -> Result<(), Error> {
        for stmt in program {
            self.check_stmt(stmt)?;
        }
        self.check_pending()
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Error> {
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let value_type = self.infer(value)?;
                let declared = if let Some(ty) = ty {
                    let declared = InferType::from_ast(ty);
                    self.unify(&declared, &value_type, stmt.line)?;
                    declared
                } else {
                    value_type
                };
                self.define(name, declared);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let variable = self
                    .lookup(name)
                    .ok_or_else(|| Error::new(stmt.line, format!("undefined variable `{name}`")))?;
                let value_type = self.infer(value)?;
                self.unify(&variable, &value_type, stmt.line)
            }
            StmtKind::IndexAssign {
                target,
                index,
                value,
            } => {
                let target_type = self.infer(target)?;
                let element = self.fresh_var();
                self.unify(
                    &target_type,
                    &InferType::List(Box::new(element.clone())),
                    target.line,
                )?;
                let index_type = self.infer(index)?;
                self.unify(&InferType::Int, &index_type, index.line)?;
                let value_type = self.infer(value)?;
                self.unify(&element, &value_type, value.line)
            }
            StmtKind::Print(expr) => {
                self.infer(expr)?;
                Ok(())
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.infer(condition)?;
                self.require_bool(&condition_type, condition.line)?;
                self.check_block(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.check_stmt(else_branch)?;
                }
                Ok(())
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.infer(condition)?;
                self.require_bool(&condition_type, condition.line)?;
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                result
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                let iterable_type = self.infer(iterable)?;
                let element = match self.resolve(&iterable_type) {
                    InferType::List(element) => *element,
                    InferType::Var(_) => {
                        let element = self.fresh_var();
                        self.unify(
                            &iterable_type,
                            &InferType::List(Box::new(element.clone())),
                            iterable.line,
                        )?;
                        element
                    }
                    other => {
                        return Err(Error::new(
                            iterable.line,
                            format!("expected a list, found `{}`", self.type_name(&other)),
                        ));
                    }
                };
                self.scopes.push(HashMap::new());
                self.define(var, element);
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                result
            }
            StmtKind::Break => {
                if self.loop_depth == 0 {
                    Err(Error::new(stmt.line, "`rac` outside loop"))
                } else {
                    Ok(())
                }
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    Err(Error::new(stmt.line, "`lelya` outside loop"))
                } else {
                    Ok(())
                }
            }
            StmtKind::Function {
                name,
                params,
                ret,
                body,
            } => {
                let mut seen = HashSet::new();
                for param in params {
                    if !seen.insert(&param.name) {
                        return Err(Error::new(
                            stmt.line,
                            format!("duplicate parameter `{}`", param.name),
                        ));
                    }
                }

                let param_types: Vec<InferType> = params
                    .iter()
                    .map(|param| InferType::from_ast(&param.ty))
                    .collect();
                let return_type = InferType::from_ast(ret);
                self.define(
                    name,
                    InferType::Function(param_types.clone(), Box::new(return_type.clone())),
                );

                self.scopes.push(HashMap::new());
                for (param, ty) in params.iter().zip(param_types) {
                    self.define(&param.name, ty);
                }
                let saved_return = self.current_return.replace(return_type.clone());
                let saved_loop = self.loop_depth;
                self.loop_depth = 0;
                let result = self.check_block(body);
                self.current_return = saved_return;
                self.loop_depth = saved_loop;
                self.scopes.pop();
                result?;

                if return_type != InferType::Unit && !block_returns(body) {
                    return Err(Error::new(
                        stmt.line,
                        "not all control paths return a value",
                    ));
                }
                Ok(())
            }
            StmtKind::Return(value) => {
                let Some(expected) = self.current_return.clone() else {
                    return Err(Error::new(stmt.line, "`entul` outside function"));
                };
                let value_type = match value {
                    Some(expr) => self.infer(expr)?,
                    None => InferType::Unit,
                };
                self.unify(&expected, &value_type, stmt.line)
            }
            StmtKind::Expression(expr) => {
                self.infer(expr)?;
                Ok(())
            }
            StmtKind::Block(body) => self.check_block(body),
        }
    }

    fn check_block(&mut self, body: &[Stmt]) -> Result<(), Error> {
        self.scopes.push(HashMap::new());
        let mut result = Ok(());
        for stmt in body {
            if let Err(error) = self.check_stmt(stmt) {
                result = Err(error);
                break;
            }
        }
        self.scopes.pop();
        result
    }

    fn infer(&mut self, expr: &Expr) -> Result<InferType, Error> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(InferType::Int),
            ExprKind::Str(_) => Ok(InferType::Str),
            ExprKind::Bool(_) => Ok(InferType::Bool),
            ExprKind::List(elements) => {
                if elements.is_empty() {
                    let element = self.fresh_var();
                    if let InferType::Var(id) = element {
                        self.pending.push((id, expr.line));
                    }
                    return Ok(InferType::List(Box::new(element)));
                }

                let first = self.infer(&elements[0])?;
                for element in &elements[1..] {
                    let element_type = self.infer(element)?;
                    self.unify(&first, &element_type, element.line)?;
                }
                Ok(InferType::List(Box::new(first)))
            }
            ExprKind::Variable(name) => self
                .lookup(name)
                .ok_or_else(|| Error::new(expr.line, format!("undefined variable `{name}`"))),
            ExprKind::Unary(operator, operand) => {
                let operand_type = self.infer(operand)?;
                match operator {
                    UnaryOp::Neg => {
                        self.unify(&InferType::Int, &operand_type, expr.line)?;
                        Ok(InferType::Int)
                    }
                    UnaryOp::Not => {
                        self.unify(&InferType::Bool, &operand_type, expr.line)?;
                        Ok(InferType::Bool)
                    }
                }
            }
            ExprKind::Binary(left, operator, right) => {
                self.infer_binary(left, *operator, right, expr.line)
            }
            ExprKind::Call(callee, args) => self.infer_call(callee, args, expr.line),
            ExprKind::Index { target, index } => {
                let target_type = self.infer(target)?;
                let element = self.fresh_var();
                self.unify(
                    &target_type,
                    &InferType::List(Box::new(element.clone())),
                    target.line,
                )?;
                let index_type = self.infer(index)?;
                self.unify(&InferType::Int, &index_type, index.line)?;
                Ok(element)
            }
        }
    }

    fn infer_binary(
        &mut self,
        left: &Expr,
        operator: BinaryOp,
        right: &Expr,
        line: usize,
    ) -> Result<InferType, Error> {
        let left_type = self.infer(left)?;
        let right_type = self.infer(right)?;

        match operator {
            BinaryOp::Add => self.infer_add(&left_type, &right_type, line),
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                self.unify(&InferType::Int, &left_type, line)?;
                self.unify(&InferType::Int, &right_type, line)?;
                Ok(InferType::Int)
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                self.unify(&left_type, &right_type, line)?;
                Ok(InferType::Bool)
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                self.unify(&InferType::Int, &left_type, line)?;
                self.unify(&InferType::Int, &right_type, line)?;
                Ok(InferType::Bool)
            }
            BinaryOp::And | BinaryOp::Or => {
                self.unify(&InferType::Bool, &left_type, line)?;
                self.unify(&InferType::Bool, &right_type, line)?;
                Ok(InferType::Bool)
            }
        }
    }

    // `+` works for Int, Str, and List. This helper keeps the type errors
    // specific instead of reporting a generic mismatch.
    fn infer_add(
        &mut self,
        left_type: &InferType,
        right_type: &InferType,
        line: usize,
    ) -> Result<InferType, Error> {
        let left = self.resolve(left_type);
        let right = self.resolve(right_type);

        match (&left, &right) {
            (InferType::Int, InferType::Int) => Ok(InferType::Int),
            (InferType::Str, InferType::Str) => Ok(InferType::Str),
            (InferType::List(left_element), InferType::List(right_element)) => {
                self.unify(left_element, right_element, line)?;
                Ok(InferType::List(left_element.clone()))
            }
            (InferType::Var(_), InferType::Int) => {
                self.unify(&InferType::Int, left_type, line)?;
                Ok(InferType::Int)
            }
            (InferType::Var(_), InferType::Str) => {
                self.unify(&InferType::Str, left_type, line)?;
                Ok(InferType::Str)
            }
            (InferType::Var(_), InferType::List(element)) => {
                self.unify(&InferType::List(element.clone()), left_type, line)?;
                Ok(InferType::List(element.clone()))
            }
            (InferType::Int, InferType::Var(_)) => {
                self.unify(&InferType::Int, right_type, line)?;
                Ok(InferType::Int)
            }
            (InferType::Str, InferType::Var(_)) => {
                self.unify(&InferType::Str, right_type, line)?;
                Ok(InferType::Str)
            }
            (InferType::List(element), InferType::Var(_)) => {
                self.unify(&InferType::List(element.clone()), right_type, line)?;
                Ok(InferType::List(element.clone()))
            }
            (InferType::Var(_), InferType::Var(_)) => Err(Error::new(
                line,
                "cannot infer the types for `+`; add type annotations",
            )),
            _ => Err(Error::new(
                line,
                format!(
                    "cannot add `{}` and `{}`",
                    self.type_name(&left),
                    self.type_name(&right)
                ),
            )),
        }
    }

    fn infer_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        line: usize,
    ) -> Result<InferType, Error> {
        let callee_type = self.infer(callee)?;
        match self.resolve(&callee_type) {
            InferType::Function(params, ret) => {
                if args.len() != params.len() {
                    return Err(Error::new(
                        line,
                        format!(
                            "function expects {} arguments, got {}",
                            params.len(),
                            args.len()
                        ),
                    ));
                }
                for (param, arg) in params.iter().zip(args) {
                    let arg_type = self.infer(arg)?;
                    self.unify(param, &arg_type, arg.line)?;
                }
                Ok(*ret)
            }
            InferType::Builtin(BuiltinKind::Len) => {
                if args.len() != BuiltinKind::Len.arity() {
                    return Err(Error::new(
                        line,
                        format!(
                            "function `{}` expects {} arguments, got {}",
                            BuiltinKind::Len.name(),
                            BuiltinKind::Len.arity(),
                            args.len()
                        ),
                    ));
                }
                let arg_type = self.infer(&args[0])?;
                match self.resolve(&arg_type) {
                    InferType::Str | InferType::List(_) => Ok(InferType::Int),
                    InferType::Var(_) => Err(Error::new(
                        args[0].line,
                        "cannot infer the type of the `not` argument; add a type annotation",
                    )),
                    other => Err(Error::new(
                        args[0].line,
                        format!(
                            "`not` expects a list or string, got `{}`",
                            self.type_name(&other)
                        ),
                    )),
                }
            }
            InferType::Builtin(BuiltinKind::Push) => {
                if args.len() != BuiltinKind::Push.arity() {
                    return Err(Error::new(
                        line,
                        format!(
                            "function `{}` expects {} arguments, got {}",
                            BuiltinKind::Push.name(),
                            BuiltinKind::Push.arity(),
                            args.len()
                        ),
                    ));
                }
                let list_type = self.infer(&args[0])?;
                let element = match self.resolve(&list_type) {
                    InferType::List(element) => *element,
                    InferType::Var(_) => {
                        let element = self.fresh_var();
                        self.unify(
                            &list_type,
                            &InferType::List(Box::new(element.clone())),
                            args[0].line,
                        )?;
                        element
                    }
                    other => {
                        return Err(Error::new(
                            args[0].line,
                            format!("`napan` expects a list, got `{}`", self.type_name(&other)),
                        ));
                    }
                };
                let value_type = self.infer(&args[1])?;
                self.unify(&element, &value_type, args[1].line)?;
                Ok(InferType::Unit)
            }
            other => Err(Error::new(
                line,
                format!("`{}` is not callable", self.type_name(&other)),
            )),
        }
    }

    fn require_bool(&mut self, ty: &InferType, line: usize) -> Result<(), Error> {
        match self.resolve(ty) {
            InferType::Bool => Ok(()),
            InferType::Var(_) => self.unify(&InferType::Bool, ty, line),
            other => Err(Error::new(
                line,
                format!("expected boolean, found `{}`", self.type_name(&other)),
            )),
        }
    }

    fn unify(
        &mut self,
        expected: &InferType,
        actual: &InferType,
        line: usize,
    ) -> Result<(), Error> {
        let expected = self.resolve(expected);
        let actual = self.resolve(actual);

        match (&expected, &actual) {
            (InferType::Var(a), InferType::Var(b)) if a == b => Ok(()),
            (InferType::Var(a), _) => {
                if self.occurs(*a, &actual) {
                    return Err(Error::new(line, "cannot construct an infinite type"));
                }
                self.substitutions[*a] = Some(actual);
                Ok(())
            }
            (_, InferType::Var(b)) => {
                if self.occurs(*b, &expected) {
                    return Err(Error::new(line, "cannot construct an infinite type"));
                }
                self.substitutions[*b] = Some(expected);
                Ok(())
            }
            (InferType::Int, InferType::Int)
            | (InferType::Str, InferType::Str)
            | (InferType::Bool, InferType::Bool)
            | (InferType::Unit, InferType::Unit) => Ok(()),
            (InferType::List(a), InferType::List(b)) => self.unify(a, b, line),
            (
                InferType::Function(expected_params, expected_ret),
                InferType::Function(actual_params, actual_ret),
            ) => {
                if expected_params.len() != actual_params.len() {
                    return Err(Error::new(line, "function types have different arities"));
                }
                for (expected, actual) in expected_params.iter().zip(actual_params) {
                    self.unify(expected, actual, line)?;
                }
                self.unify(expected_ret, actual_ret, line)
            }
            (InferType::Builtin(a), InferType::Builtin(b)) if a == b => Ok(()),
            _ => Err(Error::new(
                line,
                format!(
                    "expected `{}`, found `{}`",
                    self.type_name(&expected),
                    self.type_name(&actual)
                ),
            )),
        }
    }

    fn occurs(&self, var: usize, ty: &InferType) -> bool {
        match self.resolve(ty) {
            InferType::Var(other) => other == var,
            InferType::List(inner) => self.occurs(var, &inner),
            InferType::Function(params, ret) => {
                params.iter().any(|param| self.occurs(var, param)) || self.occurs(var, &ret)
            }
            _ => false,
        }
    }

    fn resolve(&self, ty: &InferType) -> InferType {
        match ty {
            InferType::Var(id) => match &self.substitutions[*id] {
                Some(ty) => self.resolve(ty),
                None => InferType::Var(*id),
            },
            InferType::List(inner) => InferType::List(Box::new(self.resolve(inner))),
            InferType::Function(params, ret) => InferType::Function(
                params.iter().map(|param| self.resolve(param)).collect(),
                Box::new(self.resolve(ret)),
            ),
            other => other.clone(),
        }
    }

    fn fresh_var(&mut self) -> InferType {
        let id = self.substitutions.len();
        self.substitutions.push(None);
        InferType::Var(id)
    }

    fn lookup(&self, name: &str) -> Option<InferType> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn define(&mut self, name: &str, ty: InferType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn type_name(&self, ty: &InferType) -> String {
        self.resolve(ty).to_string()
    }

    fn check_pending(&mut self) -> Result<(), Error> {
        for (var, line) in std::mem::take(&mut self.pending) {
            if matches!(self.resolve(&InferType::Var(var)), InferType::Var(_)) {
                return Err(Error::new(
                    line,
                    "cannot infer the type of this empty list; add a type annotation",
                ));
            }
        }
        Ok(())
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

// A block definitely returns when one of its statements definitely returns.
fn block_returns(body: &[Stmt]) -> bool {
    body.iter().any(stmt_returns)
}

// A statement definitely returns if it is `entul`, a returning block, or an
// `if` whose both branches return. This is conservative: loops are ignored.
fn stmt_returns(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => block_returns(then_branch) && stmt_returns(else_branch),
        StmtKind::Block(body) => block_returns(body),
        _ => false,
    }
}
