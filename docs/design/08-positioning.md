# Positioning: the scripting language of AI agents

Decisions AH1 to AH4 (2026-10-07). This document fixes what Renyi is to
its first users, what it leads with, and how that is said. The README's
opening, the documentation site and every announcement derive from it.

## 1. The niche

Renyi is the scripting language of AI agents: the language an agent
writes and a person reviews at a glance. A Renyi program declares what it
may do; the runtime admits only that; and every run can be recorded,
replayed and narrated.

The first users are people who run automation with Claude Code, Codex
and their like, and who will not run an agent's Python blind. For them
the author of a program is the agent, and their own part is the review:
they need to see, in the time it takes to read a signature, what a
program will touch, and to be sure afterwards of what it did. Renyi is
built for that division of labour. It remains a general-purpose language
(decision AH4): the niche says who comes first, not who is excluded.

## 2. What Renyi leads with (decision AH2)

Four points, in this order. Each is a promise the implementation keeps
today, with the mechanism that keeps it and what a reviewer sees.

### 2.1 Effects are capabilities, declared and enforced

Every function says what it needs: `needs console, filesystem.read("data"),
network.http("api.example")`. A need can be scoped to a path, a host or a
variable, budgeted (`at most 60 per minute`, `at most 20 per run`) and guarded
(`filesystem.read("keys") only to network.http("vault.example")`, so that
what comes in through the one capability leaves only through the other).
The checker refuses a function that uses what it does not declare, and
`main`'s declaration is the whole program's grant. At run time every
effect passes one boundary in the VM, where the grant of the running
call chain, the budgets and the guards are checked; nothing happens
outside it. A dependency's effects are declared in its package the same
way, and a program's grant must cover them (section 2.4).

What the reviewer sees: the signature of `main` is the list of
everything the program can do. `renyi run --deny network ...` or
`--allow-read data` narrows it further without touching the program.

Where it is specified: `docs/reference.md` section 11; decisions C, P1 to
P3, Q1; `docs/design/06-runtime-guarantees.md`.

### 2.2 Runs are recorded, replayed and narrated

