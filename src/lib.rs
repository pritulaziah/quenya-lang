//! A small tree-walking interpreter for a Quenya-like language.
//!
//! The pipeline has four steps:
//!
//! 1. `lexer` turns source text into tokens.
//! 2. `parser` turns tokens into an abstract syntax tree (AST).
//! 3. `interpreter` walks the AST and runs the program.
//! 4. `environment` and `value` hold runtime state.
//!
//! `main.rs` connects the steps for files and the REPL.

pub mod ast;
pub mod environment;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;
