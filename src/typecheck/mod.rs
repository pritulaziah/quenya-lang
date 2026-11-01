//! The static type checker.
//!
//! Rust checks types before a program runs. This pass does the same for
//! Quenya: it walks the AST after parsing and before interpretation.
//!
//! The checker infers local variable types and checks function signatures,
//! calls, operators, lists, conditions, returns, block and match expressions,
//! structs, enums, patterns, methods, and ownership. It is deliberately
//! small: there are no generics or traits yet. The builtins `notië`, `napan`,
//! and `clone` are special-cased until the language gets traits.
//!
//! Ownership is part of this pass because move checks need the inferred type
//! of every expression. The pass is split by concern:
//!
//! - `types` holds inference types and unification.
//! - `stmt` checks statements, declarations, and patterns.
//! - `expr` infers expression types.
//! - `ownership` holds places, mutability, and move checks.
//! - `support` holds shared control-flow helpers.

use std::collections::HashMap;

use crate::ast::Stmt;
use crate::error::Error;

use ownership::MoveState;
use types::{BuiltinKind, EnumInfo, InferType, MethodInfo, StructInfo};

mod expr;
mod ownership;
mod stmt;
mod support;
mod types;

/// One lexical scope. Types, mutability, and move state use the same names.
#[derive(Clone, Default)]
struct Scope {
    types: HashMap<String, InferType>,
    mutable: HashMap<String, bool>,
    moves: HashMap<String, MoveState>,
}

/// The type checker keeps scopes, type variables, and the current return type.
#[derive(Clone)]
pub struct TypeChecker {
    scopes: Vec<Scope>,
    substitutions: Vec<Option<InferType>>,
    current_return: Option<InferType>,
    loop_depth: usize,
    /// Type variables from empty lists, with the line of the `[`.
    pending: Vec<(usize, usize)>,
    structs: HashMap<String, StructInfo>,
    enums: HashMap<String, EnumInfo>,
    methods: HashMap<(String, String), MethodInfo>,
    /// The scope index where each active function starts. A name below this
    /// index is captured from an outer function or from the top level.
    function_bases: Vec<usize>,
}

impl TypeChecker {
    /// Create a checker with the builtin functions in the global scope.
    pub fn new() -> Self {
        let mut checker = Self {
            scopes: vec![Scope::default()],
            substitutions: Vec::new(),
            current_return: None,
            loop_depth: 0,
            pending: Vec::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            methods: HashMap::new(),
            function_bases: Vec::new(),
        };
        checker.define("notië", InferType::Builtin(BuiltinKind::Len), false);
        checker.define("napan", InferType::Builtin(BuiltinKind::Push), false);
        checker.define("clone", InferType::Builtin(BuiltinKind::Clone), false);
        checker
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

    pub(super) fn check_block(&mut self, body: &[Stmt]) -> Result<(), Error> {
        self.scopes.push(Scope::default());
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

    pub(in crate::typecheck) fn lookup(&self, name: &str) -> Option<InferType> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.types.get(name).cloned())
    }

    /// Find the innermost scope that defines a name.
    pub(super) fn lookup_scope_index(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rposition(|scope| scope.types.contains_key(name))
    }

    pub(super) fn is_mutable(&self, name: &str) -> bool {
        self.lookup_scope_index(name)
            .and_then(|index| self.scopes[index].mutable.get(name).copied())
            .unwrap_or(false)
    }

    /// A name is captured when it comes from below the current function scope.
    pub(super) fn is_captured(&self, name: &str) -> bool {
        let Some(base) = self.function_bases.last() else {
            return false;
        };
        let Some(index) = self.lookup_scope_index(name) else {
            return false;
        };
        index < *base
    }

    pub(in crate::typecheck) fn define(&mut self, name: &str, ty: InferType, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.types.insert(name.to_string(), ty);
            scope.mutable.insert(name.to_string(), mutable);
            scope.moves.insert(name.to_string(), MoveState::default());
        }
    }

    pub(in crate::typecheck) fn type_name(&self, ty: &InferType) -> String {
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
