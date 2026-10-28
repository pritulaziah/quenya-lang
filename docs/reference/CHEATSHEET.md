# Quenya: What Is Implemented

## Keywords

| Word | Meaning | Rust |
| --- | --- | --- |
| `essë` | variable | `let` |
| `maita` | function | `fn` |
| `entul` | return | `return` |
| `qui` / `mal` | if / else | `if` / `else` |
| `yallumë` | while | `while` |
| `ilyë` / `mí` | for / in | `for` / `in` |
| `rac` / `lelya` | break / continue | `break` / `continue` |
| `naitë` | true | `true` |
| `lá` | not | `!` |
| `ar` / `hya` | and / or | `&&` / `||` |
| `quétë` | print | `println!` |
| `canta` | struct | `struct` |
| `carasta` | impl | `impl` |

`immo` is the first method parameter. It is not a keyword.

Keywords use one exact spelling. Write the diacritics.

## Types

| Type | Rust |
| --- | --- |
| `Notessë` | `i64` |
| `Quetta` | `String` |
| `Nanwa` | `bool` |
| `Er` | `()` |
| `Técma<T>` | `Vec<T>` |
| `maita(T) -> R` | `fn(T) -> R` |

A struct name is a user-defined type.

## Syntax

| Item | Form |
| --- | --- |
| variable | `essë x = 1;` / `essë x: Notessë = 1;` |
| assignment | `x = 2;` / `xs[0] = 3;` |
| print | `quétë(x);` |
| if | `qui x { ... } mal { ... }` |
| while | `yallumë x { ... }` |
| for | `ilyë x mí xs { ... }` |
| break / continue | `rac;` / `lelya;` |
| function | `maita f(x: Notessë) -> Notessë { ... }` |
| return | `entul x;` / `entul;` |
| list | `[1, 2, 3]` / `xs[0]` |
| empty list | `essë xs: Técma<Notessë> = [];` |
| struct | `canta Elda { quetta: Quetta }` |
| struct literal | `Elda { quetta: "Elen" }` |
| field | `elda.quetta` / `elda.quetta = "Anar"` |
| method | `maita vista(immo, quetta: Quetta) -> Er { ... }` |
| method call | `elda.vista("Anar")` |

## Built-in Functions

| Function | Effect |
| --- | --- |
| `notië(x)` | length of `Técma<T>` or `Quetta` |
| `napan(xs, x)` | add an item, return `Er` |
| `quétë(x)` | print, this is a statement |

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
- `notië(xs)` returns the length.
- `napan(xs, x)` adds an item.
- `xs + ys` creates a new list.
- `ilyë x mí xs { ... }` iterates over a snapshot.
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
- An empty list gets its type from context: `essë xs = []; napan(xs, 1);`.
- `essë xs = [];` is an error.
- `qui` is a statement, not an expression.
- `notië` and `napan` are free functions, not methods.
