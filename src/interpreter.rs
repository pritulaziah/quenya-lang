//! The tree-walking interpreter.
//!
//! `execute` runs statements. `eval` computes expressions.
//! `Flow::Return` carries `entul` through nested blocks.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::environment::Environment;
use crate::error::Error;
use crate::value::{values_equal, Function, Value};

// `Flow` carries `entul` through nested blocks.
enum Flow {
    Normal,
    Return(Value),
}

/// Run a program in the given environment.
///
/// A top-level `entul` is an error.
pub fn run(
    program: &[Stmt],
    env: &Rc<RefCell<Environment>>,
    out: &mut dyn Write,
) -> Result<(), Error> {
    for stmt in program {
        match execute(stmt, env, out)? {
            Flow::Normal => {}
            Flow::Return(_) => {
                return Err(Error::new(stmt.line, "`entul` outside function"));
            }
        }
    }
    Ok(())
}

fn execute(
    stmt: &Stmt,
    env: &Rc<RefCell<Environment>>,
    out: &mut dyn Write,
) -> Result<Flow, Error> {
    match &stmt.kind {
        // Define in the current scope.
        StmtKind::Let { name, value } => {
            let value = eval(value, env, out)?;
            env.borrow_mut().define(name.clone(), value);
            Ok(Flow::Normal)
        }
        // Assignment walks up the scope chain.
        StmtKind::Assign { name, value } => {
            let value = eval(value, env, out)?;
            if env.borrow_mut().assign(name, value) {
                Ok(Flow::Normal)
            } else {
                Err(Error::new(
                    stmt.line,
                    format!("undefined variable `{name}`"),
                ))
            }
        }
        // Write to the output sink. Tests pass a `Vec<u8>`.
        StmtKind::Print(expr) => {
            let value = eval(expr, env, out)?;
            writeln!(out, "{value}")
                .map_err(|error| Error::new(stmt.line, format!("write error: {error}")))?;
            Ok(Flow::Normal)
        }
        // The condition must be a boolean.
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let line = condition.line;
            let condition = eval(condition, env, out)?;
            if as_bool(condition, line)? {
                execute_block(then_branch, env, out)
            } else if let Some(else_branch) = else_branch {
                execute(else_branch, env, out)
            } else {
                Ok(Flow::Normal)
            }
        }
        // Each iteration gets a fresh block scope.
        StmtKind::While { condition, body } => {
            let line = condition.line;
            loop {
                let condition = eval(condition, env, out)?;
                if !as_bool(condition, line)? {
                    break;
                }
                match execute_block(body, env, out)? {
                    Flow::Normal => {}
                    flow => return Ok(flow),
                }
            }
            Ok(Flow::Normal)
        }
        // Capture the current environment.
        StmtKind::Function { name, params, body } => {
            let function = Function {
                name: name.clone(),
                params: params.clone(),
                body: body.clone(),
                env: env.clone(),
            };
            env.borrow_mut()
                .define(name.clone(), Value::Function(Rc::new(function)));
            Ok(Flow::Normal)
        }
        // Return unit when there is no value.
        StmtKind::Return(value) => {
            let value = match value {
                Some(expr) => eval(expr, env, out)?,
                None => Value::Unit,
            };
            Ok(Flow::Return(value))
        }
        StmtKind::Expression(expr) => {
            eval(expr, env, out)?;
            Ok(Flow::Normal)
        }
        StmtKind::Block(body) => execute_block(body, env, out),
    }
}

// A block creates a new scope.
fn execute_block(
    statements: &[Stmt],
    env: &Rc<RefCell<Environment>>,
    out: &mut dyn Write,
) -> Result<Flow, Error> {
    let block_env = Environment::new_child(env.clone());
    for stmt in statements {
        match execute(stmt, &block_env, out)? {
            Flow::Normal => {}
            flow => return Ok(flow),
        }
    }
    Ok(Flow::Normal)
}

