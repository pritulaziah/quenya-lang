//! Ownership checks: mutability, moves, and places.
//!
//! This module models a small part of Rust's ownership rules:
//!
//! - A binding is immutable unless it is declared with `vistë`.
//! - Copy types are copied on use. Other types are moved.
//! - A moved value cannot be used again.
//! - A field or an index can be assigned only through a mutable root.
//! - `napan` needs a mutable list.
//! - `quétë`, `notië`, and `clone` borrow their argument.
//!
//! References and borrow checking come in a later step. For now methods
//! borrow their receiver, and closures capture outer names by reference.

use std::collections::HashMap;

use crate::ast::{Expr, ExprKind};
use crate::error::Error;

use super::types::InferType;
use super::{Scope, TypeChecker};

/// How an expression is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Consume the value. Non-Copy values move.
    Value,
    /// Borrow the value. No move.
    Place,
    /// Infer the type only. No move and no availability check. This is used
    /// for the base of a field or index access, so a partial move of one field
    /// does not block access to another field.
    Type,
}

/// A place that can be read, written, or moved.
#[derive(Debug, Clone)]
pub(super) enum Place {
    Var(String),
    Field(Box<Place>, String),
    Index(Box<Place>),
    /// A value that is not stored in a named place.
    Temporary,
}

impl Place {
    /// The variable at the root of the place.
    fn root_var(&self) -> Option<&str> {
        match self {
            Place::Var(name) => Some(name),
            Place::Field(base, _) | Place::Index(base) => base.root_var(),
            Place::Temporary => None,
        }
    }

    /// A path of field names from the root variable. `None` when the path
    /// contains an index or a temporary.
    fn field_path(&self) -> Option<Vec<String>> {
        match self {
            Place::Var(_) => Some(Vec::new()),
            Place::Field(base, name) => {
                let mut path = base.field_path()?;
                path.push(name.clone());
                Some(path)
            }
            Place::Index(_) | Place::Temporary => None,
        }
    }

    /// True when an index is part of the place.
    fn has_index(&self) -> bool {
        match self {
            Place::Index(_) => true,
            Place::Field(base, _) => base.has_index(),
            Place::Var(_) | Place::Temporary => false,
        }
    }
}

/// The move state of one variable.
#[derive(Debug, Clone, Default)]
pub(super) struct MoveState {
    /// The line that moved the whole value, if it was moved.
    fully_moved_at: Option<usize>,
    /// Moved fields, keyed by dotted path, with the line of the move.
    moved_fields: HashMap<String, usize>,
}

impl MoveState {
    /// The line that moved this path or one of its prefixes.
    fn moved_at(&self, path: &[String]) -> Option<usize> {
        if let Some(line) = self.fully_moved_at {
            return Some(line);
        }
        // Using the whole value after a partial move is an error.
        if path.is_empty() {
            return self.moved_fields.values().min().copied();
        }
        let mut key = String::new();
        for part in path {
            if !key.is_empty() {
                key.push('.');
            }
            key.push_str(part);
            if let Some(line) = self.moved_fields.get(&key) {
                return Some(*line);
            }
        }
        None
    }

    fn mark_moved(&mut self, path: &[String], line: usize) {
        if path.is_empty() {
            self.fully_moved_at = Some(line);
        } else {
            self.moved_fields.insert(path.join("."), line);
        }
    }

    fn reinit(&mut self, path: &[String]) {
        if path.is_empty() {
            self.fully_moved_at = None;
            self.moved_fields.clear();
        } else {
            let key = path.join(".");
            self.moved_fields.remove(&key);
            let prefix = format!("{key}.");
            self.moved_fields
                .retain(|field, _| !field.starts_with(&prefix));
        }
    }

