//! End-to-end tests.
//!
//! Each test runs a small program and checks the output.

use std::cell::RefCell;
use std::rc::Rc;

use quenya_lang::environment::Environment;
use quenya_lang::interpreter;
use quenya_lang::lexer;
use quenya_lang::parser;

// Run a program and capture its output.
fn run(source: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(source).map_err(|error| error.to_string())?;
    let program = parser::parse(tokens).map_err(|error| error.to_string())?;
    let env: Rc<RefCell<Environment>> = Environment::new();
    let mut output = Vec::new();
    interpreter::run(&program, &env, &mut output).map_err(|error| error.to_string())?;
    Ok(String::from_utf8(output).expect("output is UTF-8"))
}

fn ok(source: &str) -> String {
    run(source).unwrap_or_else(|error| panic!("program failed: {error}\n{source}"))
}

#[test]
fn arithmetic() {
    assert_eq!(ok("quete(1 + 2 * 3);"), "7\n");
    assert_eq!(ok("quete((1 + 2) * 3);"), "9\n");
    assert_eq!(ok("quete(-5 + 2);"), "-3\n");
    assert_eq!(ok("quete(7 / 2);"), "3\n");
    assert_eq!(ok("quete(7 % 2);"), "1\n");
}

#[test]
fn variables_and_assignment() {
    assert_eq!(ok("ea x = 5; x = x + 2; quete(x);"), "7\n");
}