fn eval(expr: &Expr, env: &Rc<RefCell<Environment>>, out: &mut dyn Write) -> Result<Value, Error> {
    match &expr.kind {
        ExprKind::Int(value) => Ok(Value::Int(*value)),
        ExprKind::Str(value) => Ok(Value::Str(value.clone())),
        ExprKind::Bool(value) => Ok(Value::Bool(*value)),
        // Look up the name in the scope chain.
        ExprKind::Variable(name) => env
            .borrow()
            .get(name)
            .ok_or_else(|| Error::new(expr.line, format!("undefined variable `{name}`"))),
        // Use checked arithmetic and report overflow.
        ExprKind::Unary(operator, operand) => {
            let value = eval(operand, env, out)?;
            match operator {
                UnaryOp::Neg => match value {
                    Value::Int(value) => value
                        .checked_neg()
                        .map(Value::Int)
                        .ok_or_else(|| Error::new(expr.line, "integer overflow")),
                    other => Err(Error::new(
                        expr.line,
                        format!("expected integer, got {other}"),
                    )),
                },
                UnaryOp::Not => match value {
                    Value::Bool(value) => Ok(Value::Bool(!value)),
                    other => Err(Error::new(
                        expr.line,
                        format!("expected boolean, got {other}"),
                    )),
                },
            }
        }
        ExprKind::Binary(left, operator, right) => {
            eval_binary(left, *operator, right, expr.line, env, out)
        }
        // Check arity, evaluate arguments, bind parameters, run the body.
        ExprKind::Call(callee, args) => {
            let callee = eval(callee, env, out)?;
            match callee {
                Value::Function(function) => {
                    if args.len() != function.params.len() {
                        return Err(Error::new(
                            expr.line,
                            format!(
                                "function `{}` expects {} arguments, got {}",
                                function.name,
                                function.params.len(),
                                args.len()
                            ),
                        ));
                    }

                    let mut values = Vec::with_capacity(args.len());
                    for arg in args {
                        values.push(eval(arg, env, out)?);
                    }

                    let call_env = Environment::new_child(function.env.clone());
                    for (param, value) in function.params.iter().zip(values) {
                        call_env.borrow_mut().define(param.clone(), value);
                    }

                    match execute_block(&function.body, &call_env, out)? {
                        Flow::Normal => Ok(Value::Unit),
                        Flow::Return(value) => Ok(value),
                    }
                }
                other => Err(Error::new(expr.line, format!("{other} is not callable"))),
            }
        }
    }
}

// `ar` and `hya` short-circuit. The right side runs only when needed.
fn eval_binary(
    left: &Expr,
    operator: BinaryOp,
    right: &Expr,
    line: usize,
    env: &Rc<RefCell<Environment>>,
    out: &mut dyn Write,
) -> Result<Value, Error> {
    if matches!(operator, BinaryOp::And | BinaryOp::Or) {
        let left_value = eval(left, env, out)?;
        let left_bool = as_bool(left_value, left.line)?;

        return match operator {
            BinaryOp::And => {
                if !left_bool {
                    Ok(Value::Bool(false))
                } else {
                    let right_value = eval(right, env, out)?;
                    Ok(Value::Bool(as_bool(right_value, right.line)?))
                }
            }
            BinaryOp::Or => {
                if left_bool {
                    Ok(Value::Bool(true))
                } else {
                    let right_value = eval(right, env, out)?;
                    Ok(Value::Bool(as_bool(right_value, right.line)?))
                }
            }
            _ => unreachable!(),
        };
    }

    let left_value = eval(left, env, out)?;
    let right_value = eval(right, env, out)?;

    match operator {
        BinaryOp::Add => match (left_value, right_value) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| Error::new(line, "integer overflow")),
            (Value::Str(mut a), Value::Str(b)) => {
                a.push_str(&b);
                Ok(Value::Str(a))
            }
            (a, b) => Err(Error::new(line, format!("cannot add {a} and {b}"))),
        },
        BinaryOp::Sub => int_op(left_value, right_value, line, |a, b| a.checked_sub(b)),
        BinaryOp::Mul => int_op(left_value, right_value, line, |a, b| a.checked_mul(b)),
        BinaryOp::Div => match (left_value, right_value) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    return Err(Error::new(line, "division by zero"));
                }
                a.checked_div(b)
                    .map(Value::Int)
                    .ok_or_else(|| Error::new(line, "integer overflow"))
            }
            (a, b) => Err(Error::new(
                line,
                format!("expected integers, got {a} and {b}"),
            )),
        },
        BinaryOp::Rem => match (left_value, right_value) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    return Err(Error::new(line, "division by zero"));
                }
                a.checked_rem(b)
                    .map(Value::Int)
                    .ok_or_else(|| Error::new(line, "integer overflow"))
            }
            (a, b) => Err(Error::new(
                line,
                format!("expected integers, got {a} and {b}"),
            )),
        },
        BinaryOp::Eq => Ok(Value::Bool(values_equal(&left_value, &right_value))),
        BinaryOp::Ne => Ok(Value::Bool(!values_equal(&left_value, &right_value))),
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            match (left_value, right_value) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(match operator {
                    BinaryOp::Lt => a < b,
                    BinaryOp::Le => a <= b,
                    BinaryOp::Gt => a > b,
                    BinaryOp::Ge => a >= b,
                    _ => unreachable!(),
                })),
                (a, b) => Err(Error::new(line, format!("cannot compare {a} and {b}"))),
            }
        }
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

// All integer operations use checked arithmetic.
fn int_op(
    left: Value,
    right: Value,
    line: usize,
    operation: fn(i64, i64) -> Option<i64>,
) -> Result<Value, Error> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => operation(a, b)
            .map(Value::Int)
            .ok_or_else(|| Error::new(line, "integer overflow")),
        (a, b) => Err(Error::new(
            line,
            format!("expected integers, got {a} and {b}"),
        )),
    }
}

// Conditions are strict: only booleans are allowed.
fn as_bool(value: Value, line: usize) -> Result<bool, Error> {
    match value {
        Value::Bool(value) => Ok(value),
        other => Err(Error::new(line, format!("expected boolean, got {other}"))),
    }
}
