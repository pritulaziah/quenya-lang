//! The command-line entry point.
//!
//! With one argument, run a file. With no arguments, start the REPL.

use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process;
use std::rc::Rc;

use quenya_lang::environment::Environment;
use quenya_lang::error::Error;
use quenya_lang::interpreter;
use quenya_lang::lexer;
use quenya_lang::parser;

// One argument is a file. No arguments start the REPL.
fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => repl(),
        2 => {
            if let Err(error) = run_file(&args[1]) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: quenya [file.q]");
            process::exit(2);
        }
    }
}

fn run_file(path: &str) -> Result<(), Error> {
    let source = fs::read_to_string(path)
        .map_err(|error| Error::new(0, format!("cannot read `{path}`: {error}")))?;
    let env = Environment::new();
    let mut stdout = io::stdout();
    run_source(&source, &env, &mut stdout)
}

// The full pipeline: lex, parse, run.
fn run_source(
    source: &str,
    env: &Rc<RefCell<Environment>>,
    out: &mut dyn Write,
) -> Result<(), Error> {
    let tokens = lexer::tokenize(source)?;
    let program = parser::parse(tokens)?;
    interpreter::run(&program, env, out)
}

// The REPL keeps one environment, so variables persist between lines.
fn repl() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let env = Environment::new();
    let mut buffer = String::new();

    println!("Quenya 0.1. Enter `:q` to quit.");

    loop {
        if buffer.is_empty() {
            print!("quenya> ");
        } else {
            print!("...     ");
        }
        let _ = stdout.flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }

        let trimmed = line.trim();
        if buffer.is_empty() && (trimmed == ":q" || trimmed == ":quit") {
            break;
        }
        if buffer.is_empty() && trimmed.is_empty() {
            continue;
        }

        buffer.push_str(&line);
        if needs_more(&buffer) {
            continue;
        }

        let source = std::mem::take(&mut buffer);
        if let Err(error) = run_source(&source, &env, &mut stdout) {
            eprintln!("error: {error}");
        }
    }
}

// Count braces and parentheses outside strings and comments.
// If a block is open, read another line.
fn needs_more(source: &str) -> bool {
    let mut braces = 0i32;
    let mut parens = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for c in source.chars() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => in_string = true,
            '#' => in_comment = true,
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            _ => {}
        }
    }

    braces > 0 || parens > 0
}
