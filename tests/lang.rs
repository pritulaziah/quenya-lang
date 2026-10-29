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
    assert_eq!(ok("quétë(1 + 2 * 3);"), "7\n");
    assert_eq!(ok("quétë((1 + 2) * 3);"), "9\n");
    assert_eq!(ok("quétë(-5 + 2);"), "-3\n");
    assert_eq!(ok("quétë(7 / 2);"), "3\n");
    assert_eq!(ok("quétë(7 % 2);"), "1\n");
}

#[test]
fn variables_and_assignment() {
    assert_eq!(ok("essë x = 5; x = x + 2; quétë(x);"), "7\n");
}

#[test]
fn strings() {
    assert_eq!(ok(r#"quétë("a" + "b");"#), "ab\n");
    assert_eq!(ok(r#"quétë("line\nbreak");"#), "line\nbreak\n");
}

#[test]
fn booleans() {
    assert_eq!(ok("quétë(naitë); quétë(lá naitë);"), "naitë\nlá naitë\n");
    assert_eq!(ok("quétë(1 < 2); quétë(1 == 2);"), "naitë\nlá naitë\n");
}

#[test]
fn if_else() {
    assert_eq!(
        ok(r#"essë x = 3; qui x > 2 { quétë("big"); } mal { quétë("small"); }"#),
        "big\n"
    );
    assert_eq!(
        ok(
            r#"essë x = 2; qui x > 2 { quétë("big"); } mal qui x == 2 { quétë("two"); } mal { quétë("small"); }"#
        ),
        "two\n"
    );
}

#[test]
fn while_loop() {
    assert_eq!(
        ok("essë i = 0; yallumë i < 3 { quétë(i); i = i + 1; }"),
        "0\n1\n2\n"
    );
}

#[test]
fn lists() {
    assert_eq!(
        ok("essë xs = [1, 2, 3]; quétë(xs[0]); quétë(xs[2]);"),
        "1\n3\n"
    );
    assert_eq!(
        ok("essë xs = [1, 2, 3]; xs[1] = 20; quétë(xs);"),
        "[1, 20, 3]\n"
    );
    assert_eq!(ok("quétë([1, 2] + [3]);"), "[1, 2, 3]\n");
    assert_eq!(
        ok("quétë([1, 2] == [1, 2]); quétë([1] == [2]);"),
        "naitë\nlá naitë\n"
    );
    assert_eq!(ok(r#"quétë(["a", "b"]);"#), "[\"a\", \"b\"]\n");
}

#[test]
fn list_builtins() {
    assert_eq!(
        ok("essë xs = [1, 2]; quétë(notië(xs)); napan(xs, 3); quétë(xs); quétë(notië(xs));"),
        "2\n[1, 2, 3]\n3\n"
    );
    assert_eq!(ok(r#"quétë(notië("hello"));"#), "5\n");
}

#[test]
fn nested_lists() {
    assert_eq!(
        ok("essë xs = [[1, 2], [3]]; quétë(xs[0][1]); xs[0][0] = 9; quétë(xs);"),
        "2\n[[9, 2], [3]]\n"
    );
}

#[test]
fn recursive_list_is_rejected() {
    // Rust also rejects a value that contains itself without indirection.
    let error = run("essë xs = [1]; xs[0] = xs;").unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Técma<Notessë>`"),
        "{error}"
    );
}

#[test]
fn infinite_type_is_rejected() {
    let error = run("essë xs = []; napan(xs, xs);").unwrap_err();
    assert!(error.contains("infinite type"), "{error}");
}

#[test]
fn for_loop() {
    assert_eq!(ok("ilyë x mí [1, 2, 3] { quétë(x); }"), "1\n2\n3\n");
    assert_eq!(
        ok("essë x = 9; ilyë x mí [1, 2] { quétë(x); } quétë(x);"),
        "1\n2\n9\n"
    );
}

#[test]
fn loop_control() {
    assert_eq!(
        ok("ilyë x mí [1, 2, 3, 4, 5] { qui x == 2 { lelya; } qui x == 4 { rac; } quétë(x); }"),
        "1\n3\n"
    );
    assert_eq!(
        ok("essë i = 0; yallumë i < 5 { i = i + 1; qui i == 2 { lelya; } qui i == 4 { rac; } quétë(i); }"),
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
    let error = run("essë xs = [1]; quétë(xs[1]);").unwrap_err();
    assert!(error.contains("out of bounds"), "{error}");
    let error = run("essë xs = [1]; quétë(xs[-1]);").unwrap_err();
    assert!(error.contains("out of range"), "{error}");
    let error = run("essë xs = [1]; xs[1] = 2;").unwrap_err();
    assert!(error.contains("out of bounds"), "{error}");
}

#[test]
fn for_requires_list() {
    let error = run("ilyë x mí 5 { quétë(x); }").unwrap_err();
    assert!(error.contains("expected a list"), "{error}");
}

#[test]
fn builtin_type_errors() {
    let error = run("quétë(notië(1));").unwrap_err();
    assert!(
        error.contains("`notië` expects a list or string"),
        "{error}"
    );
    let error = run("napan(1, 2);").unwrap_err();
    assert!(error.contains("`napan` expects a list"), "{error}");
}

#[test]
fn list_of_functions() {
    assert_eq!(
        ok("maita inc(x: Notessë) -> Notessë { entul x + 1; } essë fs = [inc]; quétë(fs[0](4));"),
        "5\n"
    );
}

#[test]
fn for_loop_closures() {
    let source = r#"
        essë fs = [];
        ilyë x mí [1, 2, 3] {
            maita f() -> Notessë { entul x; }
            napan(fs, f);
        }
        quétë(fs[0]());
        quétë(fs[1]());
        quétë(fs[2]());
    "#;
    assert_eq!(ok(source), "1\n2\n3\n");
}

#[test]
fn functions() {
    assert_eq!(
        ok("maita add(a: Notessë, b: Notessë) -> Notessë { entul a + b; } quétë(add(2, 3));"),
        "5\n"
    );
}

#[test]
fn recursion() {
    assert_eq!(
        ok("maita fib(n: Notessë) -> Notessë { qui n < 2 { entul n; } entul fib(n - 1) + fib(n - 2); } quétë(fib(10));"),
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
        essë add5 = adder(5);
        quétë(add5(3));
    "#;
    assert_eq!(ok(source), "8\n");
}

#[test]
fn print_inside_function() {
    assert_eq!(ok(r#"maita say() { quétë("x"); } say();"#), "x\n");
}

#[test]
fn bare_return() {
    assert_eq!(ok("maita f() { entul; } quétë(f());"), "()\n");
}

#[test]
fn scopes() {
    assert_eq!(
        ok("essë x = 1; { essë x = 2; quétë(x); } quétë(x);"),
        "2\n1\n"
    );
}

#[test]
fn short_circuit() {
    let source = r#"
        maita boom() -> Nanwa { quétë("boom"); entul naitë; }
        quétë(lá naitë ar boom());
        quétë(naitë hya boom());
    "#;
    assert_eq!(ok(source), "lá naitë\nnaitë\n");
}

#[test]
fn comments() {
    assert_eq!(ok("# comment\nquétë(1); # trailing\n"), "1\n");
}

#[test]
fn quenya_spellings() {
    assert_eq!(ok("essë x = 1; quétë(x);"), "1\n");
    assert_eq!(ok("quétë(naitë);"), "naitë\n");
    assert_eq!(
        ok("essë i = 0; yallumë i < 2 { quétë(i); i = i + 1; }"),
        "0\n1\n"
    );
    assert_eq!(ok("essë xs = [1, 2]; ilyë x mí xs { quétë(x); }"), "1\n2\n");
    assert_eq!(ok("essë xs = [1, 2]; quétë(notië(xs));"), "2\n");
}

#[test]
fn division_by_zero() {
    let error = run("quétë(1 / 0);").unwrap_err();
    assert!(error.contains("division by zero"), "{error}");
}

#[test]
fn undefined_variable() {
    let error = run("quétë(y);").unwrap_err();
    assert!(error.contains("undefined variable `y`"), "{error}");
}

#[test]
fn condition_must_be_boolean() {
    let error = run("essë x = 1; qui x { quétë(x); }").unwrap_err();
    assert!(error.contains("expected boolean"), "{error}");
}

#[test]
fn arity_error() {
    let error = run("maita f(a: Notessë) -> Notessë { entul a; } f();").unwrap_err();
    assert!(error.contains("expects 1 arguments, got 0"), "{error}");
}

#[test]
fn type_error() {
    let error = run(r#"quétë(1 + "a");"#).unwrap_err();
    assert!(error.contains("cannot add"), "{error}");
}

#[test]
fn type_annotations() {
    assert_eq!(ok("essë x: Notessë = 5; quétë(x);"), "5\n");
    let error = run(r#"essë x: Quetta = 1;"#).unwrap_err();
    assert!(
        error.contains("expected `Quetta`, found `Notessë`"),
        "{error}"
    );
}

#[test]
fn quenya_type_names() {
    assert_eq!(ok("essë x: Notessë = 5; quétë(x);"), "5\n");
    // Type names are case-insensitive. The diacritics are required.
    assert_eq!(ok("essë y: notessë = 6; quétë(y);"), "6\n");
    assert_eq!(ok(r#"essë s: Quetta = "a"; quétë(s);"#), "a\n");
    assert_eq!(ok("essë b: Nanwa = naitë; quétë(b);"), "naitë\n");
    assert_eq!(
        ok("essë xs: técma<Notessë> = [1, 2]; quétë(xs);"),
        "[1, 2]\n"
    );
    assert_eq!(
        ok("maita log(x: Notessë) { quétë(x); } essë u: Er = log(1); quétë(u);"),
        "1\n()\n"
    );
}

#[test]
fn function_signatures() {
    assert_eq!(
        ok("maita add(a: Notessë, b: Notessë) -> Notessë { entul a + b; } quétë(add(2, 3));"),
        "5\n"
    );
    let error = run(r#"maita f(x: Notessë) -> Notessë { entul x; } quétë(f("a"));"#).unwrap_err();
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
        ok("maita f() -> Notessë { qui naitë { entul 1; } mal { entul 2; } } quétë(f());"),
        "1\n"
    );
}

#[test]
fn empty_list_inference() {
    assert_eq!(ok("essë xs = []; napan(xs, 1); quétë(xs);"), "[1]\n");
    let error = run("essë xs = [];").unwrap_err();
    assert!(error.contains("cannot infer the type"), "{error}");
    assert_eq!(
        ok("essë xs: Técma<Notessë> = []; napan(xs, 1); quétë(xs);"),
        "[1]\n"
    );
}

#[test]
fn list_type_errors() {
    let error = run(r#"essë xs = [1, "a"];"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"essë xs = [1]; quétë(xs["a"]);"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run(r#"essë xs = [1]; xs[0] = "a";"#).unwrap_err();
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
        quétë(apply(inc, 4));
    "#;
    assert_eq!(ok(source), "5\n");
}

#[test]
fn function_type_mismatch() {
    let error =
        run(r#"maita f(x: Notessë) -> Notessë { entul x; } essë g: maita(Quetta) -> Quetta = f;"#)
            .unwrap_err();
    assert!(
        error.contains("expected `Quetta`, found `Notessë`"),
        "{error}"
    );
}

#[test]
fn builtin_as_value() {
    assert_eq!(ok(r#"essë f = notië; quétë(f("hello"));"#), "5\n");
    assert_eq!(
        ok("essë f = napan; essë xs = [1]; f(xs, 2); quétë(xs);"),
        "[1, 2]\n"
    );
}

#[test]
fn shadowing_builtin() {
    assert_eq!(
        ok("maita notië(x: Notessë) -> Notessë { entul x; } quétë(notië(1));"),
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
    assert_eq!(ok("maita log(x: Notessë) { quétë(x); } log(1);"), "1\n");
    let error = run("maita log(x: Notessë) { entul x; }").unwrap_err();
    assert!(error.contains("expected `Er`, found `Notessë`"), "{error}");
}

#[test]
fn empty_list_not_is_rejected() {
    let error = run("quétë(notië([]));").unwrap_err();
    assert!(error.contains("cannot infer"), "{error}");
}

#[test]
fn structs_and_fields() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        essë p = Point { x: 1, y: 2 };
        quétë(p.x);
        p.x = 10;
        quétë(p.x);
        quétë(p);
    "#;
    assert_eq!(ok(source), "1\n10\nPoint { x: 10, y: 2 }\n");
}

#[test]
fn nested_structs() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        canta Line { start: Point, end: Point }
        essë l = Line {
            start: Point { x: 1, y: 2 },
            end: Point { x: 3, y: 4 },
        };
        quétë(l.start.y);
        l.start.x = 10;
        quétë(l.start.x);
        quétë(l);
    "#;
    assert_eq!(
        ok(source),
        "2\n10\nLine { start: Point { x: 10, y: 2 }, end: Point { x: 3, y: 4 } }\n"
    );
}

#[test]
fn empty_struct() {
    assert_eq!(
        ok("canta Empty {} essë e = Empty {}; quétë(e);"),
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
        essë p = Point { x: 1, y: 2 };
        p.move(10, 20);
        quétë(p.sum());
        quétë(p);
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
        essë p = Point { x: 1, y: 2 };
        quétë(p.double_sum());
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
        essë p = Point { x: 0 };
        quétë(p.a());
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
        essë l = Line { start: Point { x: 1, y: 2 } };
        quétë(l.start.sum());
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
        essë ps = [Point { x: 1 }, Point { x: 2 }];
        ilyë p mí ps {
            quétë(get_x(p));
        }
        ilyë p mí [Point { x: 3 }, Point { x: 4 }] {
            quétë(p.x);
        }
    "#;
    assert_eq!(ok(source), "1\n2\n3\n4\n");
}

#[test]
fn struct_equality() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        essë a = Point { x: 1, y: 2 };
        essë b = Point { x: 1, y: 2 };
        essë c = Point { x: 2, y: 1 };
        quétë(a == b);
        quétë(a == c);
    "#;
    assert_eq!(ok(source), "naitë\nlá naitë\n");
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
            quétë("yes");
        }
    "#;
    assert_eq!(ok(source), "yes\n");
}

#[test]
fn struct_field_errors() {
    let error = run("canta Point { x: Notessë } essë p = Point { y: 1 };").unwrap_err();
    assert!(error.contains("has no field `y`"), "{error}");
    let error = run("canta Point { x: Notessë } essë p = Point {};").unwrap_err();
    assert!(error.contains("missing field `x`"), "{error}");
    let error = run(r#"canta Point { x: Notessë } essë p = Point { x: "a" };"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run("canta Point { x: Notessë, x: Notessë }").unwrap_err();
    assert!(error.contains("duplicate field `x`"), "{error}");
}

#[test]
fn struct_type_errors() {
    let error = run("essë p = Missing { x: 1 };").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Point { x: Missing }").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Point { x: Notessë } essë p: Missing = Point { x: 1 };").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("canta Node { next: Node }").unwrap_err();
    assert!(error.contains("unknown type `Node`"), "{error}");
    let error = run("canta Notessë { x: Notessë }").unwrap_err();
    assert!(error.contains("is reserved"), "{error}");
}

#[test]
fn method_errors() {
    let error = run("canta Point { x: Notessë } essë p = Point { x: 1 }; p.nope();").unwrap_err();
    assert!(error.contains("has no method `nope`"), "{error}");
    let error = run(
        r#"canta Point { x: Notessë } carasta Point { maita f(immo, x: Notessë) -> Er { } } essë p = Point { x: 1 }; p.f("a");"#,
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
        "canta Point { x: Notessë } carasta Point { maita f(immo, x: Notessë) -> Er { } } essë p = Point { x: 1 }; p.f();",
    )
    .unwrap_err();
    assert!(error.contains("expects 1 arguments, got 0"), "{error}");
}

#[test]
fn field_assignment_type_error() {
    let error =
        run(r#"canta Point { x: Notessë } essë p = Point { x: 1 }; p.x = "a";"#).unwrap_err();
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
fn enum_unit_variants_and_match() {
    let source = r#"
        cilmë Color { Red, Green, Blue }
        essë c: Color = Color::Green;
        nam c {
            Color::Red => { quétë("red"); }
            Color::Green => { quétë("green"); }
            Color::Blue => { quétë("blue"); }
        }
    "#;
    assert_eq!(ok(source), "green\n");
}

#[test]
fn enum_struct_variants_and_bindings() {
    let source = r#"
        cilmë Shape {
            Circle { radius: Notessë },
            Rectangle { width: Notessë, height: Notessë },
        }
        essë s = Shape::Circle { radius: 3 };
        nam s {
            Shape::Circle { radius: r } => { quétë(r); }
            Shape::Rectangle { width: w, height: h } => { quétë(w * h); }
        }
    "#;
    assert_eq!(ok(source), "3\n");
}

#[test]
fn enum_nested_patterns() {
    let source = r#"
        cilmë Inner { A, B }
        cilmë Outer { One { inner: Inner }, Two }
        essë o = Outer::One { inner: Inner::B };
        nam o {
            Outer::One { inner: Inner::A } => { quétë("a"); }
            Outer::One { inner: Inner::B } => { quétë("b"); }
            Outer::Two => { quétë("two"); }
        }
    "#;
    assert_eq!(ok(source), "b\n");
}

#[test]
fn enum_nested_bool_exhaustiveness() {
    let source = r#"
        cilmë Flag { On { value: Nanwa }, Off }
        essë f = Flag::On { value: naitë };
        nam f {
            Flag::On { value: naitë } => { quétë("on"); }
            Flag::On { value: lá naitë } => { quétë("off"); }
            Flag::Off => { quétë("off"); }
        }
    "#;
    assert_eq!(ok(source), "on\n");
}

#[test]
fn match_literals_and_wildcard() {
    let source = r#"
        essë x = 2;
        nam x {
            1 => { quétë("one"); }
            2 => { quétë("two"); }
            _ => { quétë("other"); }
        }
        essë s = "b";
        nam s {
            "a" => { quétë("a"); }
            _ => { quétë("other"); }
        }
        essë n = -1;
        nam n {
            -1 => { quétë("neg"); }
            _ => { quétë("other"); }
        }
    "#;
    assert_eq!(ok(source), "two\nother\nneg\n");
}

#[test]
fn match_bool_exhaustive() {
    let source = r#"
        essë b = naitë;
        nam b {
            naitë => { quétë("yes"); }
            lá naitë => { quétë("no"); }
        }
    "#;
    assert_eq!(ok(source), "yes\n");
}

#[test]
fn match_struct_pattern() {
    let source = r#"
        canta Point { x: Notessë, y: Notessë }
        essë p = Point { x: 1, y: 2 };
        nam p {
            Point { x: 0, y: y } => { quétë(y); }
            Point { x: x, y: _ } => { quétë(x); }
        }
    "#;
    assert_eq!(ok(source), "1\n");
}

#[test]
fn match_in_function_return() {
    let source = r#"
        cilmë Option { None, Some { value: Notessë } }
        maita unwrap_or(o: Option, fallback: Notessë) -> Notessë {
            nam o {
                Option::Some { value: v } => { entul v; }
                Option::None => { entul fallback; }
            }
        }
        quétë(unwrap_or(Option::Some { value: 5 }, 0));
        quétë(unwrap_or(Option::None, 9));
    "#;
    assert_eq!(ok(source), "5\n9\n");
}

#[test]
fn match_loop_control() {
    let source = r#"
        essë i = 0;
        yallumë i < 5 {
            nam i {
                2 => { i = i + 1; lelya; }
                4 => { rac; }
                _ => { quétë(i); }
            }
            i = i + 1;
        }
    "#;
    assert_eq!(ok(source), "0\n1\n3\n");
}

#[test]
fn enum_display_and_equality() {
    let source = r#"
        cilmë Color { Red, Green }
        quétë(Color::Red);
        quétë(Color::Red == Color::Red);
        quétë(Color::Red == Color::Green);
        cilmë Shape { Circle { radius: Notessë } }
        quétë(Shape::Circle { radius: 2 });
        quétë(Shape::Circle { radius: 2 } == Shape::Circle { radius: 2 });
    "#;
    assert_eq!(
        ok(source),
        "Color::Red\nnaitë\nlá naitë\nShape::Circle { radius: 2 }\nnaitë\n"
    );
}

#[test]
fn match_binding_pattern() {
    let source = r#"
        essë x = 5;
        nam x {
            1 => { quétë(1); }
            n => { quétë(n); }
        }
    "#;
    assert_eq!(ok(source), "5\n");
}

#[test]
fn match_parenthesized_variant_value() {
    let source = r#"
        cilmë Shape { Circle { radius: Notessë } }
        nam (Shape::Circle { radius: 2 }) {
            Shape::Circle { radius: r } => { quétë(r); }
        }
    "#;
    assert_eq!(ok(source), "2\n");
}

#[test]
fn enum_values_in_lists() {
    let source = r#"
        cilmë Option { None, Some { value: Notessë } }
        maita get(o: Option) -> Notessë {
            nam o {
                Option::Some { value: v } => { entul v; }
                Option::None => { entul 0; }
            }
        }
        essë xs = [Option::Some { value: 1 }, Option::None];
        ilyë o mí xs {
            quétë(get(o));
        }
    "#;
    assert_eq!(ok(source), "1\n0\n");
}

#[test]
fn enum_declaration_errors() {
    let error = run("essë c = Missing::Red;").unwrap_err();
    assert!(error.contains("unknown type `Missing`"), "{error}");
    let error = run("cilmë Color { Red } essë c = Color::Blue;").unwrap_err();
    assert!(error.contains("has no variant `Blue`"), "{error}");
    let error = run("canta Point { x: Notessë } cilmë Point { A }").unwrap_err();
    assert!(error.contains("type `Point` is already defined"), "{error}");
    let error = run("cilmë Point { A } canta Point { x: Notessë }").unwrap_err();
    assert!(error.contains("type `Point` is already defined"), "{error}");
    let error = run("cilmë Color { Red, Red }").unwrap_err();
    assert!(error.contains("duplicate variant `Red`"), "{error}");
    let error = run("cilmë Shape { Circle { r: Notessë, r: Notessë } }").unwrap_err();
    assert!(
        error.contains("duplicate field `r` in variant `Circle`"),
        "{error}"
    );
    let error = run("cilmë Empty { }").unwrap_err();
    assert!(error.contains("at least one variant"), "{error}");
    let error = run("cilmë Notessë { A }").unwrap_err();
    assert!(error.contains("is reserved"), "{error}");
    let error = run("{ cilmë Color { A } }").unwrap_err();
    assert!(error.contains("only allowed at top level"), "{error}");
    let error = run("cilmë Color { A } carasta Color { maita f(immo) -> Er { } }").unwrap_err();
    assert!(error.contains("only for structs"), "{error}");
}

#[test]
fn enum_variant_errors() {
    let error = run("cilmë Color { Red } essë c = Color { };").unwrap_err();
    assert!(error.contains("is an enum"), "{error}");
    let error = run("canta Point { x: Notessë } essë p = Point::Red;").unwrap_err();
    assert!(error.contains("is not an enum"), "{error}");
    let error = run("cilmë Color { Red } essë c = Color::Red { x: 1 };").unwrap_err();
    assert!(error.contains("has no fields"), "{error}");
    let error =
        run("cilmë Shape { Circle { r: Notessë } } essë s = Shape::Circle {};").unwrap_err();
    assert!(error.contains("missing field `r`"), "{error}");
    let error =
        run("cilmë Shape { Circle { r: Notessë } } essë s = Shape::Circle { x: 1 };").unwrap_err();
    assert!(error.contains("has no field `x`"), "{error}");
    let error = run(r#"cilmë Shape { Circle { r: Notessë } } essë s = Shape::Circle { r: "a" };"#)
        .unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
}

#[test]
fn match_exhaustiveness_errors() {
    let error = run("cilmë Color { Red, Green } essë c = Color::Red; nam c { Color::Red => { } }")
        .unwrap_err();
    assert!(error.contains("match is not exhaustive"), "{error}");

    let error = run("nam 1 { 1 => { } }").unwrap_err();
    assert!(error.contains("match is not exhaustive"), "{error}");

    let error = run("nam naitë { naitë => { } }").unwrap_err();
    assert!(error.contains("match is not exhaustive"), "{error}");

    let error = run(
        "cilmë Option { None, Some { value: Notessë } } essë o = Option::Some { value: 1 }; nam o { Option::None => { } Option::Some { value: 1 } => { } }",
    )
    .unwrap_err();
    assert!(error.contains("match is not exhaustive"), "{error}");

    let error = run("nam 1 { }").unwrap_err();
    assert!(error.contains("at least one arm"), "{error}");
}

#[test]
fn pattern_errors() {
    let error = run(r#"essë x = 1; nam x { "a" => { } _ => { } }"#).unwrap_err();
    assert!(error.contains("cannot match"), "{error}");

    let error = run(
        "canta Point { x: Notessë, y: Notessë } essë p = Point { x: 1, y: 2 }; nam p { Point { x: a, y: a } => { } }",
    )
    .unwrap_err();
    assert!(error.contains("duplicate binding `a`"), "{error}");

    let error =
        run("canta Point { x: Notessë } essë p = Point { x: 1 }; nam p { Point { y: _ } => { } }")
            .unwrap_err();
    assert!(error.contains("has no field `y`"), "{error}");

    let error = run(
        "canta Point { x: Notessë, y: Notessë } essë p = Point { x: 1, y: 2 }; nam p { Point { x: _ } => { } }",
    )
    .unwrap_err();
    assert!(error.contains("missing field `y`"), "{error}");

    let error =
        run("cilmë Color { Red } essë c = Color::Red; nam c { Color::Red { x: _ } => { } }")
            .unwrap_err();
    assert!(error.contains("has no fields"), "{error}");

    let error =
        run("cilmë Color { Red } essë c = Color::Red; nam c { Color::Blue => { } }").unwrap_err();
    assert!(error.contains("has no variant `Blue`"), "{error}");

    let error =
        run("cilmë A { X } cilmë B { Y } essë a = A::X; nam a { B::Y => { } }").unwrap_err();
    assert!(
        error.contains("pattern type `B` cannot match `A`"),
        "{error}"
    );

    let error = run(
        r#"cilmë Option { Some { value: Notessë } } essë o = Option::Some { value: 1 }; nam o { Option::Some { value: "a" } => { } }"#,
    )
    .unwrap_err();
    assert!(error.contains("cannot match"), "{error}");
}

#[test]
fn block_expression_values() {
    let source = r#"
        essë x = { essë y = 2; y + 3 };
        quétë(x);
        quétë({ 1 + 2 });
        quétë({});
        quétë([{ 1 }, { 2 }]);
    "#;
    assert_eq!(ok(source), "5\n3\n()\n[1, 2]\n");
}

#[test]
fn block_expression_scope() {
    let source = r#"
        essë a = 1;
        essë b = { essë a = 2; a + 1 };
        quétë(a);
        quétë(b);
    "#;
    assert_eq!(ok(source), "1\n3\n");
}

#[test]
fn block_expression_nested_and_struct() {
    let source = r#"
        essë x = { { 1 } };
        quétë(x);
        canta Point { x: Notessë }
        essë p = { Point { x: 1 } };
        quétë(p.x);
    "#;
    assert_eq!(ok(source), "1\n1\n");
}

#[test]
fn block_expression_in_condition() {
    let source = r#"
        qui { essë y = 1; y == 1 } {
            quétë("yes");
        }
    "#;
    assert_eq!(ok(source), "yes\n");
}

#[test]
fn block_expression_return() {
    let source = r#"
        maita f() -> Notessë {
            essë x = { entul 7; 0 };
            entul x;
        }
        quétë(f());
    "#;
    assert_eq!(ok(source), "7\n");
}

#[test]
fn block_expression_returns_without_tail_return() {
    let source = r#"
        maita f() -> Notessë {
            essë x = { entul 7; 0 };
        }
        quétë(f());
    "#;
    assert_eq!(ok(source), "7\n");
}

#[test]
fn block_expression_loop_control() {
    let source = r#"
        essë i = 0;
        yallumë i < 5 {
            essë x = { i = i + 1; qui i == 2 { lelya; } i };
            quétë(x);
        }
    "#;
    assert_eq!(ok(source), "1\n3\n4\n5\n");
}

#[test]
fn block_expression_break() {
    let source = r#"
        essë i = 0;
        yallumë naitë {
            essë x = { i = i + 1; qui i == 3 { rac; } i };
            quétë(x);
        }
        quétë("done");
    "#;
    assert_eq!(ok(source), "1\n2\ndone\n");
}

#[test]
fn block_expression_assignment() {
    let source = r#"
        essë x = 0;
        essë y = { x = 1; x + 1 };
        quétë(x);
        quétë(y);
    "#;
    assert_eq!(ok(source), "1\n2\n");
}

#[test]
fn block_expression_statement_semicolon() {
    let source = r#"
        { quétë("x"); };
        quétë("y");
    "#;
    assert_eq!(ok(source), "x\ny\n");
}

#[test]
fn block_expression_in_match_value() {
    let source = r#"
        nam { 1 } {
            1 => { quétë("one"); }
            _ => { }
        }
    "#;
    assert_eq!(ok(source), "one\n");
}

#[test]
fn block_expression_errors() {
    let error = run(r#"essë x: Notessë = { "a" };"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );
    let error = run("essë x: Notessë = { essë y = 1; };").unwrap_err();
    assert!(error.contains("expected `Notessë`, found `Er`"), "{error}");
    let error = run("quétë({ entul 1; });").unwrap_err();
    assert!(error.contains("outside function"), "{error}");
    let error = run("quétë({ rac; });").unwrap_err();
    assert!(error.contains("outside loop"), "{error}");
    let error = run("essë x = { 1 2 };").unwrap_err();
    assert!(error.contains("expected `;` or `}`"), "{error}");
}

#[test]
fn match_expression_values() {
    let source = r#"
        cilmë Color { Red, Green }
        essë c = Color::Green;
        essë x = nam c {
            Color::Red => 1,
            Color::Green => 2,
        };
        quétë(x);
        quétë(nam 1 { 1 => "one", _ => "other" });
        quétë([nam 1 { 1 => 10, _ => 0 }, nam 2 { 2 => 20, _ => 0 }]);
    "#;
    assert_eq!(ok(source), "2\none\n[10, 20]\n");
}

#[test]
fn match_expression_return() {
    let source = r#"
        cilmë Color { Red, Green }
        maita f(c: Color) -> Notessë {
            entul nam c {
                Color::Red => { entul 1; }
                Color::Green => 2,
            };
        }
        quétë(f(Color::Red));
        quétë(f(Color::Green));
    "#;
    assert_eq!(ok(source), "1\n2\n");
}

#[test]
fn match_expression_block_tail() {
    let source = r#"
        essë y = { nam 1 { 1 => 5, _ => 0 } };
        quétë(y);
        essë z = nam 1 {
            1 => { 10 }
            _ => 0,
        };
        quétë(z);
    "#;
    assert_eq!(ok(source), "5\n10\n");
}

#[test]
fn match_expression_in_condition() {
    let source = r#"
        qui nam 1 { 1 => naitë, _ => lá naitë } {
            quétë("yes");
        }
    "#;
    assert_eq!(ok(source), "yes\n");
}

#[test]
fn match_expression_as_statement() {
    let source = r#"
        {
            nam 1 { 1 => { quétë("one"); } _ => { } }
            quétë("done");
        }
    "#;
    assert_eq!(ok(source), "one\ndone\n");
}

#[test]
fn match_expression_errors() {
    let error = run("essë x = nam 1 { 1 => 1 };").unwrap_err();
    assert!(error.contains("match is not exhaustive"), "{error}");

    let error = run(r#"essë x = nam 1 { 1 => 1, _ => "a" };"#).unwrap_err();
    assert!(
        error.contains("expected `Notessë`, found `Quetta`"),
        "{error}"
    );

    let error = run("essë x = nam 1 { 1 => 1 _ => 0 };").unwrap_err();
    assert!(
        error.contains("expected `,` or `}` after match arm"),
        "{error}"
    );

    let error = run("essë x = nam 1 { };").unwrap_err();
    assert!(error.contains("at least one arm"), "{error}");

    let error = run("essë x: Quetta = nam 1 { 1 => 1, _ => 0 };").unwrap_err();
    assert!(
        error.contains("expected `Quetta`, found `Notessë`"),
        "{error}"
    );
}

#[test]
fn top_level_return() {
    let error = run("entul 1;").unwrap_err();
    assert!(error.contains("outside function"), "{error}");
}

#[test]
fn parse_error() {
    let error = run("essë x = ;").unwrap_err();
    assert!(error.contains("expected expression"), "{error}");
}
