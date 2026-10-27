//! The parser turns tokens into an AST.
//!
//! It uses recursive descent. Each grammar rule is one function.
//! Expression rules follow precedence from low to high:
//! or, and, equality, comparison, term, factor, unary, call.

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::error::Error;
use crate::lexer::{Token, TokenKind};

/// Turn a token list into a program.
pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, Error> {
    Parser::new(tokens).program()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn program(mut self) -> Result<Vec<Stmt>, Error> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    // A statement starts with a keyword, a block, or an expression.
    // An expression followed by `=` is an assignment.
    fn statement(&mut self) -> Result<Stmt, Error> {
        let line = self.peek().line;

        if self.match_token(&TokenKind::Let) {
            return self.let_statement(line);
        }
        if self.match_token(&TokenKind::Fn) {
            return self.function(line);
        }
        if self.match_token(&TokenKind::If) {
            return self.if_statement(line);
        }
        if self.match_token(&TokenKind::While) {
            return self.while_statement(line);
        }
        if self.match_token(&TokenKind::Return) {
            return self.return_statement(line);
        }
        if self.match_token(&TokenKind::Print) {
            return self.print_statement(line);
        }
        if self.check(&TokenKind::LBrace) {
            let body = self.block()?;
            return Ok(Stmt {
                kind: StmtKind::Block(body),
                line,
            });
        }

        let expr = self.expression()?;
        if self.match_token(&TokenKind::Assign) {
            let value = self.expression()?;
            self.consume(&TokenKind::Semicolon, "expected `;` after assignment")?;
            if let ExprKind::Variable(name) = expr.kind {
                return Ok(Stmt {
                    kind: StmtKind::Assign { name, value },
                    line,
                });
            }
            return Err(Error::new(line, "invalid assignment target"));
        }

        self.consume(&TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Stmt {
            kind: StmtKind::Expression(expr),
            line,
        })
    }

    fn let_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected variable name after `ea`")?;
        self.consume(&TokenKind::Assign, "expected `=` after variable name")?;
        let value = self.expression()?;
        self.consume(
            &TokenKind::Semicolon,
            "expected `;` after variable declaration",
        )?;
        Ok(Stmt {
            kind: StmtKind::Let { name, value },
            line,
        })
    }

    fn function(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected function name after `maita`")?;
        self.consume(&TokenKind::LParen, "expected `(` after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                params.push(self.consume_ident("expected parameter name")?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RParen, "expected `)` after parameters")?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::Function { name, params, body },
            line,
        })
    }

    // `mal qui` is an else-if. `mal { ... }` is an else block.
    fn if_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let condition = self.expression()?;
        let then_branch = self.block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(self.statement()?))
            } else {
                let else_line = self.peek().line;
                let body = self.block()?;
                Some(Box::new(Stmt {
                    kind: StmtKind::Block(body),
                    line: else_line,
                }))
            }
        } else {
            None
        };

        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_branch,
                else_branch,
            },
            line,
        })
    }

    fn while_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let condition = self.expression()?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::While { condition, body },
            line,
        })
    }

    fn return_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.consume(&TokenKind::Semicolon, "expected `;` after return")?;
        Ok(Stmt {
            kind: StmtKind::Return(value),
            line,
        })
    }

    fn print_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        self.consume(&TokenKind::LParen, "expected `(` after `quete`")?;
        let value = self.expression()?;
        self.consume(&TokenKind::RParen, "expected `)` after print argument")?;
        self.consume(&TokenKind::Semicolon, "expected `;` after `quete`")?;
        Ok(Stmt {
            kind: StmtKind::Print(value),
            line,
        })
    }

    // A block creates a new scope at runtime.
    fn block(&mut self) -> Result<Vec<Stmt>, Error> {
        self.consume(&TokenKind::LBrace, "expected `{`")?;
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.statement()?);
        }
        self.consume(&TokenKind::RBrace, "expected `}`")?;
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, Error> {
        self.or()
    }

    // Expression rules follow precedence from low to high.
    fn or(&mut self) -> Result<Expr, Error> {
        let mut expr = self.and()?;
        while self.match_token(&TokenKind::Or) {
            let line = self.previous().line;
            let right = self.and()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), BinaryOp::Or, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut expr = self.equality()?;
        while self.match_token(&TokenKind::And) {
            let line = self.previous().line;
            let right = self.equality()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), BinaryOp::And, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, Error> {
        let mut expr = self.comparison()?;
        while self.match_token(&TokenKind::EqEq) || self.match_token(&TokenKind::BangEq) {
            let operator = match self.previous().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::Ne,
                _ => unreachable!(),
            };
            let line = self.previous().line;
            let right = self.comparison()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), operator, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, Error> {
        let mut expr = self.term()?;
        while self.match_token(&TokenKind::Lt)
            || self.match_token(&TokenKind::LtEq)
            || self.match_token(&TokenKind::Gt)
            || self.match_token(&TokenKind::GtEq)
        {
            let operator = match self.previous().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let line = self.previous().line;
            let right = self.term()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), operator, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, Error> {
        let mut expr = self.factor()?;
        while self.match_token(&TokenKind::Plus) || self.match_token(&TokenKind::Minus) {
            let operator = match self.previous().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let line = self.previous().line;
            let right = self.factor()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), operator, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, Error> {
        let mut expr = self.unary()?;
        while self.match_token(&TokenKind::Star)
            || self.match_token(&TokenKind::Slash)
            || self.match_token(&TokenKind::Percent)
        {
            let operator = match self.previous().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => unreachable!(),
            };
            let line = self.previous().line;
            let right = self.unary()?;
            expr = Expr {
                kind: ExprKind::Binary(Box::new(expr), operator, Box::new(right)),
                line,
            };
        }
        Ok(expr)
    }

    // `la` and `-` are right-associative.
    fn unary(&mut self) -> Result<Expr, Error> {
        if self.match_token(&TokenKind::Not) {
            let line = self.previous().line;
            let operand = self.unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary(UnaryOp::Not, Box::new(operand)),
                line,
            });
        }
        if self.match_token(&TokenKind::Minus) {
            let line = self.previous().line;
            let operand = self.unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary(UnaryOp::Neg, Box::new(operand)),
                line,
            });
        }
        self.call()
    }

    // A call is a primary expression followed by `(...)`.
    fn call(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primary()?;
        while self.match_token(&TokenKind::LParen) {
            let line = self.previous().line;
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    args.push(self.expression()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(&TokenKind::RParen, "expected `)` after arguments")?;
            expr = Expr {
                kind: ExprKind::Call(Box::new(expr), args),
                line,
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let token = self.peek().clone();
        let line = token.line;

        match token.kind {
            TokenKind::Int(value) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Int(value),
                    line,
                })
            }
            TokenKind::Str(value) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Str(value),
                    line,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    line,
                })
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Variable(name),
                    line,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(&TokenKind::RParen, "expected `)` after expression")?;
                Ok(expr)
            }
            _ => Err(self.error("expected expression")),
        }
    }

    fn consume_ident(&mut self, message: &str) -> Result<String, Error> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(message)),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, Error> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> Error {
        Error::new(self.peek().line, message)
    }
}