#[test]
fn strings() {
    assert_eq!(ok(r#"quete("a" + "b");"#), "ab\n");
    assert_eq!(ok(r#"quete("line\nbreak");"#), "line\nbreak\n");
}

#[test]
fn booleans() {
    assert_eq!(ok("quete(naite); quete(la naite);"), "naite\nla naite\n");
    assert_eq!(ok("quete(1 < 2); quete(1 == 2);"), "naite\nla naite\n");
}

#[test]
fn if_else() {
    assert_eq!(
        ok(r#"ea x = 3; qui x > 2 { quete("big"); } mal { quete("small"); }"#),
        "big\n"
    );
    assert_eq!(
        ok(
            r#"ea x = 2; qui x > 2 { quete("big"); } mal qui x == 2 { quete("two"); } mal { quete("small"); }"#
        ),
        "two\n"
    );
}

#[test]
fn while_loop() {
    assert_eq!(
        ok("ea i = 0; ya i < 3 { quete(i); i = i + 1; }"),
        "0\n1\n2\n"
    );
}

#[test]
fn lists() {
    assert_eq!(
        ok("ea xs = [1, 2, 3]; quete(xs[0]); quete(xs[2]);"),
        "1\n3\n"
    );
    assert_eq!(
        ok("ea xs = [1, 2, 3]; xs[1] = 20; quete(xs);"),
        "[1, 20, 3]\n"
    );
    assert_eq!(ok("quete([1, 2] + [3]);"), "[1, 2, 3]\n");
    assert_eq!(
        ok("quete([1, 2] == [1, 2]); quete([1] == [2]);"),
        "naite\nla naite\n"
    );
    assert_eq!(ok(r#"quete(["a", "b"]);"#), "[\"a\", \"b\"]\n");
}

#[test]
fn list_builtins() {
    assert_eq!(
        ok("ea xs = [1, 2]; quete(not(xs)); napan(xs, 3); quete(xs); quete(not(xs));"),
        "2\n[1, 2, 3]\n3\n"
    );
    assert_eq!(ok(r#"quete(not("hello"));"#), "5\n");
}

#[test]
fn nested_lists() {
    assert_eq!(
        ok("ea xs = [[1, 2], [3]]; quete(xs[0][1]); xs[0][0] = 9; quete(xs);"),
        "2\n[[9, 2], [3]]\n"
    );
}

#[test]
fn list_self_reference() {
    assert_eq!(ok("ea xs = [1]; xs[0] = xs; quete(xs);"), "[[...]]\n");
}

#[test]
fn list_equality_cycles() {
    assert_eq!(
        ok("ea a = [1]; a[0] = a; ea b = [1]; b[0] = b; quete(a == b);"),
        "naite\n"
    );
}

#[test]
fn for_loop() {
    assert_eq!(ok("ilya x mi [1, 2, 3] { quete(x); }"), "1\n2\n3\n");
    assert_eq!(
        ok("ea x = 9; ilya x mi [1, 2] { quete(x); } quete(x);"),
        "1\n2\n9\n"
    );
}

#[test]
fn loop_control() {
    assert_eq!(
        ok("ilya x mi [1, 2, 3, 4, 5] { qui x == 2 { lelya; } qui x == 4 { rac; } quete(x); }"),
        "1\n3\n"
    );
    assert_eq!(
        ok("ea i = 0; ya i < 5 { i = i + 1; qui i == 2 { lelya; } qui i == 4 { rac; } quete(i); }"),
        "1\n3\n"
    );
}

#[test]
fn loop_control_outside_loop() {
    let error = run("rac;").unwrap_err();
    assert!(error.contains("outside loop"), "{error}");
    let error = run("lelya;").unwrap_err();
    assert!(error.contains("outside loop"), "{error}");
    let error = run("maita f() { rac; } f();").unwrap_err();
    assert!(error.contains("outside loop"), "{error}");
}

#[test]
fn list_index_errors() {
    let error = run("ea xs = [1]; quete(xs[1]);").unwrap_err();
    assert!(error.contains("out of bounds"), "{error}");
    let error = run("ea xs = [1]; quete(xs[-1]);").unwrap_err();
    assert!(error.contains("out of range"), "{error}");
    let error = run("ea xs = [1]; xs[1] = 2;").unwrap_err();
    assert!(error.contains("out of bounds"), "{error}");
}

#[test]
fn for_requires_list() {
    let error = run("ilya x mi 5 { quete(x); }").unwrap_err();
    assert!(error.contains("expected list"), "{error}");
}

#[test]
fn builtin_type_errors() {
    let error = run("quete(not(1));").unwrap_err();
    assert!(error.contains("`not` expects a list or string"), "{error}");
    let error = run("napan(1, 2);").unwrap_err();
    assert!(error.contains("`napan` expects a list"), "{error}");
}

#[test]
fn list_of_functions() {
    assert_eq!(
        ok("maita inc(x) { entul x + 1; } ea fs = [inc]; quete(fs[0](4));"),
        "5\n"
    );
}

#[test]
fn for_loop_closures() {
    let source = r#"
        ea fs = [];
        ilya x mi [1, 2, 3] {
            maita f() { entul x; }
            napan(fs, f);
        }
        quete(fs[0]());
        quete(fs[1]());
        quete(fs[2]());
    "#;
    assert_eq!(ok(source), "1\n2\n3\n");
}

#[test]
fn functions() {
    assert_eq!(
        ok("maita add(a, b) { entul a + b; } quete(add(2, 3));"),
        "5\n"
    );
}

#[test]
fn recursion() {
    assert_eq!(
        ok("maita fib(n) { qui n < 2 { entul n; } entul fib(n - 1) + fib(n - 2); } quete(fib(10));"),
        "55\n"
    );
}

#[test]
fn closures() {
    let source = r#"
        maita adder(x) {
            maita add(y) {
                entul x + y;
            }
            entul add;
        }
        ea add5 = adder(5);
        quete(add5(3));
    "#;
    assert_eq!(ok(source), "8\n");
}

#[test]
fn print_inside_function() {
    assert_eq!(ok(r#"maita say() { quete("x"); } say();"#), "x\n");
}

#[test]
fn bare_return() {
    assert_eq!(ok("maita f() { entul; } quete(f());"), "()\n");
}

#[test]
fn scopes() {
    assert_eq!(ok("ea x = 1; { ea x = 2; quete(x); } quete(x);"), "2\n1\n");
}

#[test]
fn short_circuit() {
    let source = r#"
        maita boom() { quete("boom"); entul naite; }
        quete(la naite ar boom());
        quete(naite hya boom());
    "#;
    assert_eq!(ok(source), "la naite\nnaite\n");
}

#[test]
fn comments() {
    assert_eq!(ok("# comment\nquete(1); # trailing\n"), "1\n");
}

#[test]
fn quenya_spellings() {
    assert_eq!(ok("ëa x = 1; quete(x);"), "1\n");
    assert_eq!(ok("quete(naitë);"), "naite\n");
    assert_eq!(ok("ea i = 0; yá i < 2 { quete(i); i = i + 1; }"), "0\n1\n");
}

#[test]
fn division_by_zero() {
    let error = run("quete(1 / 0);").unwrap_err();
    assert!(error.contains("division by zero"), "{error}");
}

#[test]
fn undefined_variable() {
    let error = run("quete(y);").unwrap_err();
    assert!(error.contains("undefined variable `y`"), "{error}");
}

#[test]
fn condition_must_be_boolean() {
    let error = run("ea x = 1; qui x { quete(x); }").unwrap_err();
    assert!(error.contains("expected boolean"), "{error}");
}

#[test]
fn arity_error() {
    let error = run("maita f(a) { entul a; } f();").unwrap_err();
    assert!(error.contains("expects 1 arguments, got 0"), "{error}");
}

#[test]
fn type_error() {
    let error = run(r#"quete(1 + "a");"#).unwrap_err();
    assert!(error.contains("cannot add"), "{error}");
}

#[test]
fn top_level_return() {
    let error = run("entul 1;").unwrap_err();
    assert!(error.contains("outside function"), "{error}");
}

#[test]
fn parse_error() {
    let error = run("ea x = ;").unwrap_err();
    assert!(error.contains("expected expression"), "{error}");
}