`renyi record` writes a recording of every effect a run makes, with the
run manifest in its header (the toolchain, the code hash, the
dependencies with their versions and hashes, the grant, the arguments,
the outcome, the output hash); `renyi run --replay` re-executes the
program offline from the recording; `renyi reproduce` replays a
recording under its manifest and compares the outcome and the output,
refusing a program whose code hash is not the manifest's. `renyi run
--explain` narrates a run through the `purpose:` clauses it passes, and
`replays` tests pin a function to a recording.

What the reviewer sees: what a run did, as data; whether the program
has changed since; and a narration in the words the program's author
wrote.

Where it is specified: decisions O1 to O4, P1, P2, Q2;
`docs/design/06-runtime-guarantees.md`; `docs/reference.md` appendix B.

### 2.3 The tooling an agent needs

`renyi index` is the project map: every definition with its signature,
its effects, its callers and callees, its metrics, its content hash and
its budgets, as text or JSON; `renyi index --diff <base>` is the semantic
diff, per definition, with what each change reaches and the version bump
it forces. `renyi mcp` serves the whole toolchain to an agent host over
standard input and output. `purpose:` is syntax, so that a definition
can be retrieved by what it is for, and every diagnostic carries a fix
(`fix: write `needs foreign``), so that an agent's next attempt is right.

What the reviewer sees: the agent's edits as a list of definitions with
their reach, not a text diff.

Where it is specified: `docs/design/05-agent-tooling.md`; decisions D1
to D6, G1, T1.

### 2.4 Package effects are computed, never widened silently

A package's effect manifest is computed by the tool from its sources,
not written by its author; `renyi add` prints the effects a dependency
brings, `renyi update` refuses a version whose effects widen unless
`--accept-effects` is given and `main` covers the new capability, and
`renyi audit` checks every locked dependency against every `main` that
reaches it. Every package's files are verified against the lockfile's
hashes and its manifest recomputed on fetch, so a package cannot
understate what it does.

What the reviewer sees: a dependency update that would let the program
do more is a refusal, not a surprise.

Where it is specified: decision AC1; `docs/reference.md` appendix B;
`docs/design/07-system-design.md` section 2.

## 3. The syntax, second (decision AH3)

The syntax is regular English with one spelling per concept: `let x be`,
`change x to`, `is at least`, `for each ... where ... sorted by ...
collect`, `otherwise fail`. There are no exceptions, no null, no
anonymous functions, no operator overloading and no implicit
conversions; documentation (`purpose:`, `example:`, `see also:`) is part
of the grammar; a program has one canonical layout, which `renyi
format` writes. The readability protocol (`docs/design/03-readability-test.md`)
measured the grammar before it was frozen (decision V11).

The syntax is not the lead because it is not what the first users lack:
they lack the guarantees of section 2, and the syntax is the means by
which a person checks those guarantees in one pass. Lead with "what the
program may do is in its signature"; show the signature; let the reader
notice that they read it without learning anything first.

## 4. Where Renyi stands among the alternatives

Each row is a claim the implementation keeps and the conformance suite
tests; none is a judgement of the other tools, which were built for other
things.

| | Renyi | Python in a sandbox | Deno / TypeScript | Shell scripts | Starlark / Lua embedded |
|---|---|---|---|---|---|
| What a program may do is declared | per function, scoped and budgeted, checked by the compiler | at the process boundary, by the sandbox's flags | at the process boundary, by `--allow-*` flags | not declared | by what the host chooses to expose |
| Dependencies' effects | computed from their sources, in the lockfile, refused when they widen | not declared | not declared | not declared | the host's |
| A run can be recorded and replayed | every effect, with the run manifest, by the runtime | with external tooling | with external tooling | with external tooling | no |
| A run can be narrated | through `purpose:` clauses, by the runtime | no | no | `set -x` traces commands | no |
| The project map and the semantic diff | `renyi index`, `--diff`, `renyi mcp` | language servers, text diffs | language servers, text diffs | none | none |
| Every error carries a fix | yes, by decision (D3) | no | partly | no | no |
| Read by a person without training | regular English, one spelling per concept | familiar to programmers | familiar to programmers | terse | familiar to programmers |
| Speed on integer loops | machine code in the process (decision AG1): `bench/primes.ry` six times faster than CPython | CPython | V8 | n/a | LuaJIT faster, Starlark slower |

## 5. What success looks like

Three measurable claims, to be measured on the starter pack of decision
AI3 before release 0.1 and reported with it:

1. **Review time.** A person who has not seen Renyi before states what a
   workflow program may touch, from its `main` signature, in under a
   minute, and is right.
2. **First-run rate.** An agent given the skill file and the MCP server
   writes a workflow that checks clean and runs on its first or second
   attempt; every failed attempt was answered by a diagnostic with a fix
   the agent applied.
3. **Reproduction.** Every example workflow has a recording, and `renyi
   reproduce` passes on it after a rebuild of the toolchain on another
   operating system.

## 6. How it is said

The one line: *Renyi is the scripting language of AI agents: the agent
writes it, you review it at a glance, and the program can only do what
it declares.*

Three sentences: *Renyi is the scripting language of AI agents. A
program declares what it may do, down to the path, the host and the
budget, and the runtime admits only that, dependencies included. Every
run can be recorded, replayed and narrated, and the syntax is plain
English so that a person reviews a program in one pass.*

The paragraph, for the README and the site: *Renyi is the scripting
language of AI agents: the language an agent writes and a person
reviews at a glance. A Renyi program declares what it may do
(`needs filesystem.read("data"), network.http("api.example")`), the
compiler refuses what is not declared, and the runtime admits only
that, for the program's dependencies too. Every run can be recorded,
replayed and narrated; the project map and the semantic diff are
commands; every error carries a fix. The syntax is regular English with
one spelling per concept, so that a reviewer reads a program in one
pass: effects in the type system, no exceptions, no null, no anonymous
functions, and documentation that is part of the grammar. It is a
general-purpose language whose first users are people who run
automation with Claude Code or Codex and will not run an agent's Python
blind.*
