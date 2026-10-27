//! Runtime values.
//!
//! Values are dynamically typed: a variable can hold an integer, a string,
//! a boolean, a function, a builtin, a list, or unit.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::Stmt;
use crate::environment::Environment;

/// A pointer that identifies a list value.
type ListPointer = *const RefCell<Vec<Value>>;
/// A pair of list pointers that the equality check has already seen.
type SeenPair = (ListPointer, ListPointer);

/// A builtin function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    /// `not(value)` returns the length of a list or string.
    Len,
    /// `napan(list, value)` appends a value to a list.
    Push,
}

impl Builtin {
    /// The name used in source code.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Len => "not",
            Builtin::Push => "napan",
        }
    }

    /// The number of arguments the builtin expects.
    pub fn arity(self) -> usize {
        match self {
            Builtin::Len => 1,
            Builtin::Push => 2,
        }
    }
}

/// A runtime value.
#[derive(Clone)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Function(Rc<Function>),
    Builtin(Builtin),
    /// A mutable list. `Rc<RefCell<...>>` gives reference semantics.
    List(Rc<RefCell<Vec<Value>>>),
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
        let mut seen = Vec::new();
        fmt_value(self, f, false, &mut seen)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

// Format a value. `nested` quotes strings inside lists.
// `seen` stops self-referential lists from recursing forever.
fn fmt_value(
    value: &Value,
    f: &mut fmt::Formatter<'_>,
    nested: bool,
    seen: &mut Vec<ListPointer>,
) -> fmt::Result {
    match value {
        Value::Int(value) => write!(f, "{value}"),
        Value::Str(value) => {
            if nested {
                write!(f, "{value:?}")
            } else {
                write!(f, "{value}")
            }
        }
        // Print booleans with the language keywords.
        Value::Bool(true) => write!(f, "naite"),
        Value::Bool(false) => write!(f, "la naite"),
        Value::Function(function) => write!(f, "<maita {}>", function.name),
        Value::Builtin(builtin) => write!(f, "<builtin {}>", builtin.name()),
        Value::List(list) => {
            let pointer = Rc::as_ptr(list);
            if seen.contains(&pointer) {
                return write!(f, "[...]");
            }
            seen.push(pointer);
            write!(f, "[")?;
            for (index, item) in list.borrow().iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                fmt_value(item, f, true, seen)?;
            }
            write!(f, "]")?;
            seen.pop();
            Ok(())
        }
        Value::Unit => write!(f, "()"),
    }
}

/// Compare two values. Functions compare by identity, not by structure.
/// Lists compare by structure, with a guard for self-referential lists.
pub fn values_equal(left: &Value, right: &Value) -> bool {
    let mut seen = Vec::new();
    values_equal_inner(left, right, &mut seen)
}

fn values_equal_inner(left: &Value, right: &Value, seen: &mut Vec<SeenPair>) -> bool {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Unit, Value::Unit) => true,
        (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
        (Value::Builtin(a), Value::Builtin(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let pair = (Rc::as_ptr(a), Rc::as_ptr(b));
            if seen.contains(&pair) {
                return true;
            }
            seen.push(pair);
            let a_items = a.borrow();
            let b_items = b.borrow();
            let equal = a_items.len() == b_items.len()
                && a_items
                    .iter()
                    .zip(b_items.iter())
                    .all(|(a, b)| values_equal_inner(a, b, seen));
            seen.pop();
            equal
        }
        _ => false,
    }
}
