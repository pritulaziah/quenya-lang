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

## Types

| Type | ASCII | Rust |
| --- | --- | --- |
| `Notessë` | `Notesse` | `i64` |
| `Quetta` | `Quetta` | `String` |
| `Nanwa` | `Nanwa` | `bool` |
| `Er` | `Er` | `()` |
| `Técma<T>` | `Tecma<T>` | `Vec<T>` |
| `maita(T) -> R` | same | `fn(T) -> R` |

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
- An empty list gets its type from context: `ea xs = []; napan(xs, 1);`.
- `ea xs = [];` is an error.
- `qui` is a statement, not an expression.
- `not` and `napan` are free functions, not methods.
