# Renyi for Visual Studio Code

Renyi (`.ry`, `.renyi`) is the scripting language of AI agents: the
agent writes it, you review it at a glance, and the program can only do
what it declares. This extension highlights it and connects the editor
to its language server.

**Highlighting**: comments (`#`), the documentation clauses
(`purpose:`, `tags:`, `see also:`, `deprecated:`, `example:`), text with
its holes and escapes, block text (`"""`), raw text, numbers, the
reserved words and phrases of the language reference (section 17), the
capability names of `needs` clauses, type names and calls. Blocks ending
in `end` indent and outdent.

**The language server** (`renyi lsp`, started for each workspace
folder): the diagnostics of `renyi check` for every file of the folder,
updated as you type; hover on a call, a type, a constant or a
declaration for its signature and its purpose, the standard library's
included; go to definition; the outline of a file (functions, types with
their fields or variants, abilities, implementations, constants, tests).

The toolchain is a separate install: <https://renyi-lang.org> (or
`cargo install renyi`). The extension runs `renyi` from the PATH; the
setting `renyi.path` names another binary. Without it the extension
highlights only and says so once.

This extension is built from `editors/vscode/` of
<https://github.com/renyi-lang/renyi> and attached to every release as a
`.vsix` ("Extensions: Install from VSIX...").
