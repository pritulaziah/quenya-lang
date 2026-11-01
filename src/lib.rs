//! A small tree-walking interpreter for a Quenya-like language.
//!
//! The pipeline has four steps:
//!
//! 1. `lexer` turns source text into tokens.
//! 2. `parser` turns tokens into an abstract syntax tree (AST).
//! 3. `typecheck` checks static types and ownership before the program runs.
//! 4. `interpreter` walks the AST and runs the program.
//! 5. `environment` and `value` hold runtime state.
//!
//! `main.rs` connects the steps for files and the REPL.

use ast::Stmt;
use error::Error;
use typecheck::TypeChecker;

pub mod ast;
pub mod environment;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod typecheck;
pub mod value;

/// Lex, parse, and check a source program.
///
/// Use this function for one-shot programs. The REPL keeps its own
/// `TypeChecker` so that declarations and inferred types persist.
pub fn compile(source: &str) -> Result<Vec<Stmt>, Error> {
    let tokens = lexer::tokenize(source)?;
    let program = parser::parse(tokens)?;
    let mut checker = TypeChecker::new();
    checker.check_program(&program)?;
    Ok(program)
}
