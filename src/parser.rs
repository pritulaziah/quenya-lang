//! The parser turns tokens into an AST.
//!
//! It uses recursive descent. Each grammar rule is one function.
//! Expression rules follow precedence from low to high:
//! or, and, equality, comparison, term, factor, unary, call.

use crate::ast::{
    BinaryOp, Expr, ExprKind, FieldDef, FieldInit, MethodDef, Param, Stmt, StmtKind, Type, UnaryOp,
};
use crate::error::Error;
use crate::lexer::{Token, TokenKind};

/// Turn a token list into a program.
pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, Error> {
    Parser::new(tokens).program()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    // A struct literal and a block both start with `{`. Conditions and
    // loop iterables disable struct literals at their top level, as Rust does.
    allow_struct_literal: bool,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            allow_struct_literal: true,
        }
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
        if self.match_token(&TokenKind::Struct) {
            return self.struct_statement(line);
        }
        if self.match_token(&TokenKind::Impl) {
            return self.impl_statement(line);
        }
        if self.match_token(&TokenKind::If) {
            return self.if_statement(line);
        }
        if self.match_token(&TokenKind::While) {
            return self.while_statement(line);
        }
        if self.match_token(&TokenKind::For) {
            return self.for_statement(line);
        }
        if self.match_token(&TokenKind::Break) {
            return self.break_statement(line);
        }
        if self.match_token(&TokenKind::Continue) {
            return self.continue_statement(line);
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
            return match expr.kind {
                ExprKind::Variable(name) => Ok(Stmt {
                    kind: StmtKind::Assign { name, value },
                    line,
                }),
                ExprKind::Index { target, index } => Ok(Stmt {
                    kind: StmtKind::IndexAssign {
                        target: *target,
                        index: *index,
                        value,
                    },
                    line,
                }),
                ExprKind::Field { target, name } => Ok(Stmt {
                    kind: StmtKind::FieldAssign {
                        target: *target,
                        name,
                        value,
                    },
                    line,
                }),
                _ => Err(Error::new(line, "invalid assignment target")),
            };
        }

        self.consume(&TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Stmt {
            kind: StmtKind::Expression(expr),
            line,
        })
    }

    fn let_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected variable name after `essë`")?;
        // Rust infers local types, but a type annotation is allowed.
        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.consume(&TokenKind::Assign, "expected `=` after variable name")?;
        let value = self.expression()?;
        self.consume(
            &TokenKind::Semicolon,
            "expected `;` after variable declaration",
        )?;
        Ok(Stmt {
            kind: StmtKind::Let { name, ty, value },
            line,
        })
    }

    fn function(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected function name after `maita`")?;
        self.consume(&TokenKind::LParen, "expected `(` after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                // Rust requires a type for every function parameter.
                let param_name = self.consume_ident("expected parameter name")?;
                self.consume(
                    &TokenKind::Colon,
                    "expected `:` and a type after parameter name",
                )?;
                let ty = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    ty,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RParen, "expected `)` after parameters")?;
        // Rust writes `-> Type` for non-unit returns. Without it, the
        // function returns `Er`, just like Rust's `()`.
        let ret = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()?
        } else {
            Type::Unit
        };
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::Function {
                name,
                params,
                ret,
                body,
            },
            line,
        })
    }

    // `canta Name { field: Type, ... }`
    fn struct_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected struct name after `canta`")?;
        self.consume(&TokenKind::LBrace, "expected `{` after struct name")?;

        let mut fields = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                let field_name = self.consume_ident("expected field name")?;
                self.consume(
                    &TokenKind::Colon,
                    "expected `:` and a type after field name",
                )?;
                let ty = self.parse_type()?;
                fields.push(FieldDef {
                    name: field_name,
                    ty,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RBrace, "expected `}` after struct fields")?;
        Ok(Stmt {
            kind: StmtKind::Struct { name, fields },
            line,
        })
    }

    // `carasta Name { maita method(immo, ...) { ... } }`
    fn impl_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let name = self.consume_ident("expected type name after `carasta`")?;
        self.consume(&TokenKind::LBrace, "expected `{` after type name")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            methods.push(self.method(&name)?);
        }

        self.consume(&TokenKind::RBrace, "expected `}` after methods")?;
        Ok(Stmt {
            kind: StmtKind::Impl { name, methods },
            line,
        })
    }

    fn method(&mut self, struct_name: &str) -> Result<MethodDef, Error> {
        self.consume(&TokenKind::Fn, "expected `maita` method in `carasta`")?;
        let name = self.consume_ident("expected method name")?;
        self.consume(&TokenKind::LParen, "expected `(` after method name")?;

        let first_line = self.peek().line;
        let first = self.consume_ident("expected `immo` as the first method parameter")?;
        if first.to_lowercase() != "immo" {
            return Err(Error::new(first_line, "method must start with `immo`"));
        }
        if self.check(&TokenKind::Colon) {
            return Err(Error::new(
                self.peek().line,
                "self parameter `immo` must not have a type",
            ));
        }

        let mut params = vec![Param {
            name: first,
            ty: Type::Struct(struct_name.to_string()),
        }];

        if self.match_token(&TokenKind::Comma) && !self.check(&TokenKind::RParen) {
            loop {
                let param_name = self.consume_ident("expected parameter name")?;
                self.consume(
                    &TokenKind::Colon,
                    "expected `:` and a type after parameter name",
                )?;
                let ty = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    ty,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RParen, "expected `)` after method parameters")?;
        let ret = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()?
        } else {
            Type::Unit
        };
        let body = self.block()?;
        Ok(MethodDef {
            name,
            params,
            ret,
            body,
        })
    }

    // Parse a type annotation.
    //
    // The source names are Quenya: `Notessë` (Int), `Quetta` (Str),
    // `Nanwa` (Bool), `Er` (Unit), `Técma<T>` (List), and function types
    // written as `maita(T, ...) -> R`. Type names are case-insensitive.
    fn parse_type(&mut self) -> Result<Type, Error> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Ident(name) => {
                self.advance();
                let lower = name.to_lowercase();
                match lower.as_str() {
                    "notessë" => Ok(Type::Int),
                    "quetta" => Ok(Type::Str),
                    "nanwa" => Ok(Type::Bool),
                    "er" => Ok(Type::Unit),
                    "técma" => {
                        self.consume(&TokenKind::Lt, "expected `<` after `Técma`")?;
                        let inner = self.parse_type()?;
                        self.consume(&TokenKind::Gt, "expected `>` after list type")?;
                        Ok(Type::List(Box::new(inner)))
                    }
                    // Any other identifier is a user-defined struct type.
                    // The type checker validates that the struct exists.
                    _ => Ok(Type::Struct(name)),
                }
            }
            TokenKind::Fn => {
                self.advance();
                self.consume(
                    &TokenKind::LParen,
                    "expected `(` after `maita` in a function type",
                )?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        params.push(self.parse_type()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(
                    &TokenKind::RParen,
                    "expected `)` after function type parameters",
                )?;
                let ret = if self.match_token(&TokenKind::Arrow) {
                    self.parse_type()?
                } else {
                    Type::Unit
                };
                Ok(Type::Function(params, Box::new(ret)))
            }
            _ => Err(self.error("expected type")),
        }
    }

    // `mal qui` is an else-if. `mal { ... }` is an else block.
    fn if_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let condition = self.expression_with_struct(false)?;
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
        let condition = self.expression_with_struct(false)?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::While { condition, body },
            line,
        })
    }

    // `ilyë name mí iterable { ... }` visits each list element.
    fn for_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        let var = self.consume_ident("expected loop variable after `ilyë`")?;
        self.consume(&TokenKind::In, "expected `mí` after loop variable")?;
        let iterable = self.expression_with_struct(false)?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::For {
                var,
                iterable,
                body,
            },
            line,
        })
    }

    fn break_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        self.consume(&TokenKind::Semicolon, "expected `;` after `rac`")?;
        Ok(Stmt {
            kind: StmtKind::Break,
            line,
        })
    }

    fn continue_statement(&mut self, line: usize) -> Result<Stmt, Error> {
        self.consume(&TokenKind::Semicolon, "expected `;` after `lelya`")?;
        Ok(Stmt {
            kind: StmtKind::Continue,
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
        self.consume(&TokenKind::LParen, "expected `(` after `quétë`")?;
        let value = self.expression()?;
        self.consume(&TokenKind::RParen, "expected `)` after print argument")?;
        self.consume(&TokenKind::Semicolon, "expected `;` after `quétë`")?;
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

    // Parse an expression with struct literals enabled or disabled at its
    // top level. Parentheses, call arguments, and list elements enable them
    // again, as Rust does.
    fn expression_with_struct(&mut self, allow: bool) -> Result<Expr, Error> {
        let saved = self.allow_struct_literal;
        self.allow_struct_literal = allow;
        let result = self.expression();
        self.allow_struct_literal = saved;
        result
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

    // `lá` and `-` are right-associative.
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
        self.postfix()
    }

    // A postfix expression is a primary followed by calls, indexes, fields,
    // or method calls.
    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(&TokenKind::LParen) {
                let line = self.previous().line;
                let args = self.arguments()?;
                expr = Expr {
                    kind: ExprKind::Call(Box::new(expr), args),
                    line,
                };
            } else if self.match_token(&TokenKind::LBracket) {
                let line = self.previous().line;
                let index = self.expression_with_struct(true)?;
                self.consume(&TokenKind::RBracket, "expected `]` after index")?;
                expr = Expr {
                    kind: ExprKind::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                    },
                    line,
                };
            } else if self.match_token(&TokenKind::Dot) {
                let line = self.previous().line;
                let name = self.consume_ident("expected field or method name after `.`")?;
                if self.match_token(&TokenKind::LParen) {
                    let args = self.arguments()?;
                    expr = Expr {
                        kind: ExprKind::MethodCall {
                            target: Box::new(expr),
                            name,
                            args,
                        },
                        line,
                    };
                } else {
                    expr = Expr {
                        kind: ExprKind::Field {
                            target: Box::new(expr),
                            name,
                        },
                        line,
                    };
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.expression_with_struct(true)?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
        }
        self.consume(&TokenKind::RParen, "expected `)` after arguments")?;
        Ok(args)
    }

    fn struct_literal(&mut self, name: String, line: usize) -> Result<Expr, Error> {
        self.consume(&TokenKind::LBrace, "expected `{` after struct name")?;

        let mut fields = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                let field_name = self.consume_ident("expected field name")?;
                self.consume(&TokenKind::Colon, "expected `:` after field name")?;
                let value = self.expression_with_struct(true)?;
                fields.push(FieldInit {
                    name: field_name,
                    value,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RBrace, "expected `}` after struct fields")?;
        Ok(Expr {
            kind: ExprKind::Struct { name, fields },
            line,
        })
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
                if self.allow_struct_literal && self.check(&TokenKind::LBrace) {
                    self.struct_literal(name, line)
                } else {
                    Ok(Expr {
                        kind: ExprKind::Variable(name),
                        line,
                    })
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression_with_struct(true)?;
                self.consume(&TokenKind::RParen, "expected `)` after expression")?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.expression_with_struct(true)?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        // Allow a trailing comma.
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                    }
                }
                self.consume(&TokenKind::RBracket, "expected `]` after list elements")?;
                Ok(Expr {
                    kind: ExprKind::List(elements),
                    line,
                })
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
