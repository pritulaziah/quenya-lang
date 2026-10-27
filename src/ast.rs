//! The abstract syntax tree (AST).
//!
//! An expression produces a value. A statement performs an action.
//! The parser builds this tree. The interpreter walks it.

use std::fmt;

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-value`
    Neg,
    /// `la value`
    Not,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic.
    Add,
    Sub,
    Mul,
    Div,
    Rem,

    // Comparison.
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical. These operators short-circuit.
    And,
    Or,
}

/// A static type written in source code.
///
/// The Rust variant names are English for code readability. The user-facing
/// names are Quenya: `Notessë`, `Quetta`, `Nanwa`, `Er`, and `Técma<T>`.
/// Rust checks types before a program runs. This language does the same in a
/// separate pass. The types are deliberately small: there are no generics,
/// traits, or lifetimes yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Integer, written `Notessë` in source.
    Int,
    /// String, written `Quetta` in source.
    Str,
    /// Boolean, written `Nanwa` in source.
    Bool,
    /// Unit, written `Er` in source.
    Unit,
    /// List, written `Técma<T>` in source.
    List(Box<Type>),
    /// A function type, for example `maita(Notessë, Quetta) -> Nanwa`.
    Function(Vec<Type>, Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Notessë"),
            Type::Str => write!(f, "Quetta"),
            Type::Bool => write!(f, "Nanwa"),
            Type::Unit => write!(f, "Er"),
            Type::List(inner) => write!(f, "Técma<{inner}>"),
            Type::Function(params, ret) => {
                write!(f, "maita(")?;
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {ret}")
            }
        }
    }
}

/// A function parameter with its type.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

/// An expression with its source line.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}

/// The shape of an expression.
#[derive(Debug, Clone)]
pub enum ExprKind {
    /// An integer literal, for example `42`.
    Int(i64),
    /// A string literal, for example `"Elen síla"`.
    Str(String),
    /// A boolean literal. `naite` is the only literal. `la naite` is an expression.
    Bool(bool),
    /// A list literal, for example `[1, 2, 3]`.
    List(Vec<Expr>),
    /// A variable lookup, for example `x`.
    Variable(String),
    /// A unary operation.
    Unary(UnaryOp, Box<Expr>),
    /// A binary operation.
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    /// A function call, for example `add(2, 3)`.
    Call(Box<Expr>, Vec<Expr>),
    /// An index lookup, for example `xs[0]`.
    Index { target: Box<Expr>, index: Box<Expr> },
}

/// A statement with its source line.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

/// The shape of a statement.
#[derive(Debug, Clone)]
pub enum StmtKind {
    /// `ea name: Type = value;` or `ea name = value;`
    ///
    /// The type annotation is optional. Local types are inferred.
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
    },
    /// `name = value;`
    Assign { name: String, value: Expr },
    /// `target[index] = value;`
    IndexAssign {
        target: Expr,
        index: Expr,
        value: Expr,
    },
    /// `quete(value);`
    Print(Expr),
    /// `qui condition { ... } mal { ... }`
    ///
    /// `else_branch` holds a block or another `if`.
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    /// `ya condition { ... }`
    While { condition: Expr, body: Vec<Stmt> },
    /// `ilya name mi iterable { ... }`
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    /// `rac;`
    Break,
    /// `lelya;`
    Continue,
    /// `maita name(params) -> Type { ... }`
    ///
    /// The return type is `Er` when `-> Type` is omitted.
    Function {
        name: String,
        params: Vec<Param>,
        ret: Type,
        body: Vec<Stmt>,
    },
    /// `entul value;`
    ///
    /// `None` returns unit.
    Return(Option<Expr>),
    /// An expression used as a statement.
    Expression(Expr),
    /// `{ ... }`
    ///
    /// A block creates a new scope at runtime.
    Block(Vec<Stmt>),
}
