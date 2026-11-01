//! Expression type inference.

use std::collections::HashSet;

use crate::ast::{BinaryOp, Expr, ExprKind, FieldInit, UnaryOp};
use crate::error::Error;

use super::ownership::{place_of, Mode, Place};
use super::types::{BuiltinKind, InferType};
use super::{Scope, TypeChecker};

impl TypeChecker {
    pub(super) fn infer(&mut self, expr: &Expr, mode: Mode) -> Result<InferType, Error> {
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

                let first = self.infer(&elements[0], Mode::Value)?;
                for element in &elements[1..] {
                    let element_type = self.infer(element, Mode::Value)?;
                    self.unify(&first, &element_type, element.line)?;
                }
                Ok(InferType::List(Box::new(first)))
            }
            ExprKind::Block { statements, tail } => {
                self.scopes.push(Scope::default());
                let mut result = Ok(());
                for stmt in statements {
                    if let Err(error) = self.check_stmt(stmt) {
                        result = Err(error);
                        break;
                    }
                }
                let result = result.and_then(|_| match tail {
                    Some(tail) => self.infer(tail, Mode::Value),
                    None => Ok(InferType::Unit),
                });
                self.scopes.pop();
                result
            }
            ExprKind::Match { value, arms } => self.check_match_expr(expr.line, value, arms),
            ExprKind::Variable(name) => {
                let ty = self
                    .lookup(name)
                    .ok_or_else(|| Error::new(expr.line, format!("undefined variable `{name}`")))?;
                let place = Place::Var(name.clone());
                match mode {
                    Mode::Value => self.move_out(&place, &ty, expr.line)?,
                    Mode::Place => self.borrow_place(&place, expr.line)?,
                    Mode::Type => {}
                }
                Ok(ty)
            }
            ExprKind::Unary(operator, operand) => {
                let operand_type = self.infer(operand, Mode::Value)?;
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
                let target_type = self.infer(target, Mode::Type)?;
                let element = self.fresh_var();
                self.unify(
                    &target_type,
                    &InferType::List(Box::new(element.clone())),
                    target.line,
                )?;
                let index_type = self.infer(index, Mode::Value)?;
                self.unify(&InferType::Int, &index_type, index.line)?;
                let element_type = self.resolve(&element);
                let place = place_of(expr).unwrap_or(Place::Temporary);
                match mode {
                    Mode::Value => self.move_out(&place, &element_type, expr.line)?,
                    Mode::Place => self.borrow_place(&place, expr.line)?,
                    Mode::Type => {}
                }
                Ok(element_type)
            }
            ExprKind::Struct { name, fields } => {
                let info = self.structs.get(name).cloned().ok_or_else(|| {
                    if self.enums.contains_key(name) {
                        Error::new(
                            expr.line,
                            format!("type `{name}` is an enum; use `{name}::Variant`"),
                        )
                    } else {
                        Error::new(expr.line, format!("unknown type `{name}`"))
                    }
                })?;
                self.check_field_inits(name, &info.fields, fields, expr.line)?;
                Ok(InferType::Named(name.clone()))
            }
            ExprKind::EnumVariant {
                enum_name,
                variant,
                fields,
            } => {
                let info = self.enums.get(enum_name).cloned().ok_or_else(|| {
                    if self.structs.contains_key(enum_name) {
                        Error::new(expr.line, format!("type `{enum_name}` is not an enum"))
                    } else {
                        Error::new(expr.line, format!("unknown type `{enum_name}`"))
                    }
                })?;
                let variant_info = info
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            expr.line,
                            format!("type `{enum_name}` has no variant `{variant}`"),
                        )
                    })?;

                if variant_info.fields.is_empty() && !fields.is_empty() {
                    return Err(Error::new(
                        expr.line,
                        format!("variant `{enum_name}::{variant}` has no fields"),
                    ));
                }

                self.check_field_inits(
                    &format!("{enum_name}::{variant}"),
                    &variant_info.fields,
                    fields,
                    expr.line,
                )?;
                Ok(InferType::Named(enum_name.clone()))
            }
            ExprKind::Field { target, name } => {
                let target_type = self.infer(target, Mode::Type)?;
                let field_type = match self.resolve(&target_type) {
                    InferType::Named(struct_name) => self
                        .struct_field(&struct_name, name)
                        .cloned()
                        .ok_or_else(|| {
                            Error::new(
                                expr.line,
                                format!("type `{struct_name}` has no field `{name}`"),
                            )
                        })?,
                    other => {
                        return Err(Error::new(
                            target.line,
                            format!("expected struct, found `{}`", self.type_name(&other)),
                        ));
                    }
                };
                let place = place_of(expr).unwrap_or(Place::Temporary);
                match mode {
                    Mode::Value => self.move_out(&place, &field_type, expr.line)?,
                    Mode::Place => self.borrow_place(&place, expr.line)?,
                    Mode::Type => {}
                }
                Ok(field_type)
            }
            ExprKind::MethodCall { target, name, args } => {
                // Methods borrow their receiver in this version.
                let target_type = self.infer(target, Mode::Place)?;
                let struct_name = match self.resolve(&target_type) {
                    InferType::Named(struct_name) => struct_name,
                    other => {
                        return Err(Error::new(
                            target.line,
                            format!("expected struct, found `{}`", self.type_name(&other)),
                        ));
                    }
                };
                let method = self
                    .method_info(&struct_name, name)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            expr.line,
                            format!("type `{struct_name}` has no method `{name}`"),
                        )
                    })?;

                if args.len() != method.params.len() {
                    return Err(Error::new(
                        expr.line,
                        format!(
                            "method `{struct_name}::{name}` expects {} arguments, got {}",
                            method.params.len(),
                            args.len()
                        ),
                    ));
                }
                for (param, arg) in method.params.iter().zip(args) {
                    let arg_type = self.infer(arg, Mode::Value)?;
                    self.unify(param, &arg_type, arg.line)?;
                }
                Ok(method.ret)
            }
        }
    }

    fn check_field_inits(
        &mut self,
        type_name: &str,
        expected_fields: &[(String, InferType)],
        fields: &[FieldInit],
        line: usize,
    ) -> Result<(), Error> {
        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.clone()) {
                return Err(Error::new(
                    field.value.line,
                    format!("duplicate field `{}`", field.name),
                ));
            }
            let expected = expected_fields
                .iter()
                .find(|(name, _)| name == &field.name)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| {
                    Error::new(
                        field.value.line,
                        format!("type `{type_name}` has no field `{}`", field.name),
                    )
                })?;
            let value_type = self.infer(&field.value, Mode::Value)?;
            self.unify(&expected, &value_type, field.value.line)?;
        }

        for (name, _) in expected_fields {
            if !seen.contains(name) {
                return Err(Error::new(
                    line,
                    format!("missing field `{name}` in `{type_name}`"),
                ));
            }
        }
        Ok(())
    }

    fn infer_binary(
        &mut self,
        left: &Expr,
        operator: BinaryOp,
        right: &Expr,
        line: usize,
    ) -> Result<InferType, Error> {
        // Equality borrows both sides, like `PartialEq` in Rust. Other
        // operators consume their operands.
        let mode = match operator {
            BinaryOp::Eq | BinaryOp::Ne => Mode::Place,
            _ => Mode::Value,
        };
        let left_type = self.infer(left, mode)?;
        let right_type = self.infer(right, mode)?;

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

    // `+` works for Notessë, Quetta, and Técma. This helper keeps the type
    // errors specific instead of reporting a generic mismatch.
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
        let callee_type = self.infer(callee, Mode::Value)?;
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
                    let arg_type = self.infer(arg, Mode::Value)?;
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
                let arg_type = self.infer(&args[0], Mode::Place)?;
                match self.resolve(&arg_type) {
                    InferType::Str | InferType::List(_) => Ok(InferType::Int),
                    InferType::Var(_) => Err(Error::new(
                        args[0].line,
                        "cannot infer the type of the `notië` argument; add a type annotation",
                    )),
                    other => Err(Error::new(
                        args[0].line,
                        format!(
                            "`notië` expects a list or string, got `{}`",
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
                let list_type = self.infer(&args[0], Mode::Place)?;
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
                let value_type = self.infer(&args[1], Mode::Value)?;
                self.unify(&element, &value_type, args[1].line)?;

                // `napan` mutates the list, so the list must be a mutable
                // place. The item is moved into the list.
                let place = place_of(&args[0]).unwrap_or(Place::Temporary);
                self.require_mutable_place(&place, args[0].line)?;
                Ok(InferType::Unit)
            }
            InferType::Builtin(BuiltinKind::Clone) => {
                if args.len() != BuiltinKind::Clone.arity() {
                    return Err(Error::new(
                        line,
                        format!(
                            "function `{}` expects {} arguments, got {}",
                            BuiltinKind::Clone.name(),
                            BuiltinKind::Clone.arity(),
                            args.len()
                        ),
                    ));
                }
                // `clone` borrows its argument and returns a new value.
                self.infer(&args[0], Mode::Place)
            }
            other => Err(Error::new(
                line,
                format!("`{}` is not callable", self.type_name(&other)),
            )),
        }
    }

    pub(super) fn require_bool(&mut self, ty: &InferType, line: usize) -> Result<(), Error> {
        match self.resolve(ty) {
            InferType::Bool => Ok(()),
            InferType::Var(_) => self.unify(&InferType::Bool, ty, line),
            other => Err(Error::new(
                line,
                format!("expected boolean, found `{}`", self.type_name(&other)),
            )),
        }
    }
}
