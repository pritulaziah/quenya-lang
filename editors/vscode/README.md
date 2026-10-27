# Quenya Lang

Syntax highlighting for the Quenya programming language in Visual Studio Code, for learners and developers.

## Description

Quenya Lang adds syntax highlighting for `.q` files. VS Code does not recognize these files by default. The extension is for learners and developers who write Quenya programs.

The extension does not run programs. Use the Quenya interpreter to run `.q` files.

## Installation

The extension is not published in the Visual Studio Marketplace.

Install the extension from a VSIX file:

1. Open the Extensions view with `Ctrl+Shift+X`.
2. Open the `...` menu and select **Install from VSIX...**.
3. Select `quenya-lang-0.1.5.vsix`.
4. Reload the window.

## Quick Start

1. Install the extension.
2. Create a file named `hello.q`.
3. Add this program:

```text
ea x = 5;
quete(x);
```

4. Open the file in VS Code. The editor highlights keywords, variables, numbers, and punctuation.

## Features

- Syntax highlighting for Quenya keywords, declarations, variables, functions, strings, numbers, and comments.
- List literals, indexing, and `ilya ... mi ...` loops with `rac` and `lelya`.
- Type annotations for function parameters, return types, and local variables.
- Comment toggling with `#`.
- Bracket matching and auto-closing for `{}`, `()`, `[]`, and `""`.
- Indentation after `{`.
- Word selection for identifiers with Quenya letters.

## License

MIT. See [LICENSE](LICENSE).