    /// Union two branch states. A value moved in either branch is moved after
    /// the branch.
    fn merge(left: &MoveState, right: &MoveState) -> MoveState {
        let fully_moved_at = match (left.fully_moved_at, right.fully_moved_at) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
        let mut moved_fields = left.moved_fields.clone();
        for (path, line) in &right.moved_fields {
            moved_fields.entry(path.clone()).or_insert(*line);
        }
        MoveState {
            fully_moved_at,
            moved_fields,
        }
    }
}

/// Build a place from an expression without checking it.
pub(super) fn place_of(expr: &Expr) -> Option<Place> {
    match &expr.kind {
        ExprKind::Variable(name) => Some(Place::Var(name.clone())),
        ExprKind::Field { target, name } => {
            let base = place_of(target)?;
            Some(Place::Field(Box::new(base), name.clone()))
        }
        ExprKind::Index { target, .. } => {
            let base = place_of(target)?;
            Some(Place::Index(Box::new(base)))
        }
        _ => None,
    }
}

/// Merge the outer scopes after an `if`. Branch-local scopes are already gone.
pub(super) fn merge_scopes(
    before: &[Scope],
    then_scopes: &[Scope],
    else_scopes: &[Scope],
) -> Vec<Scope> {
    let mut merged = before.to_vec();
    for (index, scope) in merged.iter_mut().enumerate() {
        for (name, state) in scope.moves.iter_mut() {
            let then_state = then_scopes
                .get(index)
                .and_then(|scope| scope.moves.get(name));
            let else_state = else_scopes
                .get(index)
                .and_then(|scope| scope.moves.get(name));
            *state = match (then_state, else_state) {
                (Some(left), Some(right)) => MoveState::merge(left, right),
                (Some(state), None) | (None, Some(state)) => state.clone(),
                (None, None) => MoveState::default(),
            };
        }
    }
    merged
}

impl TypeChecker {
    /// True for types that Rust copies instead of moving.
    pub(super) fn is_copy_type(&self, ty: &InferType) -> bool {
        match self.resolve(ty) {
            InferType::Int | InferType::Bool | InferType::Unit => true,
            InferType::Str | InferType::List(_) => false,
            InferType::Named(name) => {
                // Structs are moved. The language has no `Copy` trait, and a
                // copied struct would share its runtime storage. Enums are
                // immutable, so a structural Copy rule is safe for them.
                if let Some(info) = self.enums.get(&name) {
                    info.variants.iter().all(|variant| {
                        variant
                            .fields
                            .iter()
                            .all(|(_, field)| self.is_copy_type(field))
                    })
                } else {
                    false
                }
            }
            InferType::Function(_, _) | InferType::Builtin(_) => true,
            InferType::Var(_) => false,
        }
    }

    /// The line that moved a place, if it is moved.
    pub(super) fn moved_at(&self, place: &Place) -> Option<usize> {
        let root = place.root_var()?;
        let scope_index = self.lookup_scope_index(root)?;
        let state = self.scopes[scope_index].moves.get(root)?;
        match place.field_path() {
            Some(path) => state.moved_at(&path),
            None => match place {
                Place::Index(base) | Place::Field(base, _) => self.moved_at(base),
                Place::Var(_) | Place::Temporary => state.moved_at(&[]),
            },
        }
    }

    pub(super) fn check_place_available(&self, place: &Place, line: usize) -> Result<(), Error> {
        if let Some(moved_line) = self.moved_at(place) {
            let name = self.place_display(place);
            let suffix = format!("; it was moved at line {moved_line}");
            return Err(Error::new(
                line,
                format!("use of moved value `{name}`{suffix}"),
            ));
        }
        Ok(())
    }

    pub(super) fn mark_place_moved(&mut self, place: &Place, line: usize) {
        let Some(root) = place.root_var() else {
            return;
        };
        let Some(scope_index) = self.lookup_scope_index(root) else {
            return;
        };
        let Some(path) = place.field_path() else {
            return;
        };
        if let Some(state) = self.scopes[scope_index].moves.get_mut(root) {
            state.mark_moved(&path, line);
        }
    }

