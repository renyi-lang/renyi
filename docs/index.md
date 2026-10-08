# Renyi

*Renyi is the scripting language of AI agents: the agent writes it, you
review it at a glance, and the program can only do what it declares.*

Renyi is the scripting language of AI agents: the language an agent
writes and a person reviews at a glance. A Renyi program declares what
it may do (`needs filesystem.read("data"), network.http("api.example")`),
the compiler refuses what is not declared, and the runtime admits only
that, for the program's dependencies too. Every run can be recorded,
replayed and narrated; the project map and the semantic diff are
commands; every error carries a fix. The syntax is regular English with
one spelling per concept, so that a reviewer reads a program in one
pass: effects in the type system, no exceptions, no null, no anonymous
functions, and documentation that is part of the grammar. It is a
general-purpose language whose first users are people who run
automation with Claude Code or Codex and will not run an agent's Python
blind.

```
public function active_adult_emails(path: Path)
  returns List of Email
  or fails with FileError or JsonError
  needs filesystem.read
  purpose: Read users from a JSON file and return the e-mails of active users who are at least 18.

  let text be filesystem.read_text(path) otherwise fail
  let users: List of User be json.parse(text) otherwise fail
  let emails be
    for each user in users
    where user.is_active and user.age is at least 18
    sorted by user.name
    collect user.email
  return emails
end
```

## Install

On Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh
```

On Windows, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex
```

Both download the release for the machine, verify its checksum and put
`renyi` in place; with a Rust toolchain, `cargo install renyi` builds the
same binary. Then:

```sh
renyi check program.ry      # every diagnostic with its fix
renyi format program.ry     # the one canonical layout
renyi test program.ry       # every example: line and test block
renyi run program.ry        # main, under the grant it declares
```

## Read

- [The cheat sheet](cheatsheet.md): the whole language on one page.
- [The reference](reference.md): every construct with its grammar, its
  meaning, its static rules with their diagnostic codes and its run-time
  behaviour; [the grammar](grammar.ebnf) it quotes.
- [The examples](../examples/README.md): thirty programs, from a
  greeting to an HTTP service, each in canonical layout.
- [The starter pack for agents](../starter/README.md): a skill file,
  the MCP configuration and five workflows that do real work, with their
  tests and recordings.
- [Extending Renyi with Rust](extensions.md): a declaration file, a
  table of natives and a binary of three lines; what the boundary does
  for them.
- [Calling Python from Renyi](python.md): a declaration file bound to a
  Python module by the manifest, one worker per run, what crosses and
  what fails.
- [Embedding Renyi in a host](embedding.md): a module loaded with a
  grant, its public functions called, the memory budget; the same grant
  on the command line with `renyi run --sandbox`.
- [The positioning](design/08-positioning.md): the niche, the four
  points the language leads with and the comparison with the
  alternatives; [the design decisions](design/01-decisions.md), with
  their reasons, and the other design documents: [the syntax
  sketch](design/02-syntax-sketch.md), [the readability
  test](design/03-readability-test.md), [the standard
  library](design/04-stdlib-sketch.md), [the agent
  tooling](design/05-agent-tooling.md), [the runtime
  guarantees](design/06-runtime-guarantees.md) and [the system
  design](design/07-system-design.md).
- [The repository](https://github.com/renyi-lang/renyi): the Rust
  toolchain, the compiler written in Renyi, the conformance suite and
  the release procedure. Apache-2.0.
