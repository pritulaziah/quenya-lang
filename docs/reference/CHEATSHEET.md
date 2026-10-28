# Quenya: What Is Implemented

## Keywords

| Word | Meaning | Rust |
| --- | --- | --- |
| `ea` | variable | `let` |
| `maita` | function | `fn` |
| `entul` | return | `return` |
| `qui` / `mal` | if / else | `if` / `else` |
| `ya` | while | `while` |
| `ilya` / `mi` | for / in | `for` / `in` |
| `rac` / `lelya` | break / continue | `break` / `continue` |
| `naite` | true | `true` |
| `la` | not | `!` |
| `ar` / `hya` | and / or | `&&` / `||` |
| `quete` | print | `println!` |
| `canta` | struct | `struct` |
| `carasta` | impl | `impl` |

`immo` is the first method parameter. It is not a keyword.

## Types

| Type | ASCII | Rust |
| --- | --- | --- |
| `Notessë` | `Notesse` | `i64` |
| `Quetta` | `Quetta` | `String` |
| `Nanwa` | `Nanwa` | `bool` |
| `Er` | `Er` | `()` |
| `Técma<T>` | `Tecma<T>` | `Vec<T>` |
| `maita(T) -> R` | same | `fn(T) -> R` |

A struct name is a user-defined type.

## Syntax

| Item | Form |
| --- | --- |
| variable | `ea x = 1;` / `ea x: Notessë = 1;` |
| assignment | `x = 2;` / `xs[0] = 3;` |
| print | `quete(x);` |
| if | `qui x { ... } mal { ... }` |
| while | `ya x { ... }` |
| for | `ilya x mi xs { ... }` |
| break / continue | `rac;` / `lelya;` |
| function | `maita f(x: Notessë) -> Notessë { ... }` |
| return | `entul x;` / `entul;` |
| list | `[1, 2, 3]` / `xs[0]` |
| empty list | `ea xs: Técma<Notessë> = [];` |
| struct | `canta Point { x: Notessë }` |
| struct literal | `Point { x: 1 }` |
| field | `p.x` / `p.x = 1` |
| method | `maita move(immo, dx: Notessë) -> Er { ... }` |
| method call | `p.move(1)` |

## Built-in Functions

| Function | Effect |
| --- | --- |
| `not(x)` | length of `Técma<T>` or `Quetta` |
| `napan(xs, x)` | add an item, return `Er` |
| `quete(x)` | print, this is a statement |

## Functions

- Parameter types are required.
- The return type is optional. The default is `Er`.
- Local types are inferred.
- A function with a non-`Er` return type must return a value on all paths.
- Closures capture the environment.
- A function must be declared before a call.
- Function type: `maita(Notessë) -> Notessë`.

## Structs and Methods

- `canta Name { field: Type, ... }` declares a struct.
- `Name { field: value, ... }` creates a struct.
- `value.field` reads a field.
- `value.field = x` writes a field.
- `carasta Name { ... }` declares methods.
- The first method parameter is `immo` (self).
- `value.method(args)` calls a method.
- Struct and impl blocks are top-level only.
- A field type must be declared before use.

## Lists

- Type: `Técma<T>`.
- `not(xs)` returns the length.
- `napan(xs, x)` adds an item.
- `xs + ys` creates a new list.
- `ilya x mi xs { ... }` iterates over a snapshot.
- `xs[i]` reads an item. `xs[i] = x` writes an item.
- `[]` needs a type annotation or context.

## Operators

- `+` works for `Notessë`, `Quetta`, and `Técma<T>`.
- `==` and `!=` need the same type.
- `<`, `<=`, `>`, `>=` work only for `Notessë`.

## Type Checking

- The check runs after the parser and before the interpreter.
- It checks types, arity, returns, conditions, loops, and `rac`/`lelya`.
- It checks struct fields and method signatures.
- An empty list gets its type from context: `ea xs = []; napan(xs, 1);`.
- `ea xs = [];` is an error.
- `qui` is a statement, not an expression.
- `not` and `napan` are free functions, not methods.
