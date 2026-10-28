//! Variable scopes.
//!
//! Each environment holds a name -> value map and an optional parent.
//! Lookup and assignment walk up the parent chain.
//! A block or a function call creates a child environment.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::{Builtin, Value};

/// A scope with a parent link.
pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    /// Create the global scope with the builtin functions.
    pub fn new() -> Rc<RefCell<Self>> {
        let env = Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: None,
        }));
        env.borrow_mut()
            .define("notië".to_string(), Value::Builtin(Builtin::Len));
        env.borrow_mut()
            .define("napan".to_string(), Value::Builtin(Builtin::Push));
        env
    }

    /// Create a child scope. The child keeps the parent alive through `Rc`.
    pub fn new_child(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(parent),
        }))
    }

    /// Define a name in this scope. The name can shadow a parent name.
    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    /// Look in this scope first, then in the parent.
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            return Some(value.clone());
        }

        self.parent
            .as_ref()
            .and_then(|parent| parent.borrow().get(name))
    }

    /// Change an existing variable. Do not create a new one.
    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            true
        } else if let Some(parent) = &self.parent {
            parent.borrow_mut().assign(name, value)
        } else {
            false
        }
    }
}
