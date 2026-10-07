# Renyi for Visual Studio Code

Syntax highlighting for Renyi (`.ry`, `.renyi`), the scripting language
of AI agents: the agent writes it, you review it at a glance, and the
program can only do what it declares.

What is highlighted: comments (`#`), the documentation clauses
(`purpose:`, `tags:`, `see also:`, `deprecated:`, `example:`), text with
its holes and escapes, block text (`"""`), raw text, numbers, the
reserved words and phrases of the language reference (section 17), the
capability names of `needs` clauses, type names and calls. Blocks ending
in `end` indent and outdent.

The toolchain itself (`renyi check`, `renyi run`, `renyi test`, `renyi
mcp` for agent hosts) is a separate install:
<https://github.com/renyi-lang/renyi>. This extension is built from
`editors/vscode/` of that repository and attached to every release as a
`.vsix` ("Extensions: Install from VSIX...").
