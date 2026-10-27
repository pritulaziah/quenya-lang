//! Runtime values.
//!
//! Values are dynamically typed: a variable can hold an integer, a string,
//! a boolean, a function, or unit.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::Stmt;
use crate::environment::Environment;

/// A runtime value.
#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Function(Rc<Function>),
    Unit,
}

/// A user-defined function.
///
/// The function stores its body and the environment where it was declared.
/// This makes closures work.
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub env: Rc<RefCell<Environment>>,
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("params", &self.params)
            .finish()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(value) => write!(f, "{value}"),
            Value::Str(value) => write!(f, "{value}"),
            // Print booleans with the language keywords.
            Value::Bool(true) => write!(f, "naite"),
            Value::Bool(false) => write!(f, "la naite"),
            Value::Function(function) => write!(f, "<maita {}>", function.name),
            Value::Unit => write!(f, "()"),
        }
    }
}

/// Compare two values. Functions compare by identity, not by structure.
pub fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Unit, Value::Unit) => true,
        (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}
