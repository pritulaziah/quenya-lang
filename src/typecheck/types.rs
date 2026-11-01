//! Type representations, unification, and type validation.

use std::fmt;

use crate::ast::Type;
use crate::error::Error;

use super::TypeChecker;

/// A type used during inference. `Var` is a type variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InferType {
    Int,
    Str,
    Bool,
    Unit,
    List(Box<InferType>),
    Named(String),
    Function(Vec<InferType>, Box<InferType>),
    Builtin(BuiltinKind),
    Var(usize),
}

impl InferType {
    // Convert a type written in source code into an inference type.
    pub(super) fn from_ast(ty: &Type) -> Self {
        match ty {
            Type::Int => InferType::Int,
            Type::Str => InferType::Str,
            Type::Bool => InferType::Bool,
            Type::Unit => InferType::Unit,
            Type::List(inner) => InferType::List(Box::new(InferType::from_ast(inner))),
            Type::Named(name) => InferType::Named(name.clone()),
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
            InferType::Int => write!(f, "Notessë"),
            InferType::Str => write!(f, "Quetta"),
            InferType::Bool => write!(f, "Nanwa"),
            InferType::Unit => write!(f, "Er"),
            InferType::List(inner) => write!(f, "Técma<{inner}>"),
            InferType::Named(name) => write!(f, "{name}"),
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
pub(super) enum BuiltinKind {
    Len,
    Push,
    Clone,
}

impl BuiltinKind {
    pub(super) fn name(self) -> &'static str {
        match self {
            BuiltinKind::Len => "notië",
            BuiltinKind::Push => "napan",
            BuiltinKind::Clone => "clone",
        }
    }

    pub(super) fn arity(self) -> usize {
        match self {
            BuiltinKind::Len => 1,
            BuiltinKind::Push => 2,
            BuiltinKind::Clone => 1,
        }
    }
}

/// A struct definition known to the checker.
#[derive(Debug, Clone)]
pub(super) struct StructInfo {
    pub(super) fields: Vec<(String, InferType)>,
}

/// An enum definition known to the checker.
#[derive(Debug, Clone)]
pub(super) struct EnumInfo {
    pub(super) variants: Vec<EnumVariantInfo>,
}

/// One variant of an enum definition.
#[derive(Debug, Clone)]
pub(super) struct EnumVariantInfo {
    pub(super) name: String,
    pub(super) fields: Vec<(String, InferType)>,
}

/// A method signature known to the checker.
#[derive(Debug, Clone)]
pub(super) struct MethodInfo {
    /// Parameters after `immo`.
    pub(super) params: Vec<InferType>,
    pub(super) ret: InferType,
}

impl TypeChecker {
    pub(super) fn validate_type(&self, ty: &InferType, line: usize) -> Result<(), Error> {
        match ty {
            InferType::Named(name)
                if !self.structs.contains_key(name) && !self.enums.contains_key(name) =>
            {
                Err(Error::new(line, format!("unknown type `{name}`")))
            }
            InferType::List(inner) => self.validate_type(inner, line),
            InferType::Function(params, ret) => {
                for param in params {
                    self.validate_type(param, line)?;
                }
                self.validate_type(ret, line)
            }
            _ => Ok(()),
        }
    }

    pub(super) fn unify(
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
            (InferType::Named(a), InferType::Named(b)) if a == b => Ok(()),
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

    pub(super) fn resolve(&self, ty: &InferType) -> InferType {
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

    pub(super) fn fresh_var(&mut self) -> InferType {
        let id = self.substitutions.len();
        self.substitutions.push(None);
        InferType::Var(id)
    }

    pub(super) fn struct_field(&self, struct_name: &str, field: &str) -> Option<&InferType> {
        self.structs
            .get(struct_name)?
            .fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, ty)| ty)
    }

    pub(super) fn method_info(&self, struct_name: &str, method: &str) -> Option<&MethodInfo> {
        self.methods
            .get(&(struct_name.to_string(), method.to_string()))
    }
}
