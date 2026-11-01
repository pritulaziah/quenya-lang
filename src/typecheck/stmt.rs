//! Statement, declaration, and pattern checking.

use std::collections::HashSet;

use crate::ast::{
    EnumVariantDef, Expr, FieldDef, FieldPattern, MatchExprArm, MethodDef, Pattern, Stmt, StmtKind,
    Type,
};
use crate::error::Error;

use super::ownership::{merge_scopes, place_of, Mode, Place};
use super::support::{block_returns, expr_returns, is_reserved_type_name, stmt_returns};
use super::types::{EnumInfo, EnumVariantInfo, InferType, MethodInfo, StructInfo};
use super::{Scope, TypeChecker};

impl TypeChecker {
    pub(super) fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Error> {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                let value_type = self.infer(value, Mode::Value)?;
                let declared = if let Some(ty) = ty {
                    let declared = InferType::from_ast(ty);
                    self.validate_type(&declared, stmt.line)?;
                    self.unify(&declared, &value_type, stmt.line)?;
                    declared
                } else {
                    value_type
                };
                self.define(name, declared, *mutable);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let variable = self
                    .lookup(name)
                    .ok_or_else(|| Error::new(stmt.line, format!("undefined variable `{name}`")))?;
                let value_type = self.infer(value, Mode::Value)?;
                self.unify(&variable, &value_type, stmt.line)?;
                if !self.is_mutable(name) {
                    return Err(Error::new(
                        stmt.line,
                        format!(
                            "cannot assign to immutable variable `{name}`; declare it with `vistë`"
                        ),
                    ));
                }
                self.reinit_place(&Place::Var(name.clone()));
                Ok(())
            }
            StmtKind::IndexAssign {
                target,
                index,
                value,
            } => {
                let target_type = self.infer(target, Mode::Place)?;
                let element = self.fresh_var();
                self.unify(
                    &target_type,
                    &InferType::List(Box::new(element.clone())),
                    target.line,
                )?;
                let index_type = self.infer(index, Mode::Value)?;
                self.unify(&InferType::Int, &index_type, index.line)?;
                let value_type = self.infer(value, Mode::Value)?;
                self.unify(&element, &value_type, value.line)?;
                let place = place_of(target).unwrap_or(Place::Temporary);
                self.require_mutable_place(&place, target.line)?;
                Ok(())
            }
            StmtKind::FieldAssign {
                target,
                name,
                value,
            } => {
                let target_type = self.infer(target, Mode::Type)?;
                let field_type = match self.resolve(&target_type) {
                    InferType::Named(struct_name) => self
                        .struct_field(&struct_name, name)
                        .cloned()
                        .ok_or_else(|| {
                            Error::new(
                                target.line,
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
                let value_type = self.infer(value, Mode::Value)?;
                self.unify(&field_type, &value_type, value.line)?;
                let place = match place_of(target) {
                    Some(base) => Place::Field(Box::new(base), name.clone()),
                    None => Place::Temporary,
                };
                self.require_mutable_root(&place, target.line)?;
                self.check_field_assignment_base(&place, target.line)?;
                self.reinit_place(&place);
                Ok(())
            }
            StmtKind::Print(expr) => {
                self.infer(expr, Mode::Place)?;
                Ok(())
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.infer(condition, Mode::Value)?;
                self.require_bool(&condition_type, condition.line)?;

                let before = self.scopes.clone();
                self.scopes = before.clone();
                self.check_block(then_branch)?;
                let then_scopes = self.scopes.clone();

                self.scopes = before.clone();
                if let Some(else_branch) = else_branch {
                    self.check_stmt(else_branch)?;
                }
                let else_scopes = self.scopes.clone();

                let then_diverges = block_returns(then_branch);
                let else_diverges = else_branch
                    .as_ref()
                    .map(|stmt| stmt_returns(stmt))
                    .unwrap_or(false);

                self.scopes = match (then_diverges, else_diverges) {
                    (true, true) => before,
                    (true, false) => else_scopes,
                    (false, true) => then_scopes,
                    (false, false) => merge_scopes(&before, &then_scopes, &else_scopes),
                };
                Ok(())
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.infer(condition, Mode::Value)?;
                self.require_bool(&condition_type, condition.line)?;
                let before = self.scopes.clone();
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                result?;
                let after = self.scopes.clone();
                self.scopes = if block_returns(body) {
                    before
                } else {
                    merge_scopes(&before, &before, &after)
                };
                Ok(())
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                let iterable_type = self.infer(iterable, Mode::Value)?;
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
                self.scopes.push(Scope::default());
                self.define(var, element, false);
                let before = self.scopes.clone();
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                result?;
                let after = self.scopes.clone();
                self.scopes = if block_returns(body) {
                    before
                } else {
                    merge_scopes(&before, &before, &after)
                };
                self.scopes.pop();
                Ok(())
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
            StmtKind::Struct { name, fields } => self.check_struct(stmt.line, name, fields),
            StmtKind::Enum { name, variants } => self.check_enum(stmt.line, name, variants),
            StmtKind::Impl { name, methods } => self.check_impl(stmt.line, name, methods),
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
                for ty in &param_types {
                    self.validate_type(ty, stmt.line)?;
                }
                self.validate_type(&return_type, stmt.line)?;
                self.define(
                    name,
                    InferType::Function(param_types.clone(), Box::new(return_type.clone())),
                    false,
                );

                // A function body must not change the move state of the
                // scopes where the function is declared. Captures are checked
                // in a later pass. The function name stays in the checkpoint.
                let outer_checkpoint = self.scopes.clone();
                self.scopes.push(Scope::default());
                let base = self.scopes.len() - 1;
                self.function_bases.push(base);
                for (param, ty) in params.iter().zip(param_types) {
                    self.define(&param.name, ty, param.mutable);
                }
                let saved_return = self.current_return.replace(return_type.clone());
                let saved_loop = self.loop_depth;
                self.loop_depth = 0;
                let result = self.check_block(body);
                self.current_return = saved_return;
                self.loop_depth = saved_loop;
                self.function_bases.pop();
                self.scopes = outer_checkpoint;
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
                    Some(expr) => self.infer(expr, Mode::Value)?,
                    None => InferType::Unit,
                };
                self.unify(&expected, &value_type, stmt.line)
            }
            StmtKind::Expression(expr) => {
                self.infer(expr, Mode::Value)?;
                Ok(())
            }
            StmtKind::Block(body) => self.check_block(body),
        }
    }

    fn check_struct(&mut self, line: usize, name: &str, fields: &[FieldDef]) -> Result<(), Error> {
        if self.scopes.len() != 1 {
            return Err(Error::new(line, "`canta` is only allowed at top level"));
        }

        if is_reserved_type_name(name) {
            return Err(Error::new(
                line,
                format!("struct name `{name}` is reserved"),
            ));
        }
        if self.structs.contains_key(name) || self.enums.contains_key(name) {
            return Err(Error::new(
                line,
                format!("type `{name}` is already defined"),
            ));
        }

        let mut seen = HashSet::new();
        let mut field_types = Vec::new();
        for field in fields {
            if !seen.insert(&field.name) {
                return Err(Error::new(
                    line,
                    format!("duplicate field `{}`", field.name),
                ));
            }
            let ty = InferType::from_ast(&field.ty);
            self.validate_type(&ty, line)?;
            field_types.push((field.name.clone(), ty));
        }

        self.structs.insert(
            name.to_string(),
            StructInfo {
                fields: field_types,
            },
        );
        Ok(())
    }

    fn check_enum(
        &mut self,
        line: usize,
        name: &str,
        variants: &[EnumVariantDef],
    ) -> Result<(), Error> {
        if self.scopes.len() != 1 {
            return Err(Error::new(line, "`cilmë` is only allowed at top level"));
        }
        if is_reserved_type_name(name) {
            return Err(Error::new(line, format!("enum name `{name}` is reserved")));
        }
        if self.structs.contains_key(name) || self.enums.contains_key(name) {
            return Err(Error::new(
                line,
                format!("type `{name}` is already defined"),
            ));
        }
        if variants.is_empty() {
            return Err(Error::new(line, "enum must have at least one variant"));
        }

        let mut seen_variants = HashSet::new();
        let mut variant_infos = Vec::new();
        for variant in variants {
            if !seen_variants.insert(&variant.name) {
                return Err(Error::new(
                    line,
                    format!("duplicate variant `{}`", variant.name),
                ));
            }

            let mut seen_fields = HashSet::new();
            let mut field_types = Vec::new();
            for field in &variant.fields {
                if !seen_fields.insert(&field.name) {
                    return Err(Error::new(
                        line,
                        format!(
                            "duplicate field `{}` in variant `{}`",
                            field.name, variant.name
                        ),
                    ));
                }
                let ty = InferType::from_ast(&field.ty);
                self.validate_type(&ty, line)?;
                field_types.push((field.name.clone(), ty));
            }
            variant_infos.push(EnumVariantInfo {
                name: variant.name.clone(),
                fields: field_types,
            });
        }

        self.enums.insert(
            name.to_string(),
            EnumInfo {
                variants: variant_infos,
            },
        );
        Ok(())
    }

    pub(super) fn check_match_expr(
        &mut self,
        line: usize,
        value: &Expr,
        arms: &[MatchExprArm],
    ) -> Result<InferType, Error> {
        if arms.is_empty() {
            return Err(Error::new(line, "match must have at least one arm"));
        }

        let value_type = self.infer(value, Mode::Value)?;
        let value_type = self.resolve(&value_type);
        if matches!(&value_type, InferType::Var(_)) {
            return Err(Error::new(
                value.line,
                "cannot infer the type of the match value; add a type annotation",
            ));
        }

        let before = self.scopes.clone();
        let mut merged = before.clone();
        let mut result_type = None;
        let mut all_diverge = true;

        for arm in arms {
            // Each arm starts from the same state. A move in one arm must not
            // affect another arm.
            self.scopes = before.clone();
            self.scopes.push(Scope::default());
            let result = self
                .check_pattern(&arm.pattern, &value_type, arm.line)
                .and_then(|_| self.infer(&arm.body, Mode::Value));
            self.scopes.pop();
            let body_type = result?;
            let arm_state = self.scopes.clone();

            // An arm that definitely returns has no value type to unify and
            // does not continue after the match.
            let diverges = expr_returns(&arm.body);
            if diverges {
                continue;
            }
            all_diverge = false;
            merged = merge_scopes(&merged, &merged, &arm_state);
            if let Some(expected) = &result_type {
                self.unify(expected, &body_type, arm.body.line)?;
            } else {
                result_type = Some(body_type);
            }
        }

        self.scopes = if all_diverge { before } else { merged };

        let patterns: Vec<&Pattern> = arms.iter().map(|arm| &arm.pattern).collect();
        if !self.patterns_exhaustive(&patterns, &value_type) {
            return Err(Error::new(
                line,
                "match is not exhaustive; add a wildcard arm",
            ));
        }
        Ok(result_type.unwrap_or(InferType::Unit))
    }

    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        expected: &InferType,
        line: usize,
    ) -> Result<(), Error> {
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Binding(name) => {
                if let Some(scope) = self.scopes.last() {
                    if scope.types.contains_key(name) {
                        return Err(Error::new(
                            line,
                            format!("duplicate binding `{name}` in pattern"),
                        ));
                    }
                }
                self.define(name, expected.clone(), false);
                Ok(())
            }
            Pattern::Int(_) => self.require_pattern_type(expected, &InferType::Int, line),
            Pattern::Str(_) => self.require_pattern_type(expected, &InferType::Str, line),
            Pattern::Bool(_) => self.require_pattern_type(expected, &InferType::Bool, line),
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
            } => {
                let expected_name = self.pattern_named_type(expected, enum_name, line)?;
                let info = self.enums.get(&expected_name).cloned().ok_or_else(|| {
                    Error::new(line, format!("type `{expected_name}` is not an enum"))
                })?;
                let variant_info = info
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            line,
                            format!("type `{expected_name}` has no variant `{variant}`"),
                        )
                    })?;

                if variant_info.fields.is_empty() && !fields.is_empty() {
                    return Err(Error::new(
                        line,
                        format!("variant `{expected_name}::{variant}` has no fields"),
                    ));
                }

                self.check_field_patterns(
                    fields,
                    &variant_info.fields,
                    &format!("{expected_name}::{variant}"),
                    line,
                )
            }
            Pattern::Struct { name, fields } => {
                let expected_name = self.pattern_named_type(expected, name, line)?;
                let info = self.structs.get(&expected_name).cloned().ok_or_else(|| {
                    Error::new(line, format!("type `{expected_name}` is not a struct"))
                })?;
                self.check_field_patterns(fields, &info.fields, &expected_name, line)
            }
        }
    }

    fn pattern_named_type(
        &self,
        expected: &InferType,
        pattern_name: &str,
        line: usize,
    ) -> Result<String, Error> {
        match self.resolve(expected) {
            InferType::Named(name) if name == pattern_name => Ok(name),
            InferType::Named(name) => Err(Error::new(
                line,
                format!("pattern type `{pattern_name}` cannot match `{name}`"),
            )),
            other => Err(Error::new(
                line,
                format!(
                    "pattern type `{pattern_name}` cannot match `{}`",
                    self.type_name(&other)
                ),
            )),
        }
    }

    fn require_pattern_type(
        &self,
        expected: &InferType,
        pattern_type: &InferType,
        line: usize,
    ) -> Result<(), Error> {
        let expected = self.resolve(expected);
        if matches!(&expected, InferType::Var(_)) {
            return Err(Error::new(
                line,
                "cannot infer the type of the match value; add a type annotation",
            ));
        }
        if expected == *pattern_type {
            Ok(())
        } else {
            Err(Error::new(
                line,
                format!(
                    "pattern of type `{pattern_type}` cannot match `{}`",
                    self.type_name(&expected)
                ),
            ))
        }
    }

    fn check_field_patterns(
        &mut self,
        fields: &[FieldPattern],
        expected_fields: &[(String, InferType)],
        type_name: &str,
        line: usize,
    ) -> Result<(), Error> {
        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.clone()) {
                return Err(Error::new(
                    line,
                    format!("duplicate field `{}` in pattern", field.name),
                ));
            }
            let expected = expected_fields
                .iter()
                .find(|(name, _)| name == &field.name)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| {
                    Error::new(
                        line,
                        format!("type `{type_name}` has no field `{}`", field.name),
                    )
                })?;
            self.check_pattern(&field.pattern, &expected, line)?;
        }

        for (name, _) in expected_fields {
            if !seen.contains(name) {
                return Err(Error::new(
                    line,
                    format!("missing field `{name}` in pattern for `{type_name}`"),
                ));
            }
        }
        Ok(())
    }

    fn patterns_exhaustive(&self, patterns: &[&Pattern], ty: &InferType) -> bool {
        if patterns
            .iter()
            .any(|pattern| matches!(*pattern, Pattern::Wildcard | Pattern::Binding(_)))
        {
            return true;
        }

        match self.resolve(ty) {
            InferType::Bool => {
                let mut has_true = false;
                let mut has_false = false;
                for pattern in patterns.iter().copied() {
                    if let Pattern::Bool(value) = pattern {
                        if *value {
                            has_true = true;
                        } else {
                            has_false = true;
                        }
                    }
                }
                has_true && has_false
            }
            InferType::Named(name) => {
                if let Some(info) = self.enums.get(&name) {
                    info.variants.iter().all(|variant| {
                        let matching: Vec<&Pattern> = patterns
                            .iter()
                            .copied()
                            .filter(|pattern| {
                                matches!(
                                    pattern,
                                    Pattern::EnumVariant {
                                        enum_name,
                                        variant: pattern_variant,
                                        ..
                                    } if enum_name == &name && pattern_variant == &variant.name
                                )
                            })
                            .collect();
                        if matching.is_empty() {
                            return false;
                        }
                        if variant.fields.is_empty() {
                            return true;
                        }
                        variant.fields.iter().all(|(field_name, field_ty)| {
                            let subpatterns: Vec<&Pattern> = matching
                                .iter()
                                .copied()
                                .filter_map(|pattern| match pattern {
                                    Pattern::EnumVariant { fields, .. } => fields
                                        .iter()
                                        .find(|field| &field.name == field_name)
                                        .map(|field| &field.pattern),
                                    _ => None,
                                })
                                .collect();
                            self.patterns_exhaustive(&subpatterns, field_ty)
                        })
                    })
                } else if let Some(info) = self.structs.get(&name) {
                    let matching: Vec<&Pattern> = patterns
                        .iter()
                        .copied()
                        .filter(|pattern| {
                            matches!(
                                pattern,
                                Pattern::Struct {
                                    name: pattern_name,
                                    ..
                                } if pattern_name == &name
                            )
                        })
                        .collect();
                    if matching.is_empty() {
                        return false;
                    }
                    info.fields.iter().all(|(field_name, field_ty)| {
                        let subpatterns: Vec<&Pattern> = matching
                            .iter()
                            .copied()
                            .filter_map(|pattern| match pattern {
                                Pattern::Struct { fields, .. } => fields
                                    .iter()
                                    .find(|field| &field.name == field_name)
                                    .map(|field| &field.pattern),
                                _ => None,
                            })
                            .collect();
                        self.patterns_exhaustive(&subpatterns, field_ty)
                    })
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn check_impl(&mut self, line: usize, name: &str, methods: &[MethodDef]) -> Result<(), Error> {
        if self.scopes.len() != 1 {
            return Err(Error::new(line, "`carasta` is only allowed at top level"));
        }
        if self.enums.contains_key(name) {
            return Err(Error::new(
                line,
                format!("`carasta` is only for structs; `{name}` is an enum"),
            ));
        }
        if !self.structs.contains_key(name) {
            return Err(Error::new(line, format!("unknown type `{name}`")));
        }

        let mut signatures = Vec::new();
        let mut seen = HashSet::new();
        for method in methods {
            if !seen.insert(&method.name) {
                return Err(Error::new(
                    line,
                    format!("duplicate method `{}`", method.name),
                ));
            }
            if self
                .methods
                .contains_key(&(name.to_string(), method.name.clone()))
            {
                return Err(Error::new(
                    line,
                    format!("method `{}` is already defined for `{name}`", method.name),
                ));
            }

            let Some(self_param) = method.params.first() else {
                return Err(Error::new(
                    line,
                    format!("method `{}` must start with `immo`", method.name),
                ));
            };
            if self_param.ty != Type::Named(name.to_string()) {
                return Err(Error::new(
                    line,
                    format!("method `{}` must start with `immo`", method.name),
                ));
            }

            let mut param_types = Vec::new();
            let mut seen_params = HashSet::new();
            for param in &method.params {
                if !seen_params.insert(&param.name) {
                    return Err(Error::new(
                        line,
                        format!("duplicate parameter `{}`", param.name),
                    ));
                }
                let ty = InferType::from_ast(&param.ty);
                self.validate_type(&ty, line)?;
                param_types.push(ty);
            }
            let return_type = InferType::from_ast(&method.ret);
            self.validate_type(&return_type, line)?;

            let info = MethodInfo {
                params: param_types[1..].to_vec(),
                ret: return_type.clone(),
            };
            self.methods
                .insert((name.to_string(), method.name.clone()), info);
            signatures.push((method, param_types, return_type));
        }

        let outer_checkpoint = self.scopes.clone();
        for (method, param_types, return_type) in signatures {
            self.scopes = outer_checkpoint.clone();
            self.scopes.push(Scope::default());
            let base = self.scopes.len() - 1;
            self.function_bases.push(base);
            for (param, ty) in method.params.iter().zip(param_types) {
                // `immo` is mutable in this version. Reference receivers
                // (`&self` and `&mut self`) come with the borrow checker.
                let mutable = if param.name == "immo" {
                    true
                } else {
                    param.mutable
                };
                self.define(&param.name, ty, mutable);
            }
            let saved_return = self.current_return.replace(return_type.clone());
            let saved_loop = self.loop_depth;
            self.loop_depth = 0;
            let result = self.check_block(&method.body);
            self.current_return = saved_return;
            self.loop_depth = saved_loop;
            self.function_bases.pop();
            self.scopes = outer_checkpoint.clone();
            result?;

            if return_type != InferType::Unit && !block_returns(&method.body) {
                return Err(Error::new(line, "not all control paths return a value"));
            }
        }

        Ok(())
    }
}
