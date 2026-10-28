//! End-to-end tests.
//!
//! Each test runs a small program and checks the output.

use std::cell::RefCell;
use std::rc::Rc;

use quenya_lang::environment::Environment;
use quenya_lang::interpreter;
use quenya_lang::lexer;
use quenya_lang::parser;
use quenya_lang::typecheck::TypeChecker;

// Run a program and capture its output. Types are checked before execution.
fn run(source: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(source).map_err(|error| error.to_string())?;
    let program = parser::parse(tokens).map_err(|error| error.to_string())?;
    let mut checker = TypeChecker::new();
    checker
        .check_program(&program)
        .map_err(|error| error.to_string())?;
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
fn recursive_list_is_rejected() {
    // Rust also rejects a value that contains itself without indirection.
    let error = run("ea xs = [1]; xs[0] = xs;").unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Técma<Notessë>`"),
        "{error}"
    );
}

#[test]
fn infinite_type_is_rejected() {
    let error = run("ea xs = []; napan(xs, xs);").unwrap_err();
    assert!(error.contains("infinite type"), "{error}");
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
    assert!(error.contains("expected a list"), "{error}");
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
        ok("maita inc(x: Notessë) -> Notessë { entul x + 1; } ea fs = [inc]; quete(fs[0](4));"),
        "5\n"
    );
}

#[test]
fn for_loop_closures() {
    let source = r#"
        ea fs = [];
        ilya x mi [1, 2, 3] {
            maita f() -> Notessë { entul x; }
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
        ok("maita add(a: Notessë, b: Notessë) -> Notessë { entul a + b; } quete(add(2, 3));"),
        "5\n"
    );
}

#[test]
fn recursion() {
    assert_eq!(
        ok("maita fib(n: Notessë) -> Notessë { qui n < 2 { entul n; } entul fib(n - 1) + fib(n - 2); } quete(fib(10));"),
        "55\n"
    );
}

#[test]
fn closures() {
    let source = r#"
        maita adder(x: Notessë) -> maita(Notessë) -> Notessë {
            maita add(y: Notessë) -> Notessë {
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
        maita boom() -> Nanwa { quete("boom"); entul naite; }
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
    let error = run("maita f(a: Notessë) -> Notessë { entul a; } f();").unwrap_err();
    assert!(error.contains("expects 1 arguments, got 0"), "{error}");
}

#[test]
fn type_error() {
    let error = run(r#"quete(1 + "a");"#).unwrap_err();
    assert!(error.contains("cannot add"), "{error}");
}

#[test]
fn type_annotations() {
    assert_eq!(ok("ea x: Notessë = 5; quete(x);"), "5\n");
    let error = run(r#"ea x: Quetta = 1;"#).unwrap_err();
    assert!(
        error.contains("expected `Quetta`, found `Notessë`"),
        "{error}"
    );
}

#[test]
fn quenya_type_names() {
    assert_eq!(ok("ea x: Notessë = 5; quete(x);"), "5\n");
    // Type names are case-insensitive and accept ASCII spellings.
    assert_eq!(ok("ea y: notesse = 6; quete(y);"), "6\n");
    assert_eq!(ok(r#"ea s: Quetta = "a"; quete(s);"#), "a\n");
    assert_eq!(ok("ea b: Nanwa = naitë; quete(b);"), "naite\n");
    assert_eq!(ok("ea xs: Técma<Notessë> = [1, 2]; quete(xs);"), "[1, 2]\n");
    assert_eq!(
        ok(r#"ea xs: Tecma<Quetta> = ["a"]; quete(xs);"#),
        "[\"a\"]\n"
    );
    assert_eq!(
        ok("maita log(x: Notessë) { quete(x); } ea u: Er = log(1); quete(u);"),
        "1\n()\n"
    );
}

#[test]
fn function_signatures() {
    assert_eq!(
        ok("maita add(a: Notessë, b: Notessë) -> Notessë { entul a + b; } quete(add(2, 3));"),
        "5\n"
    );
    let error = run(r#"maita f(x: Notessë) -> Notessë { entul x; } quete(f("a"));"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"maita f() -> Notessë { entul "a"; }"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
}

#[test]
fn missing_return() {
    let error = run("maita f() -> Notessë { }").unwrap_err();
    assert!(
        error.contains("not all control paths return a value"),
        "{error}"
    );
    assert_eq!(
        ok("maita f() -> Notessë { qui naite { entul 1; } mal { entul 2; } } quete(f());"),
        "1\n"
    );
}

#[test]
fn empty_list_inference() {
    assert_eq!(ok("ea xs = []; napan(xs, 1); quete(xs);"), "[1]\n");
    let error = run("ea xs = [];").unwrap_err();
    assert!(error.contains("cannot infer the type"), "{error}");
    assert_eq!(
        ok("ea xs: Técma<Notessë> = []; napan(xs, 1); quete(xs);"),
        "[1]\n"
    );
}

#[test]
fn list_type_errors() {
    let error = run(r#"ea xs = [1, "a"];"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"ea xs = [1]; quete(xs["a"]);"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"ea xs = [1]; xs[0] = "a";"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"napan([1], "a");"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
}

#[test]
fn higher_order_functions() {
    let source = r#"
        maita apply(f: maita(Notessë) -> Notessë, x: Notessë) -> Notessë {
            entul f(x);
        }
        maita inc(x: Notessë) -> Notessë {
            entul x + 1;
        }
        quete(apply(inc, 4));
    "#;
    assert_eq!(ok(source), "5\n");
}

#[test]
fn function_type_mismatch() {
    let error =
        run(r#"maita f(x: Notessë) -> Notessë { entul x; } ea g: maita(Quetta) -> Quetta = f;"#)
            .unwrap_err();
    assert!(
        error.contains("expected `Quetta`, found `Notessë`"),
        "{error}"
    );
}

#[test]
fn builtin_as_value() {
    assert_eq!(ok(r#"ea f = not; quete(f("hello"));"#), "5\n");
    assert_eq!(
        ok("ea f = napan; ea xs = [1]; f(xs, 2); quete(xs);"),
        "[1, 2]\n"
    );
}

#[test]
fn shadowing_builtin() {
    assert_eq!(
        ok("maita not(x: Notessë) -> Notessë { entul x; } quete(not(1));"),
        "1\n"
    );
}

#[test]
fn duplicate_parameter() {
    let error = run("maita f(x: Notessë, x: Notessë) -> Notessë { entul x; }").unwrap_err();
    assert!(error.contains("duplicate parameter `x`"), "{error}");
}

#[test]
fn unit_function() {
    assert_eq!(ok("maita log(x: Notessë) { quete(x); } log(1);"), "1\n");
    let error = run("maita log(x: Notessë) { entul x; }").unwrap_err();
    assert!(error.contains("expected `Er`, found `Notessë`"), "{error}");
}

#[test]
fn empty_list_not_is_rejected() {
    let error = run("quete(not([]));").unwrap_err();
    assert!(error.contains("cannot infer"), "{error}");
}

#[test]
fn structs_and_fields() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        ea p = Point { x: 1, y: 2 };
        quete(p.x);
        p.x = 10;
        quete(p.x);
        quete(p);
    "#;
    assert_eq!(ok(source), "1\n10\nPoint { x: 10, y: 2 }\n");
}

#[test]
fn nested_structs() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        canta Line { start: Point, end: Point }
        ea l = Line {
            start: Point { x: 1, y: 2 },
            end: Point { x: 3, y: 4 },
        };
        quete(l.start.y);
        l.start.x = 10;
        quete(l.start.x);
        quete(l);
    "#;
    assert_eq!(
        ok(source),
        "2\n10\nLine { start: Point { x: 10, y: 2 }, end: Point { x: 3, y: 4 } }\n"
    );
}

#[test]
fn empty_struct() {
    assert_eq!(
        ok("canta Empty {} ea e = Empty {}; quete(e);"),
        "Empty {}\n"
    );
}

#[test]
fn struct_methods() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        carasta Point {
            maita move(immo, dx: Notessë, dy: Notessë) -> Er {
                immo.x = immo.x + dx;
                immo.y = immo.y + dy;
            }
            maita sum(immo) -> Notessë {
                entul immo.x + immo.y;
            }
        }
        ea p = Point { x: 1, y: 2 };
        p.move(10, 20);
        quete(p.sum());
        quete(p);
    "#;
    assert_eq!(ok(source), "33\nPoint { x: 11, y: 22 }\n");
}

#[test]
fn method_calls_method() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        carasta Point {
            maita sum(immo) -> Notessë {
                entul immo.x + immo.y;
            }
            maita double_sum(immo) -> Notessë {
                entul immo.sum() * 2;
            }
        }
        ea p = Point { x: 1, y: 2 };
        quete(p.double_sum());
    "#;
    assert_eq!(ok(source), "6\n");
}

#[test]
fn methods_can_call_later_methods() {
    let source = r#"
        canta Point { x: Notessë }
        carasta Point {
            maita a(immo) -> Notessë {
                entul immo.b() + 1;
            }
            maita b(immo) -> Notessë {
                entul 1;
            }
        }
        ea p = Point { x: 0 };
        quete(p.a());
    "#;
    assert_eq!(ok(source), "2\n");
}

#[test]
fn method_on_field() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        canta Line { start: Point }
        carasta Point {
            maita sum(immo) -> Notessë {
                entul immo.x + immo.y;
            }
        }
        ea l = Line { start: Point { x: 1, y: 2 } };
        quete(l.start.sum());
    "#;
    assert_eq!(ok(source), "3\n");
}

#[test]
fn structs_in_functions_and_lists() {
    let source = r#"
        canta Point { x: Notessë }
        maita get_x(p: Point) -> Notessë {
            entul p.x;
        }
        ea ps = [Point { x: 1 }, Point { x: 2 }];
        ilya p mi ps {
            quete(get_x(p));
        }
        ilya p mi [Point { x: 3 }, Point { x: 4 }] {
            quete(p.x);
        }
    "#;
    assert_eq!(ok(source), "1\n2\n3\n4\n");
}

#[test]
fn struct_equality() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        ea a = Point { x: 1, y: 2 };
        ea b = Point { x: 1, y: 2 };
        ea c = Point { x: 2, y: 1 };
        quete(a == b);
        quete(a == c);
    "#;
    assert_eq!(ok(source), "naite\nla naite\n");
}

#[test]
fn struct_literal_in_condition() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        carasta Point {
            maita sum(immo) -> Notessë {
                entul immo.x + immo.y;
            }
        }
        qui (Point { x: 1, y: 1 }).sum() == 2 {
            quete("yes");
        }
    "#;
    assert_eq!(ok(source), "yes\n");
}

#[test]
fn struct_field_errors() {
    let error = run("canta Point { x: Notessë } ea p = Point { y: 1 };").unwrap_err();
    assert!(error.contains("has no field `y`"), "{error}");
    let error = run("canta Point { x: Notessë } ea p = Point {};").unwrap_err();
    assert!(error.contains("missing field `x`"), "{error}");
    let error = run(r#"canta Point { x: Notessë } ea p = Point { x: "a" };"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run("canta Point { x: Notessë, x: Notessë }").unwrap_err();
    assert!(error.contains("duplicate field `x`"), "{error}");
}

#[test]
fn struct_type_errors() {
    let error = run("ea p = Missing { x: 1 };").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Point { x: Missing }").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Point { x: Notessë } ea p: Missing = Point { x: 1 };").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Node { next: Node }").unwrap_err();
    assert!(error.contains("unknown type `Node`"), "{error}");
    let error = run("canta Notessë { x: Notessë }").unwrap_err();
    assert!(error.contains("is reserved"), "{error}");
}

#[test]
fn method_errors() {
    let error = run("canta Point { x: Notessë } ea p = Point { x: 1 }; p.nope();").unwrap_err();
    assert!(error.contains("has no method `nope`"), "{error}");
    let error = run(
        r#"canta Point { x: Notessë } carasta Point { maita f(immo, x: Notessë) -> Er { } } ea p = Point { x: 1 }; p.f("a");"#,
    )
    .unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run("canta Point { x: Notessë } carasta Point { maita f(immo) -> Notessë { } }")
        .unwrap_err();
    assert!(
        error.contains("not all control paths return a value"),
        "{error}"
    );
    let error = run("canta Point { x: Notessë } carasta Point { maita f(x: Notessë) -> Er { } }")
        .unwrap_err();
    assert!(error.contains("method must start with `immo`"), "{error}");
    let error = run("canta Point { x: Notessë } carasta Point { maita f(immo: Point) -> Er { } }")
        .unwrap_err();
    assert!(
        error.contains("self parameter `immo` must not have a type"),
        "{error}"
    );
}

#[test]
fn method_arity_error() {
    let error = run(
        "canta Point { x: Notessë } carasta Point { maita f(immo, x: Notessë) -> Er { } } ea p = Point { x: 1 }; p.f();",
    )
    .unwrap_err();
    assert!(error.contains("expects 1 arguments, got 0"), "{error}");
}

#[test]
fn field_assignment_type_error() {
    let error = run(r#"canta Point { x: Notessë } ea p = Point { x: 1 }; p.x = "a";"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
}

#[test]
fn impl_errors() {
    let error = run("carasta Missing { }").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run(
        "canta Point { x: Notessë } carasta Point { maita f(immo) -> Er { } maita f(immo) -> Er { } }",
    )
    .unwrap_err();
    assert!(error.contains("duplicate method `f`"), "{error}");
    let error = run(
        "canta Point { x: Notessë } carasta Point { maita f(immo) -> Er { } } carasta Point { maita f(immo) -> Er { } }",
    )
    .unwrap_err();
    assert!(error.contains("already defined"), "{error}");
}

#[test]
fn struct_top_level_only() {
    let error = run("{ canta Point { x: Notessë } }").unwrap_err();
    assert!(error.contains("only allowed at top level"), "{error}");
    let error = run("{ carasta Point { } }").unwrap_err();
    assert!(error.contains("only allowed at top level"), "{error}");
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
