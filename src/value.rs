//! Runtime values.
//!
//! Values are dynamically typed: a variable can hold an integer, a string,
//! a boolean, a function, a builtin, a list, a struct, or unit.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::Stmt;
use crate::environment::Environment;

/// A pointer that identifies a list value.
type ListPointer = *const RefCell<Vec<Value>>;
/// A pair of list pointers that the equality check has already seen.
type SeenPair = (ListPointer, ListPointer);
/// A pointer that identifies a struct value.
type StructPointer = *const RefCell<StructInstance>;
/// A pair of struct pointers that the equality check has already seen.
type SeenStructPair = (StructPointer, StructPointer);

/// A value that the formatter has already visited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeenValue {
    List(ListPointer),
    Struct(StructPointer),
}

/// A builtin function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    /// `notië(value)` returns the length of a list or string.
    Len,
    /// `napan(list, value)` appends a value to a list.
    Push,
}

impl Builtin {
    /// The name used in source code.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Len => "notië",
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
    /// A mutable struct. Fields keep their source order for display.
    Struct(Rc<RefCell<StructInstance>>),
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

/// A struct value with named fields.
pub struct StructInstance {
    pub type_name: String,
    pub fields: Vec<(String, Value)>,
}

impl StructInstance {
    /// Read a field by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }

    /// Write a field by name.
    pub fn set(&mut self, name: &str, value: Value) -> bool {
        if let Some((_, field)) = self.fields.iter_mut().find(|(field, _)| field == name) {
            *field = value;
            true
        } else {
            false
        }
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

// Format a value. `nested` quotes strings inside lists and structs.
// `seen` stops self-referential values from recursing forever.
fn fmt_value(
    value: &Value,
    f: &mut fmt::Formatter<'_>,
    nested: bool,
    seen: &mut Vec<SeenValue>,
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
        Value::Bool(true) => write!(f, "naitë"),
        Value::Bool(false) => write!(f, "lá naitë"),
        Value::Function(function) => write!(f, "<maita {}>", function.name),
        Value::Builtin(builtin) => write!(f, "<builtin {}>", builtin.name()),
        Value::List(list) => {
            let marker = SeenValue::List(Rc::as_ptr(list));
            if seen.contains(&marker) {
                return write!(f, "[...]");
            }
            seen.push(marker);
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
        Value::Struct(instance) => {
            let marker = SeenValue::Struct(Rc::as_ptr(instance));
            let type_name = instance.borrow().type_name.clone();
            if seen.contains(&marker) {
                return write!(f, "{type_name} {{ ... }}");
            }
            seen.push(marker);

            let instance = instance.borrow();
            write!(f, "{} {{", instance.type_name)?;
            for (index, (name, value)) in instance.fields.iter().enumerate() {
                if index == 0 {
                    write!(f, " ")?;
                } else {
                    write!(f, ", ")?;
                }
                write!(f, "{name}: ")?;
                fmt_value(value, f, true, seen)?;
            }
            if !instance.fields.is_empty() {
                write!(f, " ")?;
            }
            write!(f, "}}")?;
            seen.pop();
            Ok(())
        }
        Value::Unit => write!(f, "()"),
    }
}

/// Compare two values. Functions compare by identity, not by structure.
/// Lists and structs compare by structure, with a guard for cycles.
pub fn values_equal(left: &Value, right: &Value) -> bool {
    let mut seen_lists = Vec::new();
    let mut seen_structs = Vec::new();
    values_equal_inner(left, right, &mut seen_lists, &mut seen_structs)
}

fn values_equal_inner(
    left: &Value,
    right: &Value,
    seen_lists: &mut Vec<SeenPair>,
    seen_structs: &mut Vec<SeenStructPair>,
) -> bool {
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
            if seen_lists.contains(&pair) {
                return true;
            }
            seen_lists.push(pair);
            let a_items = a.borrow();
            let b_items = b.borrow();
            let equal = a_items.len() == b_items.len()
                && a_items
                    .iter()
                    .zip(b_items.iter())
                    .all(|(a, b)| values_equal_inner(a, b, seen_lists, seen_structs));
            seen_lists.pop();
            equal
        }
        (Value::Struct(a), Value::Struct(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let pair = (Rc::as_ptr(a), Rc::as_ptr(b));
            if seen_structs.contains(&pair) {
                return true;
            }
            seen_structs.push(pair);
            let a_instance = a.borrow();
            let b_instance = b.borrow();
            let equal = a_instance.type_name == b_instance.type_name
                && a_instance.fields.len() == b_instance.fields.len()
                && a_instance.fields.iter().all(|(name, value)| {
                    b_instance
                        .fields
                        .iter()
                        .find(|(other, _)| other == name)
                        .map(|(_, other)| {
                            values_equal_inner(value, other, seen_lists, seen_structs)
                        })
                        .unwrap_or(false)
                });
            seen_structs.pop();
            equal
        }
        _ => false,
    }
}
