//! The lexer turns source text into tokens.
//!
//! It skips whitespace and `#` comments.
//! It recognizes keywords, integers, strings, identifiers, and operators.

use crate::error::Error;

/// A token kind with its value, if the token has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Literals and names.
    Int(i64),
    Str(String),
    Ident(String),

    // Keywords.
    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    True,
    And,
    Or,
    Not,
    Print,

    // Punctuation.
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,

    // Operators.
    Plus,
    Minus,
    Arrow,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    Eof,
}

/// A token and its source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

/// Turn source text into tokens.
///
/// The result always ends with `TokenKind::Eof`.
pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();

    while let Some(c) = lexer.peek() {
        match c {
            ' ' | '\r' | '\t' | '\n' => {
                lexer.advance();
            }
            '#' => {
                while let Some(c) = lexer.peek() {
                    if c == '\n' {
                        break;
                    }
                    lexer.advance();
                }
            }
            '(' => tokens.push(lexer.single(TokenKind::LParen)),
            ')' => tokens.push(lexer.single(TokenKind::RParen)),
            '{' => tokens.push(lexer.single(TokenKind::LBrace)),
            '}' => tokens.push(lexer.single(TokenKind::RBrace)),
            '[' => tokens.push(lexer.single(TokenKind::LBracket)),
            ']' => tokens.push(lexer.single(TokenKind::RBracket)),
            ',' => tokens.push(lexer.single(TokenKind::Comma)),
            ';' => tokens.push(lexer.single(TokenKind::Semicolon)),
            ':' => tokens.push(lexer.single(TokenKind::Colon)),
            '+' => tokens.push(lexer.single(TokenKind::Plus)),
            '-' => {
                let line = lexer.line;
                lexer.advance();
                let kind = if lexer.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                };
                tokens.push(Token { kind, line });
            }
            '*' => tokens.push(lexer.single(TokenKind::Star)),
            '/' => tokens.push(lexer.single(TokenKind::Slash)),
            '%' => tokens.push(lexer.single(TokenKind::Percent)),
            '=' => {
                let line = lexer.line;
                lexer.advance();
                let kind = if lexer.match_char('=') {
                    TokenKind::EqEq
                } else {
                    TokenKind::Assign
                };
                tokens.push(Token { kind, line });
            }
            '!' => {
                // A lone `!` is not an operator. The language uses `la`.
                let line = lexer.line;
                lexer.advance();
                if lexer.match_char('=') {
                    tokens.push(Token {
                        kind: TokenKind::BangEq,
                        line,
                    });
                } else {
                    return Err(Error::new(
                        line,
                        "unexpected character `!`; use `la` for negation",
                    ));
                }
            }
            '<' => {
                let line = lexer.line;
                lexer.advance();
                let kind = if lexer.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                };
                tokens.push(Token { kind, line });
            }
            '>' => {
                let line = lexer.line;
                lexer.advance();
                let kind = if lexer.match_char('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                };
                tokens.push(Token { kind, line });
            }
            '"' => tokens.push(lexer.string()?),
            c if c.is_ascii_digit() => tokens.push(lexer.number()?),
            c if is_ident_start(c) => tokens.push(lexer.identifier()),
            _ => {
                return Err(Error::new(
                    lexer.line,
                    format!("unexpected character `{c}`"),
                ));
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line: lexer.line,
    });
    Ok(tokens)
}

struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.current += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn single(&mut self, kind: TokenKind) -> Token {
        let line = self.line;
        self.advance();
        Token { kind, line }
    }

    // Read a string literal and process escapes: \n, \t, \", \\.
    fn string(&mut self) -> Result<Token, Error> {
        let line = self.line;
        self.advance();

        let mut value = String::new();
        loop {
            match self.advance() {
                None => return Err(Error::new(line, "unterminated string")),
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(other) => {
                        return Err(Error::new(self.line, format!("unknown escape `\\{other}`")));
                    }
                    None => return Err(Error::new(line, "unterminated string")),
                },
                Some(c) => value.push(c),
            }
        }

        Ok(Token {
            kind: TokenKind::Str(value),
            line,
        })
    }

    // Read a decimal integer. A leading `-` is a unary operator.
    fn number(&mut self) -> Result<Token, Error> {
        let line = self.line;
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let value = text
            .parse::<i64>()
            .map_err(|_| Error::new(line, format!("invalid integer `{text}`")))?;
        Ok(Token {
            kind: TokenKind::Int(value),
            line,
        })
    }

    // Read an identifier or a keyword. Quenya letters are allowed.
    fn identifier(&mut self) -> Token {
        let line = self.line;
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match fold_keyword(&text).as_str() {
            "ea" => TokenKind::Let,
            "maita" => TokenKind::Fn,
            "entul" => TokenKind::Return,
            "qui" => TokenKind::If,
            "mal" => TokenKind::Else,
            "ya" => TokenKind::While,
            "ilya" => TokenKind::For,
            "mi" => TokenKind::In,
            "rac" => TokenKind::Break,
            "lelya" => TokenKind::Continue,
            "naite" => TokenKind::True,
            "ar" => TokenKind::And,
            "hya" => TokenKind::Or,
            "la" => TokenKind::Not,
            "quete" => TokenKind::Print,
            _ => TokenKind::Ident(text),
        };
        Token { kind, line }
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

// Fold Quenya diacritics for keyword matching.
// `ëa` and `ea` are the same keyword. `naitë` and `naite` are also the same.
fn fold_keyword(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '\u{0300}'..='\u{036f}'))
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            other => other,
        })
        .collect()
}