    pub(super) fn reinit_place(&mut self, place: &Place) {
        let Some(root) = place.root_var() else {
            return;
        };
        let Some(scope_index) = self.lookup_scope_index(root) else {
            return;
        };
        let Some(path) = place.field_path() else {
            return;
        };
        if let Some(state) = self.scopes[scope_index].moves.get_mut(root) {
            state.reinit(&path);
        }
    }

    /// Borrow a place. The value must not be moved.
    pub(super) fn borrow_place(&self, place: &Place, line: usize) -> Result<(), Error> {
        self.check_place_available(place, line)
    }

    /// Consume a place. Non-Copy values move.
    pub(super) fn move_out(
        &mut self,
        place: &Place,
        ty: &InferType,
        line: usize,
    ) -> Result<(), Error> {
        self.check_place_available(place, line)?;
        if self.is_copy_type(ty) || self.is_captured_place(place) {
            return Ok(());
        }
        match place {
            Place::Temporary => Ok(()),
            Place::Index(_) => Err(Error::new(line, "cannot move out of an index; use `clone`")),
            Place::Field(..) if place.has_index() => {
                Err(Error::new(line, "cannot move out of an index; use `clone`"))
            }
            Place::Field(..) if place.root_var().is_none() => Ok(()),
            _ => {
                self.mark_place_moved(place, line);
                Ok(())
            }
        }
    }

    /// The root must be mutable. This is used for assignment, which may
    /// reinitialize a moved field.
    pub(super) fn require_mutable_root(&self, place: &Place, line: usize) -> Result<(), Error> {
        let Some(root) = place.root_var() else {
            return Err(Error::new(line, "cannot assign to a temporary value"));
        };
        if !self.is_mutable(root) {
            return Err(Error::new(
                line,
                format!("cannot assign to `{root}`; declare it with `vistë`"),
            ));
        }
        Ok(())
    }

    /// The base of a field assignment must exist. The field itself may be
    /// moved, because the assignment reinitializes it.
    pub(super) fn check_field_assignment_base(
        &self,
        place: &Place,
        line: usize,
    ) -> Result<(), Error> {
        let Place::Field(base, _) = place else {
            return Ok(());
        };
        let Some(root) = base.root_var() else {
            return Ok(());
        };
        let Some(path) = base.field_path() else {
            return self.check_place_available(base, line);
        };
        let Some(scope_index) = self.lookup_scope_index(root) else {
            return Ok(());
        };
        let Some(state) = self.scopes[scope_index].moves.get(root) else {
            return Ok(());
        };
        let moved = if path.is_empty() {
            state.fully_moved_at
        } else {
            state.moved_at(&path)
        };
        if let Some(moved_line) = moved {
            return Err(Error::new(
                line,
                format!(
                    "cannot assign to a field of moved value `{}`; it was moved at line {moved_line}",
                    self.place_display(base)
                ),
            ));
        }
        Ok(())
    }

    /// The root must be mutable and the place must be available. This is used
    /// for `napan` and index assignment, which do not reinitialize the list.
    pub(super) fn require_mutable_place(&self, place: &Place, line: usize) -> Result<(), Error> {
        let Some(root) = place.root_var() else {
            return Err(Error::new(line, "cannot mutate a temporary value"));
        };
        if !self.is_mutable(root) {
            return Err(Error::new(
                line,
                format!("cannot mutate `{root}`; declare it with `vistë`"),
            ));
        }
        self.check_place_available(place, line)
    }

    pub(super) fn is_captured_place(&self, place: &Place) -> bool {
        place
            .root_var()
            .map(|name| self.is_captured(name))
            .unwrap_or(false)
    }

    pub(super) fn place_display(&self, place: &Place) -> String {
        match place {
            Place::Var(name) => name.clone(),
            Place::Field(base, name) => format!("{}.{}", self.place_display(base), name),
            Place::Index(base) => format!("{}[...]", self.place_display(base)),
            Place::Temporary => "<temporary>".to_string(),
        }
    }
}
