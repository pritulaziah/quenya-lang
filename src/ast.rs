//! The abstract syntax tree (AST).
//!
//! An expression produces a value. A statement performs an action.
//! The parser builds this tree. The interpreter walks it.

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
    /// A variable lookup, for example `x`.
    Variable(String),
    /// A unary operation.
    Unary(UnaryOp, Box<Expr>),
    /// A binary operation.
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    /// A function call, for example `add(2, 3)`.
    Call(Box<Expr>, Vec<Expr>),
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
    /// `ea name = value;`
    Let { name: String, value: Expr },
    /// `name = value;`
    Assign { name: String, value: Expr },
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
    /// `maita name(params) { ... }`
    Function {
        name: String,
        params: Vec<String>,
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
