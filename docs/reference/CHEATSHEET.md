# Quenya: What Is Implemented

## Keywords

| Word | Meaning | Rust |
| --- | --- | --- |
| `essë` | variable | `let` |
| `vistë` | mutable binding | `mut` |
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
| `cilmë` | enum | `enum` |
| `nam` | match | `match` |

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

A struct or enum name is a user-defined type.

## Syntax

| Item | Form |
| --- | --- |
| variable | `essë x = 1;` / `essë x: Notessë = 1;` |
| mutable variable | `essë vistë x = 1;` |
| mutable parameter | `maita f(vistë x: Notessë) { ... }` |
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
| enum | `cilmë Color { Red, Green }` |
| variant | `Color::Red` / `Shape::Circle { radius: 1 }` |
| block expression | `{ essë y = 1; y + 1 }` |
| match | `nam x { 1 => "one", _ => "other" }` |

## Built-in Functions

| Function | Effect |
| --- | --- |
| `notië(x)` | length of `Técma<T>` or `Quetta` |
| `napan(xs, x)` | add an item to a mutable list, return `Er` |
| `clone(x)` | deep copy of `x` |
| `quétë(x)` | print, this is a statement |

## Functions

- Parameter types are required.
- The return type is optional. The default is `Er`.
- A parameter is immutable unless it is declared with `vistë`.
- Local types are inferred.
- A function with a non-`Er` return type must return a value on all paths.
- Closures capture the environment.
- A function must be declared before a call.
- Function type: `maita(Notessë) -> Notessë`.

## Block Expressions

- `{ statements; tail }` is an expression.
- Its value is `tail`, or `Er` when there is no tail.
- `{}` is `Er`.
- A block expression creates a new scope.
- `entul`, `rac`, and `lelya` inside it escape the block.
- A trailing expression must not have `;`.

## Structs and Methods

- `canta Name { field: Type, ... }` declares a struct.
- `Name { field: value, ... }` creates a struct.
- `value.field` reads a field.
- `value.field = x` writes a field.
- `carasta Name { ... }` declares methods.
- The first method parameter is `immo` (self).
- `immo` is mutable in this version. Reference receivers come later.
- `value.method(args)` calls a method. The call borrows the receiver.
- Struct and impl blocks are top-level only.
- A field type must be declared before use.

## Enums and Match

- `cilmë Name { Variant, Variant { field: Type } }` declares an enum.
- Variants are unit or struct-like. Tuple variants are not supported.
- `Name::Variant` creates a unit variant.
- `Name::Variant { field: value }` creates a struct-like variant.
- `nam value { Pattern => expr, ... }` is an expression.
- At statement position it can be used without a semicolon.
- Arms are expressions. Non-block arms need commas; block arms may
  omit them.
- Arms must have compatible types. An arm that definitely returns
  does not constrain the result type.
- Patterns: `_`, a binding, literals, `Type::Variant`, struct patterns.
- Pattern fields must be complete; `..` is not supported.
- The checker requires an exhaustive match.
- Enum values compare and print structurally.
- Enums and structs share one type namespace.
- An enum field type must be declared before use.

## Lists

- Type: `Técma<T>`.
- `notië(xs)` returns the length. The call borrows the list.
- `napan(xs, x)` adds an item. The list must be mutable.
- `xs + ys` creates a new list and moves both operands.
- `ilyë x mí xs { ... }` moves the list and iterates over it.
- `xs[i]` reads an item. `xs[i] = x` writes an item.
- A non-Copy item cannot move out of `xs[i]`. Use `clone(xs[i])`.
- `[]` needs a type annotation or context.

## Operators

- `+` works for `Notessë`, `Quetta`, and `Técma<T>`.
- `+` moves non-Copy operands.
- `==` and `!=` need the same type. They borrow both operands.
- `<`, `<=`, `>`, `>=` work only for `Notessë`.

## Ownership

- `essë` creates an immutable binding. `essë vistë` creates a mutable binding.
- Copy types: `Notessë`, `Nanwa`, `Er`, and enums whose fields are all
  Copy. Functions are Copy.
- `Quetta`, `Técma<T>`, structs, and other enums are moved.
- Use `clone(x)` to copy a non-Copy value.
- A moved value cannot be used. Reassignment reinitializes it.
- A field move is partial. Other fields remain usable.
- Using the whole struct after a partial move is an error.
- `quétë`, `notië`, and `clone` borrow their argument.
- `napan` needs a mutable list. It moves the item into the list.
- `nam` moves its value. Each arm starts from the same move state.
- A closure captures outer names by reference. Capture borrows are not
  checked yet.
- The `ilyë` loop variable is immutable.

## Type Checking

- The check runs after the parser and before the interpreter.
- It checks types, arity, returns, conditions, loops, and `rac`/`lelya`.
- It checks struct fields and method signatures.
- It checks enum variants, pattern types, and match exhaustiveness.
- It checks block expression tail types and match expression arm types.
- It checks mutability, moves, and partial moves.
- An empty list gets its type from context: `essë vistë xs = []; napan(xs, 1);`.
- `essë xs = [];` is an error.
- `qui` is a statement, not an expression.
- `notië` and `napan` are free functions, not methods.
