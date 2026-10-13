# Handoff

Last updated: 2026-10-12, session 14, on the owner's machine. Its work:
the design study of stage 29, the new backend for the VM (the section
"Stage 29, the new backend: the design study (session 14)" below: the
checker's image run attributed by category under callgrind, eleven
findings, two roads, the stages with their estimates), the owner's
eight answers to it as decision AW1 (road (A), the Cranelift tier
evolved in stages, the gate on the check row alone, stage 0 first),
stage 0 built and measured (decision AW2: the allocator entered by
mimalloc's plain functions and the module constants an image builds
before any run; the check row 3,085.3 to 2,889.9 million instructions,
-6.3%, the ratio by instructions 6.99 to 6.54 against the gate's 3),
stage 1a built and measured (decision AW3: a typed parameter's boxed
operand taken from the stack at the call site; the calls through the
VM's path 141,577 to 9,380, the check row 2,889.9 to 2,826.6, -2.2%,
the ratio 6.54 to 6.40), stage 1c built and measured (decision AW4:
the frame's leave by the static ownership state of the slots; the
leaves through `rt_truncate` 917,264 to 32,514, the check row 2,826.6
to 2,729.4, -3.4%, the ratio 6.40 to 6.18), and stage 1b built and
measured (decision AW5: the read-only flag per parameter in the
bytecode, format 7 to 8, and a direct call passing such a parameter
borrowed when its callee never moves it into an owned sink; the check
row 2,729.4 to 2,719.3, -0.4%, the ratio 6.18 to 6.16), and stage 1d
built and measured (decision AW6: the frame record of a direct call
made cheaper, the grant in the body's signature, the stack's length
and the room checks in the callee's prologue; the check row 2,719.3 to
2,681.0, -1.4%, the ratio 6.16 to 6.07), which
completes stage 1, and the character tables and the hasher built and
measured (decision AW7: `run_of` and `run_until` by a table of the
set's one-character members, the tables of the last eight sets kept,
the VM's maps and sets hashed by a folded multiplication keyed per
process in place of SipHash; the check row 2,681.0 to 2,527.2,
-5.7%, the ratio 6.07 to 5.72), and stage 2a built and measured
(decision AW8: a `with` whose base the checker typed updated by static
index, through a helper on both tiers and in the generated code on the
Cranelift tier; the check row 2,527.2 to 2,370.7, -6.2%, the
ratio 5.72 to 5.37), and stage 2b built and measured (decision AW9: a
record of two fields returned as a pair of cells to a direct caller,
kept in a pair slot and taken apart in place, past the test of an
`otherwise` that only a failure reaches; the check row 2,370.7 to
2,301.9, -2.9%, the ratio 5.37 to 5.21), and stage 2c built and
measured (decision AW10: the frees in place, a value released by its
tag in the VM's Rust, the last holder of a record or a list releasing
its fields or its elements the same way; by instructions nearly what
the glue cost, the check row 2,301.9 to 2,296.2, -0.2%, but the
glue's indirect branch per field gone, AT1's estimate -1.3%,
kept by AT1's rule; the ratio 5.21 to 5.20), which completes
stage 2, and stage 3a built and measured (decision AW11: the natives'
in-place entry, a second entry per native through AK1's registration
API, which the generated code of both tiers and the interpreter call
directly through a table the VM keeps per function, answering in the
receiver's place with the arguments consumed, no scratch vector, no
`Result`, no status and no push on the way; the check row 2,296.2 to
2,203.9, -4.0%; the ratio 5.20 to 4.99); on the way, a
Windows defect since AT5 fixed (a2c4685: every run of an image there
copied its code section, the mapping refused for want of execute access
on the file's handle); and stage 3b built and measured (decision
AW12: `status` and `into_plain` off the path, a plain record or
variant built through a helper without a status on both tiers, a
plain item pushed onto a plain list where it lies, the guard tests in
place; the check row 2,203.9 to 2,099.1, -4.8%; the ratio 4.99
to 4.75), which completes stage 3; and stage 4a built and measured
(decision AW13: `is` and `is not` answered in place as structural
equality unless the left operand's type declares its own `equals`,
two values of one variant type compared in the generated code of both
tiers by the type and the tag of their blocks; the check row 2,099.1
to 1,995.8, -4.9%; the ratio 4.75 to 4.52); and stage 4b built and measured (decision AW14: a loop's
iterator in two slots of the frame, the source listed in the first and
the position as a small Integer in the second, which the Cranelift
tier keeps in a register, the native iterator gone; the check row
1,995.8 to 1,930.3, -3.3%; the ratio 4.52
to 4.37); and stage 4c built and measured (decision AW15: a `match` with
at least three arms naming a variant entered through a jump table by
the subject's tag on both tiers and in the interpreter; the check row
1,930.3 to 1,896.4, -1.8%; the ratio 4.37
to 4.29), which completes stage 4; and the checker's own cuts between
the stages built and measured (decision AW16: the variants of a
module's scope indexed by name, the parser's cursor carrying its
looked-at token and updated in its slot, the lexer's previous token as
two Integers and the text of a run as the primitive `slice_text`, the
resolver's loop, the checker's lists updated in the slot; the check row
1,896.4 to 1,586.1, -16.4%; the ratio 4.29
to 3.59); and stage 5a built and measured (decision AW17: the moves of
AU17 written in the image and in the cache keyed by the sources, the
pass run once where a program enters the VM; the check row
1,586.1 to 1,547.9, -2.4%; the ratio 3.59 to
3.51); and stage 5b built and measured (decision AW18: the code
objects decoded on demand, a body decoded at its first read from the
image's or the cache entry's bytes; the check row 1,547.9 to
1,528.5, -1.3%; the ratio 3.51 to 3.46),
which completes stage 5; and stage 6 measured and left off (decision
AW19: AU50's expansion under the new convention, the check row +3.7%
with it on). Every stage of AW1's order is done: the ratio stands at
3.46 on the check row against the gate's 3; the owner chose road (B),
the new code generator (decision AW20), and its design study is written
(the section "Stage 31, the new code generator (road (B)): the design
study (session 14)": four measured stages on the Cranelift tier, about
340 million by the estimates), its questions answered (decision
AW21: every recommended option; the order B2, B3, B1, B4, each
measured by the old measure with the lane's micro programs), and B2
built and measured (decision AW22: the operands in registers,
`Abs::Held`; the check row 1,528.5 to 1,482.3, -3.0%;
the ratio 3.46 to 3.36); B3, the locals in registers, built,
measured and left off (decision AW23: +6.0% on the check row
against the base with the leak fixed), and the leak of stages 2b to
B2 its measure found fixed (AW23: the check row 1,493.5, the ratio
3.38); B1, the frame record lazy, done (decision AW24: the check
row to 1,482.7, -0.7%; the ratio to 3.36); B4, the counts by
dataflow, done (decision AW25: the check row to 1,437.8,
-3.0%; the ratio to 3.26); the four stages of road (B)
are measured; the owner's call after them, the reserve of AW21 (8),
is stage 32, the second round of the checker's own cuts, done
(decision AW26: four cuts in `compiler/*.ry` on B4's binary, the
check row to 1,307.5, -9.1%; the ratio to 2.96,
within the gate of 3); the owner's decision after it (AW27): the switch
of the front end deferred, the Rust front end kept, the speed round
closed; the owner's choice after it (AW28): the WASM target, then the
language server's completion, then the C API, in that order, with the
showcase game in parallel by a subagent; the game's part by the
subagent is committed (b8f700e) and stage 33's design study is
written (the section "Stage 33, the WASM target: the design study
(session 14)"), its four questions put to the owner (the start-here
item 1).
The session's record is "Done in session 14"
below.
Session 13, in the cloud environment, had as its last work the icon
and the wordmark, at the owner's request after the
handover below (decision AV1: a war hammer on the accent tile and the
name signed in Ms Madi, held by `tools/brand.py` and used in the site's
header and tab, the README, the VS Code extension and the social
preview, which waits on the owner's upload; the section "Done in
session 13" below). Before it, stage 28 (B4, the compiler's own code)
closed after eight cuts, decisions AU52 to AU57 (the section "The front
end's round" below, "B4 as built"): the Renyi checker's run on
`compiler/bodies.ry` 4,898.3 to 4,391.6 million instructions (-10.3%),
the ratio of AU36 (iii) 7.4 to 6.9 against the gate of 3; and the
owner's answers after it (decision AU58): the gate stays, and the way
to it is a new backend for the VM, its design study first, which the
next session writes (the start-here item 1). The session ended at the
owner's word, to save the progress and hand over. Earlier in the
session it
finished stage A2 of the front end's round, the cache keyed by the
sources (decision AU44: every `run`, `record` or `test` compiled from
its sources leaves the program it compiled in the cache with every file
the compile read, its warnings and, for a manifest, its code hash and
dependencies; a later run of the same path whose files all read as they
did loads the program in place of the front end and prints the warnings
again; `build --cache` finds it too; a hit of `hello` 7.6 to 5.8 ms
(-24%, -33% in instructions), the self-check's second run 822 to 762
ms, AT1's `cached run` row -6.8%, its cold rows unmoved; the section
"The front end's round" below, "A2 as built"; commit b765636), with
the gates passed and the next stage, B5, planned in the same section
("B5, the stage next"); and wrote `docs/ROADMAP.md` at the owner's
request, the map of every stage numbered and grouped by track, which
CLAUDE.md now asks each session to update when a stage closes; and,
the owner having asked to go on stage by stage, did B5, the first
stage on the VM side of the round (decision AU45: two Integers of
which at least one is boxed, the checker typing it so, compared in
place in both tiers when both are small, code format 13;
`scan_codes` -48% in instructions a turn and -55% in time, the
self-check's synchronous row -0.55% in instructions and -0.77% by the
estimate, its time -2% to -6%, the Renyi checker 806 to 756 ms, the
gate ratio 10.0 against AU34's binary; the section "The front end's
round" below, "B5 as built"); then B1, the Renyi lexer rewritten for
speed with the library as it is (decision AU46: one loop, scans
without a call per character, the word lists as sets, a step's
product a sum; the lexer alone on `compiler/bodies.ry` 983 to 514
million instructions, -48%, byte-equal by the judges; the Renyi checker
756 to 649 ms on `compiler/bodies.ry`, the gate ratio 10.0 to 8.9
against AU34's binary, the parse row 313 to 258 ms; the same section,
"B1 as built"); then the first cuts of B2, the Renyi parser's keyword
and symbol tests without a variant built per test and its token reads
without nested calls (decision AU47: the parser alone 1,197 to 1,039
million instructions per parse of `bodies.ry`, -13%; the Renyi checker
649 to 630 ms, the gate ratio 8.9 to 8.6),
then the rest of B2 (decision AU48: a look as an index with no
record per look, the expression levels as one loop; the parser alone
1,039 to 810 million instructions per parse of `bodies.ry`, -22%; the
Renyi checker 630 to 579 ms; the section "The front end's round"
below, "B2 as built"); then the variant test in place on both tiers
(decision AU49: `Op::IsVariant` without its helper, a guarded value
through it as the cold path; the self-check's synchronous run -0.9%,
the ratio's check 546 ms, 7.5 times the reference; the section's
"The variant test in place, as built", and B3 planned after it). The
same day the owner asked for the README's opening as the design
philosophy and the selling points (68c5cd2) and for the site
redesigned by an Opus subagent, minimal, modern and premium, then a
second round with the words cut and the visuals strengthened, then a
third with the other pages' layout, four concept diagrams and a
terminal demo (the record of the session below, c2a082a, af8bee4 and
84b3390); all are in.
Session 12, in the cloud environment,
put the three questions after AU27 to the owner (decision AU28: the
counts in place in template code next, then the natives building a
record in its block, the decoder of `std.json` first, and the front
end's speed a round of its own after this one) and did both stages:
decision AU29, the retain and the release in place in template code
(the template code alone -5.2% by AT1's estimate, the self-check -5.6%
in wall-clock on one hardware thread; the run part of AU22 within
AU18's 15% on this boot), and decision AU30, the decoder's records
built in their block (`json_round_trip` -3.9% in instructions, the
self-check's instructions equal after two builds that moved them; the
section "The typed round" below, "The owner's answers after AU27").
The owner answered the next questions (decision AU31: AU29's sequence
on the Cranelift tier, then the hottest natives in place), and decision
AU32 put the counts in place in Cranelift's code and stopped the
compile thread at the end of a run (the self-check -9% to -15% in
wall-clock on one to four hardware threads, `strings` -13% to -16%, the
image's run part -16%). After a study of the typed calls the owner decided AU33, and decision
AU34 put `List.at` in place in both tiers, with the list's layout
probed at startup, and `Text.contains` and `List.contains` through lean
helpers (the synchronous JIT run -4.7% by AT1's estimate, the image
-6.7%). CI passed on both jobs after every push. The owner then asked for a
progress report and answered after it (decision AU35): the typed round
closes with its three goals met, the front end's round opens with a
study, and the binary keeps the Rust front end until the compiler
written in Renyi comes within 3 times its time (11 to 14 times now).
The same session then made the front end's study (the section "The
front end's round" below), explained to the owner why the compiler
written in Renyi takes eleven times the Rust front end's time (per
operation, in instructions: a call 96, a character read and compared
135, the lexer's `is_lower` 370, the parser's step on a shared cursor
1,340, where compiled Rust spends none to a few), and did the owner's
order for the round (decisions AU36 and AU40): AU37, every file parsed
once by the Rust front end; AU38, the parser's token comparisons
inlined; AU39, the module trees shared rather than copied; AU41, the
self-check frozen under `bench/selfcheck/` as AT1's program; AU42, a
program declaring only the library modules it needs, in both front
ends. `renyi check compiler/bodies.ry` went from 437 million
instructions and 72 ms to 153 million and 34.5 ms, an empty program's
check from 6.4 to 4.7 ms, `run examples/hello.ry` from 7.8 to 6.5 ms,
every benchmark faster on the same boot. The owner then set what a hit
of the cache keyed by the sources loads (AU43), and the session stopped
in that stage, A2, with its first piece committed (the reads a compile
makes, noted); the section "The front end's round" says what remains.
Session 11, in the cloud environment, put the four questions of session
10 to the owner (decision AU20: the
template tier's follow-ups first, then the representation items of
AU1's step v; a Windows job in CI; the goal of AU18 kept; the tier on
by default) and did both parts: decision AU21, direct calls between
template functions and the field read in place; the design study of
the representation items with a census build and callgrind, and the
owner's answers to it (AU22: three cuts outside the list first, then
small texts inline, then records and variants in one allocation; lists
of unboxed Integers and the flattened `Int` deferred; AU18's goal
measured on the run part); decisions AU23 to AU25, the three cuts (the
guard bookkeeping skipped, a variant without fields read from its slot,
one shared empty list); AU26, small texts held in the value; AU27,
records and variants in one counted block. Over the session the
self-check's JIT run on one hardware thread went from 1,592.8 to 1,164.4
ms (-26.9%) and on four from 1,507.0 to 1,186.4 (-21.3%); `records`
-20% (the section "The typed round" below, from "The template tier's
direct calls are in" to "Over the session, AU19's binary against
AU27's"). The Windows job passed on every push.
Session 10, in the cloud environment,
did stages 6 to 9 of the typed round: decision AU11, the update of
a uniquely held record in place (the ops `WithSlot` and `TakeField`,
bytecode format 7, three emitter rules in both emitters, the judges
equal), kept by AU1's rule with a small gain and a finding about why;
the owner's answers after it (AU12); decision AU13, the comparisons on
borrowed operands in the generated code, -2.2% on the self-check's cold
JIT run; the owner's answers after it (AU14: the borrowed parameters
skipped, the compile cost measured, the compile thread chosen with
wall-clock as its measure); decision AU15, the JIT's compilation on
a thread of its own with the hotness factor 100, -11.6% on the
self-check's wall-clock; the owner's answers after it (AU16: the
template tier ordered as the next large item, the liveness pass as the
stage now); decision AU17, the last read of a slot as a move, a
liveness pass the VM applies to a copy of the program before it runs;
the owner's answers on the design of the template tier (AU18); and
decision AU19, the template tier as built: a small x86-64 assembler and
a fixed sequence per op, every code object on machine code from its
first call and promoted to the Cranelift tier once hot, with a defect
of the Cranelift tier's hand-back fixed on the way (the section "The
typed round" below, "Stage 6 is in" to "The template tier is in"); and
answered the owner's question on the execution model (the section "The
execution model, as the owner asked on 2026-10-09" below). The session
ended on the owner's word ("finish the current task, then pause and
hand over to the next session") with the template tier committed.
Session 9, the first in the
cloud environment (claude.ai/code), finished the profile-guided round on
strings and JSON that session 8 had paused: decision AQ, the first of the
interspersed performance items the owner set after M5 (the section "The
profile-guided round on strings and JSON as it exists" below), and did
the baseline JIT, decision AR1, in its three stages: the fused op
`LoadField` in the bytecode (AR2), direct calls between generated
functions with register arguments (AR3) and the layout of `Value`
pinned so that the value operations, the frame protocol and the field
read run in place (AR4), then the tiering measured and the hotness
factor raised (AR5), and the round closed by the owner with the
self-check 15% below where AQ left it (AR6; the section "The baseline
JIT" below); then `renyi build`, the image of a program that runs
without compiling, decisions AS1 to AS4 (the section "`renyi build` as
it exists" below), `build --exe`, the self-contained executable,
included; then the round on the size of the generated code, decisions
AT1 to AT7 (the section "The size of the generated code" below): the
cuts at no cost, the image's program in binary, the static stack
height, the image's code section mapped from the file and the reference
counts as calls, every stage of the owner's plan measured and in, the
round closed by the owner on 2026-10-09 (AT7); then the typed round,
decision AU1, opened by the profile of `bench/strings.ry` on the JIT
tier and the owner's choice of a cure over another cut: the checker's
static types brought to the code generator (the section "The typed
round" below): stages 1 to 4 (decisions AU2 to AU8), the owner's order
for the rest (AU9) and stage 5, the image cache of `renyi run` (AU10);
the session paused there at the owner's request, stages 6 to 8 planned
at the end of that section.
Session 8
(stage 2 of the gap audit of
`docs/GAPS.md`: the formal grammar `docs/grammar.ebnf` with decision V12;
the language reference `docs/reference.md`, normative, held to the
grammar file and the crates by a test; the sketch retired to the design
record; the front end written in Renyi under `compiler/`, decisions W1
to W4, held equal to the Rust parser by `crates/renyi/tests/selfhost.rs`;
then the checker written in Renyi, decisions W5 to W8, held equal to
`renyi check --json` by the same test; then the profile of the VM, W6,
with decisions X1 to X4: the development profile optimizes the VM,
`List.slice`, the interpreter loop rewritten, `renyi run --profile`;
then X5, the emitter writes a bytecode file, and X6, mimalloc; then the
residue of stage 1, decisions Y1 to Y4; then the bytecode file itself,
decisions Z1 to Z4, with `renyi compile` and the loader; then the rename
of `set` to `change`, decision AA1; then the emitter written in Renyi,
`compiler/emit.ry` and `compiler/compile.ry`, held equal to `renyi
compile` byte for byte by the same test, decision Z3; then constraints
with type arguments, open item R3-2, decision AB1; then the judges run
the front end from its bytecode; then packages, decision AC1, the first
slice of stage 3 in two commits; then the diagnostics of the front end
in Renyi, decision AD1: the Rust lexer's and parser's codes, fixes and
recovery, the three judges byte-equal on rejected programs too; then
`std.process`, decision AE1, the second slice of stage 3; then the
foreign function interface, decision AF1, the third; then machine code
for the bytecode, decisions AG1 to AG5, and the bounded VM round, AG6;
then the niche, decisions AH1
to AH4 with `docs/design/08-positioning.md` and the README's opening;
then release 0.1 decided, AI1 to AI4, and its engineering: the
release workflow, the installers, the VS Code extension and the
procedure `docs/RELEASE.md` (621fecc); the starter pack for agents
under `starter/`, AI3 (ee8b74b); the documentation site, `tools/site.py`
with `.github/workflows/pages.yml` and `docs/index.md`, and the
crates.io metadata; then decision AI5, the copies that make the two
crates self-contained, so that `cargo package --workspace` verifies
all seven; then foreign packages decided, AJ1 to AJ4: Rust natives
through a registration API, Python through a typed bridge, after
0.1; then release 0.1.0 itself, the evening of 2026-10-07: the
repository public under `renyi-lang/renyi`, the seven crates on
crates.io, the tag `v0.1.0` with its release and the site live; then
the registration API for Rust natives, decisions AK1 to AK4: the
standard library as the first extension, `renyi` a library too, the
guide `docs/extensions.md`; the domain renyi-lang.org in front of
the site; and the Python bridge, decisions AL1 to AL4: a Python module
bound by the manifest as a foreign one is, one worker per run,
`PythonError`, the guide `docs/python.md`; and the Python binder,
decisions AM1 and AM2: `renyi bind --python` writes the declaration
file from the package; and the resident world, decision AN1:
`renyi_workspace`, which `renyi mcp` holds between calls, the first of
M5's four steps in the order the owner set; and the language server,
decisions AN2 and AN3: `renyi lsp` on that world, the client in the
VS Code extension; the watch of `serve`, decision AO1: `renyi serve
--watch` runs `main` again between two requests on a clean new
version, the socket kept open, the third step; and the embedding API,
decisions AP1 and AP2: `renyi::Sandbox` with the grant as a `needs`
clause and the memory budget, `renyi run --sandbox`, the fourth step,
which completes M5).
Branch: `main` is the only branch (owner's decision, 2026-10-05); commit
and push there directly.

## Start here (session 14)

1. The work now is stage 29 of the roadmap, the new backend for the
   VM, stage by stage in AW1's order. Its design study is the section
   "Stage 29, the new backend: the design study (session 14)" below
   (the attribution by category, eleven findings, the two roads, the
   stages with their estimates, the checker's own shape, the order),
   and the owner's eight answers are decision AW1 (road (A), the
   Cranelift tier evolved in stages; the gate measured on the check row
   alone; stage 0 first; the read-only flag per parameter in the
   bytecode from both emitters; `Done` without its allocation by a
   convention per function; the natives' in-place entry through AK1's
   API; the character tables and the VM's hasher; the iterator in two
   slots). Stage 0 is done (decision AW2; the paragraph "Stage 0 as
   built" at the end of the study's section): the allocator entered by
   mimalloc's plain functions, the module constants an image builds
   before any run; the check row 3,085.3 to 2,889.9 million
   instructions (-6.3%), the ratio by instructions 6.99 to 6.54. Stages
   1a, 1c and 1b are done too (decisions AW3, AW4 and AW5; the
   paragraphs "Stage 1a as built", "Stage 1c as built" and "Stage 1b
   as built" after it): a typed parameter's boxed operand, which the
   checker typed, taken from the stack at the call site after a test
   of its tags (the calls through the VM's path 141,577 to 9,380, the
   check row 2,889.9 to 2,826.6, -2.2%, the ratio 6.54 to 6.40); the
   frame's leave by the static ownership state of the slots,
   `owned_slots` in `native/infer.rs` (the leaves through
   `rt_truncate` 917,264 to 32,514, the check row to 2,729.4, -3.4%,
   the ratio to 6.18); and the read-only flag per parameter in the
   bytecode (`Code::read_only`, format 7 to 8, both emitters, the
   judges) with a direct call passing such a parameter borrowed when
   its callee never moves it into an owned sink (the mask in the
   body's signature, `rt_retain_params` on the ways out
   that take the frame as owned; the check row to 2,719.3,
   -0.4%, the ratio to 6.16); and the frame record of a direct call
   made cheaper (decision AW6, "Stage 1d as built": the grant in the
   body's signature, the stack's length and the room checks in the
   callee's prologue; the check row to 2,681.0, -1.4%, the
   ratio to 6.07), which completes stage 1 (the check row
   2,889.9 to 2,681.0 since stage 0, -7.2%); and the character tables
   and the hasher (decision AW7, "The character tables and the hasher
   as built": the scans by a 128-bit table with the tables of the last
   eight sets kept, the VM's own hasher for its maps and sets; the
   check row to 2,527.2, -5.7%, the ratio to 5.72); and stage 2a
   (decision AW8, "Stage 2a as built": a `with` whose base the checker
   typed updated by static index, the helper on both tiers and the
   update in the generated code on the Cranelift tier; the check row
   to 2,370.7, -6.2%, the ratio to 5.37); and stage 2b (decision AW9,
   "Stage 2b as built": a record of two fields returned as a pair of
   cells to a direct caller, kept in a pair slot and taken apart in
   place, past the test of an `otherwise` that only a failure reaches;
   the check row to 2,301.9, -2.9%, the ratio to 5.21); and stage 2c
   (decision AW10, "Stage 2c as built": the frees in place, a value
   released by its tag in the VM's Rust, the last holder of a record or
   a list releasing its fields or its elements the same way; by
   instructions nearly what the glue cost, the check row to 2,296.2,
   -0.2%, but the glue's indirect branch per field gone, AT1's
   estimate -1.3%; the ratio to 5.20), which
   completes stage 2 (the check row 2,527.2 to 2,296.2); and stage 3a
   (decision AW11, "Stage 3a as built": the natives' in-place entry, a
   second entry per native the generated code of both tiers calls
   directly through the VM's table, answering in the receiver's place;
   the check row to 2,203.9, -4.0%; the ratio to 4.99); and stage 3b
   (decision AW12, "Stage 3b as built": `status` and `into_plain` off
   the path; the check row to 2,099.1, -4.8%; the ratio to
   4.75), which completes stage 3; and stage 4a (decision AW13,
   "Stage 4a as built": `is` and `is not` in place, variants compared
   in the generated code by the type and the tag; the check row to
   1,995.8, -4.9%; the ratio to 4.52); and stage 4b (decision AW14, "Stage 4b as built": the
   iterator in two slots; the check row to 1,930.3, -3.3%; the
   ratio to 4.37); and stage 4c (decision AW15, "Stage 4c as built": `match` by
   a table; the check row to 1,896.4, -1.8%; the ratio to
   4.29), which completes stage 4; and the checker's own cuts between
   the stages (decision AW16, "The checker's own cuts as built": the
   variants indexed, the parser's cursor with its token, the lexer's
   previous token as two Integers and `slice_text`, the resolver's
   loop, the lists updated in the slot; the check row to 1,586.1,
   -16.4%; the ratio to 3.59); and stage 5a (decision AW17, "Stage 5a
   as built": the moves written in the image and the cache; the check
   row to 1,547.9, -2.4%; the ratio to 3.51); and stage 5b (decision
   AW18, "Stage 5b as built": the code objects decoded on demand; the
   check row to 1,528.5, -1.3%; the ratio to 3.46),
   which completes stage 5; and stage 6 (decision AW19, "Stage 6 as
   built": the expansion measured again under the new convention, the
   check row +3.7% with it on, left off; no code).
   Every stage of AW1's order is done; the owner chose road (B) of
   AW1 (decision AW20), and the work now is stage 31, the new code
   generator: its design study is the section "Stage 31, the new code
   generator (road (B)): the design study (session 14)" (where the run
   stands, what an op costs today by the lane's micro programs, the
   four stages B2, B3, B1 and B4 with their estimates, what (B) does
   not touch), and the owner's answers to its questions are decision
   AW21; B2 is built and measured (decision AW22, "Stage B2 as
   built" at the end of the study's section: the operands in
   registers, `Abs::Held`; the check row to 1,482.3, -3.0%;
   the ratio to 3.36); B3, the locals in registers, was built,
   measured and left off (decision AW23, "Stage B3 as built" after
   B2's: +6.0% on the check row against the base with the leak fixed,
   the whole rise in the generated code), and the leak of stages 2b to
   B2 its measure found is fixed (AW23: the owned analysis past an
   `otherwise` region; `renyi_b2fix`, the check row 1,493.5, the ratio
   3.38); B1, the frame record lazy, done (decision AW24, "Stage B1
   as built" after B3's: the check row to 1,482.7, -0.7%; the
   ratio to 3.36; `renyi_b1`, the base of the next measure);
   B4, the counts by dataflow, done (decision AW25, "Stage B4 as
   built" after B1's: the check row to 1,437.8, -3.0%; the ratio
   to 3.26; `renyi_b4`, the base of the next measure); the four
   stages of road (B) are measured; the owner's call after them was
   the reserve of AW21 (8), stage 32, the second round of the
   checker's own cuts, done (decision AW26, "Stage 32 as built"
   below: four cuts in `compiler/*.ry` on B4's binary, the check row
   to 1,307.5, -9.1%; the ratio to 2.96, within the gate of 3;
   `study/cuts2/` in the perf lane, `renyi_b4` still the binary and
   the base of the next measure); the owner's decision after it (AW27,
   2026-10-13): the switch of the front end deferred although its gate
   is met, the Rust front end kept, the speed round closed at the gate;
   the owner's choice after it (AW28, 2026-10-13): stage 33, the WASM
   target, first (its design study first: the blockers surveyed by an
   exploring agent, the suspension of a program at `screen.present`
   designed, the questions to the owner in a batch of four), then the
   language server's completion, references and rename, then the C API;
   and in parallel stage 34, the showcase game, a Doom-style raycaster
   under `showcase/raycaster/` written by a subagent (Opus 5.5, medium effort, the contract of AW28
   (4)), committed as b8f700e (eleven modules and a README, 94 examples
   and tests, CI holds it like the starter pack; the fields `screen_x`
   and `screen_y`, the checker refusing single-letter names); the design
   study of stage 33 is the section "Stage 33, the WASM target: the
   design study (session 14)" (the trial build in the wasm lane, the
   blockers, the suspension, the host interface, `std.screen`, the
   stages 33a to 33d proposed), and its four questions were put to the
   owner on 2026-10-13 (the suspension, what the browser runs, the
   toolchain, the gating), to be asked again at the start of the next
   session if unanswered; the answers are the next decision, and the
   stages follow them, `std.screen` and the game's page among them (33c
   and 33d).
   Each stage: the code, the gates of item
   3, the measure
   (AU58's: the checker's image run under callgrind with the study's
   table by category beside it, AT1's rows, the benchmarks by
   instructions), then its decision entry with the table and the
   roadmap's stage 29 paragraph. The measuring is done in the perf lane
   (`D:/Projects/.worktrees/Renyi/perf/`) under WSL, from PowerShell as
   `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/.worktrees/Renyi/perf/<script>`:
   `measure_stage.sh <name>` builds the working tree for Linux, keeps
   the binary as `renyi_<name>` and runs the three callgrind rows and
   the Rust rows into `study/<name>/`; `size_rows.sh <name>` AT1's
   rows; `bench_both.sh <a> <b>` the benchmarks by `perf stat`;
   `study_table.py study/<name>/cg.check.out` the table by category;
   `study_map.py study/<name>/cg.check.out study/<name>/checker.ryi
   study/<name>/checker.ryc` the generated code per code object and the
   calls that enter one through its trampoline, ranked (the slow calls
   of finding 3; the `.ryc` is `renyi compile compiler/checker.ry`);
   `ratio_wsl.sh` the wall-clock ratio, which the file reads through
   `/mnt/d` dominate, so the ratio by instructions is the one recorded.
   The binaries kept: `renyi_au34` (44782aa, the reference), `renyi_head`
   (7cbbbe0, the study's base), `renyi_stage0` (2632cde, stage 0),
   `renyi_stage1a` (7bb927b, stage 1a), `renyi_stage1c` (1cfcd7d, stage
   1c), `renyi_stage1b` (2367c63, stage 1b), `renyi_stage1d` (ddbf36a,
   stage 1d), `renyi_tables` (the character tables alone, the table
   built per call), `renyi_hasher` (94089f9, the tables with their
   cache and the hasher), `renyi_stage2a_helper` (stage 2a's part A,
   the helper by index on both tiers), `renyi_stage2a` (1ad4399, stage
   2a's commit), `renyi_stage2b_plain` (stage 2b's part A, the pair at
   the call alone), `renyi_stage2b` (e87f3e1, stage 2b's commit),
   `renyi_stage2c_v1` (stage 2c's first build, the release out of line
   for every value, rejected), `renyi_stage2c` (f48282f, stage 2c's
   commit), `renyi_stage3a` (9828e2e, stage 3a's commit) and
   `renyi_stage3b` (1b25b85, stage 3b's commit), `renyi_stage4a`
   (stage 4a's first build, the field-less types alone) and
   `renyi_stage4a_v2` (cf06592, stage 4a's commit) and
   `renyi_stage4b` (8eed964, stage 4b's commit), `renyi_stage4c`
   (805ea3a, stage 4c's commit), `renyi_cuts` (c7b8f6c, the
   checker's own cuts' commit), `renyi_5a` (16a0dd6, stage 5a's
   commit), `renyi_5b` (0f17f67, stage 5b's commit) and
   `renyi_b2` (2758464, stage B2's commit), `renyi_b3` (stage B3 as
   built, left off) and `renyi_b2fix` (stage B2 with the owned analysis
   fixed, AW23's commit), `renyi_b1` (stage B1's commit) and `renyi_b4`
   (stage B4's commit, the base of the next measure),
   each with its
   `study/` directory. The
   road is months long (AU58); the gate is decided on the measure at
   the end.
2. Each stage is measured by AT1's rule: `tools/measure_size.sh` on the
   release binary of the stage against the binary before it (keep a
   copy of that binary before changing the code; the rows are
   deterministic), the four benchmarks by `tools/bench.py` beside it; a
   stage stays when the self-check's estimate on the cold JIT run falls,
   or a benchmark gains more than 5% while the self-check loses at most
   1% (AU1). The decision entry of each stage records the numbers. Since
   AU19 the default JIT row of `tools/measure_size.sh` varies from run
   to run with the compile thread's timing under cachegrind (12.9 to
   16.7 billion instructions for one binary): take the synchronous row
   (`RENYI_NATIVE_SYNC=1`, the path of one hardware thread) for the
   estimate, and wall-clock on one, two and four hardware threads
   (`taskset`) beside it, both binaries alternating on one boot. The
   CPU features the container reports change between its boots, so an
   image built on one boot is refused on the next: build the image on
   the boot that measures it. The release build is one codegen unit
   with fat LTO: a caller added to a function LLVM inlines into the
   interpreter's loop or a helper can change that code (AU30); after a
   change outside the generated code, compare `nm -C -S` of
   `Vm::run_frames`, `rt_construct` and the like with the binary before,
   and prefer calling what is already out of line. The estimate itself
   moves with the layout alone, by up to half a percent on a row and
   once 1.1% on the template row whose code had not changed (AU32):
   read the instructions beside it, and on two or more hardware threads
   take the time after `main` apart from the run part (the last line of
   `RENYI_NATIVE_REPORT` against the whole), which is how AU32 found the
   compile thread draining its queue at the end of a run. Since AU41
   the self-check of every measure is `bench/selfcheck/checker.ry` on
   `bench/selfcheck/bodies.ry`, a frozen copy of the compiler, so that
   the work on `compiler/` (B1, B2) does not move the VM's measure.
   AU42 is the clearest case of the layout effect so far: its estimate
   rose 1.25% on the synchronous row, all of it simulated misses of
   the instruction cache, while the instructions fell on every row and
   the self-check's wall-clock fell 1.9% to 3.5%; the entry records the
   departure from the letter of AU1's rule. For a front end stage,
   `tools/front_end_compare.py <A> <B>` times the front end's commands
   and counts their instructions, and `tools/ratio.py <stage>
   <reference>` takes the ratio AU36 (iii) asks every entry to record
   (the reference is AU34's binary, built from commit 44782aa). Both
   run with the caches off, as `tools/bench.py` and
   `tools/measure_size.sh` do, but for the latter's `cached run` row,
   which since AU44 hits both caches; a change to the caches is
   measured with them on, a hit and a miss against the cache off, as
   AU44's entry does.
3. The gates before a commit that touches `crates/` are CLAUDE.md's,
   with the owner's toolchain: `cargo +1.94.1 fmt --check`, `cargo
   +1.94.1 clippy --all-targets -- -D warnings`, `cargo +1.94.1 test`
   (the judges of `selfhost.rs` among them, about 90 s), and
   `tools/conformance.py` against the debug binary. After a change to
   the template tier, also the conformance suite and the judges with
   `RENYI_NATIVE_TIER=template`. The image cache is off for everything
   cargo runs (`.cargo/config.toml`); a test of the cache turns it on
   with its own directory. CI's Windows job runs `cargo test -p
   renyi_vm` on every push (AU20); look at it after a push that touches
   the template tier. Stop a gate by its process ids: `pkill -f` on a
   pattern its children's command lines carry kills the judges' runs
   (and the shell that typed it), and the judges then fail with an
   exit status of none, as one gate of session 12 did.
4. In the cloud container the disk is a fixed allowance: a target
   directory grows to about 9 GB, so keep at most two (the repository's
   and one for the 1.94.1 gates) and delete the rest.
5. Two checks that need a detour in the container. The cheat
   sheet's gate (`tools/count_tokens.py`, CI) counts with tiktoken's
   o200k_base and cl100k_base, which the proxy keeps tiktoken from
   fetching, and the byte estimate the script falls back to is not the
   gate (AU56: 2,999 by the estimate was 3,001, and CI was red). Both
   vocabularies ship in the `tiktoken-rs` crate on crates.io, which is
   reachable: download
   `https://static.crates.io/crates/tiktoken-rs/tiktoken-rs-0.6.0.crate`
   into an empty directory of the scratchpad, extract
   `tiktoken-rs-0.6.0/assets/o200k_base.tiktoken` and
   `cl100k_base.tiktoken`, check their SHA-256 against tiktoken's own
   (`446a9538cb6c348e3516120d7c08b09f57c36495e2acfffe59a5bf8b0cfb1a2d`
   and
   `223921b76ee99bde995b7ff738513eef100fb51d18c93597a113bcffe865b2a7`),
   copy each into a cache directory under the SHA-1 of its URL
   (`https://openaipublic.blob.core.windows.net/encodings/<name>.tiktoken`),
   and run `TIKTOKEN_CACHE_DIR=<that directory> python3
   tools/count_tokens.py`. After AU57 the sheet stands at 2,997 and
   2,992 of 3,000: a library addition needs a trim beside it. And
   valgrind hides AVX-512, so an image built outside valgrind is
   refused inside it: build the image to measure under `valgrind
   --tool=none`, as `tools/measure_size.sh` does.
   The brand's PNG files (decision AV1) take a headless Chromium:
   `python tools/brand.py --png
   /opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell`;
   a new signature takes the font from npm and `uharfbuzz` with
   `fontTools`, as the session record of AV1 says.

## Where the project stands

Milestones M0 (design), M1 (front end), M2 (type and effect checker) and
M3 (the VM: `renyi run`, `record`, `run --replay`, `reproduce`,
`--explain`, `test` with `replays`, budgets, the grant stack of Q1, the
run manifest of Q2, every library module, the `only to` guards of P3,
tasks one after the other by S2) are done, with the project map (`renyi
index`, `--budgets`, `--diff`), `renyi tools` and `renyi mcp` on top. M4
is done but for its residue (packages AC1, `std.process` AE1, the FFI
AF1; `docs/GAPS.md`, section 4); M5 is done: the registration API of
decisions AJ1 and AK1 to AK4 (the section "The registration API as it
exists" below), the Python bridge of decisions AJ2, AJ3 and AL1 to AL4
(the section "The Python bridge as it exists" below) with its binder
of AM1 and AM2 (the section "The Python binder as it exists" below),
and the four steps in the owner's order: the resident world of
decision AN1 (the section "The resident world as it exists" below),
the language server of decisions AN2 and AN3 (the section "The
language server as it exists" below), the watch of `serve` of decision
AO1 (the section "The watch of `serve` as it exists" below) and the
embedding API of decisions AP1 and AP2 (the section "The embedding API
as it exists" below); of
M6 the
machine code exists (decisions AG1 to AG5), the interpreter had its
bounded round (AG6) and the strings and the JSON path had the
profile-guided round of decision AQ (the section "The profile-guided
round on strings and JSON as it exists" below), the baseline JIT of
decision AR1 is in (the section "The baseline JIT" below: the fused op
`LoadField`, the direct calls with register arguments and the pinned
layout, decisions AR2 to AR4, the tiering AR5) and `renyi build`
writes the image a run loads in place of compiling and, with `--exe`,
the self-contained executable (decisions AS1 to AS4).
Release 0.1.0 is out
(2026-10-07, the section "Release 0.1 engineering" below): the
repository is public at `github.com/renyi-lang/renyi`, the release
page carries the three archives, their checksums and the `.vsix`,
the seven crates of 0.1.0 are on crates.io (`cargo install renyi` builds
0.1.0), the installers were run against the release, and the site is
at `renyi-lang.org` (the GitHub Pages address redirects there).
Design decisions are
in sections 0 to AU of `01-decisions.md`; the positioning in
`08-positioning.md`; the agent tooling in
`05-agent-tooling.md`, the signature capabilities in
`06-runtime-guarantees.md`, the system-level commitments in
`07-system-design.md`; the open items in section 18 of the sketch (R3-1,
`Iterable`) and section 6 of `docs/GAPS.md`.

The owner's direction since 2026-10-06 (decision V1): the readability
scores are no longer the gate; the aim is a self-hosted, independent
language that people use; the grammar freezes by decision, not by
measurement; the front end is written in Renyi first with the Rust VM as
the runtime; the repository stays private until the owner says
otherwise; the first users are people building agent workflows and
learners. Before any of that the design was audited against the
implementation (`docs/GAPS.md`, session 6) and the gaps filled (stage 1
of its section 7, session 7). The grammar is frozen by decision V11 at
commit `dc58bb3`: a change to the surface is a new decision entry first,
then the reference, the grammar, the cheat sheet, the formatter and the
conformance suite in one commit. Since session 8 the normative text is
`docs/reference.md` with `docs/grammar.ebnf` (`crates/renyi_syntax/tests/grammar.rs`
and `tests/reference.rs` hold them to the parser and to each other); the
sketch is the design record.

Stage 2 of `docs/GAPS.md` section 7 is under way in the order the owner
set (W4, W5): the lexer, the parser and the checker written in Renyi
exist under `compiler/` (the next two sections) and the VM is profiled
and its loop rewritten (the section after them, decisions X1 to X6);
the residue of stage 1 is done (decisions Y1 to Y4, the section after
the profile's); the bytecode file exists with `renyi compile` and the
loader (decisions Z1 to Z4, the section after the residue's); the
statement `set name to value` is `change name to value` since decision
AA1 (the owner's question of 2026-10-06; the section after the bytecode
file's); the emitter written in Renyi, which writes that file, exists
and is held equal to `renyi compile` byte for byte (decision Z3, the
section after the rename's): the toolchain in Renyi is the lexer, the
parser, the checker and the emitter, with the Rust toolchain as stage 0.
Stage 3 began on 2026-10-07 with packages (decision AC1, the section
"Packages" below).

The corpus has 30 programs, passes the lint, is in canonical layout,
checks cleanly, has nothing over budget, and its `example:` lines and
`test` blocks pass on the VM (six are `replays` tests answered from
recordings under `examples/fixtures/`). The cheat sheet measures
2999 of 3000 tokens. The Rust workspace has eight crates:
`renyi_json`, `renyi_syntax`, `renyi_package`, `renyi_check`,
`renyi_index`, `renyi_workspace`, `renyi_vm` and the `renyi`
binary with `check`, `format`, `tokens`, `parse [--json]
[--declarations]`, `index [--json | --budgets | --diff <map or
revision>]`, `run [--manifest] [options] <file> [arguments]`, `record
[--to file] [options] <file> [arguments]`, `reproduce <recording>
[<file>]`, `test [--strict] [--refresh name [--redact name]] [--explain]
<file>...`, `compile [--to file] <file.ry>`, `add <name> [<version>]`,
`update [--accept-effects]`, `audit`, `fetch`, `publish [--to
<directory>]`, `bind <header.h> --module <name> --library <names>
[--to <directory>]`, `tools [path]`, `mcp [path]`, `lsp`, `serve [--watch]`, `run --sandbox` and `version`; 372 tests,
clippy and fmt clean with rustc 1.94.1 on Windows (the owner's machine)
and on Linux (the cloud environment, where 1.94.1 is installed beside
its 1.97.0 for the gates). CI (`.github/workflows/ci.yml`) runs the same gates,
`renyi check compiler/*.ry`, the starter pack's workflows (check,
format, test, the cheat sheet's copy) and the conformance suite
(`tests/conformance/`, 55 cases, every `run` case a second time from
its bytecode file;
runners `tools/conformance.py` and `crates/renyi/tests/conformance.rs`)
on a toolchain pinned to the owner's machine (rustc 1.94.1), so that CI
and the local gates agree on clippy's lints; `gh run list --limit 3`
shows the runs and `gh run view <id> --log-failed` a failure's log. The
VM depends on `ureq` (HTTP, with rustls), `rusqlite` (SQLite compiled
in), decision S1, and on `sha2`; the binary allocates through
`mimalloc` (decision X6).

Readability: four live rounds exist under `tests/readability/`; the last
(round 4, `2026-10-06-c696747/`, Sonnet 5.5 through the Claude Code CLI)
stands at Predict 90, Explain 100, Complete 79, Write 40. The rounds'
`notes.md` and "Done in session 6" below hold the detail; decisions R1 to
R8 and U1 to U9 came from them; they continue only if the owner asks
(decision V1). The rounds' records (`system.txt`, the outputs) keep the
grammar they measured, with `set`; a new round reads the current cheat
sheet, with `change` (decision AA1).

## The self-hosted front end as it exists (`compiler/`)

- **The files** (decision W2). `ast.ry` declares the syntax tree, one
  type per construct, each `can ToJson` (records for nodes with one
  shape, sums for the alternatives: `Item`, `Type`, `TypeKind`,
  `Statement`, `Pattern`, `Expr`, `Outcome`, `QueryTerminal`, ...; every
  node's `span` is its last field). `lexer.ry` turns source text into
  tokens by the rules of reference section 1: `TokenKind` is `Word`
  (reserved words and phrases, one token each), `Identifier`,
  `TypeIdentifier`, `MemberName`, `IntegerToken`, `DecimalToken`,
  `TextToken(parts, is_block)` with holes lexed by a sub-lexer over the
  same character list, `RawToken`, `ClauseToken`, `CommentToken`,
  `LineBreak`, `Symbol(text)`, `ErrorToken` and `EndOfFile`; it reports
  every bad character with the Rust lexer's code, message and fix and
  reads on (`Lexed`: the tokens and the diagnostics; decision AD1).
  `parser.ry` turns tokens into the tree
  rule for rule as `crates/renyi_syntax/src/parser.rs` does; the Rust
  parser's mutable cursor is a `Cursor` record (tokens, position, open
  brackets, whether functions have bodies) threaded through every
  function, each returning `Parsed of Node` (the node and the cursor
  after it); `peek` moves past the line breaks the layout rules make
  insignificant (inside brackets, after a comma, before a continuation
  word) exactly as the Rust `peek` does, including the spans that end at
  a line break after such a move, so that the two trees agree byte for
  byte; since decision AD1 it reports every syntax error with the Rust
  parser's code, message, span and fix, recovers where the Rust parser
  recovers and returns the partial tree with the diagnostics
  (`ParsedModule`; the section "The diagnostics of the front end in
  Renyi" below). `parse.ry` is the command line: `renyi run
  compiler/parse.ry [--declarations] <file>` lexes, parses and prints
  `json.render_indented` of the tree, then the diagnostics as `renyi
  parse --json` prints them, and exits 1 on an error; `tokens.ry`
  prints the tokens as `renyi tokens` does, then the lexer's
  diagnostics (a development aid for comparing the lexers line by
  line).
- **The JSON** (decision W1) is the derived JSON of the `ast.ry` types:
  a record is an object keyed by its fields in declaration order, a
  variant an object with `kind` first, a `maybe` without a value `null`,
  a list an array, a span `{"start", "stop"}` in character offsets. The
  Rust encoder `crates/renyi_syntax/src/json.rs` prints the same
  document (its byte offsets converted through a table); the names in
  `ast.ry` keep it unambiguous (no two sums in scope share a variant
  name: `IsValue` beside the operator `Is`, the lexer's `MemberName`
  beside the expression `Member`; `exposed` for the reserved `exposing`).
- **The judge** (decision W3): `crates/renyi/tests/selfhost.rs` runs
  `renyi run compiler/parse.ry` over `examples/`,
  `tests/conformance/programs/`, `compiler/` and, with `--declarations`,
  `library/std/` on a few threads and compares with `renyi parse --json
  [--declarations]` byte for byte, the exit status and the diagnostics
  of a rejected program included since decision AD1; a second test does
  the same for the checker (next section);
  a third checks `renyi format --check compiler/*.ry`. CI runs `renyi
  check compiler/*.ry` besides. Every judge runs its driver from the
  bytecode `renyi compile` writes of it when the test starts (the
  section "The judges run from bytecode" below). Every compiler source
  is `renyi check` clean (no warnings) and in canonical layout.
- **Speed, the first measurement** (W4): on the VM the lexer and the
  parser take about 0.3 s on `examples/hello.ry`, 0.6 s on the prelude
  declarations, 1.0 s on the 750-line `lexer.ry` and 3.9 s on the
  2700-line `parser.ry` (`renyi check` of the compiler alone, which
  every run pays first, is 0.25 s). The lexer slices token text by
  characters in a loop, since `Text.drop` and `take` copy the whole text
  per call (79 s for `parser.ry` before that change). Growing a list
  with `change xs to xs.append(x)` or `xs.append_all(step.xs)` in a loop is
  linear (80 000 appends in 0.26 s, startup included: decision O1's
  `LoadMove` moves the receiver out of its slot and `take_list` in
  `value.rs` reuses the vector), so the list growth is not where the
  time goes; where it does go has not been profiled. That profile is
  the first performance item, now that the checker is written (W6).
- **Writing Renyi at this size, what bit**: field and binding names
  cannot be reserved words (`exposing`, `first`, `least`, `raw`,
  `module`, `within`, `end` ...); a text literal cannot appear inside a
  hole (bind it first); `"{"` is an unterminated hole (`"\{"`); `\r` is
  not an escape (the lexer tests `ch.trim() is ""` instead); a variant
  of a sum type cannot share a name with a variant of another sum in
  scope, and a field common to every variant cannot be read on the sum
  value (hence `expr_span`, `type_span`, `outcome_span` in the parser);
  a `failure(x)` pattern needs a type (`failure(error: Fault)`) before
  its fields can be read; a call that can fail must be handled on its
  own line (`otherwise fail`, or `match call()` directly); generic
  records (`Parsed of Node`) and function-typed parameters with `for
  any` work, so `bracketed` and `binary_chain` are shared across the
  grammar.

## The self-hosted checker as it exists (`compiler/`)

- **The files** (decision W5), each a transcription of one Rust file,
  every message and fix verbatim. `lists.ry` (`replace_at`).
  `report.ry` (`crates/renyi_syntax/src/diagnostics.rs`): `Diagnostic`
  (`is_error`, `code`, `message`, `span`, `fix: maybe Text`),
  `line_starts` and `position` over a character list, `json_string`,
  `render_json` and `render_text` as `renyi check` prints them,
  `sorted_by_start` (stable, as Rust's `sort_by_key`). `effects.ry`
  (`effects.rs`: `Grant`, capability parsing, scopes, `names_of`).
  `suggest.ry` (the closest name in scope, the Renyi spelling of a
  foreign name, `quoted`). `types.ry` (`types.rs`: `Ty`,
  `FunctionSignature`, substitution, `head_type`). `refine.ry`
  (refinements evaluated on literals, `Literal`, `Verdict`).
  `declare.ry` (3000 lines, `world.rs`): `World` with its modules,
  types, functions, abilities, implementations, params and `Builtins`;
  `add_module` registers names, `resolve_all` resolves imports, types,
  abilities, functions, implementations and constants in the Rust order,
  then the lookups, the suggestions, `resolve_type`, `declare_function`,
  `check_capabilities`, the method-signature check and `conforms`; the
  diagnostics are kept per module (`module_diagnostics`). `bodies.ry`
  (7550 lines, `check.rs`): the Rust `Checker` struct is a `Checker`
  record (world, module, diagnostics, references, variables, scopes,
  context, deferred checks, loop and task depth, literals and results
  noted for the VM) threaded through every function, each returning the
  record or `Done of Value` (a value and the record); `check_function`,
  `check_test`, `check_type_conditions`, `check_constant` and
  `check_module` are public, the rest mirrors the Rust methods one for
  one (inference, assignment and unification, abilities, scopes,
  statements, calls and overloads, constructions, patterns and
  exhaustiveness with witnesses, queries, the recorded references of
  W8). `checker.ry` is the command line: `renyi run compiler/checker.ry
  [--json] [--strict] [--library <dir>] <file>...` reads the fifteen
  library declaration files from `library/std` under the working
  directory (`--library` names another), lexes and parses with the
  Renyi front end, reads the imports from the file's directory as
  `renyi_check::imported_files` does (a stack, the last import first;
  `.ry` then `.renyi`; `std` skipped), declares, resolves, checks the
  bodies, adds `module-name` (G3) and `import-errors`, the layout
  warnings of `layout.rs` (`line-width`, `trailing-whitespace`), sorts
  by start, turns `deprecated` into an error under `--strict`, prints
  the JSON or the text, and exits 1 on any error. A file that does not
  lex or parse gets the front end's diagnostics, the Rust codes and
  fixes (decision AD1), the layout warnings and exit 1; the imports of
  its partial tree are followed as the Rust resolver follows them.
- **The judge** (decision W7): the test
  `the_renyi_checker_prints_what_the_rust_checker_prints` in
  `crates/renyi/tests/selfhost.rs` runs `renyi parse` to learn whether
  the Rust parser accepts a program, then `renyi run compiler/checker.ry
  --json [--strict]` against `renyi check --json [--strict]` on every
  program of `examples/`, `tests/conformance/programs/` and `compiler/`
  (output byte for byte and exit status, on a program with syntax
  errors too since decision AD1). All equal at the
  first full comparison after the compiler's own sources were made
  clean; the run takes about 51 s on eight threads after X3 (64 s with
  X1 alone, 170 s before X1, when a small program took 1.5 s and
  `bodies.ry` 32 s). The lane
  `D:\Projects\.worktrees\Renyi\selfhost\compare.py` runs the same
  comparison outside `cargo test` and names the first differing line;
  `bench.py` there times the parser's and the checker's runs, best of
  five, for any binary (`RENYI_ROOT` and the binary path as arguments).
- **The references** (decision W8) are recorded as `check.rs` records
  them (`Reference(body, target, span)`, `Target`, `NumberKind`, the
  `literals` and `results` lists) and returned by `check_module` in
  `Checked`; nothing reads them yet, and nothing judges them until the
  emitter exists.
- **Where the two checkers could differ, by construction** (none shows
  on a program in the repository): the Rust checker iterates `HashMap`s
  in `suggest_*`, the imports and the method index, so a tie between
  two equally close names may be broken differently; the `Path`
  literal check tests only `"\n"` (a
  NUL cannot be written in Renyi); `debug_quoted` escapes `"`, `\`,
  `\n` and `\t` where Rust's `{:?}` escapes every control character;
  the `import-errors` message joins the import's path with `/` where
  Rust uses the platform separator (no program in the repository
  imports a module with errors). Positions are characters on the Renyi
  side and bytes on the Rust side, printed as line and column in both,
  so they agree on any text.
- **Writing the checker in Renyi, what bit** (besides the parser's
  list above): a qualified type name (`ast.Branch`) cannot stand in a
  type position, so every type used is in an `exposing` list (a type
  brings its variants; the first name must follow `exposing` on the
  same line); a `with` must stay on one line, cannot nest, and swallows
  any `name: value` pairs that follow it in a call (compute the updated
  record into a local first); a `match` expression's arm holds one
  expression or an outcome (`return`, `crash with`), never a statement,
  and a `let x: maybe T be match ... end` needs the annotation when an
  arm is `nothing`; a one-line `if c then change a to x otherwise change a to
  y end` statement does not parse (write it on several lines); a call
  with one argument must not name it; an unused loop or pattern binding
  is an error, so `for each x in xs collect Constant` needs a helper; a
  local cannot share a function's name; `count`, `needs`, `all`, `any`,
  `first`, `from`, `to`, `by`, `within`, `at`, `test`, `example`,
  `failure`, `success`, `ability`, `only`, `run`, `import` and the other
  reserved words cannot name a binding or a parameter; a `failure(x)`
  binding's fields are unreadable (destructure: `when
  failure(UsageError(message))`); the formatter puts every argument of
  a wrapped call on its own line, so a function near the 60-line limit
  before formatting must be split; `renyi format` must run before
  anchors for a patch script are taken from the file; `ignore` of a
  call that has no effect is an error even when the call can fail (bind
  the result, or make the function return nothing and call it as a
  statement with `otherwise fail`); a call with one argument must not
  name it.

## The profile and the loop of the VM (W6; decisions X1 to X4)

- **Build modes.** Every number reported before this profile came from
  the development build (`target/debug`, which `cargo test`, CI and
  `cargo run` use). The release build ran the parser on `parser.ry` in
  1.0 s instead of 5 s and the checker on `bodies.ry` in 5.5 s instead
  of 32 s. Decision X1: the root `Cargo.toml` sets `opt-level = 3` for
  `renyi_vm` and for every dependency in the development profile; a
  development build now runs the interpreter at release speed (the
  parser 1.07 s, the checker on `bodies.ry` 4.2 s with the sources of
  `f770f79`) while `renyi_syntax`, `renyi_check` and the binary keep
  their fast rebuilds. The first build after the change recompiles
  every dependency (a few minutes).
- **What the profile found** (release build, the checker on
  `bodies.ry`, 148 million ops, 5.5 million calls, 3.9 million primitive
  calls, 6.7 s before any fix): a tenth of the time in `line_of` of
  `bodies.ry` (a linear scan of the line starts, called twice per
  function body); a tenth in the lexer's `slice` (`chars.at(index)` and
  a concatenation per character); `line_starts` and `check_layout` of
  the driver 6 percent together (per-character loops, the characters
  computed three times per file); `find_function` 2 percent (a linear
  scan of the world's functions per item); the primitives `at`,
  `contains`, `length` and `append` 15 percent together (half of all
  primitive calls were `at`, from `char_at`); the rest spread over the
  VM's plain ops at about 35 ns each: `Load` 14 percent, `Field` 10 (a
  linear search of the record's field names on every access), `Binary`
  10 (`equal` looked a declared `equals` up by `(TypeId, String)` with
  an allocation per comparison of a record or variant), `Call` and
  `Return` 16 (arguments popped into a `Vec` and pushed back; a `Vec` of
  handlers per frame), `Construct` and `ConstructVariant` 8. Sizes
  then: `Value` 40 bytes, `Interrupt` 48, `Result<Option<Value>,
  Interrupt>` 48 (returned by every `step`), `Op` 16.
- **What `f770f79` fixed** (sources and the library, no VM change):
  `line_of` is a binary search; the lexer's predicates test `ch is not
  ""` instead of calling `length`; `check_module` maps each item to its
  function once (`body_index`); the driver converts a file to
  characters and line starts once (`Source.chars`, `Source.starts`;
  `set_source_lines` takes the starts); the lexer's `slice` is one
  `List.slice` and one `join` (X2). Release build on `bodies.ry`: 5.5 s
  to 3.3 s (93 million ops); the parser on `parser.ry` 1.0 s to 0.76 s.
- **The loop** (decision X3, done in the same session; `vm.rs`,
  `run_frames`). The running frame's code, pc and base live in locals of
  one loop that returns nothing per op; a call to a declared function
  writes the caller's pc back, pushes the callee's frame over the
  arguments where they lie on the stack (`push_frame_in_place`) and
  reloads the locals; a return, or a handler taking a failure, reloads
  them too (`Flow::Reload`); every interrupt writes the pc back first
  (`try_op!`, `crash!`), which is what locates a crash. One handler
  stack serves every frame (`Vm::handlers`, `Frame::handler_base`,
  truncated on return and when `execute` abandons the entry frame).
  `Op::Field { name, site }` remembers per site the type, tag and index
  it found last (`Vm::field_cache`, `Program::field_sites`). A
  primitive takes its arguments as `&mut [Value]`, a slice of a buffer
  the VM reuses (`Vm::scratch`; `natives::take` moves one out for the
  seven primitives that build on a collection in place), so a call
  allocates nothing. `Value` is 24 bytes (`Decimal` boxed, a compile-time
  assertion in `value.rs`). The declared `equals`, `compare`, `to_text`
  and `to_list` of every type are found once at compile time
  (`Program::specials`; `Program::method` is gone) and every call goes
  through `Program::function_codes`, a table. Two small Integers, two
  texts or two Booleans under an operator take a fast path
  (`small_binary`, `text_binary`, `boolean_binary`), the jumps pop their
  Boolean inline, a loop over a list walks the list itself (no copy),
  and a type without refinements constructs without the loop over its
  fields. `Interrupt` is unchanged (48 bytes; only the error path moves
  it).
- **The profiler** (decision X4, `profile.rs`): `renyi run --profile`
  (also `record`; `test` refuses it). A timer thread raises an atomic
  flag every half millisecond (the sleep is coarser on Windows: about
  one sample per 1.1 ms); the loop takes the flag down before an op and
  gives the sample to the op that ran last; a primitive takes it down
  when it returns, so its time lands on the primitive; every op is
  counted by `Op::kind` (a small number and a name, `Op::KINDS` of
  them), every call by code object, every primitive by name. The report
  goes to stderr when the run ends (`Vm::report_profile`, called by the
  runner): the totals, then samples by function and op, by function, by
  op kind with the primitives by name, then the counts: by kind, by pair
  of kinds run one after the other in one code object (session 9, for the
  fused ops of decision AR1), by function, the primitives (the top twenty
  rows of each). `tests/semantics.rs` has a case. The scratch worktree
  `D:\Projects\.worktrees\Renyi\profile` (detached at `246fca4`) holds
  the prototype and the micro-benchmarks `loop_count.ry`,
  `loop_calls.ry`, `loop_natives.ry` with `micro.py` (best of five, ops
  from `--profile`, ns per op); `D:\Projects\.worktrees\Renyi\base`
  is a worktree at `f770f79` with a release build, the baseline the
  numbers below were measured against.
- **Measured** (release build, best of five, the two binaries run back
  to back on the same machine; timings vary by 10 to 40 percent between
  runs, so only such pairs compare): the parser on `parser.ry` 963 to
  619 ms, the checker on `lexer.ry` 686 to 433 ms, on `bodies.ry` 3650
  to 2339 ms (1.56 times each; the op counts are unchanged, 17.3 and
  93.0 million); the counting loop 1475 to 794 ms (8.8 ns per op), the
  calling loop 629 to 282 ms, the loop of primitive calls 868 to 422 ms.
  The optimized development build (what `cargo test` runs) does the
  three runs in 718, 661 and 2717 ms; the gap to the release build is
  the unoptimized Rust front end that checks the program first, not the
  VM. `cargo test` takes 1 m 45 s, the two judges 51 s of it (64 s
  before X3). The target of X3 (1.5 to 2 times) is met at its low end
  on the real programs and exceeded on the micro-benchmarks.
- **What the profile says now** (the checker on `bodies.ry`, 2.2 s
  under the profiler): `Call` 13 percent, `Load` 11, `Field` 10,
  `Return` 10, `Binary` 6, the primitive `at` 6, `Construct` 5, `Store`
  4, `Const` 3, `ConstructVariant` 3, `IterNext` 3, `contains` 3; the
  parser on `parser.ry`: `Call` 13, `Load` 12, `Construct` 9, `Field`
  9, `ConstructVariant` 6, `Binary` 6, `Return` 6, `at` 6, `contains`
  6. The time is spread over the plain ops at about 20 ns each; a
  call and its return cost about 70 ns (the frame, the locals filled
  and dropped, the grant), a record two allocations (the `Rc` and its
  field vector), and every allocation goes to the system allocator,
  which is slow on Windows. Decision X6, asked and measured in the same
  session: `mimalloc` is the global allocator of the `renyi` binary
  (`crates/renyi/src/main.rs`, `#[global_allocator]`; `cargo add
  mimalloc -p renyi`); release build, best of five, back to back
  against the same commit without it: the parser on `parser.ry` 811 to
  496 ms, the checker on `lexer.ry` 510 to 389 ms, on `bodies.ry` 2712
  to 2138 ms; the micro-benchmarks, which allocate little, within
  noise. Against `f770f79` the three runs are now about 1.9, 1.8 and
  1.7 times faster. After this, speed comes from the emitter and what
  it can precompute, not from the loop.

## The residue of stage 1 (decisions Y1 to Y4)

The owner's order after X6: finish what stage 1 left open before the
emitter. Asked as one batch of four, answered with the recommended
options, done in one commit.

- **Y1, the boundary's denials.** `serve` outside the grant fails with
  `StartError.PermissionDenied(port)`, past a budget with the new
  `StartError.OverBudget(port)` (`library/std/server.ry`, the sketch;
  `vm.rs`, `denied` and `over_budget` take the call's arguments to name
  the port; `tests/network.rs`,
  `the_server_reports_a_denial_and_a_budget_through_start_error`). A
  primitive that cannot fail (`filesystem.exists`, `environment.get`)
  crashes by rule, naming the function whose `needs` narrowed the grant;
  containment of a path scope is lexical by rule (nothing resolved on
  disk); both in the reference, section 11, and in
  `06-runtime-guarantees.md`.
- **Y2, the recording's grant header.** `Vm::replay_with` checks every
  capability of the header against the effective grant before it checks
  the calls, refusing with "the recording's grant names X, which the
  grant G does not cover" (`recording.rs`,
  `a_run_is_recorded_and_replayed`: the header first, a trimmed header
  then the call, a widened header refused although no call uses it). The
  six fixtures fit their tests' `needs`.
- **Y3, documentation only.** Collections compare and hash by structure
  and consult no declared `equals` or `hash` (reference, section 5);
  `repeat`, `pad_left`, `pad_right` and `rounded` crash on an `Integer`
  past a machine word (section 7).
- **Y4 (i), the loop's mark.** `Op::MarkStack(slot)` at the entry of
  every statement loop stores the operand stack's height in a temp;
  `Op::UnwindStack(slot)` before the jump of every `break` and
  `continue` truncates to it (`bytecode.rs`, kinds 49 and 50, `Check`
  now 51, `KINDS` 52; `compile/mod.rs`, `LoopContext.mark`,
  `enter_loop(span)` emits the mark and is called before the position a
  `continue` returns to, `leave_regions_of_loop` emits the unwind;
  `compile/stmt.rs`, the three loops; `vm.rs`, the two arms). Tested by
  `tests/semantics.rs`,
  `break_and_continue_inside_an_expression_drop_its_operands`, which
  counts the ops of `main` and checks the output.
- **Y4 (ii), `refinement-field`.** A field's condition binds that field
  alone and the checker carries `ConditionScope { type_name, field,
  others }` while it checks it; a bare name that is another field of the
  type is `refinement-field` ("the condition of `high` reads `low`,
  another field of `Interval`"; fix "a refinement sees only its own
  field; check both where the value is built"), in `check.rs`
  (`check_type_conditions`, `check_condition_body`, `infer_name`) and
  mirrored in `compiler/bodies.ry` (`check_field_conditions`,
  `check_condition_body`, `refinement_field_error`); the reference's
  section 4 and appendix A; `rules.rs`,
  `refinement_conditions_are_checked_as_bodies`; conformance case 38,
  `refinement_field.ry`. The VM's condition code still takes every field.
- **R3-2** (constraints with type arguments) is done: decision AB1, the
  section "Constraints with type arguments" below.

## The bytecode file (decisions Z1 to Z4)

The owner's batch after the residue, answered with the recommended
options: JSON, a hand-written loader, byte equality plus reruns as the
judge, `renyi compile` with loading by extension. The file side is done;
the emitter written in Renyi is next.

- **The format** is `compiler/bytecode.ry`: the VM's `Program` as Renyi
  types, each `can ToJson` (`Program`, `ModuleSource`, `BuiltinIds`,
  `TypeMeta`, `TypeShape`, `FieldMeta`, `VariantMeta`, `ImplEntry`,
  `MethodBinding`, `AbilityMeta`, `MethodEntry`, `FunctionMeta`,
  `ConstantMeta`, `TestMeta`, `ExampleMeta`, `Expected`, `Code`,
  `CodeKind`, `CodeConstant`, `Specials`, `GroupFold`, `Op` with its 52
  variants in the order of `bytecode.rs`, `OpConst` to `OpCheck`);
  `Ty`, `FunctionSignature` (`compiler/types.ry`) and `Grant`,
  `BudgetSpelling`, `SinkPath` (`compiler/effects.ry`) derive `ToJson`
  since this entry so that the format can hold them. A `.ryc` file is
  the derived JSON, `format` first (1). Names that are reserved words
  in Renyi are renamed in the file: `function_id`, `owner` (a module
  id) or `module_name` (its text), `result`, `failures`,
  `capabilities`, `fixture`, `summary`, `set_type`, `ability_id`,
  `is_descending`, `size`. Number constants are their digits in a
  string: `Int` and `Decimal` through `Display`, a Float through
  `render::float_text`, so that `Float.to_text` in Renyi gives the same
  digits.
- **The Rust side** is `crates/renyi_vm/src/file.rs`: `render(&Program)
  -> String` builds a `renyi_syntax::json::Json` tree (the writer of
  `renyi parse --json`, held equal to `json.render_indented` by the W3
  judge) with `impls` sorted by (ability, type) and methods by name and
  `method_index` sorted by (type, name); `load(&str) -> Result<Program,
  String>` reads with `natives::json::read_json`, names the place of a
  misfit (`the file.codes[3].ops[7].target: expected a number, found a
  string`), refuses another `format`, and ends with a consistency pass
  over every index (codes, functions, constants, jumps against the
  code's length, slots against `locals`, constants against the code's
  table, types, field sites, result types). `is_bytecode(path)` tests
  the extension `ryc`.
- **Spans count characters** everywhere now (`compile/mod.rs`:
  `Context.characters` holds a byte-to-character table per module with
  a source, built as `json.rs`'s encoder builds it; `Compiler::emit`
  converts every span it stores, `TestMeta.span` and `ExampleMeta.span`
  are converted where they are built; `text_of` and `source_text` keep
  taking the tree's byte spans). `Program.sources` is `Vec<Option<
  SourceLines { name, line_starts }>>` by module id, the line starts in
  characters, `None` for a library module; `Program::source(module)`
  and `location(module, span)` (`SourceLines::line_of`, a binary
  search) serve crash locations and test reports, and `runner.rs`'s
  `fixture_path` finds a `replays` file beside `source.name`. The
  repository's sources are ASCII, so nothing moved; a program with a
  wide character before a crash now reports the right line
  (`tests/file.rs`).
- **The commands** (`crates/renyi/src/main.rs`): `renyi compile [--to
  <file.ryc>] <file.ry>` (`compile_command`: the diagnostics of
  `check`, nothing written on an error, the default target
  `<stem>.ryc` in the working directory, "renyi: compiled X to Y" on
  stderr; a `.ryc` given to it is refused). `compile_with_sources`
  returns `(Program, Hashed)` where `Hashed::Sources(files)` hashes
  `main` through the project map as before and `Hashed::File(text)`
  hashes the file's bytes (`recording::sha256_of`); `run`, `record`,
  `test` and `reproduce` go through it, so a `.ryc` path works in each
  (`load_bytecode` prefixes the path to the loader's message).
- **The judges so far**: `crates/renyi/tests/conformance.rs` runs every
  `run` case without a `diagnostics` key a second time from
  `target/conformance/case<N>.ryc` and compares the same things (the
  thirteenth `run` case is the guarded print, whose failure comes from
  the boundary at run time and so from the file too);
  `crates/renyi/tests/compile.rs` compiles `hello.ry`, runs it, checks
  the manifest's hash is the file's, records and reproduces from the
  file, checks the default target, runs `weather_client.ry`'s tests
  from a file (the fixture found beside the source), and the refusals;
  `crates/renyi_vm/tests/file.rs` renders a program with every kind of
  code object and constant, loads it, renders again (equal), runs both
  and their tests (equal), locates a crash after wide characters, and
  tries the misfits (format, broken JSON, a wrong type, an index out
  of range).
- **Size**: `hello.ryc` is about 170 KB, because every library module's
  functions, types and signatures are in it (the VM needs them for
  calls, rendering and JSON decoding); fine for now, and the first
  thing to cut if files ever matter.

## The rename of `set` to `change` (decision AA1)

The owner asked, while the bytecode file was being built, whether to
rename `set` to `change` and forbid every other way of changing a
binding. The second half was already the rule (no `=`; a name is bound
once per function, `shadowing`; `set` on a `let mutable` binding the
only mutation), so the question was the rename alone, asked back as one
structured question and answered "before the emitter". A surface change
under V11, one commit:

- **The grammar and the documents**: `docs/grammar.ebnf` (`'change'
  Identifier 'to'`), the reference (sections 1, 6, 8 and 17; 88 reserved
  words still, `set` out, `change` in), the cheat sheet (the same list,
  the examples; 2998 tokens), section AA of the decisions,
  `docs/GAPS.md`.
- **The Rust front end**: `Word::Change` in `token.rs`,
  `StmtKind::Change` in `ast.rs`, `parser.rs` (and its statement
  message), `json.rs` (the kind `Change`), `format.rs` (`change x to`);
  `check.rs` (the arm and the fixes `write `change x to
  x.method(...)``), `metrics.rs`, the VM's `compile/stmt.rs`.
- **The front end in Renyi**: `compiler/lexer.ry` (the reserved list),
  `ast.ry` (`Change(name, value, span)`), `parser.ry`
  (`change_statement`), `bodies.ry` (the arms, the purpose, the fixes),
  and every statement of the compiler's own sources (about 700).
- **The corpus and the tests**: the examples, the conformance programs
  (`body_length.ry` is sixty of them), the crates' tests, the lint's
  reserved words and `=` message. The sweep was one script
  (`sweep_set.py`, five patterns: a line start, after `\n` in a Rust
  string, after a backtick or quote, after `then` or `otherwise`, after a
  quote and spaces), then the keyword plumbing by hand; nothing else in
  the repository spells the statement.
- **Unchanged**: the `Set` type, `Map.set(key:, value:)`, `to_set`, the
  readability rounds' records, the sketch (`02-syntax-sketch.md`, the
  design record) and the earlier decisions, which keep `set` as history.

## The compiler written in Renyi (decision Z3)

The last piece of stage 2's toolchain: `compiler/emit.ry` turns the
checked tree into the `Program` of `compiler/bytecode.ry`, and
`compiler/compile.ry` writes it as `renyi compile` does.

- **`project.ry`** (new): what the command lines share, moved out of
  `checker.ry` unchanged: `Source` and `Front`, `source_of`, the front
  end (`front_end`), the library (`library_trees`), the imports
  (`imported_files`, `directory_of`, `join_path`), `read_file`,
  `print_block`, `has_error`, and `check_project`, which now returns a
  `Project` (the world and one `CheckedModule` per file: its module id,
  its path, its diagnostics and the references of its bodies).
  `checker.ry` keeps the options, the layout checks and `diagnose`.
- **`emit.ry`** (new, about 3500 lines): the transcription of
  `crates/renyi_vm/src/compile/{mod,expr,stmt,pattern,query}.rs` and
  `types.rs`, function for function. The Rust `Compiler` is the record
  `Emitter`, threaded through every function as `Checker` is in
  `bodies.ry` (`Step of Value` carries a value and the emitter); the
  shared `Context` (the world, the globals, the references by body, the
  result types, the field-site counter, and the tables being built:
  codes, function codes, constants, tests, examples, types) travels
  inside it and comes back out between bodies. The references are keyed
  by module and body (`"{owner}/{body key}"`), then by span
  (`"{start}:{stop}"`), in the order the checker recorded them, so
  "the first `Function` target at this span" means the same in both
  compilers. A code's constants are pooled as typed values (`Pooled`:
  an Integer, a Decimal, a Float, a text, a Boolean, nothing or a
  function id), compared with `is` and rendered to digits at the end,
  which reproduces the Rust pool's "an equal constant of the same kind
  is reused" exactly (`1.5` and `1.50` are one Decimal constant). Spans
  need no conversion: the Renyi parser's spans already count
  characters. A `match` of either kind goes through one `emit_match`
  over `GenericArm`s (`BlockBody` or `OutcomeBody`); `run concurrently`
  runs its tasks one after the other (S2), as the Rust compiler does.
  `bodies.ry` made `debug_quoted`, `statement_span`, `expr_span` and
  `pattern_span` public for it.
- **`compile.ry`** (new): `renyi run compiler/compile.ry [--to
  <file.ryc>] [--library <dir>] <file>` reads the file and its imports,
  checks the project, prints every module's diagnostics as `check`
  prints them, exits 1 when any is an error, and otherwise writes
  `json.render_indented(program)` with a trailing line break to the
  target (`<stem>.ryc` in the working directory by default, as `renyi
  compile` names it). A `.ryc` as input is refused with `renyi
  compile`'s message.
- **The judge** (`selfhost.rs`, `the_renyi_compiler_writes_what_renyi_
  compile_writes`): for every program of the corpus, the conformance
  programs and `compiler/`, named relative to the root with `/`, `renyi
  compile --to target/selfhost/rust/<name>.ryc` and `renyi run
  compiler/compile.ry --to target/selfhost/self/<name>.ryc`; when the
  Rust one succeeds the Renyi one must succeed and the two files must be
  equal byte for byte (the first differing line is reported); when the
  Rust one refuses, the Renyi one must refuse. The first full run agreed
  on every program: the 30 examples, the 29 conformance programs (9
  compiled, 20 refused by both) and the compiler's own 18 files, the
  7600-line `bodies.ry` among them.
- **One change on the Rust side.** `renyi_check::imported_files` now
  names an imported file as the importing file's directory and the
  import's segments joined with `/` on every platform (it joined with
  the platform's separator before, `\` on Windows), so that a bytecode
  file, which remembers the paths of its modules, is the same file
  wherever it is written; the Renyi `imported_files` always joined with
  `/`.
- **Time.** The Renyi compiler, run on the VM, compiles a corpus program
  in about a second and `bodies.ry` in about three (checking and
  loading the whole front end included); the judge adds about a minute
  to `cargo test` on eight threads. The judges run the front end from
  its bytecode since the end of the session (the section "The judges
  run from bytecode" below), which takes the checking out of every run.

## Constraints with type arguments (decision AB1)

Open item R3-2, chosen by the owner after the emitter; the four design
questions and their answers are the entry AB1.

- **The rule.** `for any Bag, Item where Bag can Iterable of Item`: a
  constraint, and an ability's `where self can` requirement, names an
  ability with as many type arguments as it declares, any types in
  scope; `type-arity` otherwise ("`Iterable` takes 1 type argument,
  found 0", fix "write `Bag can Iterable of Item`"). In the body the
  parameter is walked with items of `Item`, and the ability's methods
  are called on it with the ability's parameters substituted. At a call
  site `require_constraint` matches the arguments the argument's type
  has the ability with (`World::implemented_args`: the implementation's
  arguments with its parameters read as the type's, a parameter's
  constraint or what that constraint's ability requires, a subtype's
  base, an empty list for a derived ability) against the constraint's,
  which binds what the constraint leaves open; a mismatch is
  `missing-ability` naming the arguments, fix "implement `Iterable of
  Integer` for `Deck`". One implementation per ability and type:
  `duplicate-implementation`.
- **The collections are `Iterable`.** `library/std/prelude.ry` holds
  implementations for `List of Item`, `Set of Item`, `Map of Key to
  Value` (pairs), `Range` and `Text` (characters), with the methods'
  heads alone (both parsers read an implementation without bodies in a
  declaration file; `ImplDeclaration` in the grammar); the free
  `to_list` of `Set` and `Range` moved into them, the library sketch and
  `crates/renyi_syntax/tests/library.rs` follow. The checker's own loop
  rules for the collections are gone: `loop_items` asks
  `implemented_args` for `Iterable`, for a type and a parameter alike.
  An implementation head takes full types after `of` (`Implementation`
  in the grammar; `ability()` of both parsers parses types after `of`
  and demands names when no `for` follows).
- **Rust.** `parser.rs` (`ability`), `world.rs` (`AbilityRef`, carried
  by `ParamInfo.constraints` and `AbilityInfo.requirements`; `resolved`
  on `AbilityInfo`, so that the parameters of every ability of a module
  are known before a requirement is counted; `for_any_params`,
  `resolve_abilities`, `check_requirements`, `implemented_args`,
  `show_ability`, `check_ability_arity`, the duplicate check in the
  implementation branch of `resolve_functions`; an implementation's
  methods take the ability's visibility), `check.rs` (`Deferred` with
  `args`, `require_constraint`, `match_ability_args`,
  `report_missing_ability`, `show_ability`, `is_own_type`,
  `loop_items`, the ability arm of `infer_method_call`, the constraint
  loop of `call_known`), `vm.rs` (`builtin_type_id`: an ability call on
  a base value dispatches by its declared type; `has_type` uses it),
  `natives/prelude.rs` (`iterable_to_list` for `List`, `Map` and
  `Text`).
- **Renyi.** `parser.ry` (`type_list`, `parameter_names`,
  `implementation` with `ability_args`, bodies by
  `cursor.declarations`), `declare.ry` (`AbilityRef`, `PendingAbility`
  and the two-phase `resolve_ability_items`, `add_resolved_constraint`,
  `resolved_requirement`, `check_ability_arity`, `padded`,
  `implemented_args`, `param_args`, `implementation_for`,
  `bind_target_params` moved in from `bodies.ry`, `show_ability`,
  `has_constraint`, `note_duplicate_implementation`,
  `implemented_elsewhere`, `pair_params` public, `adopt_method` sets
  `is_public`), `bodies.ry` (`require_constraint`,
  `match_ability_args`, `missing_ability` with `args`, `show_ability`,
  `is_own_type`, `settle_deferred`, `iterable_item` over
  `implemented_args`, `source_without_iterable`,
  `call_ability_method` substitutes, `check_instance_constraints`),
  `types.ry` (`substitute_maybe` public). Judged by the three judges of
  `selfhost.rs` as before.
- **Tests and documents.** `rules.rs`
  (`a_constraint_names_an_ability_with_its_type_arguments`); the
  conformance cases 39 to 42 (`constraint_arguments.ry`, a run case;
  `constraint_arity.ry`, `duplicate_implementation.ry`,
  `constraint_mismatch.ry`); the reference (sections 2, 3, 4, 5 and 8,
  appendix A), the grammar, the cheat sheet (3000 of 3000 tokens: the
  `at_least`/`at_most` sentence went, the library line keeps them), the
  sketch (section 5, R3-2 in section 18), the library sketch, the
  decisions (section AB), `CLAUDE.md`, `docs/GAPS.md`, this file.

## The judges run from bytecode (session 8, after AB1)

The owner's choice after R3-2 among the three pieces "Next steps"
named. Each judge of `crates/renyi/tests/selfhost.rs` compiles its
driver with `renyi compile` when the test starts (`front_end`:
`compiler/parse.ry`, `checker.ry` or `compile.ry` to
`target/selfhost/front/<driver>.ryc`, written anew on every run, so
that the file is never older than the sources) and passes that file to
`renyi run` in place of the source, which loads the front end (decision
Z4) instead of checking its 25 000 lines first. The emitter's judge
adds the fixed point: the file the Renyi compiler writes of
`compiler/compile.ry` while running from its bytecode must equal the
file it ran from (the first differing line is reported). The lane
script `judge_emit.sh` runs the same way.

- **What it saves**, best of five on the development build, a corpus
  program as the input: a run of the parser 225 to 185 ms, of the
  checker 440 to 303 ms, of the compiler 521 to 345 ms (2432 to 2222 ms
  with `bodies.ry` as the input). `renyi check` of `compile.ry` alone
  takes 419 ms where a one-page program takes 163 ms, and loading the
  9.8 MB `compile.ryc` costs about 100 ms of a run, so a run saves the
  checking of the front end less the loading of its file. The selfhost
  test binary on eight threads: 56.3 s before, 50.3 s after.
  The rest of the judges' time is the VM running the front end on the
  inputs, the compiler's own sources above all (`bodies.ry`: about a
  second to parse, two to compile).
- **Not changed.** The files are build products under `target/`, not in
  the repository; `renyi run compiler/parse.ry` and the other two
  source forms work as before, and `README.md` names them.

## Packages (decision AC1; stage 3, the first slice: two commits)

The owner's choice after the judges ran from bytecode: stage 3 (M4) and,
beside it, the parity of the Renyi front end's diagnostics with the Rust
parser's ("1+2"); the four design questions of packages and their
answers are the entry AC1 (a static registry, JSON manifests, the
package name as the first segment of an import, packages before
`std.process` and the FFI).

- **The rule.** A program's project root is the directory of the nearest
  `renyi.json` in its file's directory or above it (a relative path
  stops at the working directory, an absolute one at its root), else
  the file's directory (J17); its own imports resolve from the root.
  The manifest names the dependencies (name to the version required,
  `major.minor.patch`; a requirement means the same major and at least
  that version) and the registry, a directory (absolute, or relative to
  the root) or an `http://` or `https://` base whose packages are
  fetched into `.renyi/packages/<name>/<version>/` under the root; the
  lockfile `renyi.lock.json` names every package's version and the
  `sha256:` hash of its `package.json`. `import <name>` is the package's
  root module `<name>.ry` and `import <name>.<path>` its file
  `<path>.ry`, both under `<registry>/<name>/<version>/`; the checker
  names the modules `<name>` and `<name>.<module>`; inside a package an
  import reaches the package's own files first, then one of its own
  dependencies; an import reaches a package only when the manifest (or
  the package's `package.json`) lists it as a dependency. A dependency's
  capabilities are covered as a library's are (the kind statically, the
  scope by the grant stack at run time) and the diagnostic names the
  package: "`announce` (package `greeting` 1.0.0) needs `console`, which
  `main` does not declare". `package-missing` (not in the lockfile, no
  registry, not at the registry), `package-mismatch` (the hash differs;
  the package is read all the same) and `manifest-invalid` (a manifest,
  a lockfile or a package file refused: not JSON, not an object, a field
  the format lacks, a wrong type, a bad name or version) are reported on
  the import; the manifest's and the lockfile's problems at the start of
  the main file.
- **Rust.** `crates/renyi_json` (the VM's `Json`, `read_json` and
  `write_json` moved out; `natives/json.rs` re-exports them).
  `crates/renyi_package`: `version.rs`; `manifest.rs` (`Manifest`,
  `Lock`, `PackageFile`, `Versions`, `Budgets`, `Effect`; strict readers
  with one message per refusal, `render` with two-space indentation and
  a fixed key order, so that a package's hash is the hash of its
  rendered `package.json`); `registry.rs` (`Registry`, `STORE`,
  `hash_of`, `is_absolute`, `join`: a textual join with `/` and `.` and
  `..` folded, so that every toolchain names a file the same way);
  `resolve.rs` (`Project::of`, `resolve`: a stack of pending imports,
  the last import of a file first as before, `seen` by qualified name,
  `locate_package` reporting once per package). `renyi_syntax`:
  `SourceFile.package` and `Package`. `renyi_check`:
  `ModuleInfo.package`, `World::set_package` (the rename),
  `imported_module`, `check_project_with_problems`, `charge_of` in
  `check.rs`, `check_file` and `imported_files` through `resolve`; the
  binary's `compile_sources` the same; `renyi_index::load_project` skips
  `.renyi/` and indexes the dependency files it reaches;
  `Bytes.sha256()` in the prelude (`sha2`).
- **Renyi.** `compiler/project.ry` (`resolve`, `root_of` and
  `root_above`, `root_at`, `read_manifest`, `read_lock`,
  `read_package_file` with the Rust reader's messages in the Rust
  reader's order, through `std.json`'s `JsonValue` and a `Refused`
  failure type; `join_path(base, relative)` is the textual join;
  `check_project_with_problems`), `declare.ry` (`PackageTag`,
  `ModuleInfo.package`, `set_package`, `imported_module`), `bodies.ry`
  (`Charge`, `charge_of`, `same_package`), `checker.ry` and
  `compile.ry` (`resolve` in place of `imported_files`). The three
  judges run over the fixture too (`PROGRAM_DIRECTORIES` in
  `selfhost.rs`).
- **Where the two resolvers differ, by construction**: a key given
  twice in a manifest is refused by the Rust reader alone (a Renyi map
  keeps one value per key); a budget spelled `1.0` or `1e2` is a whole
  number to the Renyi side alone; a version part beyond 64 bits is
  refused by Rust alone.
- **The fixture** `tests/conformance/packages/`:
  `registry/greeting/1.0.0/` (`package.json` as `renyi publish` will
  render it, `greeting.ry` importing `words`, `words.ry`,
  `versions.json` beside the version), `project/` (`renyi.json`, the
  lockfile with the right hash, `report.ry` run as case 43,
  `uncovered.ry` case 44), `stale/` (a wrong hash, case 45),
  `unlocked/` (no lockfile, case 46), `broken/` (a field the format
  lacks, case 47). `crates/renyi_package/tests/fixture.rs` holds
  `package.json` canonical and the lock hash right and checks the
  resolver's file order and tags; `rules.rs`,
  `a_dependency_is_charged_with_its_package_named`. The fixture was
  written by `make_fixture.py` in the session's scratchpad; `renyi
  publish` will regenerate `package.json` once it exists.
- **The second commit: the commands** (`crates/renyi/src/packages.rs`,
  `crates/renyi_package/src/select.rs`). `select` chooses one version
  per package: a requirement means the same major and at least that
  version, the choice is the highest the registry has, the chosen
  packages' own requirements join the manifest's, and the rounds repeat
  until the choice is stable; requirements that disagree on the major
  are refused naming both parties. `renyi add <name> [<version>]`
  (refuses `std`, a name that is not a package name, a name that is a
  directory of the project, and needs a `renyi.json` with a `registry`)
  writes the requirement (the version given, else the registry's
  highest), chooses, fetches and verifies every package, prints the
  added package's public functions with their effects and failures, and
  writes both files. `update [--accept-effects]` takes every dependency
  to the highest version its requirement allows; the capability kinds
  of the new version's effect manifest are compared with the old
  version's, a widening is refused without the flag, and with it each
  `main` of the project that reaches the package (found through the
  index: a public or private function `main` of an own module; reach
  through `resolve_in`) must declare the new kind. `audit` prints every
  locked dependency's kinds, whether each `main` that reaches it covers
  them (exit 1 when one does not), and the capabilities of a `main` no
  dependency uses. `fetch` takes the locked packages (the lock's hash
  must be the registry's). `publish [--to <directory>]` requires the
  project to check clean, publishes every `.ry` and `.renyi` under the
  root (`.renyi/` left out), writes `package.json` with the files'
  hashes and the effect manifest, updates `versions.json`, never
  overwrites a version, and checks the version against the semantic
  diff of the own definitions since the highest published version (G1:
  a new major, a new minor, or greater). A URL registry is read through
  `ureq` (404 is "not there"), its packages are written into the store
  and verified there; a directory registry is read in place. The effect
  manifest is recomputed from the sources on every fetch through the
  index (every public function or method of the package's own modules,
  its transitive effects and failures, sorted by name), with the package
  checked as a project of its own rooted where its files are and its
  dependencies read from the registry or, for a URL, from the store.
  The run manifest names every package the program reaches with the
  lockfile's version and hash (`Dependency` in `recording.rs`, after
  `code`; none for a program loaded from a `.ryc` file, whose hash
  covers them), and `reproduce` refuses a run whose dependencies differ.
  `renyi index` labels a dependency's modules and definitions with
  `package: <name> <version>` (the text line and the JSON field, which
  `maps.rs` reads back; the canonical copies the index checks keep the
  tag, which they lost before), and `--budgets` reads the thresholds of
  `renyi.json`. The commands act on the working directory, so they are
  not conformance cases: `crates/renyi/tests/packages.rs` runs the
  binary in scratch projects under `target/packages/` (seven tests:
  `publish` writes the fixture's `package.json` byte for byte and
  refuses an overwrite, an unclean project and a version too small;
  `add`, the run manifest, `reproduce` on a changed dependency; `audit`;
  `fetch` on a changed file and a stale lock; `update` refused without
  the flag and until `main` declares the new kind; a URL registry served
  by a thread; the map's labels and the budgets).
- **Next**: nothing of M4. The parity of the Renyi front end's
  diagnostics with the Rust parser's (the "2" of the owner's answer)
  is done: the next section; `std.process` (decision AE1) and the FFI
  (decision AF1) are done: the sections after it.

## The diagnostics of the front end in Renyi (decision AD1)

The "2" of the owner's answer "1+2" (2026-10-07): the front end written
in Renyi reports what the Rust front end reports, so that the three
judges compare output and exit status on every program, rejected ones
included, and `checker.ry` and `compile.ry` answer as `renyi check` and
`renyi compile` do on a file with syntax errors too.

- **The lexer** (`compiler/lexer.ry`): `lex(source) returns Lexed`, the
  tokens and the diagnostics in source order; every error site of
  `lexer.rs` is mirrored with its code, message, span and fix (`tab`,
  `crlf`, `equals-sign`, `semicolon`, `symbolic-operator`,
  `unknown-character`, `identifier-shape`, `single-letter-identifier`,
  `type-name-shape`, `number-shape`, `unterminated-text`,
  `unknown-escape`, `empty-hole`, `text-in-hole`, `unterminated-hole`);
  a bad character becomes an `ErrorToken`, which the parser's `expected`
  is silent about, as the Rust parser is. A carriage return is found
  through the base64 of its byte (`\r` is not an escape in Renyi);
  `tokens.ry` prints it as `\r`.
- **The parser** (`compiler/parser.ry`, 3400 lines): the `Cursor`
  carries the diagnostics so far and the source characters; a construct
  that cannot be parsed stops with `Halt`, a failure type that carries
  the cursor (its position, nesting and diagnostics); every error site
  of `parser.rs` is mirrored with its code, message, span and fix
  through the helpers `expected` (silent on an error token, the foreign
  spelling of decision C4 as the fix), `describe`, `token_for` (the
  Rust parser's `token_at`), `foreign_spelling`, `halt` and `diagnose`;
  `identifier` takes a reserved word with `reserved-word` and goes on,
  as the Rust one does; the recovery points are the Rust parser's
  (`recover` to the end of the line after a bad import, a bad statement
  in a block, a bad clause or example, and `skip_to_next_item` to the
  next item at the margin after a bad item; `lenient_end_of_line` and
  `lenient_symbol` where the Rust parser ignores the result);
  `parse_hole` parses a hole on its own cursor and reports
  `hole-syntax`; `parse(lexed, chars, declarations) returns
  ParsedModule`, the partial tree and the diagnostics, the lexer's
  first. The transcription left the line-break handling untouched.
- **The drivers**: `parse.ry` prints the tree, then `report.render_json`
  of the diagnostics when there are any, and exits 1 on an error, as
  `renyi parse --json` does; `project.ry`'s `Front` is `Parsed(tree)` or
  `Rejected(tree, diagnostics)`, the partial tree kept because the Rust
  resolver follows the imports of a file that does not parse;
  `check_project` gives a rejected file its parse diagnostics and no
  module, `checker.ry`'s `diagnose` appends the layout warnings and
  sorts, `compile.ry` prints them as `renyi compile` does. `report.ry`'s
  `json_string` writes `\u00xx` for a control character other than
  `\n`, `\t` and `\r` (the code read off the base64 of the byte), as
  the Rust one does.
- **The judges** (`selfhost.rs`): the parser judge compares stdout and
  exit status on every program; the checker judge no longer asks `renyi
  parse` whether a program parses; the emitter judge compares stdout
  and exit status, then the files when both wrote one. The lane scripts
  `judge_parse.sh`, `judge_check.sh` and `judge_compile.sh` under
  `D:\Projects\.worktrees\Renyi\selfhost\` run the same comparisons
  over the repository's programs and the probes under `probes/` (21
  lexer probes from `make_lex_probes.py`, 24 parser probes from
  `make_parse_probes.py`, both in the session's scratchpad), which
  exercise the error sites no program in the repository reaches: 135
  files, all equal, at the end of the session. Conformance cases 48
  and 49 (`lexer_errors.ry`, `syntax_recovery.ry`) hold a sample of
  both in the suite.
- **What bit**: a pattern binds a record's field by name, so
  `failure(Halt(cursor: halted))` is the spelling (`Halt(halted)` is
  `unknown-field`); `x with a: 1,` followed by another field of the
  enclosing constructor swallows that field (parenthesize the `with`);
  `return` followed by a line break ends the statement; `/` on two
  Integers is `integer-division` (`quotient`); a message over 100
  columns is built from two halves in a function, since a line of a
  constant cannot be broken.

## The process module (decision AE1; stage 3, the second slice)

The owner's four answers of 2026-10-07, all the recommended ones: one
call per run (no handle), a status other than 0 a failure, the scope
matched text for text, the parent's environment inherited.

- **The module.** `library/std/process.ry` and section 13 of the
  library sketch: `Completion` (`status`, `output`, `errors`, `bytes`),
  `Options` (`directory: maybe Path`, `environment: Map of Text to
  Text`, `input: Text`, `limit: maybe Duration`), `ProcessError`
  (`NotFound`, `CannotStart`, `Exited(program, status, output,
  errors)`, `Timeout`, `ProgramNotAllowed`, `OverBudget`), the functions
  `execute`, `execute_with`, `attempt`, `attempt_with` (all `needs
  process`) and `defaults()` (pure: no directory, no variables, no
  input, no limit; a program writes `process.defaults() with input:
  text`). `run` is a reserved word, hence `execute`.
- **The natives.** `crates/renyi_vm/src/natives/process.rs`:
  `std::process::Command` with the arguments one by one (no shell), the
  three streams piped, the output and the errors read on two threads
  while the parent waits (a child that fills a pipe does not stall), the
  input written on a third and the pipe closed after it; a limit polls
  `try_wait` every 5 ms and kills the child past it (`Timeout`); the
  status is the code, or 128 plus the signal on Unix; `output` and
  `errors` are decoded with replacement characters, `bytes` is the raw
  standard output. `NotFound` is the spawn error of that kind,
  `CannotStart(program, detail)` any other.
- **The boundary.** Nothing new in `call_primitive`: `grant::effect_of`
  already took the first `Text` argument of a `process` primitive as
  the scope, `scope_contains` compares a program name text for text,
  the counters count it; `vm.rs` maps a denial to
  `ProcessError.ProgramNotAllowed(program)` and an exhausted budget to
  `ProcessError.OverBudget(program)`, as the other modules do; a denied
  call is not counted, a `NotFound` one is. A guarded value (`only to`)
  as the program or an argument fails with `Guarded(origin, sink:
  "process(\"...\")")` before anything starts. A recording holds the
  call with its `Completion`; `run --replay` answers it and starts
  nothing (and prints nothing, as a replay does); `reproduce` shows the
  output.
- **The checker.** `process.ry` is in `LIBRARY` of
  `crates/renyi_check/src/lib.rs` and in `library_modules` of
  `compiler/project.ry` (the order the judges depend on);
  `effects::unavailable` kept `foreign` alone (`capability-unavailable`)
  in `effects.rs` and `compiler/effects.ry` until decision AF1 removed
  it (the next section).
- **The tests.** `crates/renyi/tests/process.rs` (six, every child the
  `renyi` binary itself running a small program, so nothing else needs
  to be installed): `renyi version` through `execute` (status, output,
  the empty errors, the byte count); `renyi nonsense` is `Exited` for
  `execute` and a completion with status 1 for `attempt`; the options
  (a child prints its directory, a variable, the line it read; a
  sleeper is killed at 300 ms); the scope (`ProgramNotAllowed`), a
  missing program and the budget (`OverBudget`); a guard; a recording
  replayed and reproduced with no marker file written. Conformance case
  50 (`process_errors.ry`) prints `NotFound`, `OverBudget` and
  `ProgramNotAllowed` without starting anything (case 20 held
  `foreign` unavailable until decision AF1).
- **What bit.** `run` as a function name (reserved); a call with one
  argument may not name it (`report("x")`); an `otherwise` on a
  continuation line is `otherwise-line` (break inside the parentheses
  instead); a replay prints no console output, by design.

## The foreign function interface (decision AF1; stage 3, the third slice)

The last slice of M4 in the order the owner chose (packages, then
`std.process`, then the FFI); the four design questions and their
answers are the entry AF1.

- **The rule.** A foreign module is a declaration file of the project
  (bodiless `public function`s, each `needs foreign` and nothing else)
  that the manifest's `foreign` section binds to shared libraries:
  `"foreign": {"libc": {"library": ["ucrtbase", "libc.so.6",
  "libSystem.B.dylib"], "symbols": {"renyi_name": "c_symbol"}}}`; the
  libraries are tried in order, `symbols` is optional. The resolver
  tags the file (`Project::foreign_of_file` for the main file by its
  path from the root, `Project::foreign_module` for an own import by
  its qualified name; `renyi_package::tagged` for a command's file,
  which `diagnose` of `main.rs` and `load_project` of the index use),
  `SourceFile.foreign` carries the `ForeignModule`, `check_project`
  parses a tagged file with `parse_declarations` and declares it with
  `is_library = true` (its functions are primitives: no body, recorded
  at the boundary), `ModuleInfo.foreign` holds the binding.
- **The types.** `library/std/foreign.ry`: `Int8` to `UInt64` and
  `Size` as refinements of `Integer`; `crates/renyi_check/src/foreign.rs`:
  `CType` (the spellings `i8` to `u64`, `size`, `f64`, `bool`, `text`,
  `bytes`), `CResult` (`void`, a type, `text_or_null`), `World::c_type_of`
  and `c_result_of`. `declare_function` of `world.rs` runs
  `check_foreign_signature` after the needs loop: `foreign-signature`
  (needs other than `foreign` alone, `or fails with`, `for any`),
  `foreign-type` (a parameter or a result the boundary does not carry),
  `foreign-arity` (over six words, `Bytes` counting two). A declared
  function's parameters are not `unused-binding` (`check_function` of
  `check.rs` marks them used when there is no body).
  `capability-unavailable` is gone from both checkers, the reference
  and the suite.
- **The bytecode.** `FunctionMeta.foreign: Option<Foreign>` (libraries,
  symbol, parameter spellings, result spelling) filled in
  `compile/mod.rs` from the module's binding; `file.rs` writes it as
  `"foreign"` after `"code"` and `FORMAT` is 2; `compiler/bytecode.ry`
  (`ForeignBinding`) and `compiler/emit.ry` (`foreign_binding`) mirror
  it; `compiler/declare.ry` holds the port of `foreign.rs` beside
  `world.rs`'s (`c_type_of`, `c_result_of`, the checks), `project.ry`
  the manifest section, the tagging and the declarations parse.
- **The VM.** `natives/foreign.rs`: at the first call `bind` loads the
  first library of the list that loads (`libloading`, kept in
  `Vm.libraries` by its name list) and looks the symbol up; `invoke`
  marshals the arguments into the words of `foreign_abi.rs` (generated
  by `tools/gen_foreign_abi.py`: `call(address, slots, returns)` matches
  the arity, the class mask and the result class to a `transmute`d
  `extern "C"` signature; at most six words), calls, and reads the
  result back by its C type (a narrower integer by its width, `Text`
  copied from the `char *`, a null `Text` a crash, a null `maybe Text`
  nothing). `Vm::run_primitive` dispatches a function without a native
  to it; the boundary (`call_native`) does the rest: the grant check
  (`foreign`, no scope, no budget), the recording, the replay, the
  `only to` guard. A library or a symbol that is missing, or a `Text`
  with a NUL, is a crash at the call. This is the VM's only unsafe
  code; what the C function does is outside every guarantee (Q3).
- **The commands.** `renyi run` prints "renyi: this program can call
  native code through `libc`" on the standard error when `main` grants
  `foreign` (07-system-design.md, section 2.2); `renyi bind
  <header.h> --module <name> --library <name>[,<name>...] [--to
  <directory>]` (`bind.rs`) writes `<name>.ry` from the prototypes of a
  header (comments and preprocessor lines removed, `extern "C"`
  dropped, the C types mapped; `long`, `float`, pointers other than
  `char *` and variadic functions left as `# skipped:` comments with
  the reason; names snake-cased and never reserved, a single letter
  becoming `argument_N`) and the entry into `renyi.json` when there is
  one (printed otherwise); `renyi publish` refuses a project with
  foreign modules; `renyi format` does not read a foreign module (a
  declaration file, like the library's).
- **The tests.** `crates/renyi/tests/foreign.rs` (seven, every library
  the C library of the platform: `ucrtbase` on Windows, `libc.so.6`
  and `libm.so.6` on Linux, `libSystem.B.dylib` on macOS): `strlen`,
  `abs`, `atoi`, `sqrt` through a renamed symbol and `getenv` as
  `maybe Text`, from the source and from the bytecode file; a
  recording replayed and reproduced with the libraries renamed to
  nothing that loads; `--deny foreign`; a missing symbol and a missing
  library; `bind` on a header, the module written, the manifest
  updated, the module checked; `publish` refused. The fixtures
  `tests/conformance/foreign/` (`libc.ry`, `length.ry` printing 5 and
  7; case 51) and `tests/conformance/foreign_bad/` (`bad.ry`, the
  three diagnostics; case 20) are under the three judges, the Renyi
  checker and compiler byte-equal on them; `tests/fixture.rs` holds
  their manifests canonical; `tests/library.rs` counted fourteen files
  (fifteen since the bridge).
- **What bit.** A heredoc un-escapes backslashes (a patch script
  written through it got a raw carriage return); a patch applied twice
  duplicated the additions whose anchor survived the first run; the
  `check` command parsed its file before the resolver tagged it;
  `json` is the namespace of an import in `project.ry` (no local of
  that name); a name is bound once per function (two `match` arms may
  not bind the same name); a single-letter parameter name is an error
  (`single-letter-identifier`), so the generator avoids it.

## Machine code for the bytecode (decisions AG1 to AG5)

The owner's direction of 2026-10-07: performance first, at the root,
ahead-of-time compilation included; then the language's niche; then the
release. The performance half is done to the point the decisions
record; the niche and the release are decided and not yet written (the
next sections of the plan, below).

- **What exists.** `crates/renyi_vm/src/native/`: `infer.rs` (an
  abstract interpretation over a code object's stack bytecode: the
  entry state of every pc, the kind of every slot, the handler regions
  and the marks; `Abs` is `Unset`, `Int`, `Bool`, `Float`, `Range` or
  `Boxed`, `SlotKind` the same plus `Mark`, `RangeIter`, `Iter` and
  `Deadline`; a code object the analysis cannot settle stays with the
  interpreter), `codegen.rs` (every op to Cranelift IR: typed operands
  and slots are Cranelift variables, boxed operands stay on the VM's
  stack in order; an op that may fail branches to one shared block per
  handler region and operand stack below it, which settles the failure
  on the region's floor and jumps to the handler; a status other than
  `CONTINUE` and `FAILURE` leaves the function; `deopt_here` writes the
  registers into a stack slot and calls `rt_deopt`), `runtime.rs` (the
  helpers, `extern "C"`, each the interpreter's arm on the VM's stack
  through the shared `Vm::op_*` methods; `rt_call` goes through
  `call_from_stack` and `run_top_frame`, so a callee runs natively when
  it has code; `rt_deopt` rebuilds the interpreter's frame), `mod.rs`
  (`Jit`: the Cranelift module, the helpers declared as imports, the
  state of every code object, the hotness rule and the report).
  `vm.rs`: `run_top_frame` tries the generated code first; the `call!`
  macro of the loop does the same for a callee; `hotness` counts the
  ops the interpreter runs per code object (`ran` in the loop, flushed
  at `reload!` and `leave!`); `Options.interpret`. The binary runs its
  command on a thread with a 64 MB stack (`main` spawns `dispatch`),
  and `DEPTH_LIMIT` (200) bounds the generated frames nested on it.
- **The tiering.** A code object is compiled once `hotness >=
  HOT_FACTOR * ops` (8000 since decision AR5, 2000 before it): Cranelift
  spends about a quarter of a million instructions per op compiled and
  a compiled op saves a few tens each time it runs, so a code object
  pays for its compilation after some ten thousand runs of each of its
  ops. The machine code takes over at the
  next call, or at the next turn of a loop the interpreter is in: the
  `Op::Jump` arm of the loop, on a jump backwards of a hot code object,
  asks `native_resume_of_top` for the entry at the target and runs it
  (`RETURNED` leaves the frame as `leave!` would, `DEOPT` reloads,
  `STAY` marks the code's resumes refused). The generated function's
  prologue dispatches on its `pc` parameter: a loop header's entry takes
  every typed slot from the frame through `rt_resume_*` (a `Nothing`
  slot, not yet assigned, gives a zero; a slot that does not fit returns
  `STAY`), then jumps to the header's block. Only headers whose entry
  state has nothing on the operand stack in registers get an entry.
  `RENYI_NATIVE_HOT=0` compiles everything at its first call and enters
  every loop at its first turn (the conformance suite's third pass and
  `tests/native.rs` use it). The interpreter iterates a range of small
  Integers through `Native::RangeIterator` (the next value, the last and
  the step), the shape the generated code keeps, so `rt_deopt` and
  `rt_resume_range` convert without listing the items.
- **The numbers** (development build unless said): `bench/primes.ry`
  84 ms against CPython's 529 ms; `hello` 46 ms either way; the front
  end forced native on `compiler/lexer.ry`: 676 code objects, 39,502
  ops, 240,713 Cranelift instructions in 22,202 blocks, 0.42 s, of
  which Cranelift 0.31 s (register allocation 0.23 s; `single_pass`
  was slower, `opt_level=speed` 25% slower for no run-time gain, the
  verifier another 25%); the compiler on `compiler/bodies.ry` (release,
  Linux): interpreter 1.6 s CPU, native 1.5 s plus 0.36 s compiling, so
  the machine code brings nothing to such programs (decision AG5 has
  the profile).
- **The tests.** `crates/renyi_vm/tests/native.rs` (six: overflow to a
  big Integer, a guarded Integer in a typed parameter, recursion past
  `DEPTH_LIMIT`, failures handled and unhandled, a crash's line, Floats
  and Booleans in registers and a stepped range; each run both ways
  and compared); `crates/renyi/tests/conformance.rs` runs every `run`
  case from its bytecode file on the interpreter and every case that
  runs a program a third time with `RENYI_NATIVE_HOT=1`; the three
  selfhost judges run the front end from its bytecode with the default
  tiering.
- **The development aids.** `RENYI_NATIVE_REPORT`, `RENYI_NATIVE_HOT`,
  `RENYI_NATIVE_OPT`, `RENYI_NATIVE_VERIFY` (decision AG5 describes
  them). Under `D:\Projects\.worktrees\Renyi\`: `selfhost/bench_hot.py`
  (the front end per hotness factor), `selfhost/bench_primes/`,
  `profile/micro.py` (the loop micro-benchmarks), `perf/` (the Linux
  build under WSL with `perf`: `build_linux.sh`, `profile_linux.sh`,
  `callers_linux.sh`, `native_vs_interp.sh` and their reports).
- **What bit.** Cranelift's verifier rejected a block never switched
  to (a fallthrough block made by `edge_to`); `stack_load` and
  `stack_store` take the pointer type first; `icmp_imm` and `bxor_imm`
  are deprecated for the `_s` forms; a heredoc un-escapes backslashes
  (again); the reserved words `sum`, `count`, `by`, `within` and `tags`
  and the named arguments of calls with two or more arguments in the
  test programs; `/` on two Integers is `integer-division`
  (`quotient`).

## The VM round (decision AG6)

- **What changed**, all in `crates/renyi_vm/src/vm.rs`, none of it
  visible to a program, the bytecode file or the compiler written in
  Renyi (the judges and the conformance suite pass unchanged): the
  `Op::Load` arm reads the field in the slot when the slot holds a
  record or a variant, the next op is `Op::Field` and the site's cache
  names the type (otherwise the load clones as before and `Op::Field`
  does the rest, including the guard wrapper and the cache miss);
  `Frame::grant` is an index into `Vm::grants` (the run's grant first),
  `frame_grant` returns an index, `Vm::narrows` says per code object
  whether its function narrows the grant at all, and
  `push_frame_in_place` fills the locals with one `resize`;
  `construct_variant` keeps a variant without fields in
  `Vm::unit_variants` (`unit_base` gives each type its slots).
- **Tried and dropped** (decision AG6 has the numbers): a store of a
  binding fused with the load that follows it; the fields of a record
  or a variant inline in its allocation (`smallvec`), which cost more
  instructions than the second allocation had.
- **How it was measured** (the owner's game loaded the CPU; wall and
  CPU time swung by a third between two runs, the instruction count did
  not): `perf stat -e instructions:u,cycles:u` on the release build
  under WSL, through the lane scripts of `D:\Projects\.worktrees\Renyi\perf\`:
  `build_linux.sh` (the release build with symbols into the lane),
  `perf_stat.sh <binary> <args>` (one command's counts), `measure_all.sh
  <binary>` (the self-check both ways and the five micro-benchmarks of
  `..\profile\`), `ab_stat.sh <a> <b> <rounds> <args>` (two binaries
  interleaved, the minimum of each count), `ab_linux.sh` (the same with
  user time), `chain_build5.sh`/`chain_build6.sh` (a build queued after
  the running one); the binaries of each step are kept there as
  `renyi_base`, `renyi_f1f4`, `renyi_f2`, `renyi_f5`, `renyi_f6` and
  `renyi_f7` (the final one); the last pushed commit is checked out in `..\base\` for a
  development-build twin. `..\profile\micro.py` reports user time
  (`GetProcessTimes`) and forces the interpreter unless
  `MICRO_MODE=native`; `loop_field.ry` and `loop_construct.ry` joined
  the micro-benchmarks there.
- **The numbers** are in decision AG6: 5.4% fewer instructions and
  4.1% fewer cycles on the compiler's self-check on the interpreter,
  from the fused field read (about 4%), the frame (1%) and the unit
  variants.

## The plan after the machine code (the owner's answers of 2026-10-07)

1. **A bounded round of VM work** (decision AG5; done, decision AG6,
   the section above): the fused field read, the grant by index, the
   unit variants; 5.4% fewer instructions and 4.1% fewer cycles on
   the compiler's self-check. The clone and drop of values,
   the stack traffic and the allocator remain, for the baseline JIT or
   `renyi build` after 0.1.
2. **Positioning** (decisions AH1 to AH4, recorded;
   `docs/design/08-positioning.md` written, the README's opening and
   its "what Renyi leads with" list derive from it): the niche is
   "the scripting language of AI agents": the language an agent writes
   and a person reviews at a glance, where the program declares what it
   may do, the runtime admits only that, and every run can be recorded,
   replayed and narrated; the target users run automation with Claude
   Code or Codex and will not run an agent's Python blind. Four selling
   points: effects as capabilities (declared, scoped, budgeted,
   enforced at the boundary, dependencies included); recorded,
   replayed and narrated runs; the tooling for agents (the project map,
   the MCP server, purpose as syntax, a fix on every error); package
   effect manifests computed by the tool, never widened silently. The
   English-like syntax is second: lead with the guarantees, the syntax
   is the means of review. No non-goals are written; the posture stays
   that of a general language. Still to do from it: the documentation
   site's front page (AI2) and the measurements of its section 5 on the
   starter pack (AI3).
3. **Release 0.1** (decisions AI1 to AI5): done on 2026-10-07. The
   engineering is the section "Release 0.1 engineering" below; the
   release itself is its last bullet and `docs/RELEASE.md` section 5.
   What remains of it: the announcement (the owner, with the texts of
   `08-positioning.md` section 6), the VS Code Marketplace publisher
   (optional: the `.vsix` is on the release page), the domain
   renyi-lang.org (AI4, when the owner wants it), and the three
   measurements of the positioning's section 5.
4. **Deferred** (decision AG5): both done in session 9: the baseline
   JIT that inlines the boxed operations (decisions AR1 to AR6: the
   self-check 15% fewer instructions on machine code than before the
   round, `bench/records.ry` 21% fewer) and `renyi build`, the image of
   bytecode and machine code and, with `--exe`, the self-contained
   executable of A1 (decisions AS1 to AS4).
5. **Foreign packages** (decisions AJ1 to AJ4, the owner's answers of
   2026-10-07, evening): after 0.1 and before the rest of M5, the
   registration API for Rust natives (the standard library's
   mechanism opened to extensions: a declaration file with the
   signature, the capability and the failures, the native built into
   the binary; half of M5's embedding API), then the typed Python
   bridge as the first extension package (one worker process per
   run, JSON messages, each call one recorded primitive) under the
   capability `python("<package>")`. The registration API exists
   (decisions AK1 to AK4, the section "The registration API as it
   exists" below), and so do the bridge and the binder (decisions AL1
   to AL4, AM1 and AM2, their sections below).

## The registration API as it exists (decisions AJ1, AK1 to AK4; session 8, 2026-10-07, late)

One commit, after the owner's four answers (AK1 to AK4, every
recommended option). The pieces:

- `crates/renyi_vm/src/extension.rs`: `Native` (module, name, the
  receiver as the checker spells it or its head, or none;
  `Native::function` and `Native::method` are `const fn`, so a table is
  a constant), `Extension` (name, version, the declaration files, the
  table), `Registry` (`standard()`, `add`, `with`, `extensions`,
  `extras` for the manifest, `library()` for the checker, `lookup` with
  the rule full spelling, then head, then bare, and `verify`, which
  reports every mismatch of the tables at once and the first problem of
  the files).
- The standard library is the first extension: `natives::standard()`
  joins the eleven per-module tables (`pub(crate) const NATIVES` in each
  file under `natives/`, written by a script from the old `lookup`
  functions; 200 entries) into one `Extension` behind a `OnceLock`;
  `natives::lookup` is gone and `Vm::new` asks `options.registry`. The
  two-way check passed on the standard library at its first real run.
- `renyi_check::Library` (`standard()`, `empty()`, `add`, `modules`,
  `world()`), `check_project_in(library, files, problems)` and
  `check_file_in`; the old names check against the standard library.
  `World::resolve_all` is public: a world of declarations alone needs
  it before its functions exist (without it the check passed
  vacuously, which the test caught).
- `renyi_index::index_files_in` and `tools_of_in`.
- `renyi_vm::Options::registry` (default `Registry::standard()`). The
  boundary's refusals are found by the shape of the declared failure
  types (`Vm::boundary_failure`: the variant named `PermissionDenied`,
  `HostNotAllowed`, `ProgramNotAllowed` or `OverBudget` with one field,
  the scope or, without one, the first argument), which replaced the
  match on the five module names and reports an extension's own types
  the same way.
- `Manifest::extensions` (`"extensions": ["demo 0.1.0"]`, left out when
  empty), rendered and parsed; `renyi reproduce` warns when the
  recording's list differs from the binary's.
- The crate `renyi` is a library and a binary: `src/lib.rs` is the old
  `main.rs` with `pub fn main_with(extensions: Vec<Extension>) ->
  ExitCode` (verifies when there are extensions, stores the registry in
  a `OnceLock`, then the 64 MB thread as before), `registry()` and
  `library()` for the commands, and `src/main.rs` is
  `renyi::main_with(Vec::new())`; every check, index, tools, run, test,
  reproduce and MCP path goes through them; `renyi version` prints one
  `extension name version` line per extension; the bin target has
  `doc = false` against the name collision.
- The test `crates/renyi_vm/tests/extension.rs`: the standard library
  verifies; the lookup rule; a three-native extension (`twice`, `shout`
  under `console`, `peek` under `filesystem.read` with its own
  `PeekError`) checks, runs, is recorded and is refused through
  `PeekError.PermissionDenied`; its native fails with `Missing`; the
  check names a declaration without a native, a native without a
  declaration, a capability outside the reference, a module declared
  twice and a file registered under another name.
- The documents: decisions AK1 to AK4; `docs/extensions.md`, the guide
  (the declaration file, the natives, the `Extension` value, the binary
  of three lines, what the boundary does, what an extension is trusted
  with); the reference (section 2: extension modules import by name;
  section 11: the refusal rule by shape; appendix B: the manifest,
  `reproduce`, `version`); `06-runtime-guarantees.md` and
  `07-system-design.md` on the manifest's `extensions` and the
  host-facing half of the embedding API; the front page and the site's
  navigation; the README's status; CLAUDE.md.

Gates: `cargo fmt`, `cargo clippy --all-targets` clean, `cargo test`
278 passed (270 before, the eight of the extension test new); the
corpus and `compiler/` untouched.

## The Python binder as it exists (decisions AM1 and AM2; session 8, 2026-10-07)

One commit after the bridge, from the owner's four answers (AM1, every
recommended option): `renyi bind --python <package> [--module <name>]
[--to <directory>]` writes the declaration file from the package.

- **The command.** `crates/renyi/src/bind.rs` parses `--python` next to
  the header form and dispatches to `bind/python.rs`; the writing of
  the file and of the manifest entry is shared (`write_module`,
  `ManifestEntry::{Foreign, Python}`: the `foreign` or the
  `python.modules` section, the entry printed as `"python":
  {"modules": {...}}` when there is no manifest), as are `renyi_name`
  (now with the prefix of a reserved word, `c_` or `py_`) and
  `declaration` (now with the trailing clauses given: `needs foreign`,
  or `or fails with PythonError` and `needs python("<package>")`). The
  module name defaults to the package name and must be a module name
  (`scipy.stats` is one; `Geometry` is not: name one with `--module`).
- **The inspection.** `bind/python_inspect.py`, embedded with
  `include_str!` and passed to the interpreter with `-c`, the package
  and the directory as its arguments: the directory first on
  `sys.path`, `sys.stdout` swapped for `sys.stderr` so that what the
  package prints on import cannot mix with the report, one JSON object
  on the standard output: the version, the file, the functions (name,
  parameters with kind, default and annotation tree, the return tree,
  the docstring's first line, the signature as `inspect` prints it),
  a function `skipped` with the reason, or `error`. The functions:
  the names of `__all__` that are routines when the module has it,
  else the public routines whose `__module__` is the module, in
  `vars()` order. An annotation is a tree: `{"class": "builtins.int"}`,
  `{"generic": "builtins.list", "args": [...]}`, `{"union": [...]}`,
  `{"none": true}`, `{"other": "<text>"}`; `inspect.signature(...,
  eval_str=True)` first, the unevaluated signature when that raises.
  The interpreter is the first of
  `renyi_vm::natives::python::interpreters(configured)` (the former
  `candidates`, now public: the manifest's, `RENYI_PYTHON`, the PATH)
  that gives a report line; an import that fails in it is the answer
  ("`python` cannot import `x`: ModuleNotFoundError: ...").
- **The mapping** (AM2, `Mapper` in `bind/python.rs`): the scalars,
  the list-like, set-like and map-like origins (`LISTS`, `SETS`,
  `MAPS`), `Optional[T]` and `T | None`, `Annotated`, a bare `list`,
  `set` or `dict` as the same of `JsonValue`; everything else
  `JsonValue` at that position with a note, the notes written as
  `# as JsonValue: \`x\` (Foo), the result (no annotation)` above the
  function; `import std.json exposing JsonValue` only when used. The
  parameters: the positional ones declared and required, `*args`,
  `**kwargs` and a keyword-only one with a default left out, a
  keyword-only one without a default skips the function (`# skipped:
  <signature> (a keyword-only parameter without a default: \`key\`)`),
  as does a function without a signature. A single-letter parameter is
  `argument_<n>`; the purpose is the docstring's first line as a
  sentence, else the signature in backticks, cut to the 100-column
  line.
- **The tests.** The unit tests of `bind/python.rs` (a hand-written
  report to the file and the entry, the mapping table, the purposes);
  `bind_writes_a_module_from_a_python_package_and_the_manifest_entry`
  in `tests/python.rs` through the binary: `geometry.py` without a
  manifest (the entry printed) and with one, `listed.py` with
  `__all__` and a re-export from `support.py` as `--module exports`,
  the manifest after both, a program `shapes.ry` run through the two
  generated modules, and the refusals (a package that does not import,
  a name that is not an import name, a package name that is not a
  module name, a header with `--python`).
- **The documents.** Decisions AM1 and AM2; the reference (section 11's
  Python bullet, appendix B's row); `docs/python.md` section 3 "The
  file written for you" (the sections after it renumbered); the
  library sketch's section 15; `README.md`; `CLAUDE.md`; the usage
  text of the binary.
- **What bit.** `renyi` had no dependency on `renyi_json` (added to its
  `Cargo.toml`); a Renyi call with two or more arguments names every
  argument and a construction names every field, so the test program
  is written that way; `otherwise fail` cannot start a line
  (`otherwise-line`), and `renyi check`'s diagnostics go to the
  standard output, so a test that shows the standard error on failure
  showed nothing; the scratch directory of a test was once locked on
  Windows right after a run that wrote `__pycache__` (the rerun
  passed; no worker lingered, `tasklist` showed none).

## The Python bridge as it exists (decisions AJ2, AJ3, AL1 to AL4; session 8, 2026-10-07)

One commit, after the owner's four answers (AL1 to AL4, every
recommended option); the pieces mirror the foreign function interface
(decision AF1) layer for layer.

- **The rule.** A Python module is a declaration file of the project
  (bodiless `public function`s) that the manifest's `python` section
  binds: `"python": {"interpreter": "python3", "modules": {"analysis":
  {"package": "analysis", "symbols": {"mean": "average"}}}}`;
  `interpreter`, `package` (the module's own name when left out) and
  `symbols` are optional, `Manifest::python` is a `PythonSection`
  (`manifest.rs`, with `symbols_of` and `symbols_json` now shared with
  the foreign section). The resolver tags the file
  (`Project::python_of_file` by its path, `Project::python_module` by
  its qualified name, `Project::tag` and `tagged` for a command's file;
  the `foreign` section wins when both name a module),
  `SourceFile.python` carries a `PythonBinding` (the `PythonModule` of
  `renyi_syntax`, the interpreter and the project root),
  `check_project_in` parses a tagged file with `parse_declarations` and
  declares it with `is_library = true`, `ModuleInfo.python` holds the
  binding.
- **The checker.** `crates/renyi_check/src/python.rs`: each function
  needs `python("<package>")` with the package of its binding and
  nothing else, fails with `PythonError` of `std.python` and nothing
  else, takes no type parameters (`python-signature`); its parameters
  are types that can `ToJson` and its result a type that can
  `FromJson`, or nothing (`python-type`): `World::carried` is the
  structural rule of `check.rs::has_ability` on declared types
  (derives, implementations, a subtype's base, lists and sets of
  carried items, maps with `Text` keys, `maybe`, the scalars),
  `World::json_ability` finds the prelude's abilities. `effects::TREE`
  has `python`, `takes_scope` too; `python` takes no budget.
  `library/std/python.ry` (and the crate's copy) declares `PythonError`:
  `Raised(exception, message)`, `NotCarried(detail)`,
  `Unavailable(detail)`, `PermissionDenied(package)`; the first field
  was `kind` at first, which the JSON encoding of a variant uses for
  its name (decision K10), so a replay decoded the recorded failure
  wrongly until it was renamed.
- **The bytecode.** `FunctionMeta.python: Option<Python>` (package,
  symbol, interpreter, root) filled in `compile/mod.rs` from the
  module's binding; `file.rs` writes it as `"python"` after
  `"foreign"` and `FORMAT` is 3; `compiler/bytecode.ry`
  (`PythonBinding`) and `compiler/emit.ry` (`python_binding`;
  `symbol_of` takes the list of renames, for both bindings) mirror
  it; `compiler/declare.ry` holds the port of `python.rs`
  (`check_bound_signature` dispatches to the foreign or the Python
  rules, so that `declare_function` stays under 60 lines; `carried`,
  `carried_shape`, `json_ability`), `project.ry` the manifest section
  (`python_section`, `python_modules`, `python_entry`), the tagging
  (`python_of_module`, `python_of_file`, `module_of_file` shared with
  the foreign tagging, `step_python`, `python_tagged`) and the
  fifteenth library module in `library_modules`.
- **The VM.** `natives/python.rs` with `python_worker.py` (embedded
  through `include_str!`, passed to the interpreter with `-c` and the
  project root as its argument): `Worker::start` tries the candidates
  of decision AL3 (`candidates`: the manifest's interpreter, else
  `RENYI_PYTHON`, else `python` then `python3` on Windows and `python3`
  then `python` elsewhere) and keeps the first that spawns and answers
  the greeting `{"ready": true, "version": ...}`; `Worker::call` writes
  one request line and reads one answer line, checking the call
  number; `run` encodes the arguments with `json::encode` by position,
  starts the worker at the first call (`Vm.python`), decodes the answer
  by the declared type with `json::decode` (a result the declaration
  does not mention is dropped) and maps the worker's `error` by its
  `where`: `call` to `Raised`, `result` to `NotCarried`, else
  `Unavailable`; a worker that stops answering is dropped (killed on
  `Drop`) and the next call starts another. The worker imports a
  module once and caches it, swaps `sys.stdout` for `sys.stderr` so
  that a package's prints cannot corrupt the protocol, and answers
  with `ensure_ascii=False, allow_nan=False`. `grant::effect_of` gives
  a `python` call the scope of its declared need; `Vm::run_primitive`
  dispatches a function with a `python` binding to the bridge.
- **The commands.** `renyi run` prints "renyi: this program can run
  Python through `analysis`" on the standard error when `main` grants
  `python` (`bound_modules` and `grants` of `lib.rs`, shared with the
  foreign notice); `renyi publish` refuses a project with Python
  modules; `diagnose` parses a tagged file as declarations.
- **The tests.** `crates/renyi/tests/python.rs` (six, a `helpers.py`
  beside the manifest): the run from the source and from the bytecode
  file, the notice and a Python print on the standard error; a
  recording replayed and reproduced with the interpreter renamed to
  nothing that starts; `--deny python`; an interpreter that is not
  there, through the manifest and through the variable; a worker ended
  by `sys.exit` and started again; `publish` refused.
  `crates/renyi_check/tests/rules.rs` has the capability and the
  signature rules, `manifest.rs` the section's round trip. The fixtures
  `tests/conformance/python/` (`analysis.py`, `analysis.ry`, `stats.ry`
  printing five lines; case 53) and `tests/conformance/python_bad/`
  (`bad.ry`, the two diagnostics; case 21) are under the three judges,
  the Renyi checker and compiler byte-equal on them; `tests/fixture.rs`
  holds their manifests canonical; `tests/library.rs` counts fifteen
  files; `tests/file.rs` and `tests/compile.rs` name the format numbers
  (3 read, 4 refused). The suite and the tests need a Python 3 on the
  PATH: CI's `setup-python` step moved before `cargo test`, and the
  release workflow has one before its suite.
- **The documents.** Decisions AL1 to AL4; the guide `docs/python.md`
  (the manifest, the declaration file and the types that cross, the
  program and the worker, what fails) in the site's navigation and on
  the front page; the reference (the module list, the manifest, the
  capability table, the scopes, the static rules, the run time,
  appendix A); the library sketch's section 15 (`std.python`; the two
  after it renumbered); the syntax sketch's capability list; the
  runtime and system documents; `extensions.md`; the cheat sheet
  (`python`, `std.python`; 2999 tokens after four words were trimmed);
  the starter skill; `README.md`; `CLAUDE.md`; the conformance
  `README.md`.
- **What bit.** `tags` and `count` are reserved words, as field names
  too, and a text literal cannot stand inside a hole (the first fixture
  tripped on all three); a purpose line is held to 100 columns like any
  line; `ability` is reserved as a parameter name; the `kind` field of
  `Raised` (above); the Renyi resolver's fixed list of library modules
  had to grow (without it the judges saw `std.python` as unknown and
  every bytecode file differed by the missing type);
  `declare_function` passed 60 lines with the second dispatch, hence
  `check_bound_signature`. After the gates, `cargo test` found what the
  cheat sheet's trim and the format number had left behind: the two
  copies of the cheat sheet (`crates/renyi/cheatsheet.md` under
  `tests/cheat_sheet.rs`, `starter/skill/renyi/cheatsheet.md` under
  CI's `cmp`) are copied again after every trim, `tests/library.rs`
  counts the library's files, and `tests/file.rs` refuses the next
  format by number; CI installed Python after `cargo test`, which the
  bridge's tests now need before it.

## Release 0.1 engineering (decisions AI1 to AI4; session 8, 2026-10-07)

Three commits after the positioning: 621fecc (the workflow, the
installers, the extension, the procedure), ee8b74b (the starter pack)
and the commit of the site and the crates.io metadata.

- **The release workflow** `.github/workflows/release.yml` runs on a
  tag `v<version>`: three native builds (`cargo build --release
  --locked` on ubuntu, macos and windows runners), each refusing a tag
  whose version is not the workspace version, the conformance suite
  run against each release binary, archives with `README.md` and
  `LICENSE` and a `.sha256` each, the VS Code extension packaged with
  `vsce` (node 22), then a GitHub Release with generated notes
  (`softprops/action-gh-release@v2`). `install.sh` and `install.ps1`
  download the archive for the machine, verify the checksum and put
  the binary in place (`RENYI_VERSION`, `RENYI_INSTALL_DIR`,
  `RENYI_REPO`). `editors/vscode/` is the extension: a TextMate
  grammar with dot-guarded keyword patterns, the language
  configuration, packaged locally once to check it. `docs/RELEASE.md`
  is the procedure: what a release is, release 0.1 step by step with
  the owner's steps marked, every release after it, where the pieces
  are.
- **The starter pack** `starter/` (decision AI3): `README.md` (install,
  the skill, the MCP server with its ten tools, the five workflows
  with the `needs` line of each `main`, how to review a program in one
  pass, record and reproduce); `skill/renyi/SKILL.md` (the loop, the
  rules the checker enforces with their codes, a module that checks
  clean, the commands, the library modules) with `cheatsheet.md` a
  copy of `docs/cheatsheet.md` that CI holds equal with `cmp`;
  `mcp.json`; `workflows/`: `repo_digest.ry` (git log by author,
  `std.process`), `api_digest.ry` (the GitHub API), `expense_report.ry`
  (CSV files to `out/report.md`), `log_triage.ry` (error codes
  ranked), `link_check.ry` (every link of a file fetched); their data
  under `data/`, the three recordings under `fixtures/` made with
  `renyi test --refresh` from live runs on 2026-10-07 (the recordings
  hold public data only: this repository's log, the GitHub record of
  rust-lang/rust, two pages of example.com and one of iana.org). CI
  checks, format-checks and tests them. Pitfalls met while writing
  them, for the next workflow: `count` is reserved (a parameter named
  `limit`), an `otherwise fail with` on its own line is `otherwise-line`
  (break inside the call's parentheses), `otherwise fail` on a `maybe`
  is `otherwise-fail-maybe` (`otherwise crash with` in a test), a test's
  head is one line that the formatter never breaks (shorten the test
  name), `purpose:` lines over 100 columns are shortened by hand.
- **The site** (decision AI2): `tools/site.py` renders every document
  under `docs/` but `HANDOFF.md`, the grammar, `examples/README.md`
  with each example's source and `starter/README.md` as pages with one
  header and relative links (46 pages; the `markdown` package;
  `python tools/site.py [_site]`); `docs/index.md` is the front page
  (the positioning's paragraph, install, the links);
  `.github/workflows/pages.yml` builds the site on every push to
  `main` and deploys it from a public repository or by hand
  (`workflow_dispatch`); on the private repository the deploy job is
  skipped, the build job still checks the script.
- **crates.io** (decisions AI2 and AI5): every path dependency carries
  its version through `[workspace.dependencies]` of `Cargo.toml` (cargo
  refuses to package without one). `renyi_check` embeds the standard
  library and `renyi` the cheat sheet; a crates.io tarball carries only
  the files under the crate, so each embeds a copy under its own
  directory (`crates/renyi_check/library/std/`,
  `crates/renyi/cheatsheet.md`), and a test in each crate holds the copy
  equal to the canonical file (`tests/library_copy.rs`,
  `tests/cheat_sheet.rs`); after editing a library declaration or the
  cheat sheet, copy the file again (the starter pack's cheat sheet
  too). `cargo package --workspace --allow-dirty` packages all seven
  crates and verifies the six libraries from their tarballs; on the
  binary it stops with a cargo internal error ("no hash listed for
  renyi_index v0.0.1", cargo 1.94.1), which comes from the temporary
  registry cargo builds for a workspace package and not from the
  crate: the extracted tarball `target/package/renyi-0.0.1/`, copied to
  the lane with its five dependencies pointed at the workspace crates,
  builds and runs (`renyi 0.0.1`). `cargo publish -p renyi` verifies
  against crates.io, where the libraries are by then. `cargo package` prints
  `--no-verify` in its usage, which the cc-enforcer hook refuses on the
  command line (it reads it as git's flag); use `--list` or the full
  verification instead, and never while another cargo command runs on
  the same target directory (a `cargo test` beside it failed to build).
- **Release 0.1.0, as it went** (2026-10-07, 20:00 to 23:30 UTC;
  `docs/RELEASE.md` section 5 has the lessons). The owner created the
  organisation and transferred the repository; the session renamed it
  to the lowercase `renyi` of AI4 (`gh repo rename`), registered the
  Pages site (`gh api -X POST repos/renyi-lang/renyi/pages -f
  build_type=workflow`; the organisation is on the Team plan), scanned
  the tracked files and the history for secrets and private paths
  (none), made the repository public (`gh repo edit --visibility
  public`), bumped the version to 0.1.0 (the fixture's `package.json`,
  the lockfile hash and two test strings followed; commit 868db78
  "Release 0.1.0"), published the crates with the login already on
  the machine (five at once, then crates.io's limit on new crates:
  one more every ten minutes, so `renyi_vm` at 22:46 and `renyi` at
  22:56 UTC), tagged `v0.1.0` on 868db78, and the release workflow
  built the three binaries, ran the conformance suite on each,
  packaged the extension and published the release page (about
  fifteen minutes; the Windows build the slowest). Checked after:
  the Site workflow deployed on the push of 868db78 and the pages
  answer; `install.sh` under WSL installed and ran 0.1.0 from the
  release; the Windows archive's checksum and binary were verified
  by hand; `cargo install renyi` built the binary from crates.io.

## The typed round (decisions AU1 to AU10; session 9, 2026-10-09), in progress

The owner asked, after the size round closed, for a cure of the
performance rather than another cut, and chose the route on
2026-10-09 (decision AU1, with the reasons and the profile's numbers):
the checker's static types brought to the code generator, as a typed
bytecode (format 5) that both emitters write. The root cause the
profile showed: the bytecode and the VM are dynamically typed under a
statically typed language, the baseline JIT recovers only the three
scalars, and the generated code is the interpreter unrolled (14% fewer
instructions than the interpreter on the self-check, 7% faster in
wall-clock; `primes` 9.7 times faster than CPython because it is all
Integers, `strings` slower than CPython because it is all boxed).

The profile that opened it (`valgrind --tool=callgrind` on the release
binary, `renyi run bench/strings.ry`, the JIT tier; the inclusive and
exclusive listings are in the decision): 1,149 million instructions,
966 per glyph over 1,188,895 glyphs; 43% the primitive boundary
(`rt_call`, `call_from_stack`, `call_primitive`, `run_primitive`,
`status`, `guarded`, `effect_of`, the drops of the arguments, the
scratch copy), 12% the search itself, 8% the iteration, 6% the retains
and clones of the constant and the glyph, 7% the generated code, the
rest the first half of the program (`rt_to_text` through `core::fmt`,
`rt_concat`, `append` through the boundary). Wall-clock on this
machine, best of five: the whole program 130 ms, its first half alone
33 (of which about 10 ms is the process and the compile), CPython 84.
The glyphs allocate nothing (the ASCII table of `Value::character`,
AQ), so inline small texts would save only the reference counts here.

The plan (AU1), each stage measured by AT1's rule with the benchmarks
against CPython beside it, a stage kept when the self-check's estimate
falls or a benchmark gains more than 5% without the self-check losing
more than 1%:

1. **The call to a primitive without the boundary.** A `Native` may
   carry a typed entry from a fixed family (`fn(&[Value]) ->
   Option<T>`, `T` one of `bool`, `i64`, `f64`, `Value`; `None` sends
   the call down the general path), called by the generated code with
   the arguments borrowed on the stack and the result in a register; a
   pure primitive without one runs on a flat path (`memory_check`,
   `plain_in_place`, the native, `guarded`, nothing else); an operand a
   typed call borrows is neither retained nor released when a `Load` or
   a `Const` pushed it. The interpreter takes the flat path too. The
   registration API gains the entry (an addition to AK2; the guide
   `docs/extensions.md`).
2. **The loop over a list in the generated code**: the list iterator
   in a fixed layout (`#[repr(C)]`: the items' address and count, the
   position, the list that keeps them alive), read and advanced by the
   generated code with a tag check and the helper as the fallback.
3. **The typed bytecode, format 5.** Both checkers record the type of
   every expression at its span (`Target::Typed`, the zonked type,
   skipped when a variable remains), as `Target::Result` and
   `Target::Number` already travel from `check.rs` and
   `compiler/bodies.ry` to `compile/` and `compiler/emit.ry`; both
   emitters write a type table per program (first use first) and per
   code object the type of what each op pushes (`emit` looks the span
   up, so no emission site changes); `file.rs`, `binary.rs`,
   `compiler/bytecode.ry`; the judges hold the two emitters equal; the
   VM checks the annotations against its inference of the three scalars
   on the corpus, the conformance suite and `compiler/` before it uses
   them.
4. **What the types buy**: the field read at a static index, the
   ability method chosen statically, the loop variable typed, the
   retains and releases elided wherever the consumer borrows.
5. **Later rounds**: the representation (small texts inline, `Int`
   flattened, lists of unboxed Integers, values in registers across
   ops).

Safety does not rest on the types: the generated code reads a tag
before every fast path and hands the frame to the interpreter when it
does not match (AR1), so a type is the choice of the fast path, never a
promise. The recordings, the replay, the narration and the `.ryc` as
JSON stay as they are.

**Stage 1 is in (decision AU2, 2026-10-09).** `Typed` and
`Native::with_typed` in `extension.rs` (`Registry::lookup_entry`,
`verify` holds an entry to the declared result's kind and to a function
without `needs`); the `plain_*` helpers in `natives/mod.rs`; forty-two
typed entries in `natives/prelude.rs` (the `_typed` functions beside the
natives); `Vm::call_scratch`, `call_pure_from_stack`,
`call_typed_in_place`, the tables `typed` and `pure`, `CallKind` and
`call_kinds` in `vm.rs`; `rt_call_pure`, `rt_call_typed` and the status
`BOXED` in `native/runtime.rs`; `borrowed_operands`, `kind_agrees`,
`Gen::call_typed`, the `calls`, `borrowed` and `masks` of `Gen` in
`native/codegen.rs`; `Jit::new` and `image::build` take the call kinds
(the latter the registry); `Pinned::forget_from`. Tests:
`tests/extension.rs` (a typed entry asked on both tiers, answering and
declining; `verify` on a wrong kind and on a function with `needs`),
`tests/native.rs` (`typed_entries_of_the_prelude_answer_or_decline_on_both_tiers`,
the decline of `absolute` on the smallest Integer and the hand-back of
its big answer). The guide `docs/extensions.md` has the entry. Measured:
the self-check's estimate on the JIT run 14.83 to 13.86 billion cycles
(-6.5%), the interpreter 17.92 to 17.23 (-3.9%; 12.85 to 12.21 billion instructions), the image 11.18 to
10.27 (-8.2%; 7.97 to 7.16 billion instructions, -10.2%; the machine code 5.62 to 5.63 MB, 122 bytes of body per op as before); `strings` 134 to 94 ms (CPython 81); the profile after the
stage in AU2: `rt_call_typed` itself 111 instructions a call, the
iteration 82 a glyph, the loop variable's retain and release 54, the
first half of the program a fifth of the whole (`core::fmt` for every
Integer written into a text, `append` freeing and allocating its `Rc`
each call).

**Stage 1b is in (decision AU3, 2026-10-09)**: the small cuts the
profile named. `rt_call_typed` syncs the pc only on the general path
and the error paths and drops nothing when every argument was borrowed
(`consume_arguments`); `integer::push_digits` and `digits` write a
machine-word Integer without `core::fmt` (used by `render` and
`op_concat`); `append`, `append_all`, `set`, `add` and `without` on
lists, maps and sets go through `Rc::make_mut` and keep the allocation
(`take_list` and `take_set` are gone from the prelude). Measured:
strings 94 to 84 ms (87 by `tools/bench.py`, CPython 79; 77 as an
image, past CPython), the self-check's estimate within half a percent
on every tier (13.86 to 13.83 billion on the JIT run); the profile 733
to 669 million instructions, 563 a glyph: the iteration 82, the search
72, the typed call's helper 80, the generated code 56, the entry's match
20, the loop variable's retain and release about 40, the rest the first
half of the program.

**Stage 2 is in (decision AU4, 2026-10-09)**: the loop over a list in
the generated code. `value.rs`: `Native` is `repr(C, u8)`,
`Native::Iterator(ListIter)` with `ListIter { items, len, position,
list }` and `ListIter::next`; `layout::TAG_NATIVE`, `NATIVE_TAG`,
`NATIVE_PAYLOAD`, `NATIVE_ITERATOR`, `ITER_ITEMS`, `ITER_LEN`,
`ITER_POSITION`, held to the types by
`a_list_iterator_lies_where_the_generated_code_reads_it`;
`native/codegen.rs`: `Op::IterNext` on an `Iter` slot reads the slot's
tag and the native's, walks a list iterator in place and falls back to
`rt_iter_next` for anything else; `runtime.rs`: `iterator_of` and
`iterator_next` through `ListIter`. Test:
`loops_over_lists_texts_sets_and_ranges_agree_on_both_tiers` in
`tests/native.rs`. Measured: the self-check's estimate on the JIT run
13.83 to 13.66 billion (-1.3%), the interpreter +0.4%, the image 10.22
to 10.08 (-1.4%; the machine code 5.63 to 5.70 MB, 123 bytes of body per op, the inline walk's price); strings 87 to 78 ms by `tools/bench.py` against CPython's
79 (at par; 76 as an image), 84 to 83 best of seven; the profile 669 to
625 million instructions, 525 a glyph.

**Stage 2b is in (decision AU5, 2026-10-09)**: `rt_call_typed` is the
fast path alone (`typed_call_general`, `typed_value_answer`,
`typed_over_memory` beside it; `Vm::memory_is_over`), `text_has` in the
prelude (a one-byte part in a text of at most 32 bytes by a plain loop,
for `contains` and its typed entry), `push_digits` pushes characters.
Measured: strings 83 to 78 ms (77 by `tools/bench.py`, CPython 79 to
82), the profile 625 to 589 million instructions; the self-check's
estimate on the JIT run +0.6% (the code's layout: instruction misses up,
instructions level), kept by the rule's second clause.

**Stage 3 is in (decision AU6, 2026-10-09)**: the typed bytecode,
format 5 (binary encoding 2, image format 4). `check.rs`: `infer` wraps
`infer_inner` and notes every expression's type (`typed`), `finish_body`
records `Target::Typed`; `compiler/bodies.ry`: `infer`/`infer_inner`,
`record_typed`, `TypedTarget`. `compile/mod.rs`: `emit` writes
`Code::types[at]` from `type_index_at` (the table `result_types`,
`type_index`), `emit_untyped` for the literal pattern's comparison
(`pattern.rs`); `compiler/emit.ry`: the same (`type_index_at`,
`type_index`, `typed_target`, `emit_untyped`, `types` on `Emitter` and
`Code`). `bytecode.rs`: `Code::types`, `Op::pushes_its_expression`;
`file.rs`, `binary.rs`: the field written, read and checked;
`native/infer.rs`: `abs_of_type` sees through a refined subtype;
`renyi_index/src/edges.rs` and `lsp/describe.rs` ignore the new target.
Tests: `crates/renyi_vm/tests/typed.rs` (the annotations against the
inference over the corpus, the clean conformance programs and the
compiler; more than half of the compiler's ops typed), the judges of
`selfhost.rs` hold both emitters equal on the new field (in 117 s),
`tests/compile.rs` and `tests/file.rs` on format 5. Measured: the
compiler's checker as a `.ryc` 8.99 to 9.67 MB; 65% of its 46,272 ops
typed, 425 types in the table; the self-check's estimate rose with the Renyi checker's own new work, see the decision. The VM
reads the types nowhere yet: stage 4 is what they buy (the field read at
a static index when the holder's type is known, the ability method
chosen statically, the loop variable typed from the list's item type,
the retains and releases elided where the consumer borrows, `Binary`
comparisons first).

**Stage 3b is in (decision AU7, 2026-10-09)**: the notes recorded
cheaply. `compiler/bodies.ry`: `infer` keeps a body's typed notes in
chunks of 64 (`typed`, `typed_chunks`, the binding `rotated`),
`record_typed` appends to `typed_references`, one list per body, which
`all_references` joins into the module's references once;
`compile/mod.rs`: `type_index` through `Context::type_indices`, a map.
Measured: the self-check's estimate on the JIT run 16.67 to 14.58
billion (6.0% above AU5's 13.75: the Renyi checker's own remaining cost
of the notes, see the decision), the interpreter 20.20 to 18.04, the
image 12.75 to 10.66; the judges in 102 s (117 at AU6). Noted for a
later round: `record with field: record.field.append(x)` copies the
list on every call throughout the compiler; an emitter that recognised
the update of a field of a uniquely held record would make the compiler
faster by a large factor on its hottest paths.

**Stage 4, steps A and B are in (decision AU8, 2026-10-09)**: the types
in the abstract state and the field read at a static index.
`native/infer.rs`: `Abs::Boxed(Option<u32>)` and
`SlotKind::Boxed(Option<u32>)` carry the type's index,
`Analysis::slot_types` (the loads' annotations, else the stores', else
the parameter's declared type), `abs_of_params` decides by
`param_type_indices`, `abs_of_field_at`, `static_field`;
`native/codegen.rs`: a `LoadField` or a `Field` whose holder has a known
record type checks the tag and the type and reads the field at its index
with no cache, `push_boxed` types what the op pushes, the landings, the
hand-backs and the state check against the analysis are keyed by the
kinds alone (`Abs::untyped`); `compile/mod.rs`, `file.rs`, `binary.rs`,
`compiler/bytecode.ry`, `compiler/emit.ry`: `param_type_indices` per
function, looked up in the finished table and never added to it
(`known_type_index`; format 6, binary encoding 3, image format 5);
`tools/image_census.py` reads image format 5. Measured: the self-check's
estimate on the JIT run 14.58 to 14.55 billion, the image 10.66 to
10.54. The profile of the JIT run (in the decision) puts the generated
code at 9.5% of the instructions, the interpreter at 23% (30 code
objects compiled), the reference counts at 17%, the copies of records
and lists under `with` and `append` at several percent more: the round's
next steps are there, not in what the generated code computes; step iii
of AU1 (the ability method chosen statically) has nothing to buy in the
compiler, which emits no `CallAbility` on its hot paths. The run with
every function compiled (the image) costs 28% less than the JIT run: a
cache of the image across runs would give `renyi run` that steady state
from the second run on.

**Decision AU9 (2026-10-09)** records the owner's answers after AU8's
profile: the image cache first, then, in this round, the update of a
uniquely held record in place, the comparisons on borrowed operands and
the parameters borrowed across direct calls; the ability method chosen
statically and the typed loop variable are dropped; the cold JIT run,
without the cache, stays the row that decides, the cached run reported
beside it. The hotness factor was measured again (4000 is 0.5% below
8000 on the self-check; it stays).

**Stage 5 is in (decision AU10, 2026-10-09)**: the image cache of
`renyi run`. `crates/renyi/src/cache.rs`: `directory()` (`RENYI_CACHE_DIR`,
else `renyi/images` under the system's cache directory; off under
`RENYI_NO_CACHE`), `Entry::of` (the name: a hash of this `renyi`'s
version, the host target and the program's binary encoding),
`Entry::lookup` (a hit is an image whose header fits and whose program
is byte for byte the run's, marked used by its modification time),
`Entry::store_later` (a lock file, then `renyi build --cache <path>`
spawned with no standard streams, in a process group of its own),
`Entry::store` (a `.part` file renamed, the lock removed, the least
recently used images pruned to 256 MB). `lib.rs`: `consult_cache` before
`run`, `record`, `test` and `reproduce` (not for an image, not on the
interpreter, not under `--no-cache`), the background store after a run
or a test file whose run compiled machine code (`Run::compiled` and
`TestReport::compiled` in `renyi_vm::runner`), `build --cache`
(`build_into_cache`), the flag `--no-cache`, the usage text.
`.cargo/config.toml` sets `RENYI_NO_CACHE` for everything cargo runs and
`ci.yml` for the workflow, so that the tests run the same way every time
(the conformance runner's runs under `RENYI_NATIVE_HOT=0` must test the
JIT); `tools/measure_size.sh`, `tools/measure_native.sh` and
`tools/bench.py` run cold, and `measure_size.sh` prints a fourth row,
`cached run`. Test: `a_run_that_compiled_machine_code_leaves_its_image_for_the_next`
in `crates/renyi/tests/build.rs` (the background store waited for, the
next run loads from the image, `--no-cache`, `RENYI_NO_CACHE`, an entry
holding another program's image is a miss, `build --cache`, `test`).
Measured: the self-check's estimate from the cache 11.23 billion cycles against
14.23 cold (-21%), the image alone 10.31; the cold row's instructions
unchanged (10.27 billion; its estimate moved from 14.55 through the
simulated data-cache misses, the binary's layout). Wall-clock with a
cachegrind run beside it: 2.2 to 2.5 s cold, 1.6 to 1.8 s from the
cache.

**Stage 6 is in (decision AU11, 2026-10-09, session 10)**: the update of
a uniquely held record in place. `bytecode.rs`: `Op::WithSlot { slot,
fields }` (the base is the record in the slot, moved out and updated in
place when held once, `Rc::make_mut` in `Vm::with`, which keeps the
allocation now in every case; the refinements checked by
`refinement_violation`, shared with `refined_record`) and `Op::TakeField
{ slot, name, site }` (the field moved out of a record held once,
`Nothing` left until the `WithSlot` fills it, else cloned;
`op_take_field` beside `op_load_field`, through the site's cache);
`file.rs` (format 7), `binary.rs` (encoding 4), `image.rs` (image format
6, code format 4: `rt_with_slot` and `rt_take_field` among the helpers),
`native/codegen.rs` (both through their helper; a take typed as a field
read, `unbox_top` after it), `native/infer.rs`; `compile/stmt.rs`:
`with_in_slot` (`change x to x with U`, `return x with U`),
`update_plan` (`Takes`: every update `f: x.f.method(args)`, the names
distinct, no argument pinning `x`; `Move`: no update may leave the loop;
`Copy`: as before), `taken_receiver`, `move_candidate` with the argument
form (`change x to f(..., x, ...)`), `pins` (the old `mentions`, which
now also counts a `break` or `continue` outcome: the fix of a latent
hole in O1's move); `compile/expr.rs`: `member` emits `TakeField` for
the `take_receiver`; `compiler/emit.ry`: the same rules (`with_in_slot`,
`returned_value`, `update_plan`, `taken_receiver`, `field_method_call`,
`move_candidate` with `moved_receiver` and `moved_argument`, `pins` and
its family, `is_take_receiver`, the `take_receiver` field of `Emitter`;
`format: 7`); `compiler/bytecode.ry`: `OpWithSlot`, `OpTakeField`;
`tools/image_census.py` reads image format 6. Tests:
`a_record_held_once_is_updated_in_place_and_its_aliases_keep_their_value`
in `crates/renyi_vm/tests/native.rs` (the op counts of every form, both
tiers against each other and against the expected text) and the
conformance case `record_update`
(`tests/conformance/programs/record_update.ry`, run from its bytecode
file and under the JIT too); the judges hold both emitters equal with
the rules (95 s). Measured by AU1's rule, both binaries on this machine:
the cold JIT run 14.23 to 14.16 billion cycles (-0.5%; 10.27 to 10.26
billion instructions), the interpreter 17.76 to 17.80 (+0.2%), the image
10.31 to 10.23 (-0.7%), the run from the cache 11.23 to 11.16 (-0.7%);
`tools/bench.py` best of seven alternating: `records` 53 to 52 ms, the
self-check 1,763 to 1,774 ms on the JIT run and 1,364 to 1,313 as an
image, noise. In the compiler's checker program 71 of 159 `with` sites
are updates in the slot, 17 with the field taken, and 429 values move
into calls where 180 did; counted on the self-check with the VM
instrumented for the purpose (not committed), of 206,427 updates run
70,922 are `WithSlot` and 23,848 find the record held once, and 2,305 of
8,258 takes move the field: the hot updates of the checker find their
record shared, by a `Done` record that carries the checker beside its
value (`let after be done.checker`, then `done with checker: updated`)
or by a binding still live after the statement, so the in-place path
rarely runs. The profiles before and after (callgrind) agree: `op_with`
and `refined_record` 209 to 161 million instructions (`with`,
`refinement_violation`, `op_with`, `op_with_slot`), the list copies in
`make_mut` 83 to 81 million, the drops of records unchanged.

**The owner's answers after stage 6 (decision AU12, 2026-10-09)**:
stage 7 now and stage 8 after it, the copies of the checker's shared
records left to the representation round (AU1's step v), where a
liveness pass over the bytecode would serve every form at once; the
round's second goal (the self-check a quarter faster than AT7's 14.83
billion estimated cycles) judged on the cold JIT run, with a new profile
after stages 7 and 8 deciding the stages that follow until the goal is
met or the owner closes the round; the front end written in Renyi
switched in, or not, after this round.

**Stage 7 is in (decision AU13, 2026-10-09, session 10)**: the
comparisons on borrowed operands, in the generated code alone (no change
to the bytecode, the emitters or the interpreter). `native/runtime.rs`:
`rt_compare` (the two operands compared where they lie on the stack:
small Integers, texts, Booleans, `Nothing` beside a value, fieldless
variants of a type without its own `equals`; the Boolean into the out
slot and the operands consumed, a borrowed one without a drop; anything
else the general path, `binary_values`, with the borrowed operands given
their reference and the boxed answer back as `BOXED`),
`compare_in_place`, `has_declared_equals`; `native/codegen.rs`:
`is_comparison`, `borrowed_operands` marks the operands of a `Binary`
comparison on boxed operands when a `Load` of a boxed slot, a boxed
`Const` or `Nothing` pushed them just before (the mask's bit 0 the left
operand), `Gen::compare_boxed` (the helper, the Boolean read into a
register on `CONTINUE`, `unbox_top` on `BOXED`, the exit on anything
else); `image.rs`: code format 5. Test:
`comparisons_on_borrowed_operands_agree_with_the_general_path` in
`crates/renyi_vm/tests/native.rs` (every shape on both tiers, the
tallies). Measured by AU1's rule against AU11's binary on this machine:
the cold JIT run 14.44 to 14.12 billion cycles (-2.2%; 10.26 to 9.95
billion instructions), the interpreter 18.07 to 18.09 (the instructions
equal, after `#[inline(always)]` on the three fast paths of
`binary_values`: with `rt_compare` as a second caller the compiler had
stopped inlining `text_binary`, +1% on the interpreter), the image 10.40
to 10.05 (-3.4%), the run from the cache 11.38 to 11.03 (-3.1%); the
profile: `rt_binary` and its `status` gone, `rt_compare` 307 million
instructions, `binary_values` 236 to 92 million, the operands' retains
and drops down; wall-clock noisy today, the self-check 2.57 to 2.42 s on
the JIT run and 2.00 to 1.77 s as an image best of seven alternating.
The stage stays by the rule's first clause.

**The owner's answers after stage 7 (decision AU14, 2026-10-09)**: the
parameters borrowed across direct calls (AU9's third item) are skipped
(2% retains and 2% releases in the profile, the parameters a part of
them; every exit of a frame would carry a borrowed mask); the next stage
is a faster compilation tier, measured first: 42 microseconds an op, two
thirds register allocation, no factor below 8000 pays on the
interpreter's thread; the owner chose the most thorough route, the
compile thread first (AU15) and a template tier decided by its
measurement; the measure of that stage is the self-check's wall-clock,
best of seven alternating, the cachegrind rows beside it.

**Stage 8 is in (decision AU15, 2026-10-09, session 10)**: the JIT's
compilation on a thread of its own. `native/mod.rs`: `Worker` (the
thread with its own ISA and contexts, a channel of requests and one of
results, joined on drop), `ProgramRef` (the program as the thread reads
it, with the safety argument), `State::Queued`, `Jit::queue` (the thread
started at the first hot code object), `collect` (the finished
compilations placed), `install` (what `compile` did after compiling),
`report_skipped`; `HOT_FACTOR_BACKGROUND` (100) beside `HOT_FACTOR`
(8000, the interpreter's thread); `Jit::new` decides the path:
`RENYI_NATIVE_SYNC`, `RENYI_NATIVE_HOT=0` or one hardware thread keep
the old one; the report line says which thread compiled, how many were
handed over and how many were still with the thread at the end. The
tests run as before (`RENYI_NATIVE_HOT=0` is synchronous); the corpus
tests run with the thread. Measured by AU14's rule against AU13's
binary: the self-check 1,727 to 1,527 ms (-11.6%) at the factor 100, the
sweep in the decision; confirmed on the final binary 1,723 to 1,548 ms
(-10.1%), `primes` -11%, the others within noise; cachegrind's JIT row
9.95 to 13.22 billion instructions (the compile thread's work counted in
the total; the interpreter, the image and the cached rows unchanged),
the reason AU14 changed the measure for this stage.

**The owner's answers after stage 8 (decision AU16, 2026-10-09)**: the
template tier is to be built as the next large item (measured after the
commit of AU15 with `taskset`, the self-check alternating best of seven:
on one hardware thread the old path runs, 1,835 to 1,850 ms, noise; on
two 1,831 to 1,718 ms, -6.2%; on four -10.1%); the stage now is the
liveness pass over the bytecode.

**Stage 9 is in (decision AU17, 2026-10-09, session 10)**: the last read
of a slot as a move. `crates/renyi_vm/src/liveness.rs`: `prepared`
(the program copied with the pass applied), `move_last_reads` (the
backward liveness over the ops of one code object: the successors, the
handler intervals, the reads and writes of slots, the fixed point over
bitsets, the rewrite of `Load` to `LoadMove`), five unit tests;
`Program` derives `Clone` (and `Types` with it); `runner.rs` applies it
in `run_measured` (every run and reproduction) and `run_tests`,
`native/image.rs` in `build` (the machine code of the prepared program,
the image's own program as the emitters wrote it), `crates/renyi/src/sandbox.rs`
at the sandbox's load. The generated code borrows a moved load for a
typed call or a comparison as it borrows a plain one
(`borrowed_operands` in `codegen.rs`: the slot keeps its value, which
nothing reads again, and the helper counts the copy out), since the
first build moved such operands out and released them after the call
where the plain load had cost nothing, which cost `bench/strings.ry`
10%; the native test `moved_loads_are_borrowed_by_typed_calls_and_comparisons`.
The bytecode file, the judges, the cache's key and `tests/typed.rs`
are untouched. Measured against AU15's binary on one boot, by
`tools/measure_size.sh`: the cold JIT run 18.29 to 17.89 billion
cycles by the estimate (-2.2%; -2.1% in instructions, the compile
thread's work counted in), the interpreter -3.0%, the image -3.3%, the
run from the cache -3.2%; in wall-clock, best of fifteen alternating,
the self-check -4.9% on four hardware threads, -6.8% on two, -3.3% on
one, `records` -2.3%, `json_round_trip` -2.2%, `primes` -1.1%,
`strings` within noise, and on the interpreter the self-check -7.7%
and `strings` -18%; the counts: 6,261 of the checker program's 9,611
`Load`s become moves, 13.0 of the 24.6 million run; the updates that
find their record held once 23,848 to 36,551 of 206,427, the takes
that move the field 2,305 to 3,491 of 8,266; the pass costs 5 ms on
the checker program (0.9 ms of it the copy). The first build moved
the operands typed calls borrow and cost `strings` 10%, the lesson
being that a moved load must stay borrowable (see above).

**The owner's answers on the template tier (decision AU18,
2026-10-09)**: every recommended option of the design study (the
section "The template tier: the design study" below): a small x86-64
assembler of the VM's own; template code at a code object's first
call, synchronously, on every machine; the hotness counted by the
template code with today's factors (100 with the compile thread, 8000
without) and the promotion to the Cranelift tier at a call or at a loop
header; x86-64 (Linux, macOS, Windows) in this round, aarch64 later.

**The template tier is in (decision AU19, 2026-10-09, session 10)**:
`crates/renyi_vm/src/native/template/x64.rs` (the assembler: `Reg`,
`Cond`, `Label`, `Asm` with thirty-six instruction forms, `Abi` with
`SYSTEM_V` and `WINDOWS_X64`, `host_abi`; unit tests of the encodings
and `every_form_dumped_on_request`, which writes every form to the file
`RENYI_X64_DUMP` names, for `objdump -D -b binary -m i386:x86-64 -M
intel`); `template/mod.rs` (`compile`, `Template`, the generator `Gen`:
the registers it keeps (`VM` rbx, `BASE` r12, `FRAME` r13, `HELPERS`
r14, `BASE24` r15, `COUNT` rbp, the address of the code object's count;
scratch rax, r10, r11, never an argument register of either convention),
`call` and `materialize` (a helper through `NativeState::helpers` by its
index in `codegen::SIGNATURES`, the frame pointer read again unless the
helper is in `STACK_SAFE`), `check` (a settled status: `FAILURE` to the
landing of the innermost handled region), `pushes` and
`leave_on_interrupt` (an unsettled one), `scalar_or_boxed` (the out slot
of `rt_call_typed` and `rt_compare`), `prologue` (the six pushes, the
frame of 40 or 88 bytes, the room check, the count's address, the entry
by pc), `block_lengths` (the basic blocks whose lengths `body` adds to
the count as each starts), `body`, `tail` (the landings, the back edges
that check the count and call `rt_osr`, the Boolean crash, the leave,
the epilogue), `op`, the sequence per op). `native/mod.rs`:
`TemplateCode`, `Jit::templates`, `template_tier`, `promotion`,
`template_entry` (made at the first call, packed into one executable
chunk by `CodeArena::place_packed`, `RENYI_TEMPLATE_DUMP` writes every
template's bytes), `entry` (the Cranelift code when ready, else the
template), `promoted_resume`, the report's third line. `runtime.rs`:
`rt_osr` (the last helper of `SIGNATURES`, code format 6), the
hand-back's handled regions in `rt_deopt`. `vm.rs`:
`NativeState::hotness` (the pointer to `Vm::hotness`, set once the VM is
built). `codegen.rs`: `DeoptPoint`'s `handlers`, a few items made
`pub(crate)` for the tier. `image.rs`: image format 7 (the regions of a
deopt point), with `tools/image_census.py`. Tests: `tests/template.rs`,
`tests/promotion.rs`, `tests/native.rs`'s
`a_hand_back_inside_a_handled_region_keeps_the_region`; the conformance
suite and the judges pass with `RENYI_NATIVE_TIER=template`. The defect
fixed on the way: the Cranelift tier keeps the handled regions static,
the interpreter on `Vm::handlers`, and a hand-back pushed none, so a
failure after a hand-back inside a region crashed as unhandled
(`guarded(4000000000)` in the test: the interpreter prints 7, the binary
of AU17 crashed). Measured on one boot against AU17's binary: the
synchronous JIT run (the path of one hardware thread) 13.99 to 13.12
billion cycles by AT1's estimate (-6.2%), the template code alone 16%
below the interpreter's estimate; in wall-clock the self-check -1.3% to
-4.8% on one hardware thread, -3.1% on two, neutral on four (the median
of twenty-five runs -1.1%, the best +1.9%), the benchmarks within 3%;
the image and the cached rows +0.8% and +0.6% from the binary's layout.
The default JIT row under cachegrind no longer measures anything (the
compile thread's timing gave AU17's binary 12.94 to 16.70 billion
instructions); judge by the synchronous row and by wall-clock. Two
corrections came from the first measurement: the first build counted the
whole code at every entry (88 code objects compiled on one thread
instead of 54, the self-check +6.6%), and it placed every template in
fresh pages of its own (564 mappings, every function at the same page
offset: +6.8% in wall-clock on one thread while cachegrind, which models
no TLB, said -5.9%); the count per basic block and the packed chunk
fixed both. The goal of AU18 is not reached: on one hardware thread the
JIT run takes 1,897 ms, the image 1,335 (+42%, against the 15% asked).

**The template tier's direct calls are in (decision AU21, 2026-10-10,
session 11)**, the first part of the owner's "1+2" (AU20):
`template/mod.rs`: `call_direct`, the sequence of `Op::Call` of a
declared function (the callee's entry from `NativeState::entries`, null
or at `DEPTH_LIMIT` native frames to the slow path; the entry kept in the
frame's spare slot; room on the stack for the callee's locals and in
`Vm::frames` for its record, `rt_room` and `rt_grow_frames` when short;
the stack's length set past the callee's locals; its fresh locals written
`Nothing` in place up to `CLEARS_INLINE` (4) and by `rt_clear_slots` past
that, since a Cranelift callee may hand its frame to the interpreter at
its trampoline's start, before its own prologue clears them, and the
interpreter drops them as values; the grant, the caller's read from its
record or `rt_frame_grant` when the callee narrows it; the record written
through `r8` and `r9`; `depth` up by one around the call through the
spare slot; then the status: 0 with the result where the arguments were,
a failure tag there landing as `check` lands it; anything else to the
cold `Handed`); the `Cold` enum, the rare paths emitted after the body
(`Call`: through `rt_call`, which makes the callee's template on its way;
`Handed`: `rt_finish_frame` for a frame handed to the interpreter,
`rt_abandon_frame` and the leave for an interrupt; `Failed`; `Field`:
`rt_load_field`, which fills the cache); the promotion at the start
(`rt_promote` when the count is past the threshold, since a direct call
no longer passes `Jit::entry`); `Op::LoadField` read in place (the
holder's tag record or variant, its type and tag against the site's
entry in `Vm::field_cache`, the field copied and `rt_retain_at`; no range
check, the index having been cached from a holder of the same type and
tag). `x64.rs`: three more forms (`add_m64i`, `sub_m64i`, `test_rr`;
thirty-nine). `runtime.rs`: `rt_finish_frame` (`Vm::execute` from the
frame's entry), `rt_abandon_frame` (as `run_top_frame` abandons on an
interrupt), `rt_promote` (`Jit::promote`, the count reset),
`rt_clear_slots`, the last four of `SIGNATURES`. `native/mod.rs`:
`Jit::entries`, set when a template is placed, when the Cranelift code
is installed (from the compile thread or synchronously) and when an
image is loaded; `Jit::promote`. `vm.rs`: `NativeState::entries`.
`codegen.rs`: the frame and site layout constants, `frames_offset`,
`handlers_offset`, `field_cache_offset`, `STATE_DEPTH` and the new
`STATE_ENTRIES` made `pub(crate)`; `rt_clear_slots` in `STACK_SAFE`.
`image.rs`: code format 7 (the four helpers and the state's `entries`).
Tests: `tests/direct.rs` (synchronous at the default factor: a short
callee promoted while its caller stays on template code, then called
with an Integer past the machine word, which deopts at the trampoline's
start with the fresh locals cleared by the caller; a callee that narrows
the grant and one that does not). Measured on one boot against AU19's
binary (bbea69b): in wall-clock, the two binaries alternating, best of
eleven, on one hardware thread the self-check 1,595.9 to 1,398.8 ms
(-12.4%), `primes` -3.2%, `records` -1.2%, `json_round_trip` -0.7%,
`strings` +0.5%; on two the self-check -2.6%, the benchmarks -0.2% to
-4.7%; on four -1.3%, the benchmarks -1.0% to -2.1%. By AT1's estimate
under cachegrind: the synchronous JIT run 12.87 to 11.71 billion cycles
(-9.0%; instructions -12.0%, the instruction misses +10% with the
larger code), the template code alone 14.50 to 12.18 (-16.0%; 29% below
the interpreter's 17.24, where AU19's was 16% below), the interpreter
and the image untouched (17.24 and 9.67 on this boot). The template code of the self-check's run grows from 1.58 to 2.76 MB
(the call's sequence and the field read in place, with their rare paths
after the body). The goal of AU18 is nearer, not reached: on one hardware thread the JIT
run, alternating with the image, takes 1,461.5 ms against 1,162.9
(+25.7%, best of fifteen; +21% by the estimate), where AU19 left it at
+42%; the goal is 15%. The run-to-run spread of this machine is wide
(the same binary's best of eleven was 1,398.8 ms in one series and
1,476.5 in the next), so compare a stage's binaries, and the JIT run
with the image, alternating in one series.

**The representation stages are in (decisions AU23 to AU27, session
11)**, in the order the owner set in AU22, each measured against the
binary before it on one boot:
- **AU23, the guard bookkeeping skipped** (2abc849): `Vm::guarding`,
  set by `begin_run` when the grant carries a guard and never cleared;
  while it is false the constructions (`MakeList`, `Construct`,
  `ConstructVariant` and their helpers) take their operands as they
  are, and `run_native` wraps a result only when there are origins.
  The synchronous JIT run -2.4% by the estimate; the self-check -1.0%,
  -2.0% and -6.5% in wall-clock on one, two and four hardware threads.
  A first build inlined `guarded`'s fast path everywhere instead and
  made `records` 3 to 4% slower in wall-clock with fewer instructions
  (moves of a 24-byte value that cachegrind does not price): call no
  wrapper at all rather than inline one. Test: `tests/guards.rs`,
  `a_guard_after_a_run_without_one_follows_the_data_into_a_record`.
- **AU24, a variant without fields read from its slot** (f26c1ca):
  `Vm::unit_variants` is a pinned vector, `Nothing` until a slot's
  variant is built, `vm::unit_slot` its index; both tiers copy a built
  one in place and raise its count with an inline increment; the
  helper builds it the first time. Code format 8. The synchronous JIT
  run -1.3%. Tests: `variants_without_fields_are_copied_from_their_slot_and_counted`
  in `tests/native.rs` and `tests/template.rs`.
- **AU25, one shared empty list** (1c27240): `Value::list` of no items
  shares a list per thread. Neutral (the estimate -0.03%): most of the
  self-check's empty lists are appended to later, which copies them out
  of the shared one. Kept by AU1's rule, if barely.
- **AU26, small texts in the value** (5be5e60): `Value::SmallText`, tag
  20, the length and up to 15 bytes in the payload, zero after them;
  `Value::text` and `Value::character` choose by the length (a text has
  one form), every reader takes either (`as_text`, `natives::text`,
  `plain_text`, the renderer, the comparisons, `op_concat`, the JSON,
  the file and the image's program, which write a text constant as
  before); equality and hashing by the characters; `copy_short` copies
  without a call (a first build's `copy_from_slice` called `memcpy` and
  `memset` per character and cost `strings` 7%); `is` and `is not` on
  operands noted as Text compare two small texts in place in both tiers
  (`compare_small_texts`, `infer::is_text`). Code format 9. The
  synchronous JIT run -7.4%, the image -9.0%; the self-check -7.8%,
  -7.4% and -6.8% in wall-clock. Tests: `value.rs` (every length, both
  forms) and `texts_held_in_the_value_compare_in_place_and_others_as_before`
  on both tiers.
- **AU27, records and variants in one allocation** (14ce593): `Composite`
  in `value.rs`, a thin pointer to one block (the count at 0, the number
  of fields at 8, the type at 16, the tag at 24, the fields from 32,
  `layout::RECORD_*`), `Deref` to `Shape` (a type with a slice tail),
  the count raised and lowered inline as an `Rc`'s, `Composite::free`
  out of line, `make_mut` and `get_mut` for the updates in place;
  `Vm::construct_from_top` and `construct_variant_from_top` move the
  operands from the stack into the block when the type has no
  refinement; the field reads of both tiers add the index to the block.
  Code format 10. Two corrections from the first build's profile: the
  drop was a call at every lowered count, and the clone's check against
  `usize::MAX` called `abort`, which gave `Value::clone` a stack frame on
  every path; the clone is a plain increment now, as the generated code
  makes it. The synchronous JIT run -4.6%, the image -6.1%; the
  self-check -10.5%, -3.9% and -4.4% in wall-clock, `records` -10% to
  -13%, `json_round_trip` within +2% (the decoder's vectors were copied
  into the blocks, 0.9% more instructions, until AU30). Tests: `value.rs` (the
  layout, the copy before an update) and
  `records_in_one_block_are_built_read_copied_and_compared` on both
  tiers.

Over the session, AU19's binary against AU27's on one boot: the
self-check 1,592.8 to 1,164.4 ms on one hardware thread (-26.9%, best
of eleven) and 1,507.0 to 1,186.4 on four (-21.3%); `records` -20%,
`primes` -6%, `json_round_trip` and `strings` level. The goal of AU18 as
AU22 measures it, the run part on one hardware thread
(`RENYI_NATIVE_REPORT`'s last line; `runpart.py` in the session's
scratchpad alternated the JIT run and the image eleven times): 1,112.2
ms against the image's 934.4 at the best (+19.0%), 1,213.8 against
1,045.2 at the median (+16.1%); the whole run +29.6%, the front end and
the loading 116 ms of the JIT run's 1,228 and 14 ms of the image's 948.
The goal (15%) is near and not reached.

**The owner's answers after AU27 and their two stages (decisions AU28
to AU30, session 12).** AU28: (i) the retain and the release in place
in template code next; (ii) after it, as a small stage, the natives
building a record in its block, the decoder of `std.json` first; (iii)
the front end's speed a round of its own after this one. Both stages
are in, each measured on one boot against the binary before it:
- **AU29, the counts in place in template code** (8b642fd): `retain`
  and `release` in `template/mod.rs` read the tag and test its bit in
  `COUNTED_OR_INTEGER` (`RC_TAGS` and the Integer's) with `bt`; a value
  with neither skips the rest with a short jump (`jcc_short`, the
  one-byte offset that `finish` refuses past its reach; `x64.rs` has
  forty-one forms now); a counted block has its first word raised or
  lowered in place; an Integer goes to `Cold::Retain` or
  `Cold::Release`, where a big one raises its block's count or goes to
  `rt_drop_at`; a count lowered to zero is raised back to one and goes
  to `rt_drop_at`, which frees. No image holds template code: the code
  format stays 10. The self-check's template code +24.8% (2.79 to 3.48
  MB, 91 to 114 bytes per op); by AT1's estimate the template code alone
  -5.2%, the synchronous JIT run -1.9%; the self-check -5.6%, -0.8% and
  -3.6% at the best in wall-clock on one, two and four hardware
  threads. AT6 had kept calls on the Cranelift tier against AR4's
  longer generic sequence; this sequence tests one bit and skips, and
  the indirect branches the model counts as missed fell 39%. Test:
  `counts_raised_and_lowered_in_place_free_the_last_and_spare_big_integers`
  in `tests/template.rs`.
- **AU30, the decoder's records in their block** (81aa432): `decode_fields` in `natives/json.rs` decodes the fields
  onto the VM's stack and the decoder's own `construct_from_top` moves
  them into the block with `Composite::from_top`; a refined type and a
  variant without fields go through `construct` and
  `construct_variant`; the stack is cut back on an error or an
  interrupt, and `decode` asserts it in a debug build.
  `json_round_trip` -3.9% in instructions, -1.1% to -2.0% at the best in
  wall-clock; the self-check's instructions equal. The first build
  called `Vm::construct_from_top`, which LLVM then stopped inlining into
  the interpreter's loop and `rt_construct` (+0.7% on the self-check);
  the second forced the inlining and the interpreter's loop compiled
  otherwise (+1.2% on the interpreter row); the third calls what was
  already out of line. Test:
  `decoded_records_are_built_in_their_block_and_keep_their_refinements`
  in `tests/semantics.rs` (no test had run the decoder's refined
  records before).
- **AU31**, the owner's answers after AU30 (6836ee0): AU29's sequence
  on the Cranelift tier next, then the hottest natives in place, each a
  stage; AU18's goal called met only after a measurement on another
  boot; the natives that build records from vectors left alone; the
  front end's round to start with a study.
- **AU32, the counts in place in Cranelift's code** (7353f11):
  `branch_on_count`, `big_integer`, `raise_count`, `retain` and
  `release` in `codegen.rs`, `COUNTED_OR_INTEGER` in `value::layout`
  for both tiers; a release has one call, shared by the
  last reference and a big Integer; `rt_retain_at` gone, code format 11.
  Four builds were measured (two calls per release; every Integer
  through the helpers; a big Integer's retain through the helper, which
  made the code larger and slower, since a call in a cold block costs
  the register allocation of the whole body; one call per release,
  kept). The machine code +33.6%; the image -4.6% by the estimate and
  -16% in wall-clock; the JIT row's estimate level within the layout's
  spread (two builds of the same code measured -0.3% and +0.4%); the
  self-check -9% to -15% in wall-clock on one to four hardware threads,
  `strings` -13% to -16%, `records` -4% to -7%. With it, `Worker`'s drop
  stops the compile thread after the compilation under way: closing the
  channel had let it compile its whole queue before the process ended
  (187 to 324 ms after `main` on two hardware threads with AU32's
  code). Test: the counts test in `tests/native.rs`, with a `maybe
  Integer` weight so that big Integers are counted in this tier's code.
- **AU33**, the owner's answers after the study of the typed calls
  (4951fec): of 2.77 million typed calls in the self-check, `List.at`
  1.39 million (its own work 46 instructions, the path around it about
  150), `Text.contains` 0.66 million and `List.contains` 0.16 (their
  search the bulk); `List.at` in place with the vector's layout probed,
  the two `contains` through lean helpers, the general path made lean
  after; AU18's goal met with AU30's binary, the ratio recorded only.
- **AU34, `List.at` in place and the lean helpers** (44782aa): `layout::list_layout` (the probe: a vector of known
  address, length and capacity, its three words read through the `Rc`)
  and `NativeState::list_items` and `list_len`; `CallKind::Typed { kind,
  in_place }` with `InPlace` and `natives::prelude::in_place`;
  `list_at_in_place` and `call_lean` in `codegen.rs` and in
  `template/mod.rs`; `rt_text_contains` and `rt_list_contains` with
  `DECLINED` in `runtime.rs`; code format 12. The synchronous JIT run
  -4.7% by the estimate (instructions -4.2%), the image -6.7%, the
  self-check -2% to -6% in wall-clock. Tests: the probe in `value.rs`;
  `list_items_read_in_place_and_contains_through_lean_helpers` in
  `tests/native.rs` and `tests/template.rs`. A lesson for tests of the
  Cranelift tier: a big Integer in a slot of the function under test
  hands its frame to the interpreter for good (the loop's entry is
  refused), so the code under test never runs; put it in a function of
  its own and look at `RENYI_NATIVE_REPORT`'s deopts and refusals.

AU32 moved the ratio of AU18's goal: on one hardware thread the image's
run part fell 16.4% and the JIT run's 1.8%, so that the JIT run over the
image went from +8.6% to +27.6% at the best. The JIT run's code is on
templates for most of the run; what makes Cranelift's code faster
widens the gap, which the template tier's own speed closes.

The goal of AU18 on this boot, AU29's binary: the run part +9.7% and
+9.1% at the best, +7.7% and +3.0% at the median, in two measurements
of eleven alternations (`runpart.py`); AU27's binary on the same boot
+11.0% and +13.6%, +12.2% and +19.7%. The goal is reached on this boot.
The spread between boots is wider than the margin (AU27's binary
measured +19.0% and +16.1% in session 11), so a measurement on another
boot confirms it before the goal is called met.

**The round closed after AU34 (decision AU35)**; what follows is what
was put to the owner before the close. AU33 (iii) ordered the
typed call's general path made lean next (the generated code releasing
the arguments it does not borrow and pushing the answer itself, the
entry called through a thin shim per kind of answer). After AU34 that
path carries 0.56 million calls of the self-check (`List.last` 0.18
million, `List.join` 0.16, `List.length` 0.07, the rest under 0.03
each) and about 63 million instructions of dispatch and value answers,
0.9% of the run, the most the stage can take back; so the owner is asked
first, in one batch: the stage as ordered; or `List.last`,
`List.length` and `List.is_empty` in place by AU34's means (a load or
two each); or the round closed and the front end's study opened (the
front end is 10.1% of the self-check's JIT run). The list below is the
one the owner answered after AU30, kept for its numbers. The profile of the self-check's
synchronous JIT run on AU29's binary, which AU30 leaves as it is (7.07
billion instructions): the front end that compiles the checker program
716 million (10.1%: `check_project_in` 357 million, `compile_project`
158 million, the parse and the loading the rest); Cranelift about 367
million (5.2%); the templates 121 million (1.7%); the machine code
itself 1.41 billion (19.9%); the values' lifecycle about 690 million
(`drop_glue<Value>` 335 and 136 million, `Value::clone` 218 million;
`rt_retain_at` 48 million and `rt_drop_at` 42 million, now from the
Cranelift tier's calls and the cold paths); the typed natives' path
about 510 million (`rt_call_typed` 181 million, `typed_value_answer`
146 million, `status` 181 million; `text_contains_typed` 71 million,
`list_at_typed` 64 million, `list_contains_typed` 45 million);
`memcpy` 253 million (the lists grown by `append`); `Value::eq` 168
million; `leave_frame` 154 million; `rt_compare` 152 million; the
allocator about 510 million (7.2%). The candidates:
1. **The hottest natives in place in the generated code**: `List.at`
   with a small Integer index (the bounds, the copy, the count, the
   `maybe` answer), `Text.contains` and `List.contains` on small
   values; both tiers and the image gain alike.
2. **AU29's sequence on the Cranelift tier**: AT6 compared the calls
   with AR4's generic sequence (the tag's bit, the big Integer, both
   payload words at every site); AU29's tests one bit and skips. The
   JIT run and the image would both gain; the code grows, so AT1's rule
   judges it.
3. **The compile cost on one hardware thread** (Cranelift and the
   templates, about 7% of the instructions): Cranelift's single-pass
   register allocator was tried and its code ran 11.6% slower; fewer
   code objects compiled, or less IR per op, remain.
4. **Deferred by AU22**: lists of unboxed Integers and the flattened
   `Int`, until a numeric program calls for them. **Skipped in AU14**:
   the parameters borrowed across direct calls (the hazard is a frame
   handed back to the interpreter, which drops its locals at the
   return).
5. **The round closed and the front end's round opened** (AU28 iii):
   the front end is the largest single share of the self-check's JIT
   run left to a round of its own (10.1%), and every program pays it
   on every run without the image cache.
6. **The owner's answers (AU31)**: 2, then 1, each a stage; AU18's
   goal called met only after a measurement on another boot; the
   natives that build records from vectors left until a profile points
   at them; the front end's round (5) to start with a study, the cache
   keyed by the sources' hash among its candidates. Item 2 is AU32;
   item 1 is the next stage, AU33: `List.at` with a small Integer
   index, `Text.contains` and `List.contains` in the generated code of
   both tiers, with the typed natives' path (about 7% of the
   self-check's instructions) the measure of what it can take.

Each stage is measured by AT1's rule (`tools/measure_size.sh` on the
release binary, about fifteen minutes now with the fourth row; in a
fresh container, and after a restart of the container, measure the
previous stage's binary too, built from its commit, since the simulated
cache misses differ between machines and between boots while the
instructions do not) against
the binary before it, with `tools/bench.py` beside it; the judges
(`cargo test -p renyi --test selfhost`, about 95 s) after every change
to an emitter. A lesson of session 10: never `git checkout <file>` to
drop a temporary instrumentation from a file with uncommitted work; it
reverted the whole file, which had to be rewritten from the session's
own edit script.

## The front end's round (decision AU35), the study first

The owner opened the round on 2026-10-10 (AU35), with the study of
AU31 (iv) first: the profile of the parse, the check and the compile on
the self-check and the corpus, with a cache keyed by the sources' hash
that skips the front end among the candidates (and, from AU35's report,
the copies `append` makes of a shared list, 3.6% of the self-check in
`memcpy`), then the owner orders the stages. The numbers it starts
from, on AU34's binary: the front end that compiles the checker program
before it runs is 10.6% of the self-check's synchronous JIT run (716
million instructions); `renyi check compiler/bodies.ry` takes 72 to 77
ms, `renyi parse --json` 78, `renyi compile` 103; `examples/hello.ry`
runs in 7 to 9 ms on the JIT, 5 from its image. The ratio of AU35 (ii)
on the same file: the Renyi checker 820 to 850 ms from its image (10.6
to 11.9 times), the parser 348 (4.4), the compiler 1,132 (11.0). A
faster Rust front end makes that ratio larger: the study says what each
candidate does to it.

**Why the Renyi compiler is 11 times the Rust front end's time**, as
measured for the owner on 2026-10-10 (callgrind on the same binary and
file, the Renyi checker from its image so that nothing is compiled):
the Rust checker runs 437 million instructions with 0.51 million
allocations, 72% of the instructions in the parse of the 13,280 lines
of `bodies.ry` and its eight imports, which it parses two and three
times over (the study below); the Renyi checker runs 4.99 billion
(11.4 times) with 5.0 million allocations and 36.5 million drops of a
value. Its instructions: the program's own machine code 31% (1.54
billion, 3.5 times the whole Rust run: a baseline JIT, no inlining
across functions, no escape analysis, boxed values in memory), the
counts 12.5%, the allocator 8.1%, copies 4.0% (immutable counted values,
a shared list or map copied when changed), comparisons through
`Value::eq` and `rt_compare` 7.2%, the list natives 7.2%, calls and
frames 5.5%, the rest of the runtime 16% (`status`, `rt_truncate`,
`Vm::with`). And the compiler's own style, which the library allows no
better: the lexer holds the source as a `List of Text`, one 24-byte text
per character, calls `char_at` for every character, classifies a
character with a search (`"abcdefghijklmnopqrstuvwxyz".contains(ch)`),
calls a closure per character in `scan_while`, and returns a record of
two lists per step. The levers the analysis names: a text method that
answers a code point as an Integer (a library addition), with the lexer
rewritten on it; interned names; a faster hash; and in the VM inlining
of small functions, escape analysis, parameters read without a count
(AU14's skipped item) and updates in place when held once.

**The study** (2026-10-10, AU34's binary on one boot: callgrind for the
instructions, the best of seven runs for the times; two study programs
beside a copy of `compiler/` in the session's scratchpad, not
committed: `phases.ry`, which runs the Renyi front end's phases up to a
named one on the files given, and `scan.ry`, a lexer's scan in one loop
over the characters held as texts and as Integer codes).

*The Rust front end.* `renyi check compiler/bodies.ry` (nine files,
13,280 lines, 442,656 characters) runs 437 million instructions in
71.6 ms. The parse is 314 million of them, 72%, because every file is
parsed twice and the main file three times: the resolver parses each
file to read its imports (`renyi_package::resolve::imports_of`, 118.5
million), the check parses each again (`check_project_in`, 118.8
million), and `diagnose` parses the main file before both (70.9
million); the library's fifteen declaration files take 5.9 million. One
parse of the nine files is about 28 million instructions of lexing and
91 million of parsing. The declarations (`World::resolve_all`) take 37.8
million, the bodies 63.8 million, and three passes of the declarations
clone a module's whole item list to walk it (`Vec<Item>::clone`, 23.8
million, 5.4%). Inside the parse, `TokenKind::eq` alone is 80 million
instructions, 18% of the run: 2.69 million calls of about 30
instructions, half of them `peek`'s test for a line break, since the
derived comparison of a kind that carries texts is a function the
compiler does not inline; the comparisons' temporaries are dropped
through `drop_glue<TokenKind>` (24 million), and `advance` clones the
token it returns, which 75 of its 85 callers drop (0.36 million
clones). In the self-check's run (`compiler/checker.ry`, 13 files and
19,511 lines; 716 million instructions, 10.6% of the synchronous JIT
run): the resolver's parse 188 million, the check's 184 million, the
bodies 100 million, the declarations 58 million, the library 6 million,
the bytecode 158 million, of which the function bodies are 68 million
and the table of the checker's recorded targets (`HashMap<Span,
Vec<Target>>` with the default SipHash, grown as it fills) about 38
million.

*The fixed cost of every program.* `renyi --version` takes 2.6 ms (a C
program that returns at once, 1.0 ms in the same harness; 1.6 million
instructions and the pages of an 18 MB binary). `renyi check` of an
empty program takes 6.2 ms and 11.3 million instructions, half of them
the standard library's: its fifteen declaration files (746 lines) are
parsed (6.1 million) and declared (3.0 million, 1.3 million of them in
the clones above) at every start. `renyi run examples/hello.ry` takes
7.2 ms (13.9 million instructions, 65% of them the library's), 4.9 from
its image, and 7.3 with the image cache, whose hit is decided after the
front end has run (AU10): the cache saves a small program nothing.

*The Renyi front end.* On the same nine files, from images: reading
them 77 million instructions (19 ms with the start), the characters and
the line starts 64 million more (9 ms), the lexer 1,555 million (203
ms: 3,500 instructions a character, about 56 times the Rust lexer's),
the parser 1,568 million (239 ms: 17,900 a token, about 17 times the
Rust parser's), the declarations, the bodies, the layout check and the
rendering 1,730 million (381 ms, about 15 times the Rust checker's
part); 4.99 billion and 850 ms in all, 11.9 times the Rust check on this
boot. The lexer's cost is its style more than the VM's: a scan in one
loop over the same `List of Text` that classifies by `contains` and
builds a record per token runs 255 million instructions, where the
lexer as written calls `char_at` for every character (0.64 million
calls), classifies by searches (`is_lower`, 0.39 million calls), returns
a `Step` record with two lists for every blank and every token, scans
through a closure call per character (`scan_while`) and looks every word
up in the list of the 88 reserved words. The same scan over Integer
codes ran 444 million, more than over texts: two Integers the generated
code holds boxed (read from a list, a field, an `otherwise`) are
compared through `rt_compare` (184 million of the 444, about 100
instructions a comparison), since the inline comparison wants both in
registers. The parser builds a `Peek` record at every look and a new
cursor at every step, compares a keyword as a text (`token.kind is
Word(spelling: spelling)` builds a variant and compares its text) and
calls `token_at` and `token_under` 0.82 million times.

*The candidates*, with what each does to the times and to the ratio of
AU35 (ii), the Renyi compiler's time over the Rust front end's in the
same binary (the Renyi compiler parses each file once already: its
`Source` holds the parse):
- A0, each file parsed once by the Rust front end: the resolver keeps
  the trees it parses, the check takes them, `check` takes the main
  file's from the same. `check` on `bodies.ry` from 437 to about 248
  million instructions (-43%, about 42 ms), the self-check's front end
  from 716 to about 528 million (-26%, -2.8% of the run). The ratio
  from 12 to about 20.
- A1, token kinds compared by their variant in the Rust parser
  (`matches!`, and an inlined comparison that looks at a reserved
  word's word), `advance` without the clone where the token is dropped:
  about 30% of a parse; with A0, `check` about 210 million (-52% in
  all, about 36 ms), the self-check's front end about 470 million. The
  ratio about 24.
- A2, a cache keyed by the sources, extending AU10's: an entry lists
  every file a compile read with its hash, the image the run left and
  the warnings the compile printed; a run whose files all hash as
  listed loads the image and runs no front end. `hello` from 7.2 to
  about 5 ms (-30%), the self-check about 100 ms less (-10%). The ratio
  unchanged (it is measured without the cache). Its questions: what
  else invalidates an entry (the binary is in the key; the manifest and
  the lockfile are files read), `record`'s code hash (kept in the
  entry), `test`.
- A3, the Rust bytecode compiler's tables with a faster hash, sized
  before they fill: about -25 million of the self-check's 158.
- A4, the library's fixed cost: the prelude and the modules a program
  imports declared, not all fifteen (the list of the modules kept for
  the diagnostics that name one), the item lists shared rather than
  cloned: `hello`'s 9 million to about 4 (about -1.5 ms of 7.2).
  Another way, larger: the library's declarations built once into the
  binary.
- B1, the Renyi lexer rewritten for speed with the library as it is:
  one loop, the tokens appended in place, characters classified by
  `contains` on short texts, the reserved words in a `Set`: from 1,555
  to an estimated 350 to 600 million (about -20% of the Renyi checker,
  850 to about 670 ms). A text method answering code points as Integers
  (`code_points()`, a library addition, not a change of the surface)
  gains only after B5.
- B2, the Renyi parser: keywords and symbols as variants without
  fields, compared by their tag (such a variant is a cached value since
  AG6), a cursor that builds no `Peek` per look: from 1,568 to an
  estimated 800 million (about -15%).
- B5, in the VM: a comparison of two boxed values the checker typed
  Integer, inline in both tiers (both small Integers: the payloads
  compared in place; else `rt_compare`), for every program that compares
  Integers read from a list, a field or an `otherwise`; the self-check's
  1.13 million `rt_compare` calls are about 2.6% of its instructions;
  the condition for a lexer on code points.
- B3, in the VM: inlining small functions on the Cranelift tier
  (`char_at`, `token_at`, `is_symbol`, `is_lower`: 0.1 to 0.6 million
  calls each), the largest lever after B1 and B2 on every program and
  the largest work of the list; B4, interned names and a faster hash in
  the Renyi checker's maps, for the checker's part.

*The ratio's gate.* B1 and B2 bring the Renyi checker to about 500 to
550 ms: 7 to 8 times today's Rust check, 14 to 15 times the Rust check
after A0 and A1. The 3 times of AU35 (ii) needs about 215 ms against
today's Rust front end and about 110 against it after A0 and A1: beyond
B1 and B2 it needs the VM's own work (B3 and what follows it: values
unboxed across calls, an optimizing tier). Two more effects to weigh:
the self-check is the program AT1's measure runs, so B1 and B2 change
the VM's measure (a copy of today's compiler kept under `bench/` would
hold the measure fixed), and A0 and A1 make the ratio of AU35 (ii)
larger without any change on the Renyi side.

**The owner's answers (decision AU36)** and the stages they set
(stage 1 is in, decision AU37: `check compiler/bodies.ry` -44% in
instructions and -35% in time, the self-check's estimate -2.4%; stage 2
too, decision AU38: the same check -23% more, -57% since AU34; the
first part of stage 3, decision AU39: the module trees shared, the same
check -17% more, -64.5% since AU34, an empty program's check -21%; the
second part measured and put to the owner, who ordered it and set A2's
design, decision AU40; the second part is in, decision AU42: an empty
program's check -35% in instructions and -14% in time, the thirty
examples checked -11%, `hello`'s bytecode 231 to 125 KB):
1. A0, every file parsed once by the Rust front end: the resolver
   (`renyi_package::resolve`, which parses each file only to read its
   imports, `imports_of`) keeps the trees, `check_project_in` takes
   them instead of parsing again, and `renyi check` (`diagnose` in
   `crates/renyi/src/lib.rs`) takes the main file's parse from the
   same instead of parsing it first; the outputs byte for byte as they
   are (the judges and the conformance suite hold them). The compiler
   in Renyi already works this way (`project.ry`'s `Source` holds the
   parse).
2. A1, the Rust parser's token comparisons by variant: `peek`'s test
   for a line break and every `== TokenKind::X` by `matches!`, `at` and
   `at_word` through a comparison the compiler inlines (a reserved
   word's word, else the variant alone), and `advance` without its
   clone where the caller drops the token.
3. A4, the library's fixed cost: the prelude and the modules a program
   imports declared, not all fifteen, with the list of the modules
   kept for the diagnostics that name one; the clones of a module's
   item list in `World::resolve_types`, `resolve_abilities` and
   `resolve_functions` replaced by a shared list.
   The second part (AU40 i): a program declares the prelude, the
   library modules it imports and those they import; a fix that names
   a library module not imported reads the whole library on that
   error's path only; both front ends in one commit, every program's
   bytecode changes.
4. A2, the cache keyed by the sources (an extension of AU10's image
   cache), as AU40 (ii) to (iv) set it: `run`, `record` and `test`; on
   a hit the cached image runs without the front end, the compile's
   warnings printed again, `record`'s code hash kept in the entry; an
   entry holds the content hash of every file read and the places a
   manifest was looked for and not found; on by default under the
   image cache's switch. What a hit loads (AU43): every run compiled
   from its sources leaves the program (AT3's binary encoding) in the
   cache keyed by the sources, a hit loads it in place of the front
   end, and AU10's image, left only by a run that compiled machine
   code, is then found by the program as before.
5. The copy of the compiler under `bench/selfcheck/` (AU36 iv) is
   made (decision AU41: the thirteen files of `compiler/checker.ry` at
   commit f1ba01b), with `tools/bench.py`, `tools/measure_size.sh`,
   `tools/measure_native.sh` and CI's checks pointed at it; the ratio
   of AU35 (ii) is still taken on `compiler/`. Every measure of AT1's
   rule from AU41 on runs `bench/selfcheck/checker.ry` on
   `bench/selfcheck/bodies.ry`.
6. B5, the comparison of two boxed values the checker typed Integer,
   inline in both tiers; B1, the lexer in one loop (tokens appended in
   place, classification by `contains` on short texts, the reserved
   words in a `Set`); B2, the parser's keywords and symbols as
   variants without fields compared by tag, a cursor that builds no
   `Peek` per look; B3, small functions inlined on the Cranelift tier.
   B1 and B2 keep the judges equal; B3 is held to the interpreter by
   the hand-back tests like every change to the generated code.
Each stage records, as AU35 (ii) and AU36 (iii) ask, the ratio of the
Renyi compiler's time to the Rust front end's on `compiler/bodies.ry`
(checking, parsing, compiling) against AU34's binary on the same boot
and against the stage's own binary.

**Where the stages stand** (end of session 13): A0 (AU37), A1 (AU38),
both parts of A4 (AU39, AU42), A2 (AU44), B5 (AU45) and B1 (AU46) are
in, the frozen self-check too (AU41), and B2's first cuts (AU47); `renyi check compiler/bodies.ry` 437 million
instructions and 72 ms on AU34's binary, 153 million and 34.5 ms now;
the ratio of AU36 (iii) 10.7 times against AU34's Rust front end (the
Renyi side has not changed yet) and 21.7 times against the same
binary's at AU42, 20.8 on AU44's boot, unmoved by A2, whose cache is
off where the ratio is taken. CI green after every push but e91cec0 (AU52's natives table, `tests/extension.rs`; fixed by the next commit) and d172887 to fa27d73 (the cheat sheet at 3,001 tokens by the gate's tokenizer; fixed in 81c5ef9).

**A2 as built** (AU40 ii to iv, AU43; decision AU44). The module
`renyi_package::reads` notes every file a compile reads (`noting`,
`read_text`, `unchanged`; the binary's own read of the main file goes
through `read_text` too). In `crates/renyi/src/cache.rs`, beside AU10's
`Entry`, `SourceEntry::of(path, library, directory)` names the entry
(`<32 hex digits>.rys`) by a hash of the binary (the version, the
executable's path, size and modification time, the library modules'
names and sizes), the working directory and the path as given;
`lookup(manifest)` reads the entry, checks `unchanged(reads)` and that
a manifest is held when one is wanted, touches the file and answers a
`Sources` (the reads, the warnings, the `Kept` manifest with the code
hash and the dependencies, the encoding); `store` writes a `.part` and
renames; `prune` counts both kinds of entry against the 256 MB. The
file is the line `renyi sources 1`, one line of JSON and the program
in AT3's binary encoding. In `lib.rs`, `compile_cached(path, no_cache,
manifest)` answers a `Loaded` (the program, a `Hashed::Kept` for the
manifest, no image, the encoding), used by `run`, `record`, `test` and
`build --cache`; `consult_cache` names AU10's entry from the encoding
(`Entry::of_encoding`) without encoding the program again; a hit
prints the warnings again. `RENYI_CACHE_REPORT=1` prints a line on a
hit and on a store. The tests are in `crates/renyi/tests/build.rs`
(a run leaves its program and the next runs no front end, an import
changed and a manifest made above are misses, `--no-cache` and
`RENYI_NO_CACHE` leave the cache alone; `record` keeps its manifest in
the entry and the recording reproduces). What a hit still pays, from
callgrind on `hello`: the binary's start (`renyi version`: 1.66
million instructions, the loader's relocations, mimalloc's reading of
the environment and the registry's natives table, 0.44 million, among
them), the files hashed again (sha256 in software on a machine
without SHA extensions, about 55 instructions a byte: 600 KB for the
self-check), the
encoding hashed for the image cache's name (AU10's cost, unchanged),
the program decoded (0.86 million for `hello`, mostly the function
metas) and the VM's start. The lesson of its first build: hashing the
library's text for the key cost more than the front end skipped
(decision AU44); a key must be cheaper than what it saves.

**B5 as built** (AU36, stage 6 of the list above; decision AU45). A
comparison on boxed operands calls `rt_compare`
(`crates/renyi_vm/src/native/runtime.rs`; `compare_in_place` answers
two `Int::Small`, texts, Booleans and `Nothing` where they lie, the
operands a `Load`, a `Const` or `Nothing` pushed borrowed per
`masks[pc]`, AU13). Before that call, when the checker's type of both
operands is `Integer` (`is_integer` in `native/infer.rs`, beside
`is_text`, from `Code::types`) or the analysis holds one as a small
Integer in a register (`Abs::Int`, a literal bound), the generated
code tests the value's tag and the Int's tag of the boxed ones
(`TAG_INTEGER`, `INT_SMALL`), compares the payloads at `INT_PAYLOAD`,
or the register's value, with the op's condition (`comparison_cc`,
shared with the register path) and takes the Boolean into its register
(`compare_small_integers` in `native/codegen.rs`, after
`compare_small_texts`), or writes it over the left operand on the
template tier (`template/mod.rs`, `integers_compared` and
`compare_small_integers`, where a `Const` lies boxed in its slot), the
boxed operands dropped without a release; a big Integer or another
type falls through to the helper. Code format 13. The lesson of its
first build: with two boxed operands required, the common shape, an
Integer from a list against a literal, never took the path and the
scan's count per turn did not move; check the shape a stage serves
under callgrind before measuring the rest. The tests: one on each
tier (`tests/native.rs`, `tests/template.rs`: every op, Integers from a
list, a field and a `maybe`, two big Integers among the values), the
judges and the conformance suite on the template tier. The
micro-benchmark `bench/micro/scan_codes.ry` (a lexer's classification
of forty codes read from a list, the shape B1 wants) is in
`tools/measure_native.sh`'s list. What it gave (AU45): `scan_codes`
33,793 to 17,525 instructions a turn on the Cranelift tier and 33,384
to 16,877 on the template tier, 55 to 26 ms in time; the self-check's
`rt_compare` 152 to 123 million instructions, its synchronous row
-0.55%, its time -2.2% on four hardware threads and -6.3% on one; the
Renyi checker on `compiler/bodies.ry` 806 to 756 ms, so the gate ratio
10.7 to 10.0 against AU34's binary (built again from 44782aa into
`target/renyi-au34` on this boot; build it again on the next).

**B1 as built** (AU36, stage 6 of the list above; decision AU46).
`compiler/lexer.ry`: `lex_range` is one loop (a blank skipped, a line
break taken, else `next_step` with the character already read and
the previous token kept in a binding); `Step` is a sum, `Single(token)`
or `Several(tokens, diagnostics, next)`, matched in the loop
(`last_of` keeps the previous token right after a `Several`); the
scans `scan_word`, `scan_name_tail`, `scan_alphanumeric`,
`scan_digits(underscores)`, `scan_spaces`, `scan_operator_tail` and
`plain_run_end` replace `scan_while` and its closures, each a loop
reading `chars.at(position) otherwise ""` and classifying by
`contains` on `lower_letters`, `upper_letters` and `digits` (module
constants), the likeliest first; `phrase_set` and `phrase_starter_set`
are `to_set()` constants beside the lists they come from (the lists
stay in `words.ry` since AU53: the sets are built from them on first
use), and a reserved word or a symbol is known by the one lookup that
gives its code (`words.word_codes`, `words.symbol_codes`); `lex_word`
builds its token from the text it sliced;
`previous_is_adjacent_dot` and the raw-text test match the kind
instead of building a variant to compare with, and test the position
first; `scan_segment` copies a plain run by one `slice`. The
measuring driver `compiler/lex_only.ry <file> [turns]` lexes a file
`turns` times and prints a count: the lexer alone is the difference
between three turns and one under callgrind (`RENYI_NATIVE_SYNC=1`),
983 to 514 million instructions per lex of `bodies.ry` over the
session's four cuts (AU46 records each). Three classifications were
measured before writing (a search in a short text 341 instructions a
character, a `Set` lookup 649, two orderings 305: the search stays).
The quick judge of the session, `renyi tokens <file>` against `renyi
run compiler/tokens.ry <file>` on every program of the corpus, the
conformance suite, `compiler/`, `bench/` and the starter pack, ran
after every cut (a script in the session's scratchpad; the real judges
are `cargo test -p renyi --test selfhost`). What the profiler
(`renyi run --profile compiler/lex_only.ry compiler/bodies.ry 3`)
leaves: `scan_word` 28% (the `contains` per character is the floor
with the library as it is), `lex_range` 21%, `lex_word` 9%, the rest
under 4% each. What it gave (AU46): the lexer alone 112 to 65 ms a
lex of `bodies.ry` (983 to 514 million instructions), the token dump
388 to 335 ms, the Renyi checker on `bodies.ry` 756 to 649 ms, the
gate ratio 10.0 to 8.9 (checking), the parse row 5.3 to 4.2, the
compile row 10.5 to 9.1, all against AU34's binary on this boot.

**B2 as built** (AU36, stage 6 of the list above; decisions AU47 and
AU48). `compiler/parser.ry`: the keyword and symbol tests match the kind
and compare the texts instead of building a variant per test, `token_at`
is an `otherwise` and `peek` reads its token itself (AU47); a look is an
index, `peek_at(cursor)` (the position, or past the line breaks the
layout rules skip: inside brackets, after a comma, before a continuation
word, the rule of the Rust parser's `peek`), `token_at` reads the token
at it and `moved(cursor, look)` is the cursor there (the cursor itself
when the look did not move), so nothing is built per look; a consumer
advances with `cursor with position: look + 1` (in place when held once,
decision AU11); `second_token(cursor, look)` looks past the look;
`bump`, `past_word` and `past_symbol` serve the callers that drop the
token, `advance`, `expect_word` and `expect_symbol` the ones that keep
its span (the last two read it back as `previous(after)`);
`expression`'s `otherwise`, `not`, `with` and `power` test their word
inline; `peek` and its `Peek` stay at 11 sites of rare paths (AU48 i).
The expression parser is one loop, `binary(cursor, lowest)`: a `not`
head with its operand when `lowest` allows, else a postfix expression;
then while the next token is a binary operator whose level lies between
`lowest` and a ceiling (`binary_operator` maps the token to its
`BinaryOp` in one match, `level_of` gives the level: `or` 1, `and` 2,
`not` 3, a comparison 4, `with` 5, additive 6, multiplicative 7, `power`
8, module constants), the operator is consumed and the right operand
parsed from the next level (from the same for `power`); a comparison
lowers the ceiling to 3 (one per operand), `with` takes its update list
and lowers it to 4, a `not` head lowers it to 2; `or_expression` and
`additive` enter at their levels (AU48 ii). The judges: the quick judge
of the session (`renyi parse --json` against `renyi run
compiler/parse.ry` on every program of the corpus, the conformance
suite, `compiler/`, `bench/`, the starter pack and the library with
`--declarations`, 117 files; `parse_judge.sh` in the session's
scratchpad), the four judges of `cargo test -p renyi --test selfhost`,
and 24 probes of the levels' edges written for the loop (valid and
malformed expressions through both parsers). What it gave: the parser
alone (`compiler/parse_only.ry <file> [turns]`, three turns less one
under callgrind with `RENYI_NATIVE_SYNC=1`; the default mode's compile
thread varies under valgrind and does not measure) 1,197 million
instructions per parse of `bodies.ry` at the stage's start, 1,039 after
AU47, 998.6 after the look as an index (-3.9%: the record was a small
part of a look's cost beside its calls), 809.6 after the loop (-18.9%;
-22% in the session, -32% for the stage, against the 800 the study
estimated); 96 ms a parse in the steady state, from 118.7 at AU47; the
ratio (`tools/ratio.py target/renyi-b5v2 target/renyi-au34`): checking
578.6 ms (from 630.3), parsing 231.7 (from 263.7), compiling 873.0 (from
909.8), the gate 6.4 times on a run where the reference binary took 90.2
ms (7.9 by the 73.3 it took at AU47's run: the Rust side's time moves
between runs, so a session compares its own runs). What callgrind leaves
on the three-turn run beside the generated code: the frees of values
11%, `rt_compare` 6.6% inclusive (the keyword tests, the lexer's
one-character comparisons), `leave_frame` 5.5%, `rt_construct` 4.5%, the
`with` copies 3.5%, `rt_is_variant` 2.2%, the reads of module constants
0.5%; the profiler (`--profile`, the interpreter) `peek_at` 10%,
`token_at` 7%, `parse` 6% (its two queries over the tokens), `binary`
4%, `bracketed` 3%, `postfix` 3%, `is_symbol` 3%, `moved` 3%,
`after_breaks` 3%. Not done: the keywords and symbols as variants
without fields (AU36's other cut for B2), bounded by `rt_compare`'s
share; it waits for the measure after the VM's cuts. A VM-side lesson
from this stage: `change x to f(x) otherwise fail` does not move `x`
into the call (`move_candidate` in `compile/stmt.rs` looks for a `Call`,
and the value is an `Otherwise`), so the callee's `with` on it copies;
only 8 such sites exist in `compiler/`, so the rule stays.

**The variant test in place, as built** (decision AU49, commit d6b686e).
`Op::IsVariant(tag)`, the test every `match` arm on a variant pattern
runs (`Load(slot)`, `IsVariant(tag)`, `JumpIfFalse`, from both
emitters), is in place on both native tiers: the value's tag read at its
address, a variant's block tag at `RECORD_TAG` through the `Rc` at
`PAYLOAD` compared with the op's, the value released where it lies, the
answer a Boolean (a register on the Cranelift tier, boxed where the
value lay on the template tier); a typed top is false without a read; a
`Guarded` value (tag 19, `TAG_GUARDED` in `value::layout`, pinned by the
layout test) takes the helper `rt_is_variant` as the cold path
(Cranelift's cold block with the stack's length stored for the pop; the
template tier's `Cold::Guarded`). Code format 14. The tests:
`tests/native.rs` and `tests/template.rs` each match fieldless variants,
variants with fields and a guarded JSON document (a file read through
`only to`, parsed by `json.parse`, whose result carries the origins
since `run_native` wraps it) in loops, against the interpreter. The
gates: fmt, clippy, the full tests, the conformance suite on both tiers,
the four judges with `RENYI_NATIVE_TIER=template`. The measures (the
scripts' logs in the session's scratchpad): the self-check's synchronous
JIT run 6,424.9 to 6,370.0 million instructions (-0.9%), the estimate
-0.5%; the image's run -1.1% (-1.4% by the estimate), the cached run the
same; the image +1.7% (166.7 to 169.9 bytes of body per op); the micro
benchmarks unchanged; the parser alone 809.6 to 798.0 million per parse
(-1.4%); the ratio: checking 546.0 ms against the reference's 72.7 (7.5,
the gate; AU48's run had the reference at 90.2 ms and the Renyi checker
at 578.6, a noisier run), parsing 226.3 (3.8), compiling 849.6 (8.3).
The measure's lesson is in the scripts now: `tools/measure_size.sh`
prints a `JIT run, sync` row (the comparable one) and
`tools/measure_native.sh` exports `RENYI_NATIVE_SYNC=1`; without it the
micro benchmarks of two binaries differed by up to 15% between runs.
What stays, a candidate for after B3: a test that borrows the slot
(`TestVariant { slot, tag }`), sparing the clone and the release per
arm, a new op through both emitters, the bytecode file, the binary
encoding, the interpreter and both tiers.

**B3 as built** (AU36, the last stage of the list; decision AU50,
commits fb0cc2d and 2cc361c). The owner's answers of 2026-10-11 to the
four questions of the study: a rewrite of the bytecode the Cranelift
tier sees; a hand-back inside an expanded callee rebuilds the frames;
callees of at most 20 ops, without a loop or `needs`, not recursive,
handlers, crashes and nested calls allowed (one more level), a caller
growing by at most half; the gate reconsidered after the stage.
`crates/renyi_vm/src/native/inline.rs`: `expand(program, code)` gives
`codegen::compile` the code object's ops with every small callee it
calls directly expanded (`Expansion { code, regions }`), or nothing; a
`Region` names the callee, the pc of the `Call` (now a `Jump` into the
region), where its prologue, its body and its end lie, the slot its slot
0 maps to and the region it is nested in. The caller's own ops keep
their pcs: the loop headers, the deopt points of the caller's ops, the
template tier's promotion and the interpreter see nothing. The regions
lie after the caller's ops: the stores of the arguments into the
region's slots (fresh slots past the caller's locals), the callee's ops
with slots, constants (appended to the caller's pool) and jump targets
moved, `Return` a jump to the exit (the region's slots released,
`LoadMove` and `Pop` each, then a jump to the op after the call), a
reachable `ReturnNothing` a jump to a tail that pushes `Nothing` first.
The rules (`callee_of`, `shape_of`): a declared function's code, as many
parameters as arguments, at most `CALLEE_LIMIT` (20) ops, no `needs`
(its frame would narrow the grant), no backward jump, none of
`IterInit`, `IterNext`, `ListPush`, `GroupInsert`, `GroupFold`,
`SortByKey`, `Deadline`, `CheckDeadline`, `MarkStack`, `UnwindStack`,
`Check`, `Fail` (`rt_fail` leaves the frame, which would be the
caller's), not the caller nor a callee the region lies in, an analysis
that settles, every reachable `Return` with exactly the result on the
stack and no handled region of the callee's own open; the candidates of
a caller are admitted smallest first while the budget lasts (half the
caller's ops; a region takes the arguments, the callee's ops, two per
local and one or three for the tails), then the calls inside the new
regions once more (`NESTING_LIMIT` 2). Around it: the generated code's
prologue stores the expanded height (the frame the caller pushed has the
code object's own locals, and the first helper would push into a
region's slot otherwise); an entry at a loop header moves the operands
past the regions' slots and writes `Nothing` into them
(`make_room_for_expansion`), `resume_at` reads only the caller's own
slots; the frame leave releases the caller's own locals alone, the
regions' holding `Nothing` at a return (2cc361c); the JIT keeps the
regions and a constant pool per expanded code object (`Jit::regions`,
`Jit::pools`; `NativeState::constants` points at the pool;
`Vm::constant_at` for the helpers); the image carries the regions (image
format 8, `ImageCode.regions`; `tools/image_census.py` reads it); a
crash's location inside a region is the callee's line
(`inline::location`, `Jit::inlined_location`, from `Vm::abandon`); a
hand-back (`rt_deopt`) at a pc past the caller's ops calls
`rebuild_inlined_frames`: the expanded frame's values (boxed by the
point's slots and stack) are split into the caller's frame, stopped
after the call (its locals, its operands below the call, the marks
recomputed; at a region's prologue the arguments already stored go back
and the caller stops at the call, which the interpreter makes), and a
frame per expanded callee around the point (its slots, the operand depth
its own analysis gives at that op, the handled regions open under the
frames that own them, the floors), with `vm.frame_grant` for each;
`Jit::rebuilt_frames` tells `rt_direct_after` how many frames lie above
the callee's, so a direct call site runs the callee's frame to its end;
a hand-back at a caller's own op drops the regions' slots and recomputes
the marks. The development aids: `RENYI_NATIVE_INLINE=1` (on; off by
default since the measure), `RENYI_NATIVE_INLINE_BUDGET=<ops>` (the
budget a caller gets, in place of the half rule; the tests set 400 so
that small programs expand), `RENYI_NATIVE_INLINE_LIMIT=<ops>` (the
largest callee), `RENYI_NATIVE_INLINE_DUMP=1` (every expansion with its
ops, why a call was not expanded, the regions' slot kinds, and every
hand-back's rebuilt frames and stack), `RENYI_NATIVE_IR=<code name>` (a
code object's Cranelift IR, in `codegen.rs`). Tests:
`crates/renyi_vm/tests/inline.rs` (the doors of an expanded callee, a
callee's own handler, a crash's line, the hand-backs in the body and at
the door, nested once; against the interpreter). The two bugs the stage
met and the probes that found them (the session's scratchpad, `au50/`):
the stack's length at entry (the interpreter sets it to the base plus
the code object's own locals, the expanded code assumed the expanded
locals, and the first helper pushed into a region's slot), and the
direct call's entry frame after a hand-back that built frames
(`rt_direct_after` took the frame count after the rebuild, so the
innermost callee's result came back as the direct callee's). The gates:
fmt, clippy, the full tests, the conformance suite on both tiers, with
the expansion on and off. What the half rule admits on the compiler (the
dump): the parser's three-turn run expands 24 callees into 10 of 34
compiled code objects, the checker's run 12 into 8 of 47; the budget
refuses 27 calls (`token_at` seven times, `is_symbol` three, `moved`
two), the size limit 32 (`peek_at` is 56 ops, `bump` 22, `second_token`
28, `level_of` 53), a loop 7, the recursion 2, a `fail` 1 (`past_word`,
`past_symbol`); `peek_at` (56 ops, a budget of 28) takes `end_token`
(15) and refuses its four `token_at` sites (19 each). The measures (the
logs in the session's scratchpad, `measure-au50.log`, `steady-au50.log`,
`remeasure-au50.log`): the binary with the expansion off is AU49's to
the fourth digit on the image and the cached rows (the synchronous JIT
run +0.3% in instructions and +1.4% by the estimate, all of it the
layout's instruction-cache misses); the expansion on against off: the
synchronous JIT run +0.6% in instructions, -0.1% by the estimate; the
image's run +0.7% and +1.3%; the cached run +0.6% and +1.2%; the image
+10.9% (190.0 bytes of body per op, 1,049 deopt points); the rules
loosened (callees up to 60 ops, a budget of 1,000): the JIT run +9.1%
and +8.6%, the image's run +6.1% and +10.8%, the image +121%; the parser
alone (three turns less one, `RENYI_NATIVE_SYNC=1`) 799.6 million
instructions per parse off, 845.6 on (+5.8%), 1,344.7 loosened (+68%),
the difference the compile (the same 34 code objects and 1,731 ops give
15% more Cranelift instructions and 65% more compile time, the register
allocation superlinear in a function's size); the steady state
(`RENYI_NATIVE_HOT=0`) 572.6 off, 574.5 on (+0.3%; 583.2 before
2cc361c), 635.6 loosened (+11.0%); the micro benchmarks a typed Integer
call 595 to 562, a record call 727 to 626, a field read and a record
built unchanged. The ratio (`tools/ratio.py target/renyi-au50
target/renyi-au34`): checking 577.1 ms against the reference's 68.7 (8.4
times, the gate) and this binary's 36.1 (16.0), parsing 215.9 against
60.8 (3.6) and 58.7 (3.7), compiling 882.4 against 106.3 (8.3) and 80.2
(11.0); the expansion off checking 583.0 against 78.8 (7.4) and 38.5
(15.2), parsing 213.2 against 66.9 (3.2) and 64.5 (3.3), compiling 821.1
against 101.0 (8.1) and 79.4 (10.3), on a machine loaded by the site's
build, where the reference binary itself took 68.7, 78.8 and 75.4 ms in
the three tables, so the tables compare within themselves and the
expansion's cost is the Renyi checker's 577 against 583 on a reference
13% apart; loosened checking 679.1 against 75.4 (9.0) and 37.4 (18.1),
parsing 250.4 against 62.3 (4.0) and 59.0 (4.2), compiling 988.0 against
108.9 (9.1) and 78.1 (12.6). By AU1's rule the expansion is off unless
`RENYI_NATIVE_INLINE=1`; the mechanism stays for the evaluation.

**After B3: the evaluation** (AU36's answer 4: "after B3, evaluate
again"). The distance to the gate: AU35 (ii) switches the binary's front
end to the compiler written in Renyi when it is within 3 times the Rust
front end's time on `compiler/bodies.ry`; the ratio stood at 10.0 at
AU45, 8.9 at AU46, 8.6 at AU47, 7.9 (6.4 on a noisy run) at AU48, 7.5 at
AU49, 7.4 with the expansion off and 8.4 with it on, the reference
binary 78.8 and 68.7 ms in the two tables at AU50, so AU36's list closed
with the ratio 2.5 times the gate, and B3, the largest lever of the list
on paper, gave nothing: AU21's direct call was already lean (a typed
Integer call costs 595 instructions a turn in the micro benchmark, of
which the expansion saves 33), and the expansion's own costs took the
rest back. Where the time goes now (the profiler on one parse of
`bodies.ry`, 1.0 million calls and 1.4 million primitive calls):
`lexer.scan_word` 15%, `lexer.lex_range` 12%, `parser.peek_at` 7%,
`lexer.lex_word` 4.5%, `parser.parse` 3.8%, `parser.token_at` 3.7%,
`parser.binary` 2.9%, the primitives `at` 2.6% and `contains` 1.9%; on
the three-turn parse under callgrind beside the generated code: the
frees of values 11%, `rt_compare` 6.6%, `leave_frame` 5.5%,
`rt_construct` 4.5%, the `with` copies 3.5%. The levers that remain,
none decided: (a) in the compiler's own code, `peek_at`'s four
`token_at` sites as one, the keywords and symbols as variants without
fields (AU48's deferred cut, bounded by `rt_compare`'s 6.6%), the
lexer's `contains` per character and the checker's loops over every
character for the line starts (`check_layout` 6.5% and `line_starts`
4.8% of the checker's interpreter samples) as library primitives; (b) in
the expansion, the borrowing of a region's boxed arguments (a parameter
the callee only reads aliased without a retain, some 30 instructions a
call), the admission of the hot sites (a snapshot of the hotness in the
compile request, or one region shared by a callee's sites with a return
dispatch), and the compile cost of Cranelift on a big function (a
cheaper allocation, or expansion only into small callers); (c) in the VM
beyond it, the study's items (values unboxed across calls, an optimizing
tier, interned names and a faster hash in the Renyi checker). The
questions put to the owner at the end of the session (AskUserQuestion,
four at once): whether the gate stays at 3 or moves; whether the next
lever is the compiler's own code (a), the expansion's refinement (b) or
the VM's deeper items (c); whether the expansion's code stays (off, for
the evaluation) or goes; and whether the keywords as fieldless variants,
the cut deferred at AU48, is taken first as the cheapest measured
candidate. The owner answered on 2026-10-11 (decision AU51, every
recommended option): the gate stays at 3 and the work goes on finding
levers; the next lever is the compiler's own code (a); the expansion's
code stays, off; the site's fourth round is final. The stage this opens
is B4 (stage 28 of the roadmap), the paragraph "B4 as built" below.

**B4 as built** (decision AU51; stage 28 of the roadmap, closed;
decisions AU52 to AU57, commits 4d8cccc, e91cec0 and the next). The
compiler's own code, cut by cut, each judged byte-equal by the four
judges and the binary's tests, measured by its driver under callgrind
(`compiler/lex_only.ry`, `parse_only.ry` and `checker.ry` on
`compiler/bodies.ry`, three turns less one or the checker's one run,
`RENYI_NATIVE_SYNC=1`): (i) `report.line_starts(text)` measures the
pieces of `Text.split("\n")` and `checker.check_layout(chars, starts)`
walks the lines from the starts, reading only each line's trailing run
(the checker 4,898.3 to 4,802.1 million instructions, -2.0%); (ii) the
parser's `Cursor` carries `look`, computed once by `at_position` and
`with_nesting` (from `look_from`, the former `peek_at`) wherever a
cursor is built at a new position or nesting, and read by every test
(the parser alone 799.4 to 792.9 million per parse, -0.8%); (iv) two
list primitives, `run_of(members, start, stop)` and `run_until(stops,
start, stop)` (the sketch, `library/std/prelude.ry` and the checker's
copy, the natives with typed entries in `natives/prelude.rs`, the cheat
sheet and its copies within the budget, a test in
`crates/renyi_vm/tests/semantics.rs`), and the lexer's seven scans as
one call each over a character class kept as a set (the lexer alone
513.9 to 506.6 million per lex, -1.4%; the checker 4,750.9, -3.0% for
the three cuts). The lesson of (iv): the loop was a small part of a
scan; the hash of each one-character text against the set is the rest,
and the primitive keeps it; a class table built once per set (an ASCII
table beside the set, or natives for the classes themselves) would spare
it, the next cut of the lexer if the profile still points there. (iii)
The reserved words, phrases and symbols as Integer codes (AU53):
`compiler/words.ry` holds the lists (the lexer imports them back), the
codes and a public constant per spelling; the lexer's `Token` carries
`code`, which the one lookup that replaced the set test gives it
(`words.word_codes.get(text) otherwise 0`, `symbol_codes.get(ch)`); the
parser's tests take a code and compare it in place, `binary_operator` by
a chain of such compares; 153 call sites rewritten by a scanner, the
`match` arms with literal field patterns left. The cut alone was slower
(the parser +1.7%, the lexer +10.9% through `make_token`, the checker's
run +3.2%): a module constant read was a call to `rt_global` on both
tiers, so the VM's part of the cut reads one in place (the table of
built constants the JIT owns, `GlobalSlot` behind
`NativeState::globals`, copied and retained in place by both tiers once
the VM has built the constant, the helper until then; a constant built
to a small Integer a literal in the Cranelift code, from the snapshot
the compile request carries; code format 15; a test per tier). After the
three steps: the lexer alone 494.4 million per lex (-2.4%), the parser
alone 764.6 million per parse (-3.5%), the checker's run 4,691.7 million
(-1.2%), all against the numbers before the cut; AT1's synchronous row
on the self-check 6,389.0 to 6,376.4 million instructions (-0.2%) and
9,002.1 to 8,955.4 million by the estimate (-0.5%). The ratio after the
four cuts (`tools/ratio.py target/renyi-b4v target/renyi-au34`):
checking 589.8 and 606.6 ms on two runs of this boot against the
reference's 82.5 and 74.9 (7.2 and 8.1 times, the gate; 7.5 after the
three cuts with the reference at 78.5) and this binary's 37.6 and 36.4
(15.7 and 16.7), parsing 221.4 and 229.4 against 61.9 and 67.4 (3.6 and
3.4), compiling 899.3 and 825.9 against 122.9 and 111.3 (7.3 and 7.4):
unchanged within the wall-clock's noise, which is wider than the cut's
1.2% in instructions. The fifth cut (AU54): `find_binding` and the
emitter's `lookup` walk their scopes by index (no reversed copy per
lookup), the lexer's end-of-file append in AU11's form, and the natives
`append` and `append_all` copying a shared list once with room (the
exact-length clone plus the reallocation on the push was the `memcpy`
under `realloc`); the checker's run 4,558.4 million with the VM's part
alone (-2.8%), 4,544.4 million with the two Renyi parts (-3.1%), the
lexer alone 491.4 million per lex (-0.6%). The sixth cut (AU55): the
last read of a local moves it (`LoadMove`, or `TakeField` for `x.f`), in
both emitters (`compile/liveness.rs`, the same in `emit.ry`, held equal
by the compile judge), with a taken field read in place on both tiers
(the helper had cost more than the moves saved) and the checker's code
reshaped by a script where it defeated the rule (27 stale reads of the
`checker` parameter now read the derived value, 113 `Done` values bound
before their checkers); the checker's run 4,470.1 million (-1.6%), the
parser alone 767.9 million per parse (+0.4%), the lexer alone 498.1
million per lex (+1.4%), the Rust front end's compile of the checker
+4.7% for the analysis. The seventh cut (AU56): the frame leave drops
only the slots whose tag says they hold a counted block
(`Pinned::truncate_values`, used by the generated code's `leave_frame`,
the interpreter's leave and `rt_truncate`; the checker's run -0.8%, the
parser alone -0.6%, the lexer alone -0.9%). The eighth cut (AU57):
`List.with_at(index, item)`, a library addition, the item at the index
replaced in place when nothing else holds the list, on which
`lists.replace_at` is one call where it built three lists (twenty call
sites in the checker, the declarations and the emitter; the checker's
run -0.9%); its other half, a text held in the value hashed as its two
words, measured slower under SipHash (the lexer alone +4.8%: sixteen
bytes cost more than a token's two or three) and is left out. The stage
closed there: the checker's run 4,898.3 to 4,391.6 million instructions
(-10.3%), the ratio 7.4 to 6.9, each cut one to three percent; the gate
of 3 asks for the checker 2.3 times faster still (AU58's measure: its
image run 3,080.0 million instructions against three times AU34's
`renyi check` of the same file, 442.8 million), which cuts of this size
will not give. The owner, asked with four options (a new backend, its
design study first; the gate moved to 5 and the front end switched
then; the backend's first stage at once, without a study; the speed
work paused for the roadmap's other candidates), chose the first
(AU58, the next paragraph). What B4 left, for the study to place: `for
each` without an iterator allocated per loop (263,438 per run,
`rt_iter_init` 79 million inclusive), a `Done` record per expression in
the checker (about 657,000 constructions per run, `rt_construct` 57
million and their frees as much), the same reshaping of `emit.ry` and
`declare.ry` for the compile row, a hasher of the VM's own for its maps
and sets (AU57), and the analysis's name sets as bits for the Rust
front end's 4.7% (AU55).

**Stage 29, the new backend: its design study first** (decision AU58;
stage 29 of the roadmap). Not begun: the session ended at the owner's
word before it. The study is a section of this file, written before
any code, then put to the owner (AskUserQuestion, four at a time, the
recommended option first), whose answers make an entry. Its parts, as
AU58 sets them:
(a) the attribution: the checker's run from its image, the gate's form
(`valgrind --tool=none renyi build --to <image> compiler/checker.ry`,
then callgrind on `renyi run <image> compiler/bodies.ry` with
`RENYI_NO_CACHE=1`; the image is built under valgrind because valgrind
hides AVX-512 and an image is refused on a CPU whose features differ;
3,080.0 million instructions on this session's last binary), split by
category: the generated code of each tier, the counts (in place and
through the helpers), the frames (`rt_call`, `rt_return`,
`leave_frame`, the push in place), the field reads and the constructs
(`rt_construct`, `rt_take_field`, the field cache), the iterators
(`rt_iter_init`, `rt_iter_next`), the natives by name, the allocator,
and the hashing and equality of maps and sets (`hash_one`,
`Hasher::write`, `get_index_of`, `PartialEq`); the same table for the
parse and the compile rows of the ratio;
(b) the design: the calling convention first (AU58 (4): which
parameters are only read, settled by an analysis in both emitters and
carried as a flag per parameter in the bytecode, both emitters equal
under the compile judge; scalars passed and returned in registers on
the Cranelift tier, where AR3's direct calls already pass register
arguments; what the template tier does with the flag; the hand-back
and the frames it rebuilds; the image format), then the values unboxed
across ops on the Cranelift tier by the checker's types (AU1's
annotations, `native/infer.rs`), the records laid out as structures
with their fields unboxed by type, and the counts that static ownership
elides (AU55's moves, the borrowed parameters), each with what it costs
the interpreter, the template tier and the images;
(c) the checker's own shape, which no backend fixes: the `Done` record
per expression, the scopes as lists searched by index, the hash of
each character in the lexer, with the share each costs and the rewrite
each needs (the judges hold the output, not the shape);
(d) the stages in order, each with its estimate against the gate (the
checker's image run at 1,328 million instructions, 2.3 times fewer)
and its measure (the checker's run under callgrind, the ratio, the
table by category, AT1's rows for the VM's part), with the levers
outside the backend placed among them (the VM's own hasher, the
iterator in two slots, the Rust front end's name sets).
Then the first stage the owner confirms. Done in session 14: the study is
the section "Stage 29, the new backend: the design study (session 14)"
below, and the owner's answers are decision AW1.

## Stage 33, the WASM target: the design study (session 14)

**What the owner ordered (AW28)**: the WASM target of M6 first among
the next stages, its design study before any code and the study's
questions in a batch of four; and the showcase game's need beside it
(AW28 (2) and (3)): a program that writes its own frame loop and
hands each frame to the host through `screen.present`, which suspends
the program and gives control back to the page until the next frame.

**What exists.** The decisions: A5 names "server-side WASI; browser
WASM" among the first-class platforms, H3 puts "AOT and WASM targets"
in M6, and AG1 (iv) set WASM aside as "another target, not a
speed-up". The machine: `rustup target list --installed` on 2026-10-13
had the two darwin and the two windows targets and no wasm32 one;
`wasm32-unknown-unknown` was installed in this session (`rustup target
add`, cargo 1.94.1); the machine has no `wasm-pack`, `wasm-bindgen`,
`wasm-opt`, clang or WASI SDK. The site: `tools/site.py` renders the
Markdown and the grammar under `docs/`, the corpus and the starter
pack's README into static pages, `.github/workflows/pages.yml` builds
them with Python alone (`pip install markdown`) on every push to
`main` and deploys them from a public repository, and no request
leaves a page. The crates have no features, no optional dependency,
no `cfg(feature)` and no `build.rs`: the JIT, the FFI, the HTTP
client, SQLite and mimalloc are built unconditionally, and the only
target gates are a dozen `cfg` sites (the executable of AS4, the cache,
the process module, the Python bridge, the template tier's
convention, the image's file).

**The blockers surveyed** (the greps of 2026-10-13, the survey of the
crates and of the third-party sources at the lockfile's versions, and
one trial build in the wasm lane, `D:\Projects\.worktrees\Renyi\wasm\`,
`check_asis.log`):

- *The front end builds as it is.* `cargo check --target
  wasm32-unknown-unknown` of `renyi_json`, `renyi_syntax`,
  `renyi_package`, `renyi_check` and `renyi_index` finished in 5.4
  seconds with no error: they depend on `regex` and `sha2` alone, pure
  Rust (`renyi_package` reads files through `std::fs`, which compiles
  on `wasm32-unknown-unknown` and answers "unsupported" at run time;
  every read of the resolver goes through one function,
  `reads::read_text`, where a virtual file system would go).
- *The VM does not.* The same check of `renyi_vm` stops in `getrandom
  0.2`, a `compile_error!` on the wasm32-unknown-unknown target
  ("you may need to enable the `js` feature"), reached through `ureq`,
  `rustls` and `ring`, before anything of the VM's own is compiled.
  Behind it: `region` (executable memory) and `libloading` (`dlopen`)
  have no wasm backend and fail to compile on both wasm targets, and
  `libloading::Library` is a field of `Vm` (`vm.rs:344`); `rusqlite`
  switches on wasm32-unknown-unknown to `sqlite-wasm-rs`, which
  compiles SQLite's C with `cc` (clang for wasm32 needed) and brings
  `wasm-bindgen` and `js-sys` into the lockfile, and the value type
  holds a connection (`value::Native::Connection`, `value.rs:494`,
  matched in `grant.rs`, `render.rs` and `vm.rs`); `cranelift-codegen`'s
  build script finds no ISA for wasm32; `memmap2` and
  `wasmtime-internal-jit-icache-coherence` compile with stubs; the
  binary crate's `libmimalloc-sys` has no backend for the target. Every
  one of these serves a feature the browser cannot have (the JIT and
  its images, the FFI, the HTTP client, SQLite, the allocator), so they
  go behind cargo features of `renyi_vm`, on by default and off for the
  web crate: the natives of a module left out are absent from its
  table, and a call to a declared function without a native already
  crashes with "`m.f` is not available in this build of the VM"
  (`vm.rs:1035`); the page's grant covers `screen` and `console` and
  nothing else, so `begin_run` refuses a `main` that needs more before
  anything runs, with the capability named, and the crash is never
  reached.
- *The JIT cannot be cut by dropping `mod native`.* `runtime.rs` holds
  the status codes the interpreter and the in-place shims use and the
  op helpers of `impl Vm` (`sync_pc`, the deadlines, the queries), and
  needs only `infer.rs` and `DeoptPoint`, both pure; `codegen.rs`,
  `template/`, `image.rs`, `inline.rs` and the `Jit`, `CodeArena` and
  compile thread of `native/mod.rs` are the gated part, with a `Jit`
  stub whose `new` answers `None` for the twenty sites of `vm.rs` that
  reach it through `Option<Box<Jit>>`; `runtime.rs:210` asserts `Value`
  at 24 bytes and 8-aligned, `RC_VALUE` and `list_layout` are 64-bit,
  all three the JIT's. The interpreter alone runs in the browser
  (`Options::interpret` is a mode the VM has: a narrated, a profiled
  and a budgeted run use it).
- *A bug for any 32-bit target.* `Composite::allocate` writes a
  record's type and tag at the fixed offsets 16 and 24 and its fields
  at 32 (`layout::RECORD_TY`, `RECORD_TAG`, `RECORD_FIELDS`,
  `value.rs:215-216` and `:560-563`), while `shape()` reads them through
  the `repr(C)` `Shape` placed at `size_of::<Head>()` (`value.rs:289`,
  the head two words); the two agree at 64 bits only, and on wasm32
  (the head 8 bytes, `TypeId` 4) every record and variant would read
  garbage. The offsets become multiples of the word size, the
  generated code's constants unchanged on x86-64.
- *The clock.* `std::time::SystemTime::now()` and `Instant::now()`
  panic on `wasm32-unknown-unknown` ("time not implemented on this
  platform"), and they sit on the main path: `Vm::new` seeds the random
  generator from the system time (`vm.rs:652`) and `now_millis`
  (`natives/mod.rs:74`) is called in `Vm::new` and `begin_run`
  (`vm.rs:718`, `:768`), at every live effectful call for the budget
  counters, at every `within` deadline (`vm.rs:2531`,
  `runtime.rs:1501`, `:2301`) and by `time.now` and `time.today`;
  `run_measured` calls `Instant::now()` unconditionally
  (`runner.rs:115`). A clock of the VM's own, `std`'s on the native
  targets and a function the host hands in on wasm32 (`Date.now()`;
  `performance.now()` for the elapsed times), takes the place of the
  direct calls. `time.sleep` sleeps the thread (`time.rs:212`) and
  `environment.current_directory` crashes when the system has none
  (`system.rs:73`): both behind capabilities the page does not grant.
- *Threads, files, sockets, processes.* `std::thread` in the compile
  thread (gated with the JIT), the profiler's sampler, the process
  module's pipes, `time.sleep` and the server's watch; `std::fs` in
  the filesystem natives (their errors are the `FileError` failure
  value already), the test fixtures and the images; `std::net` in the
  server (the listener of AO1 is a type in `vm.rs` and `runner.rs`,
  which compiles); `std::process` in the process module and the Python
  bridge, which answer `CannotStart` and `Unavailable`. All of it
  compiles on `wasm32-unknown-unknown` and fails at run time, and all
  of it is behind a capability the page does not grant: no stub is
  needed for the semantics, only the features for the build. The
  command's own thread of 64 MiB (`renyi/src/lib.rs:285-296`) has no
  place in a module; the wasm's stack is set at link time
  (`-C link-arg=-zstack-size=`, 16 MiB), since the render and the
  parser recurse on the Rust stack while Renyi's own calls do not.
- *The size.* The release binary is 10.4 MB with Cranelift, SQLite,
  the client and the FFI; the page needs the interpreter with the
  prelude, `std.console`, `std.time`, `std.random`, `std.json`,
  `std.csv` and `std.regex`, plus the loader of the bytecode file. The
  estimate before the build: 2 to 4 MB of WASM, 0.7 to 1.2 MB over the
  wire (Pages serves gzip); `opt-level = "s"` and `wasm-opt -Oz` are
  later cuts, not dependencies of the first build.

**The suspension at `screen.present`.** What the VM has: the frames of
a run are the VM's own (`Vm::frames`, the loop of decision X3), not the
Rust stack; a call of a declared function pushes a frame and continues
in the same loop (`call!`, `vm.rs:2122`), the fetch has moved the pc
past the op (`pc += 1` before the match, `vm.rs:2181`), and
`run_frames` leaves on an interrupt with the pc written back into the
running frame and the frames in place (`try_op!` and `sync!`,
`vm.rs:2060`); a primitive's arguments leave the stack before it runs
(`call_scratch`, `vm.rs:1046`) and its answer is settled onto the stack
after; `Interrupt` is the enum of the ways a run stops before the
program says so (a crash, an exit, the watch's reload, the memory
budget), and the watch of `serve` (decision AO1) is the precedent of a
native stopping the run from inside (`Interrupt::Reload` from
`server.rs:188`), after which the runner drops the VM. What stands in
the way: `execute` and `run_top_frame` hand every `Err` to `abandon`
(`vm.rs:1820`, `:1865`), which locates a crash and gives the frames up;
the boundary records nothing on an `Err` (`vm.rs:1248`), an exit
excepted; the template tier compiles a function at its first call, so
on x86-64 nearly every call runs as machine code unless the run is
interpreted; and about a dozen sites run a nested loop on the Rust
stack, inside which a suspension cannot be resumed: a module constant
built at its first read (`global`, `vm.rs:2659`), a refinement checked
(`holds`, `vm.rs:2822`), a declared `equals` (`equal`, `vm.rs:3086`),
a declared `compare` (`declared_compare`, `render.rs:276`, the sorts
and `largest`, `smallest`, `at_least` and `at_most` through it), a
declared `to_text` and a derived ability (`derived_ability`,
`vm.rs:2613`), a decode into a refined type (`json`, `sqlite`), and the
handler of `server.serve`, the only library function that calls back
into Renyi. The design: a fifth way out and a way back in.
`screen.present(frame)` hands the frame to the host through a field of
`Options` (a trait with one method, handed in as the streams and the
watch are). A host that can answer at once (the console host of `renyi
run`: it draws the frame as text at the terminal's size, reads a line,
and the letters of the line are the keys held, the elapsed time the
real one) returns the `Input`, and the program continues as from any
primitive. A host that cannot (the page, whose main thread must return
to the browser to draw) answers "later": the native returns
`Err(Interrupt::Suspended)`, which `execute` passes through without
`abandon`, `run_frames` having left with the pc past the call and the
stack without the arguments; the runner hands back a run that is
suspended, with the frame to draw; the host draws it, waits for the
next animation frame, and resumes with the `Input`: the VM pushes the
record where the primitive's answer would have landed, records the
call then (so that `renyi record` of a session is a demo and a
`replays` test replays a game), and runs the frames from the top one
at its pc through `run_frames` (the interpreter's entry, since the
generated code's entries are a code object's start and its loop
headers). Three rules: a suspension inside a nested loop (a counter of
them in the VM, and `native_state.depth` for machine code) crashes
with a message that names the construct; in this stage a run that may
suspend stays on the interpreter (`interpret`, as a budgeted run
does), because a suspension under generated frames would need every
compiled frame handed back to the interpreter at the call, which the
hand-back of AU1 does for one frame at a deopt point and the records
of AW24 prepare but no path does yet, and the browser has no generated
code anyway; and the console host answers in place, so `renyi run` of
the game keeps the JIT. The holder: `Vm<'p>` borrows its program, so a
suspended run is a session that owns the program and the VM over it,
dropped in order; `Sandbox::call` makes a VM per call today
(`sandbox.rs:395`) with `Options::default()` for the streams, and would
hand back `Suspended` with a `resume`; the C API later, `renyi_resume`.
The alternatives weighed: (a) Asyncify, binaryen's rewrite of the WASM
so that an import can unwind and rewind the whole stack: no change to
the VM, but a tool in the build (`wasm-opt`), the rewritten functions
slower and larger, and a mechanism outside the VM's knowledge that the
C API and a Rust host cannot share; (b) the VM in a Web Worker,
blocking inside `present` on a `SharedArrayBuffer` (`Atomics.wait`)
until the page writes the input: no change to the VM either, but
cross-origin isolation headers that GitHub Pages cannot set (a
service-worker shim is the known workaround, with a reload on the first
visit) and again a mechanism of one host; (c) the host-driven frame
function the owner declined in AW28 (3).

**The host interface.** A crate `crates/renyi_web/` of a few hundred
lines, `crate-type = ["cdylib"]`, built for `wasm32-unknown-unknown`
with the VM's features off, over the VM's own entry points (`Vm::new`,
`begin_run`, `run_main`, `call_function`, the program from `file::load`
of a `.ryc` or `binary::decode`, held `'static`), exporting a C ABI the
page's script calls: `renyi_alloc` and `renyi_free` for the buffers,
`renyi_load(bytes, len)` taking a bytecode file (the `.ryc` of decision
Z1, written by `renyi compile` at the site's build), `renyi_run()`
returning a status (finished, failed, crashed, suspended),
`renyi_frame()` the suspended run's frame as a flat buffer of shapes (a
kind, the integers and the colour of each, the labels' texts after),
`renyi_output()` what the program printed since the last call,
`renyi_resume(forward, backward, left, right, fire, quit, elapsed_ms)`;
and one import from the page, the clock. No `wasm-bindgen`, no
`wasm-pack`, no npm: the glue is one script of the site
(`docs/assets/play.js`, about 150 lines: fetch the two files,
instantiate, draw the shapes with `fillRect` and `fillText` on a canvas
of 320 by 200 scaled up, read the keys, `requestAnimationFrame`). The
page `docs/play.md`, rendered by `tools/site.py` with the canvas and
the script, a "Play" entry of the Learn group; the Pages workflow gains
a Rust step (`rustup target add wasm32-unknown-unknown`, `cargo build
--release --target wasm32-unknown-unknown -p renyi_web`, `renyi compile
showcase/raycaster/raycaster.ry --to _site/play/raycaster.ryc`), the
two files served by the site, so still no request leaving a page; CI's
Linux job builds the web crate. What the browser runs is the VM alone
on bytecode: the front end stays native, and the playground (the whole
toolchain in WASM, a visitor editing and running a program, which
needs the resolver's one read function over a virtual file system) is
a later slice.

**`std.screen`.** The library module (`library/std/screen.ry` and the
checker's copy; a new section of the stdlib sketch): the types
`Colour` (three channels), `Shape` (`Column`, `Rectangle`, `Label`
with the fields the game's contract has, `screen_x` and `screen_y`
among them, since the checker refuses single-letter names; the logical
screen of 320 by 200 as two constants) and `Input` (the keys held and
the milliseconds elapsed), and `present(frame: List of Shape) returns
Input needs screen`. The capability `screen` is a new root of the
built-in tree: `effects.rs`'s `TREE` and the mirror in
`compiler/effects.ry` (decision AD1), the reference's table, the cheat
sheet's line and the sketch, a library addition under decision V11,
not a surface change. The native (`natives/screen.rs`) reads the list
of variants into a frame of plain Rust and asks the host; the console
host of `renyi run` is in the binary (no window library, AW28 (2));
the game's modules switch from their own `shapes` and `input` to the
library's types once it exists.

**The measure of the stage**: the WASM's size raw and over the wire;
the interpreter's speed in the browser on `bench/primes.ry` and on one
frame of the game, counted as ops per frame and frames a second (the
demo's 237 frames run in 0.16 s on the interpreter natively, 0.7 ms a
frame; the column's width is the game's knob if the browser needs
one); and the suspension's cost per frame.

**The stages proposed**: 33a the VM for wasm32 (the features, the
composite's offsets, the clock, the web crate with `load`, `run` and
`output`, a program printing to the page's log, the size and the speed
measured, CI building it); 33b the suspension (`Interrupt::Suspended`,
the host in `Options`, the session and `resume`, the nested-loop
counter, the recording at the resume, the sandbox's `Suspended`, tests
on the interpreter); 33c `std.screen` (the module, the capability, the
native, the console host, the game on the library's types); 33d the
page (the frame and the input across the boundary, `play.js`,
`docs/play.md`, the Pages workflow), which closes stage 34's
integration with it. The questions put to the owner at the end of the
study: the suspension (the VM's own, Asyncify, or the worker), what the
browser runs (the VM alone on bytecode, the whole toolchain, or both
now), the toolchain (a C ABI by hand with no tool beyond cargo,
`wasm-bindgen` and `wasm-pack`, or `wasm32-wasip1` with a WASI shim),
and the gating (cargo features of `renyi_vm`, stubs and target `cfg`s
without features, or a core crate split out).

## Stage 32, the second round of the checker's own cuts (session 14)

**Stage 32 as built (decision AW26, 2026-10-13).** The reserve of AW21
(8), the owner's answer after B4: cuts in the compiler's own sources
(`compiler/*.ry`) on stage B4's binary, which the round leaves as it is.
The method: B4's check row attributed by code object (`study_map.py`),
the callers of `rt_record_copy` resolved to the code objects that copy
(the scratchpad's `copy_callers.py`, copied to `scratch_cuts2/`), a cut
written by a script, gated by `renyi check`, the canonical layout and
the four judges (`gate_cut.sh <label> b4`), and kept by the check row of
`perf stat` on the same binary (`quick_check_row.sh b4 <label>`, about
ten seconds). Four cuts kept: (1) the parser's result records read after
their cursor bind the other field first, 138 sites (`hoist_fields.py`);
the quick row 1,400.7 to 1,389.6 million. (2) `let X be Y with ...`
written as `let mutable X be Y` and `change X to X with ...`, 12 sites
in `bodies.ry`, `declare.ry` and `parser.ry` (`let_with_to_change.py`);
to 1,335.7. (3) `identifier`, `primary` and `word_primary` move the
cursor in the arms that take it, `clause_text` and the callers of
`with_nesting` bind the number first (`cut3_parser.py`); to 1,302.7. (4)
`is_word`, `is_symbol` and `token_at` inlined and removed, `moved`
inlined into its slot and in `peek`, `advance`, `bump`, `parse_block`,
`eat_word` and `eat_symbol`, `var_at` inlined in `resolve`
(`cut4_inline.py`); to 1,277.1. Measured and not kept: the four pinned
moves `at_position(cursor: current, position: current.position + 1)`
bound first (`unpin_moves.py`, no change), and an experiment on the Rust
emitter's liveness rule, a write at a block's own level killing the name
(`kill_rule.py`, the diff `kill_rule_experiment.diff`), no change: the
VM's pass of AU17 already moves every slot's last read by a dataflow
over the bytecode. The measure (callgrind, `renyi_b4`, `study/cuts2/`
against `study/b4/`): the check row 1,437.8 to 1,307.5 (-9.1%), the
parse row 670.0 to 629.5 (-6.0%), the compile row 2,598.6 to 2,468.1
(-5.0%); the ratios 2.96, 2.21 and 6.75, the check row within the gate
of 3 (1,324.8 million); the generated code 708.2 to 651.4, the counts
and drops 131.0 to 94.7, the records 46.1 to 61.1 (the updates in the
slot), VM other 89.2 to 59.3; the record copies 109,613 to 19,266; the
shim's calls 999,647 to 999,410. What the map leaves: `at_position` 56.6
million over 114,188 calls (the code per op), the lexer's loop and word
path 103 million (every character through `Text.contains` and two text
comparisons, every word through four natives), the shim (999,410 calls
at about 110 instructions), `find_binding` 19.0 million by a text
comparison per binding: the VM's items, or structural. The whole suite
382 tests. The scripts in `scratch_cuts2/` in the perf lane, with
`chain_cuts2.sh` and `chain_cuts2_wsl.sh`, the chain of a sources-only
stage (the three rows, the Rust rows, the table, the map, the helpers;
AT1's rows, the benchmarks and the micro programs unchanged by
construction), `numbers_cuts2.py` and `render_cuts2.py`.

## Stage 31, the new code generator (road (B)): the design study (session 14)

The study the owner's answer of 2026-10-12 asked for (decision AW20:
after every stage of AW1's order, the check row at 1,528.5 million
instructions and the ratio at 3.46 against the gate's 3, the owner
chose road (B) of AW1, the new code generator, over a second round of
the checker's own cuts, the emitter's round and the switch at 3.46).
Written before any code and put to the owner after it (the questions
and the answers are decision AW21). Its measure is AU58's, on stage
5b's binary (0f17f67, `renyi_5b` in the perf lane) under WSL: the
check row by category from `study/5b/table.check.txt`, the
interpreter's profile of the same run (`renyi run --interpret
--profile`, `study/5b/profile_interp.txt`), the Cranelift IR of the
hottest small functions (`RENYI_NATIVE_IR`, `study/5b/ir_*.txt`), the
micro benchmarks of `bench/micro/` (`tools/measure_native.sh`,
`study/5b/micro.txt`) and four micro programs written for this study
in the lane (`micro_b/`, measured per turn at two sizes by
`micro_b.sh` as `tools/measure_native.sh` measures, with the IR of
their loop functions in `study/5b/micro_b/`).

**(a) Where the run stands.** The check row 1,528.5 million
instructions: the generated code 800.8 (52.4%), the natives 255.8
(16.7%), the counts and drops 131.0 (8.6%), the VM's own 79.6 (5.2%:
`release_last` 51.9, `rt_record_copy` 18.1), mimalloc 76.8 (5.0%),
the texts and memory copies 74.3 (4.9%, `memcpy` 60.6 on 1,757,630
calls), the records 46.1 (3.0%), the hashing and equality 35.2
(2.3%), the load 10.0, the calls through the VM's path 8.5, the
comparisons through the helper 6.5, the glue 3.6. The generated code
by module: the parser 327.8, `bodies` 244.7, the lexer 144.2, `types`
41.1, `declare` 31.4; 2,136,802 calls enter bodies directly and 9,383
through the trampolines (the parser's function values, `argument`
6,501 and `parameter` 1,956 among them). The hottest code objects per
call: `at_position` 506 instructions for 77 ops (125,905 calls, 63.7
million), `token_at` 173 for 12 ops (171,276 calls), `moved` 116 for
12 ops (132,621), `is_symbol` 119 for 5 ops (105,097), `after_breaks`
369 for 18 ops, `find_binding` 911 for 42, `resolve` 263 for 41,
`next_step` 265 for 89, `substitute` 321 for 86, `lex_word` 631 for
138, `lex_range` 112,833 per call for its 140 ops (511 calls, 57.7
million, the lexer's loop). What the run does (the interpreter's
profile): 49,461,043 operations, 2,136,937 calls of declared functions
and 1,746,647 of primitives; by kind `LoadMove` 6.63 million (13.4%),
`Load` 6.42 (13.0%), `Store` 6.00 (12.1%), `Call` 3.87 (7.8%),
`LoadField` 3.20 (6.5%), `Binary` 2.83 (5.7%), `JumpIfFalse` 2.60
(5.3%), `Const` 2.49 (5.0%), `Return` 2.14 (4.3%), `Jump` 2.14 (4.3%),
`PushHandler` and `PopHandler` 1.09 each, `JumpIfTrue` 0.99,
`JumpIfAbsent` 0.89, `TakeField` 0.74, `ConstructVariant` 0.72,
`Global` 0.68, `Construct` 0.68, `IterNext` 0.60, `Dup` 0.53; by pair
`LoadMove LoadMove` 1.66 million, `Store Load` 1.46, `Load Load` 1.45,
`LoadMove Call` 1.45, `Load Call` 1.37, `Binary JumpIfFalse` 1.35,
`Const Binary` 1.29, `Store LoadField` 1.29, `Store LoadMove` 1.21,
`LoadField Store` 1.20. The loads, moves, stores, field reads, takes,
constants and dups are 26.0 million of the 49.5, more than half of
every op the run executes; the calls and returns 6.0 million.

**(b) What an op costs today**, measured per turn of a loop whose
range iterator, sum and jump stay in registers (the micro programs of
the lane; the counts are cachegrind's on the Cranelift tier, the
interpreter's beside them):

| program | the turn | native | interpreter |
|---|---|---|---|
| `calls_typed` | a direct call with an Integer argument and result, the callee `value + 1` | 120 | 1,112 |
| `calls_boxed` | a direct call with a record argument, the callee reading one field | 215 | 1,181 |
| `boxed_fields` | two typed field reads of a record parameter | 131 | 1,053 |
| `boxed_moves` | a record copied between two locals and back, one typed field read | 146 | 1,023 |

The loop's own turn (`IterNext` on a range in registers, two Integer
adds, the store, the jump) is about 12 of each. So a direct call with
a typed argument costs about 100 instructions beside its callee's one
op (AW1's finding 1 put it at 130 before stage 1), about 130 with a
record argument passed borrowed; a typed field read in place costs
about 58; a boxed local's load with its retain and a store with its
release of a record about 18 each (the two loads and two stores of
`boxed_moves`, 73 together). The IR says where they go. The field
read (`static_field`, decision AU8): the holder's tag and type tested,
then `push_copy`: the field's three words copied onto the VM's stack,
the retain (the tag's class tested, the small-Integer test, nothing to
raise for an Integer), the stack's length written; then `unbox_top`:
the value tested again and its payload taken into a register, the
length written back. A value the checker typed Integer crosses the
VM's stack and two tests to reach the register that holds it. A boxed
load: three words copied from the slot to the top, the retain (a tag
test, the class test, a branch, the block's count loaded, raised and
stored), the length written; a store: the release of the slot's old
value (the same tests, the count lowered, the test for the last
reference), three words copied, the length written. A direct call (the
5-op `is_symbol`: a field taken through the site's cache, a move, a
comparison in place, a return, 119 per call): the site loads the
callee's address from the direct table, tests and raises the depth,
writes the frame record (the code, the pc, the base, the handlers'
height, the grant: four loads and seven stores with the frames'
length), calls, reloads the stack's pointer, lowers the depth, tests
the status; the callee's prologue tests the stack's room, reloads the
pointer, writes the length, writes `Nothing` into every local that is
not a boxed parameter; its leave releases every slot that may own a
block (`owned_slots`), pops the frame record (three loads, two
stores), restores the handlers' height and returns the result in a
register. About 100 of `is_symbol`'s 119 are the call's; the cost of
the three ops that do its work is about 20.

**(c) The design.** Road (B) as AW1 named it: the operand stack and
the locals in registers and machine-stack slots, a value as its words,
the VM's stack written only where a helper reads it, the frame record
built lazily, the counts elided by dataflow, the hand-back rebuilding
the interpreter's frame from a map per point. AW1 weighed it as a
rewrite of `codegen.rs` with months without a measure; this study
finds the Cranelift tier as it stands after AW2 to AW19 carries every
mechanism (B) needs in a narrower form, so that (B) can be built on it
in four stages, each measured as AW1's were, with the judges and the
conformance suite green at every step: the abstract interpretation of
`native/infer.rs` already keeps small Integers, Booleans and Floats in
registers (`Abs::Int`, `SlotKind::Int`) and boxes them onto the VM's
stack before a helper that reads them (`box_position`); the hand-back
already carries the registers' values to `rt_deopt` through a buffer
described by a `DeoptPoint`; a pair slot (decision AW9) already holds
a record as two cells; the leave already releases by a static
ownership state (`owned_slots`, AW4) and the borrowed operands are
already a per-op property (`borrowed_operands`, AW5 and AW11). The
four stages, in the order recommended below:

*B2, the operands in registers.* A new operand kind `Abs::Held(type)`:
a boxed value held as its three words (the tag word, the two payload
words of the 24-byte `Value`) in SSA variables, with the type the
checker noted as `Abs::Boxed` carries it. Every op that pushes a boxed
value pushes a held one (`Load`, `LoadMove`, `LoadField`, `TakeField`,
`Const`, `Global`, `Dup`, a direct call's boxed result, a pair's
cells); an op the generated code performs itself reads the words (a
`Store`, a comparison in place, `IsVariant`, `JumpIfAbsent` and
`JumpIfFailure` on the tag, a field read of a held record, `Return`:
the result's three words go to the caller in registers, the status
beside them, as a typed result does today); an op that goes through a
helper writes the held operands the helper consumes onto the VM's
stack at their heights first (`box_position` generalised: three stores
and the length, exactly what every push costs today, so a value that
is pushed and consumed by a helper at once costs what it costs now,
and nothing is lost on the helper paths), and reads a helper's boxed
answer back into registers only when the op after keeps it. The
retain and the release work on the words (the block's count through
the payload word). The analysis settles per op where each operand
lies, as it settles `Int` today, so the generated code and the
hand-back agree: a `DeoptPoint` gains the `Held` kind, three words in
the buffer, which `rt_deopt` pushes as the interpreter's operand
(image format 9 to 10, since an image writes the kinds). The template
tier, the interpreter, the bytecode and the emitters are untouched.
The gain: the three-word copies to the stack and the length writes of
every boxed load, move, constant, dup and field read whose value stays
in the code (8 to 10 instructions each), and the typed field read's
crossing (15); with three quarters of the 26.0 million such ops boxed
(the checker's values are records, lists and texts; the Integers are
the codes and the positions, in registers already): about 180 million
(12%). The micro programs say what it should do: `boxed_moves` 146 to
about 80 per turn, `boxed_fields` 131 to about 45.

*B3, the locals in registers.* `SlotKind::Held(type)` for a boxed slot
the analysis settles (every store into it of a held or boxed value;
not an iterator, a mark or a deadline slot), its three words as
variables; a pair slot of AW9 is two held values and the `pair_*`
machinery folds into it. The slot's cell on the VM's stack is written
only before a helper that reads the slot by its index (`rt_with_slot`
and `rt_with_slot_by_index`, `rt_take_field` and `rt_load_field` on
their slow paths, `rt_iter_init` of a list in a slot), by the
hand-back (the buffer carries the held slots) and at a leave through
`rt_truncate`; the prologue writes no `Nothing` into them; the stack's
room is checked for the operands the helpers read alone; the entry at
a loop header (`resume_at`) loads the words from the frame as the
interpreter left them, three loads per slot and no helper; the leave
releases the owned slots from their words. The gain: a load's three
loads and a store's three stores (6 per op on the 14 million boxed
loads, moves and stores), the `Nothing` stores of the prologue (about
3 per call), the leave's releases from registers: about 90 million
(6%). After it the expansion of AU50 is measured again, since an
expanded callee's slots are registers then and AW19's loss (the
borrowed parameters and the pair return) is gone.

*B1, the frame record lazy.* A direct call pushes no record on
`Vm::frames`: the callee's machine frame holds a node (its code, its
base, the link to the caller's node, a flag whether its record exists)
and `NativeState` the chain's head; a helper that reads `Vm::frames`
(the grant of a callee that narrows, `rt_frame_grant`; `rt_call` to an
interpreted callee, whose frame goes on top; the hand-back; an
interrupt's location; `rt_osr`; the profiler and the narration run on
the interpreter and never see one) first materialises the chain's
unrecorded nodes into `Vm::frames` in order and marks them; a leave
pops its record when marked (one test). The enclosing grant is in the
body's signature already (AW6). The gain: the record's four loads and
seven stores and the pop's five at every direct call, about 20 of the
100: 2,136,802 calls, about 40 million (2.6%). The risk is in the
readers (26 sites in `vm.rs`, 18 in `runtime.rs`), every one audited;
the tests: the hand-backs of `tests/native.rs`, a crash's location
from generated frames, the grant stack under `--sandbox`, the
conformance suite on both tiers.

*B4, the counts by dataflow.* A pass over the held values after B2 and
B3: a retain at a load or a field read is elided when the value's only
consumer borrows it (a comparison, a typed or in-place native call, a
direct call's read-only parameter: `borrowed_operands` generalised
from `Load`, `LoadMove` and `Const` to every pushing op, `LoadField`
and `TakeField` among them) or when the slot is dead after it (a move
the emitters did not write, found again on the held values); a release
at a store is elided when the slot's old value is known plain or moved
out (AW4's state per op, already there). The gain: a retain and a
release on each field read that feeds a comparison (`token.kind is
X`: about a million, by the pairs `Const Binary` 1.29 million and
`Binary JumpIfFalse` 1.35), and the rest: about 30 million (2%).

The sum: B2 180, B3 90, B1 40, B4 30, about 340 million, the check row
to about 1,190 million, 2.7 times the reference, under the gate's 3
(1,324.8) with a margin of about 135 million. The uncertainty: the
boxed share of the loads and stores (three quarters assumed; the
profile counts by kind, not by representation), the register pressure
(Cranelift spills a held value to the machine stack, one instruction
a word, against the VM stack's three and the length), the hand-backs
(unchanged in number, three words more each). AW1's estimates summed
to 1,540 million and the stages measured 1,557 (3,085.3 to 1,528.5,
the checker's cuts among them), so the method has held once; if B2
and B3 measure at half their estimates the gate is out of reach by
135 million at B4's end, and the reserve is the checker's own shape
(the parser's result records read after their cursor, 186 sites,
`rt_record_copy` about 110,000 calls, and AU55's `Done` rewrite). The
stages that touch a format bump the image's (B2, B3) and nothing
else; the bytecode file, the emitters and the judges do not change.
The measure of each stage: AU58's with the table by category, AT1's
rows (the tiers change at every stage), the four benchmarks, the four
judges and the conformance suite on both tiers, and the lane's four
micro programs per turn, which say per op what the stage did; the
keep rule as the owner set it (the check row down by at least 0.5%, or
AT1's synchronous estimate down at all). The work, at session 14's
pace (AW2 to AW19 in one session): B2 the largest, every op's operand
side, the spill protocol, the hand-back and the image, about three to
four days; B3 about two; B1 about one; B4 about one.

**(d) What (B) does not touch**, for the road after it: the natives
(255.8 million, `list_append` copying shared lists, `slice_text`, the
character scans), mimalloc (76.8), the record copies of `with` on
shared records (`rt_record_copy` 109,613 calls, 18.1 million self and
most of `memcpy`'s 60.6), the frees (`free_block` 637,423 calls, 47.2
million; `release_last` 51.9), the hashing (35.2). After (B) the
generated code would be about 460 million of 1,190, 39%, and the
natives and the allocator the next targets; the checker's own shape
(the parser's `Parsed`, `Peek` and `Eaten` records read after their
cursor) is where the record copies come from.

**Stage B2 as built (decision AW22, 2026-10-12).** The first stage of
road (B): a value a load of a boxed slot, a boxed constant, `Nothing`,
a module constant, a field read or a loop's item pushes is held as the
three words of its `Value` in Cranelift variables (`Abs::Held` in
`native/infer.rs`, when `analyse` is asked to hold values; the
Cranelift tier is, the template tier and the expansion are not), and
the VM's stack is written only where something reads it there: an op
whose result goes on the stack writes every held operand under its
operands back first, three stores at the top each (`spill_below`), the
analysis spilling the same way at its `push!`; a helper that takes its
arguments from the stack finds them written there by `box_position`
(by index through `rt_insert_held` where values lie above); a direct
call reads a held typed argument from its words, a comparison in place
a held operand's tags and words from its registers, a test of the top
the tag from the first word, a store writes the words into the slot, a
`Return` unboxes a held result from the words or leaves it where the
frame's first slot was, and the counts are raised and lowered in the
words. Off the straight line a landing and an exit write every held
operand back by its index (`spill_all_held_by_index`, the landings
keyed by the whole operand stack, `exit_for_state`), the helpers that
leave the frame find them written back, a hand-back carries the words
in its buffer (`rt_deopt`, kind code 7, image format 10), and a frame
left releases the held operands below the result. The analysis
computes its entry states again from the final slot kinds once they
settle, so that no state carries a kind an earlier round's slot kinds
gave an op (a load of a slot boxed in a later round pushed a register
in an earlier one). The measure: the check row 1,528.5 to
1,482.3 million (-3.0%), the ratios by instructions
3.36, 2.39 and 7.28; the generated code
800.8 to 756.8 million, the counts and drops 131.0 to
131.0; the micro programs per turn `calls_typed`
119 to 120, `calls_boxed`
215 to 201, `boxed_fields`
131 to 123, `boxed_moves`
145 to 120; AT1's synchronous
row 5,161.6 to 5,195.4 (+0.7%) and its estimate
7,479.8 to 7,372.5 (-1.4%). The table is in
AW22. Against the study's estimate of about 180 million the stage took
46.2 million off the check row: a load that feeds a direct call's boxed parameter (`Load Call` and `LoadMove Call`, 2.8 million pairs in the interpreter's profile of the run), a `with`, a construct or a native that reads the stack still writes the value where the callee's frame, the update or the native reads it, three stores where the copy was three loads and three stores, and only the loads whose consumer reads in place (a comparison, a test, a store into a slot, a field read, a typed parameter, a `Return`) stopped copying; `at_position`, the hottest body, kept 267 of its 305 stores, its calls' arguments and its `with` among them. B3 moves the parameters into registers, and a direct call can then pass a held argument's words without the stack: the study's split of the estimate between B2 and B3 was wrong where its sum need not be, and B3's measure will say. Next in AW21's
order: B3, the locals in registers.

**Stage B3 as built, measured and left off (decision AW23,
2026-10-12).** The second stage of road (B): every value slot held as
the three words of its `Value` in Cranelift variables (`SlotKind::Held`
in `native/infer.rs`, a pair slot of AW9 as six words), the cell on the
VM's stack written only where the VM reads it (a helper that reads the
slot by its index, one that leaves the frame, an exit with the frame
alive, a hand-back), a direct call's value arguments in registers, and
one leave block per body releasing every slot from its registers; the
gates passed (the judges, the suite on both tiers, 410 tests). The
measure against B2: the check row 1,482.3 to 1,583.1 million (+6.8%),
AT1's estimate 7,372.5 to 8,061.6 (+9.3%), the image +45%, `calls_typed`
120 to 133 per turn; and against B2 with the leak below fixed
(`renyi_b2fix`, the check row 1,493.5): +6.0% on the check row, +9.0% by
AT1's estimate, the helpers equal and the whole rise in the generated
code (757.4 to 867.8 million, every body slower per call: three SSA
words per slot spilled around every call and moved through every block's
parameters). Left off by the owner's rule and answer; the diff is the
lane's `scratch_b3/b3_as_built.diff` (the fix and the test on top), the
binary `renyi_b3`, the study `study/b3/` and `study/b3_inline/` (AU50's
expansion measured again, +0.9%, left off). The leak its measure found:
since stage 2b (AW9) the owned analysis of AW4 stopped at the ops of an
`otherwise` region a pair's call skips, so a frame leaving by a loop's
exit skipped the pair slots the body stored (the parser's
`if_statement`, one `Cursor` per `if`, 233,339 blocks per checker run);
the fix in `owned_slots` and its test in `tests/native.rs` are AW23's
code; B2 with the fix is the base of B1 (the check row 1,493.5, +0.8%;
the ratio 3.38). Next in AW21's order: B1, the frame record lazy.

**Stage B1 as built (decision AW24, 2026-10-13).** The third stage of
road (B): a direct call between compiled bodies pushes no record on
`Vm::frames`; the callee keeps a node in its machine frame (`FrameNode`
in `vm.rs`: the caller's node, the code, the base, the grant, whether
the record is made; the pc a helper syncs while the record is unmade
beside the head, `NativeState::head_pc`), linked as `NativeState::head`
by the prologue of a body entered directly (`link_node`) and unlinked by
every return (`unlink_node`); `Vm::record_frames` makes the chain's
records in call order, cuts the links and leaves the head null (a null
head the whole test of whether the innermost frame has its record)
before anything reads a frame by its record (`push_frame_in_place`,
`interrupt`, `rt_deopt`, `denied`), and `effective_grant`,
`frame_grant`, `sync_pc` and `leave_frame` read the unrecorded head
instead; the leave in generated code pops the record only when made;
code format 24; the template tier, the trampoline, the interpreter and
the emitters untouched; the test
`direct_calls_make_their_frame_records_only_when_a_helper_reads_them` in
`tests/native.rs`. The measure against `renyi_b2fix`: the check row
1,493.5 to 1,482.7 (-0.7%), the parse row 685.3 (-0.5%), the compile row
2,656.9 (-0.6%); the ratio 3.36; the generated code 757.4 to 744.9;
AT1's synchronous row 5,228.4 (+0.8%) and its estimate 7,557.1 (+2.2%);
the image 10,383,184 bytes (+0.5%); `calls_typed` 120 to 116 per turn,
`calls_boxed` 202 to 199. Kept by the owner's rule (AW21). The binary
`renyi_b1` and the study `study/b1/` in the perf lane. Next in AW21's
order: B4, the counts by dataflow.

**Stage B4 as built (decision AW25, 2026-10-13).** The fourth and last
stage of road (B), three parts. A leak since AU13 fixed on both tiers:
the walk of `borrowed_operands` crossed a position another path enters
at, so that the value of an `if` expression or of an `otherwise`
compared at once had one branch's push borrowed and the other's, moved
out of its slot, released by nobody (100 KB a turn in the probe);
`jump_targets` in `codegen.rs`, and the walk stops before a target. On
the Cranelift tier a `LoadField` of a slot that is not a pair, pushing a
value, and a `Global` are pushed borrowed for a comparison, a typed or
in-place entry or a direct call's accepted parameter: the fast paths
skip the retain, the helper paths note the reference they take in the
variable `slow_owned`, which every consumer masks its static bits by at
run time (`runtime_mask`), and give the borrowed operands below them a
reference when the helper fails (`check_status_over_borrowed`); the
template tier keeps the shorter walk. A store releases the slot's old
value only where the owned analysis of AW4 says the slot may hold one
(`release_in_slot`). The interpreter, the emitters and the code format
untouched; the tests `tests/leaks.rs` (a program twice per tier under a
counting allocator) and two in `tests/native.rs`. The measure against
`renyi_b1`: the check row 1,482.7 to 1,437.8 (-3.0%), the parse row
670.0 (-2.2%), the compile row 2,598.6 (-2.2%); the ratio 3.26; the
generated code 744.9 to 708.2; the counts and drops 130.6 to 131.0;
AT1's synchronous row 5,172.5 (-1.1%) and its estimate 7,477.1 (-1.1%);
the image 9,561,248 bytes (-7.9%); `calls_typed` 116 to 117 per turn,
`boxed_fields` 122 to 122. Kept by the owner's rule (AW21). The binary
`renyi_b4` and the study `study/b4/` in the perf lane. The four stages
of road (B) are measured; what comes next is the owner's call.

## Stage 29, the new backend: the design study (session 14)

The study decision AU58 asked for, written before any code and put to
the owner after it (the questions and the answers are decision AW1).
Its measure is AU58's: the Renyi checker run from its image on
`compiler/bodies.ry` under callgrind, the image built under `valgrind
--tool=none` so that the CPU features match, the caches off; the
binary is the release build of 7cbbbe0 (the session's start) for Linux
under WSL, the reference AU34's binary built from 44782aa the same way.
The scripts live in the perf lane (`D:/Projects/.worktrees/Renyi/perf/`:
`study_callgrind.sh` runs the three rows of `tools/ratio.py` under
callgrind, `study_rust_rows.sh` the Rust front end's rows,
`study_table.py` sums a callgrind file by category and names the
helpers' calls, `study_map.py` names the generated functions by the
image's offsets and the bytecode file's code objects); they move into
`tools/` with the first stage.

**(a) The attribution.** The three rows of the ratio, in millions of
instructions, by category (the generated code is every function
callgrind sees at an address of the image's section; the allocator is
mimalloc, which the binary uses through the `mimalloc` crate; the
counts and drops are `rt_truncate`, the drop glue of `Value`,
`Composite::free`, `Value::clone` and the `Rc` drops; the frames are
the calls that enter generated code through the VM's path, `rt_call`,
`call_from_stack`, `push_frame_in_place`, `run_top_frame`, `Jit::ready`
and the trampolines' parameter helpers; the glue is `status`, which
every helper that answers a value returns through, and `into_plain`;
the records are the constructs, the updates and the field helpers; the
natives are the primitives with their calling helpers `rt_call_typed`,
`rt_call_pure` and `call_pure_from_stack`; the hashing is SipHash,
`hash_one`, `get_index_of`, `insert_full` and `Value::eq`):

| category | check | parse | compile |
|---|---|---|---|
| generated code | 1,011.8 (32.8%) | 348.1 (27.7%) | 1,347.4 (27.8%) |
| counts and drops | 448.2 (14.5%) | 113.3 (9.0%) | 775.8 (16.0%) |
| natives, with their calling helpers | 358.8 (11.6%) | 171.6 (13.7%) | 615.2 (12.7%) |
| allocator (mimalloc) | 258.9 (8.4%) | 114.1 (9.1%) | 389.3 (8.0%) |
| hashing and equality | 219.8 (7.1%) | 103.0 (8.2%) | 366.6 (7.6%) |
| records: constructs, updates, fields | 208.1 (6.7%) | 82.8 (6.6%) | 272.0 (5.6%) |
| helper glue: `status`, `into_plain` | 130.9 (4.2%) | 43.6 (3.5%) | 202.6 (4.2%) |
| comparisons and arithmetic | 124.0 (4.0%) | 44.0 (3.5%) | 300.4 (6.2%) |
| texts and memory copies | 121.1 (3.9%) | 98.3 (7.8%) | 215.9 (4.5%) |
| frames: the calls through the VM | 114.1 (3.7%) | 42.9 (3.4%) | 123.5 (2.5%) |
| load: the image and the program | 46.2 (1.5%) | 87.4 (7.0%) | 124.3 (2.6%) |
| iterators | 40.6 (1.3%) | 0.1 (0.0%) | 63.1 (1.3%) |
| other | 2.9 (0.1%) | 7.9 (0.6%) | 48.7 (1.0%) |
| **total** | **3,085.3** | **1,256.9** | **4,844.9** |

The Rust front end on the same file: the reference (AU34's binary)
`check` 441.6, `parse --json` 284.7, `compile` 365.6; this binary
178.8, 266.8 and 471.1. The ratios by instructions, against the
reference: checking 6.99, parsing 4.41, compiling 13.25; the gate of
AU35 (ii) at 3 asks for 1,324.8, 854.1 and 1,096.7 million: the check
row 2.33 times fewer, the parse row 1.47, the compile row 4.42. The
wall-clock ratio of `tools/ratio.py` puts the compile row near 7
(AU55: 922.5 ms against 143.3) because the Rust `compile` spends its
time writing the 6 MB bytecode file, not in instructions; by the
gate's own measure the compile row is the far one.

*What the run does* (the interpreter's profile, `renyi run --interpret
--profile`): 58,089,897 operations, 2,770,541 calls of declared
functions and 2,056,082 of primitives; by kind `Load` 8.21 million
(14.1%), `LoadMove` 7.23 (12.4%), `Store` 6.32 (10.9%), `Call` 4.82
(8.3%), `JumpIfFalse` 3.80, `LoadField` 3.09, `Binary` 3.07, `Return`
2.77, `Const` 2.48, `Jump` 2.47, `PushHandler` and `PopHandler` 1.36
each, `IsVariant` 1.35, `JumpIfAbsent` 1.16, `TakeField` 1.10,
`JumpIfTrue` 0.97, `IterNext` 0.85, `ConstructVariant` 0.81, `Global`
0.69, `Construct` 0.68. The generated code's 997.6 million in bodies
(14.2 in trampolines) by module: the parser 413.6, `bodies` 274.8, the
lexer 196.0, `declare` 71.9, `types` 39.7, the rest 1.6: the lexing and
parsing of the 24 files (the program's nine and the library's fifteen,
751 lines) are three fifths of the generated code of the check row.
The hottest code objects, with the calls and the instructions per call:
`lex_range` 70.5 million (511 calls), `token_at` 58.9 (361,397; 163 per
call for 12 ops), `look_from` 42.8 (129,298; 331), `parse` 33.6 (24),
`lex_word` 31.4 (44,354; 707), `at_position` 23.1 (119,904; 193),
`next_step` 21.5 (78,773; 272), `find_binding` 21.2 (25,432; 836), the
two `expr_span` 28.1, `resolve` 17.0 (76,867; 221), `substitute` 16.7,
`binary` 16.5 (13,225; 1,246), `moved` 15.8 (132,395; 119 for 12 ops),
`lookup_variant` 15.4 (650; 23,754), `is_symbol` 12.8 (104,973; 122
for 5 ops), `type_at` 12.8 (72,990; 175), `slice` 12.4 (94,725; 131).

*The findings*, each with its number:

1. *A call of a declared function costs about 130 instructions besides
   its body.* The five-op `is_symbol` runs 122 per call, the twelve-op
   `moved` 119, the eight-op `slice` 131: the callee's prologue (the
   room check, the frame pointer, the stack's length), its leave (the
   releases of its holder slots, the handlers' height, the frame
   popped, the result moved to the frame's first slot) and the caller's
   site (`push_frame_inline`: the grant read from the caller's record,
   two capacity checks, the record of five words, the stack's length,
   the depth counted twice, the status tested) come to 110 to 160 for
   a small function. On 2,770,541 calls that is about 400 million, 13%
   of the run, inside the generated code's 1,011.8.
2. *A third of the frames leave through `rt_truncate`*: 917,264 of the
   2,770,541 returns, 301.8 million inclusive (9.8%, 329 per leave): a
   frame with more than four holder slots (`RELEASES_INLINE`) hands its
   leave to the helper, which reads every slot's tag and drops the
   counted ones, 16.2 million drops of `Value` (151.3 million) in the
   run; the callers are the functions with many locals (`next_step`
   78,773, `resolve` 76,867, `substitute` 47,968, `substitute_all`
   45,368, `lex_word` 44,354, `expr_span` 34,145, `find_binding`
   25,432). AU55's moves leave most slots holding `Nothing` at the
   return; the leave tests them all the same, because it drops by the
   tags at run time and not by what the analysis knows.
3. *306,392 calls, one in nine, enter generated code through the VM's
   path* (`rt_call` 296,968 of them, `rt_call_value` 9,362), at about
   435 instructions each (`call_from_stack`, `push_frame_in_place`
   46.1 million inclusive, `run_top_frame`, `Jit::ready` 14.4,
   `Jit::entry`, the trampoline, `rt_param_int` 368,980 times, the
   result boxed by `rt_insert_int`): 133 million, 4.3%. Every callee
   so entered has an Integer parameter whose operand is boxed at the
   site: `is_symbol` 73,543 and `is_word` 50,694 (their `code` comes
   from a module constant, `words.symbol_comma` and the like, which an
   image cannot know as a literal because no constant is built when
   `renyi build` runs, so `Global` stays a boxed operand), `type_at`
   69,493 and `var_at` 7,460 (the id read from a variant's field),
   `lex_reserved` 18,762, `assign_app` 13,546, `binary` 12,547,
   `eat_word` 12,034, `line_layout` 7,209, `past_word` 6,983; and
   `argument` 6,501 through `bracketed`, which takes the parser of an
   item as a function value. AR3's direct call wants a typed operand
   for a typed parameter; a boxed value the checker typed Integer is
   not unboxed at the site.
4. *Record updates cost a thousand instructions each*: `rt_with_slot`
   157,460 calls, 167.3 million inclusive, `rt_with` 43,745, 51.7:
   219.0 million, 7.1%. `Vm::with` finds each updated field by name
   (`field_index`, 54.6 million self), takes the pairs through a
   vector, asks `refinement_violation` for every type (201,205 calls,
   7.0 million), and copies the record field by field when it is
   shared (`make_mut`, 1,227,334 clones: `at_position` updates a
   `Cursor` the caller still holds, 119,904 times); the old record is
   freed later. The constructs are lean beside it: `rt_construct`
   677,653 calls at 63 each (42.7 million inclusive),
   `rt_construct_variant` 280,398 at 217 (60.8).
5. *The frees are the records' real cost*: `Composite::free` 1,084,633
   calls, 251.2 million inclusive (8.1%, 232 per free): the fields'
   drops (the drop glue of `Value` recursing into lists and records),
   then `mi_free`. The allocator's side: 2,381,541 allocations and
   2,417,161 frees, 258.9 million (8.4%, 109 per pair); the `mimalloc`
   crate calls `mi_malloc_aligned(size, align)` for every allocation
   (its `lib.rs:51`), and 641,578 of them take mimalloc's generic
   aligned path (`mi_theap_malloc_zero_aligned_at_generic` 27.6
   million, `_at_overalloc` 23.0, `mi_bin` 11.0, `mi_good_size` 7.7),
   which `mi_malloc` on an alignment of 8 or 16 never enters.
6. *The natives and their calling helpers are 703 million inclusive*
   (22.8%): `rt_call_typed` 527,883 calls, 382.7 million inclusive;
   `rt_call_pure` 341,454, 320.3. The glue is about 150 million of it:
   `call_pure_from_stack` 138 instructions per call (the arguments
   drained into the scratch vector, the result through `Result` and
   `status`), `rt_call_typed` 80 self, `typed_value_answer` 79,
   `status` 34 on 2,334,594 calls (79.4 million), `Pinned::insert` 42
   on 272,986, `into_plain` 33 on 1,319,263 calls (43.5 million; a
   function call to test a tag). The work: `list_append` 187,030 calls,
   93.6 million inclusive, copying 2,044,396 values (the list appended
   to is shared, as a scope's bindings are, so AU54's copy with room
   runs on most appends); the lexer's `slice` of a run, `List.slice`
   94,734 calls (44.0 million, 572,753 clones) then `join` (68.8);
   `characters` 26.7 million on 81 texts; `Text.contains` 154,702
   calls (18.8), the lexer's `is_lower` and its kin on a text of the
   letters; `run_of` and `run_until` (53,046 and 1,334 calls), whose
   loops hash each character against the set: 342,059 and 67,211 of
   the run's 555,512 set and map lookups.
7. *Hashing and equality are 219.8 million* (7.1%): `hash_one` 547,570
   calls, 133.9 million inclusive (244 per hash: SipHash's `write`
   1,589,974 calls, 59.7 million; `Value::hash` 24.8), `get_index_of`
   129.9 inclusive, `Value::eq` 998,506 calls, 41.6. Three quarters of
   the lookups are the lexer's character classes (finding 6); the rest
   `Map.get` 121,816 (`words.word_codes`, the world's modules and
   types by name) and `Set.contains` 20,129.
8. *Comparisons through the helper*: `rt_compare` 521,364 calls, 127.5
   million inclusive (4.1%, 245 each): `parse` 187,664, `look_from`
   129,298, `after_breaks` 70,272 (`token.kind is not LineBreak`: two
   values of a variant type without fields, compared through
   `Vm::compare`, `equal`, `has_declared_equals` and `PartialEq`);
   `types.is_error` 33,746 and `assign` 16,117 on `Ty` values, which
   are structural and belong to the helper. AU45 and AU49 compare two
   small Integers and test a variant's tag in place; two values typed
   as a field-less variant type are not compared in place.
9. *A `for each` costs 330 instructions before its first item*:
   `rt_iter_init` 238,534 calls, 78.8 million inclusive (`iterator_of`,
   `iterate` cloning the list's `Rc`, `ListIter::new`, the `Rc<Native>`
   allocated), then the iterator's drop (`drop_glue::<Native>` and
   `Rc::drop_slow`, 10.9 million) and its free: about 95 million, 3%.
10. *The load is 46.2 million* (1.5%): the program decoded from the
    image (about 25 million) and `liveness::prepared`, 37.0 million
    inclusive, which computes AU17's moves again at every run because
    the image carries the program as the emitters wrote it.
11. *The generated code's own sequences*, the 1,011.8 million less the
    calls' 400: about 600 million for 55 million ops, 11 per op. A
    `Load` of a boxed slot copies three words and retains (a tag test,
    a branch, a count raised in the block: about 13), a `Store`
    releases what the slot held (about 10 when it held a plain value),
    a `LoadField` typed by the checker reads in place and retains, an
    `IterNext` reads the item in place and retains; the jumps, the
    constants and the in-place compares are a few each. The retains and
    the releases are the larger half of it: the counts are kept per
    reference at run time where the emitters' moves (AU55) already know
    most slots are moved out or only read.

The same table for the parse row shows the same shape with the load's
share larger (87.4 million of 1,256.9: the program's decoding and the
moves are a fixed cost that the parse of one file does not amortise)
and the texts' share doubled (the JSON printed, 11.6 MB); the compile
row adds the emitter in Renyi (`emit` 337.9 million of generated code),
whose hottest functions are linear searches: `type_index` 56.1 million
(18,919 calls at 2,965), `function_at` 32.9 (569 calls at 57,763),
`expr_bound` 17.3; and the counts and drops rise to 16.0%.

**(b) The design.** Two roads were weighed. (A) *The Cranelift tier
evolved in stages*: each stage a change to the tier as it exists
(`native/codegen.rs`, `native/runtime.rs`, the template tier where it
shares the convention, the emitters where the bytecode changes),
measured by AU58's measure, the judges and the conformance suite
green at every step, the image's formats bumped as they are today. (B)
*A new code generator*: the operand stack and the locals in registers
and machine-stack slots (a value as its two words, the tag and the
payload), the VM's stack written only where a helper reads it, the
frame record built lazily from a chain the generated frames keep on
the machine stack, the counts as explicit retain and release
operations that a dataflow pass elides, the hand-back rebuilding the
interpreter's frame from a map per point as `rt_deopt` does today from
the deopt slot: a rewrite of `codegen.rs` (4,365 lines) and of the
helpers' convention in `runtime.rs` (2,209), the template tier kept on
the interpreter's layout with a conversion at each boundary. (B) is
what "a new backend" names; what it gains over (A)'s stages is the
loads and stores of boxed values as register moves (finding 11: the 26
million `Load`, `LoadMove`, `Store`, `LoadField` and `TakeField` ops
at 8 to 13 instructions each fall to one or two: about 200 million)
and the counts elided by dataflow rather than by the emitters' static
rules (perhaps 100 million more than stage 1 below), 10% of the run,
for three times the work and months without a measure: the first
number comes when the new generator runs the whole corpus under the
conformance suite and the judges. (A)'s stages are the ground (B)
would need anyway (the calling convention, the natives' convention,
the ownership rules in the emitters), so the study recommends (A),
with (B)'s register-resident values as the last stage if the gate is
still out of reach after them. What follows is (A).

*Stage 0, two items of the VM measured first, a day:* (i) the
allocator: `renyi::Allocator` (`crates/renyi/src/main.rs`,
`renyi_vm::memory::Counting` around the `mimalloc` crate) calls
`mi_malloc` and `mi_realloc` for an alignment of at most 16 and the
aligned entries only above it, which keeps every record, list and text
out of the generic aligned path (finding 5): about 100 million, 3.2%,
by the per-call costs of the paths; (ii) the constants in an image: a
module constant whose code object is a `Const` of a small Integer and a
`Return` is a literal the JIT may read without the snapshot
(`Jit::small_globals`), so that an image's call sites with a constant
code reach the direct path (finding 3: 150,000 of the 306,392 slow
calls): about 65 million, 2.1%. Neither touches a format.

*Stage 1, the calling convention (AU58 (4)), examined again.* What
exists: AR3 passes Integer, Boolean and Float arguments and results in
registers between compiled bodies (`direct_signature`), and the
template tier calls compiled bodies through their trampolines with
boxed arguments (AU21); the convention's gaps are findings 1 to 3.
(1a) *A typed parameter's boxed operand unboxed at the site*: a call
site whose operand the analysis holds as `Boxed(Some(Integer))` (a
field, a `maybe` after its `otherwise`, a `Global`) tests the tag and
the small-Integer flag in place and passes the payload, else takes the
helper's path as today; the template tier passes boxed arguments and
the trampoline unboxes them (`rt_param_int`), unchanged. The 306,392
slow calls fall to the function-value calls (9,362): about 130 million
less stage 0's 65, 2.1%. (1b) *A parameter that is only read, passed
without a count*: a flag per parameter in the bytecode, which both
emitters compute as a pass over the code object's finished ops (a
parameter's slot is read-only when no op stores into it, moves it out,
takes a field out of it or updates it in place: no `Store`,
`LoadMove`, `TakeField` or `WithSlot` on the slot), held equal by the
compile judge (decision Z3), written in the bytecode file (format 7 to
8: `Code` gains `borrowed`, a Boolean per parameter, in `file.rs`,
`binary.rs`, `compiler/bytecode.ry` and both emitters). The protocol:
a direct call between compiled bodies passes a read-only parameter
borrowed (the caller's `Load` copies without a retain; a `LoadMove`
feeding such a parameter degrades to a borrow, the slot keeping its
value, since the callee only reads it; a temporary operand, a call's
result or a construct, is spilled to a slot of the caller's frame
first, so that it is a borrow too), and the callee's leave skips the
flagged slots when it was entered directly (the `direct` argument of
the body's signature says so); every other entry (the interpreter's
`call!`, the trampoline from `rt_call`, the template tier's direct
call) passes the parameters owned as today, so the interpreter does
not change and a frame handed back keeps its ownership (`rt_deopt`
retains the flagged slots of a frame entered directly, one count each,
on the rare path). The template tier can join later with a bit in
`NativeState` that its caller sets and the callee's prologue clears,
since its `Entry` signature has no room for the argument; the first
stage leaves it on the owned protocol. The gain: a retain and a
release per borrowed argument, 2,770,541 calls with about one and a
half boxed parameters each that are only read: about 80 million, 2.6%,
and fewer holder slots at the leave, which moves frames under the
inline threshold of finding 2. (1c) *The leave by the static state*:
the analysis of `native/infer.rs` gains, per slot and per op, whether
the slot may own a counted block (unset or empty after a move or a
take, owned after a store or an owned parameter, borrowed by 1b), and
each return site releases only the slots that may own, inline, with
`rt_truncate` kept for the frames with more than eight; the interpreter
keeps dropping by tags (right for both). Finding 2's 301.8 million fall
to about 100: 200 million, 6.5%. (1d) *The frame record made cheaper*:
the grant kept in a register of the body and passed to the callee's
record, the three vectors' pointers read once per body, the capacity
checks folded into the prologue's one check where the callee's locals
are known: `push_frame_inline` from about 45 instructions to 25; the
record itself stays, because `sync_pc`, the grant, the handlers and the
hand-back read `frames.last()` in every helper, and making it lazy is
(B)'s chain. About 55 million, 1.8%. The tiers: the interpreter
unchanged throughout; the template tier takes 1a (nothing to do), 1c
(its leave uses the same analysis) and 1d, 1b later; the images bump
the code format at 1b and 1c and the image format at 1b (the program's
encoding). Stage 1 in all: about 400 million, 13%, by the findings'
counts; the measure after each of 1a to 1d, each kept by AT1's rule.

*Stage 2, the records.* (2a) *The update by static index*: where the
checker typed the base of a `with` (AU1's annotations, as
`static_field` uses them for a read), the generated code knows the
record's type, so the field indices, the refinement check (skipped for
a type without refinements) and the copy of a shared record (the block
allocated and the fields copied with their counts raised, in place)
are compiled into the site, with the helper kept for a base the
checker did not type and for the refined types; the template tier
calls a helper that takes the indices. Finding 4's 219.0 million to
about 70: 150 million, 4.9%. (2b) *`Done` without its allocation*: a
function whose declared result is a record of two fields that the
caller takes apart at once (`let done be f(...)`, `done.value`,
`done.checker`, the record dead after, which `last_uses` sees) returns
the two fields in the caller's two stack slots under a convention per
function (the result kind `Pair`, beside AR3's scalar kinds: the body
writes the fields where the record would have gone and answers
`D_RETURNED`; the trampoline boxes them into the record for the VM's
path), and the caller's takes read the slots: no block allocated,
constructed, taken apart and freed. The checker constructs 677,653
records a run, most of them `Done` (187 sites in `bodies.ry` return
one); the per-record cost is the construct (63), the two takes, the
free (232) and the allocation's share: about 150 million, 4.9%; the
emitter in Renyi has the same shape (`Step`). (2c) *The frees in
place*: the last release of a record walks its fields in the generated
code when their count is small and they are plain or counted, and
calls `rt_drop_at` only for a nested collection: finding 5's 251.2
million to about 150: 100 million, 3.2%; the interpreter unchanged.
Stage 2 in all: about 400 million, 13%; the bytecode unchanged but for
2b's result kind, which the VM derives from the declared type and the
caller's ops (no flag), the code format bumped at each.

*Stage 3, the natives' convention.* (3a) *Natives answering in place*:
a second entry per native, `fn(vm, at: *mut Value, count) -> status`,
the result written where the receiver lay, the arguments consumed by
the native (no scratch vector, no `Result`, no `status`, no
`Pinned::insert`), registered through the API of AK1 beside the typed
entries of AU1, for the standard library first (the hot ones: `at`,
`append`, `slice`, `join`, `contains`, `get`, `length`, `count`,
`with_at`, `run_of`, `run_until`); finding 6's glue, 150 million, to
about 50: 100 million, 3.2%, on both tiers (the same helper). (3b)
*The character classes as tables*: `run_of` and `run_until` over a set
of one-character texts build a 128-bit table of the set's ASCII members
once per call (53,046 and 1,334 calls over sets of at most 64
members) and scan with it, the hash kept for the characters outside
ASCII: 409,270 lookups at about 330 each, 135 million, to about 10:
125 million, 4.1%; a change to two natives. (3c) *The hasher of the
VM's own* (AU57's candidate): a multiplicative hash over the words of
a small text, an Integer or a Boolean, keyed per run, in place of
SipHash for the VM's maps and sets (`IndexMap` and `IndexSet` with the
hasher as their `BuildHasher`): the remaining 146,242 lookups from 244
instructions a hash to about 60: 30 million, 1.0%, and the compile
row's share with it. (3d) *`status` and `into_plain` out of the path*:
the helpers that cannot fail answer without `Result` (`rt_make_list`,
`rt_make_pair`, the constructs without refinements, `rt_list_push`),
and `into_plain` is inlined with its fast path a tag test: about 60
million, 1.9%. Stage 3 in all: about 315 million, 10%; no format
change but 3a's registration API, a library-side addition.

*Stage 4, comparisons, variants and iterators.* (4a) *Two values of a
field-less variant type compared in place*: when the checker typed
both operands as a variant type whose variants carry no fields, `is`
and `is not` compare the type and tag words of the two blocks (AU24's
unit variants) in place, the helper kept for the rest: finding 8's
`parse`, `look_from` and `after_breaks` compares, 387,234 of the
521,364, from 245 to about 12: 90 million, 2.9%; both tiers. (4b)
*`match` on a variant by a table*: a chain of `IsVariant` tests on one
value whose arms are tags becomes a jump by the tag when the chain has
four arms or more (`expr_span`'s 189 ops, `infer_inner`, the parser's
dispatch): about 20 million, 0.6%; the bytecode unchanged (the pattern
is recognised in the ops). (4c) *The iterator in two slots*: `IterInit`
keeps the list in the slot it has and its position in a second slot
the emitters reserve beside it (a bytecode change: `IterInit {slot,
position}`, both emitters, the judges, format 8 with 1b), the
generated code and the interpreter reading the item at the position
with no `Native` allocated; the range iterator stays in its registers:
finding 9's 95 million to about 20: 75 million, 2.4%. Stage 4 in all:
about 185 million, 6%.

*Stage 5, the load:* the moves of AU17 computed by `renyi build` and
written in the image (finding 10's `liveness::prepared`, 37.0 million,
1.2%) and the decoding of the program's 1,102 code objects deferred to
the first call of each (the ops are not read when the image's code
runs): about 50 million, 1.6%, on the image row alone, which the ratio
measures.

*Stage 6, the small callees expanded again* (AU50's expansion, off by
its measure, taken when a call cost what stage 1 removes): with
the convention of stage 1 the expansion of a callee of at most twelve
ops (`token_at`, `is_symbol`, `is_word`, `moved`, `type_at`,
`at_position`, `is_bound_var`, `slice`: 780,000 of the run's calls)
removes the 30 instructions a direct call still costs: about 25
million, 0.8%, measured and kept if it pays.

*The sum.* The stages' estimates, each by the findings' counts and the
per-call costs read from the profile, come to about 1,540 million
(stage 0 165, stage 1 465, stage 2 400, stage 3 315, stage 4 185, stage
5 50, stage 6 25, some of them overlapping: the frees of stage 2 and
the appends' copies of (c) share the drop glue), which would put the
checker's run near 1,550 million, 3.5 times the reference against the
gate's 3.0; the parse row, with the same stages, near 650 million
against its 854; the compile row near 2,500 against its 1,097. An
estimate of this kind has landed within a third of the measure in the
rounds so far where the count behind it was known, and missed where a
path was unmeasured (AU50's expansion was expected to gain and
measured none). The last 250 million of the
check row and the compile row's distance are the checker's and the
emitter's own shape, (c), or (B)'s register-resident values.

**(c) The checker's own shape, which no backend fixes.** (i) *The
lexer's text of a run*: `slice(chars, start, stop)` builds a list of
one-character texts and joins it, 94,725 times a run (finding 6: `slice`
44.0 million inclusive, `join` 68.8, `characters` 26.7): a native that
makes the text of a range of a character list in one step
(`List.text_of(start, stop)`, a library addition), or the lexer keeping
the source text and cutting it by offsets: about 100 million, 3.2%,
and the parse row's share. (ii) *The lists shared when appended*:
`list_append` copies 2,044,396 values a run because the list appended
to is still held by the record it came from (a scope's bindings, the
tokens, the diagnostics): the idiom `scope with bindings:
scope.bindings.append(binding)` takes the field (AU55's `TakeField`)
only when `scope` itself is held once; where the scope sits in the
checker's list of scopes it is shared, and the whole chain copies. The
rewrite is the checker's: the scopes as one flat list of bindings with
the scope boundaries as indices, updated by `with_at` (AU57), which
`find_binding` and `update_binding` walk from the end: about 60
million, 1.9%, with `find_binding` (21.2) and `update_binding` (9.2)
shortened with it. (iii) *`Done` per expression* is stage 2b's; the
checker's 187 sites stay as they are. (iv) *The linear searches*:
`lookup_variant` (650 calls at 23,754 instructions, walking the world's
types through `type_at`), `variant_index` (13,957 at 949), and in the
emitter `type_index` (18,919 at 2,965) and `function_at` (569 at
57,763): a map by name beside each list: about 35 million of the check
row, 1.1%, and 110 of the compile row. (v) *The library's fifteen
files* are 751 lines, a twentieth of what the check row lexes and
parses: not a lever. (vi) *The parser's cursor*: `at_position` updates
a `Cursor` the caller still holds, 119,904 times, and `look_from`
computes the look each time (129,298 calls at 331): stage 2a makes the
copy cheap; a cursor that carries its look lazily, or the look kept
beside the tokens as a table computed once per file (the next
significant index per position, 100,000 Integers), would remove most
of `look_from` and `after_breaks` (57 million, 1.8%): the parser's
rewrite, a day. The judges hold the output of every rewrite here, not
the shape (W3, W7, Z3). (c) in all: about 250 million, 8%, of which
(i), (ii) and (vi) are the ones worth their day each.

**(d) The order, and the measures.** By gain over cost: stage 0 (a
day, 5%), stage 1 in the order 1a, 1c, 1b, 1d (two to three weeks of
sessions, 13%: 1a and 1c are a few days each and touch no format; 1b
is the bytecode change with both emitters and the judges; 1d is a
measured try), stage 3b and 3c (days, 5%), stage 2 (two to three
weeks, 13%: 2a first, then 2b, the convention per function, then 2c),
stage 3a and 3d (a week, 5%), stage 4 (a week, 6%), (c) (i), (vi) and
(ii) between the stages as the checker's own cuts (days, 7%), stage 5
and stage 6 last (days, 2%). Every stage is measured by AU58's measure
(the checker's image run under callgrind, with the table by category
from the lane's script beside the number, and the ratio of
`tools/ratio.py` on the same boot), AT1's rows for the VM's part where
the stage touches the tiers (`tools/measure_size.sh`, the synchronous
JIT row), the four benchmarks, the four judges and the conformance
suite, on both tiers where the template tier shares the change; a
stage stays by AT1's rule, and its entry records the table. The levers
outside the backend are placed: the hasher is 3c, the iterator in two
slots 4c, the character classes 3b, the Rust front end's name sets as
bits (AU55's 4.7% of the Rust compile) a cut of the Rust emitter on its
own day, and the Rust front end is not otherwise touched by the round.
The road is months long, as AU58 acknowledges: about three months of
sessions for stages 0 to 4 at the pace of the typed round, and the
gate decided on the measure at the end, with the compile row's own
distance (13.25 by instructions) to be settled by the owner's reading
of AU35 (ii): the check row is the measure AU58 fixed, the compile row
the emitter's round after it.

**The questions put to the owner** (AskUserQuestion, two batches of
four, the recommended option first): the road ((A) the stages, (B)
the new generator, or stages 0 to 2 then the choice), the gate's rows
(the check row by AU58's measure, all three rows, or the check and
parse rows), the first stage (stage 0's two items, or stage 1 as AU58
(4) names it, or stage 2), the read-only flag (in the bytecode from
both emitters, derived by the VM at load with no format change, or
none with 1c alone), `Done` (the convention per function, the checker
rewritten, or left), the natives' convention (a second entry for every
native through AK1's API, lean helpers for the hot ones only, or none),
the hasher (the VM's own with the tables, or the tables only), and the
iterator (the bytecode change, a lighter native in the slot, or left).
The answers are decision AW1.

**Stage 0 as built (decision AW2, 2026-10-11).** (i) The allocator:
`renyi::Allocator` takes mimalloc's plain entries for an alignment of
at most 8 (`Plain` in `crates/renyi/src/lib.rs`, the binary on
`libmimalloc-sys` itself), the aligned ones above. (ii) The constants:
the study's assumption that the checker's constants are literals was
wrong (of 184, 30 are; 118 are `symbol_code(",")` and the like), so
besides the literal ones `Jit::new` fills (`literal_constant` in
`native/mod.rs`), `image::build` evaluates the pure constants
(`pure_constants` in `native/image.rs`: a fixed point over the code
objects, no deadline, ability call or function value reached, natives
without `needs` only) on an interpreter VM and notes them with the JIT,
so that the small Integers among them are literals in the image's
code; the test is in `crates/renyi_vm/tests/native.rs`. The measure:
the check row 3,085.3 to 2,889.9 million (-6.3%, the estimate was 5%),
mimalloc 258.9 to 146.8, the calls through the VM's path 114.1 to 57.3
(`call_from_stack` 306,330 to 141,589 calls), the parse row -7.5%, the
compile row -4.9%, AT1's synchronous row -3.6% and -2.5% by the
estimate, `records` -11.5%; the ratios by instructions 6.54, 4.08 and
12.61. The table is in AW2.

**Stage 1a as built (decision AW3, 2026-10-11).** The slow calls that
remained after stage 0, attributed by `study_map.py` (the calls through
each code object's trampoline): 141,577 into 62 code objects, half of
them `type_at`, then `lex_reserved`, `assign_app`, `var_at`,
`line_layout`, `module_at`, `lookup_param`, `function_at`,
`call_known`, every one an Integer parameter whose operand the
generated code held boxed although the checker typed it Integer (a
variant's field bound by a pattern, `when Named(id)`, which `Unpack`
pushes untyped and the slot's `Load` notes; a `maybe` after its
`otherwise`; a difference of two such); the two function values
(`argument`, `parameter`, through `bracketed`) are not of this kind.
The change (`native/codegen.rs`, `native/infer.rs`): the direct call's
condition admits a boxed operand the checker noted as the typed
parameter's type (`noted_kind`), and `call_direct` tests its tags in
place before the table lookup (an Integer that is small, a Boolean, a
Float), reads the payload from the stack into the register the body
takes, and leaves the operand for the frame push to move into the
parameter's slot, where the trampoline's entry reads it; anything else
takes the helper's path with the state untouched. The interpreter and
the template tier are untouched; no format changes. The test is in
`crates/renyi_vm/tests/native.rs` (an Integer, a Boolean and a Float
parameter with a pattern's binding and a `maybe` after its
`otherwise`, a big Integer through the same site). The measure: the
calls through the trampolines 141,577 to 9,380 (`rt_call` 132,227 to
30), the check row 2,889.9 to 2,826.6 million (-2.2%, the estimate was
2.1%), the calls through the VM's path 57.3 to 7.4 million, the parse
row -0.9%, the compile row -1.6%, AT1's synchronous row -0.1% (-0.2%
by the estimate), the benchmarks unchanged to the sixth figure; the
ratios by instructions 6.40,
4.05 and 12.41. The table is in AW3.

**Stage 1c as built (decision AW4, 2026-10-11).** Finding 2: a third
of the frames left through `rt_truncate` (917,264 leaves, 301.5
million inclusive after 1a), which drops every slot by its tag,
because the leave in place released the locals one by one only when
the slots that are not typed, with the operands, were at most four,
and the functions with many locals went to the helper although AU55's
moves leave most slots holding `Nothing`. The change: `owned_slots` in
`native/infer.rs`, a fixed point over the ops' flow (the analysis's
entry states and handler regions) giving, per op, the slots that may
hold a counted block on entry to it: a boxed parameter from the entry,
a boxed slot from a store of anything but a plain scalar until the
`LoadMove` that moves it out (a borrowed moved load keeps the slot's
value, AU17, so it counts as no move) or the `WithSlot` that moves the
record out, a slot of another kind always, a typed slot never; a
failure flows to its handler with the state before the op; an op the
analysis does not reach has every slot counted. `leave_frame_inline`
in `native/codegen.rs` (and the boxed result's leave through it)
releases the slots so counted at the return's op with the operands,
inline up to eight (`RELEASES_INLINE` four to eight), and calls
`rt_truncate` past that. The interpreter and the template tier are
untouched (the template's return goes through `rt_return`, by tags;
its leave by the same analysis is a later item); no format changes.
The tests: `crates/renyi_vm/tests/native.rs` (a frame with thirteen
locals left by three returns, both tiers) and
`crates/renyi/tests/sandbox.rs` (the same frame left two hundred
thousand times under a memory budget of sixteen megabytes through the
binary, on both tiers: a slot skipped wrongly would leak past it, as a
run that keeps every result alive does). The measure: the leaves
through `rt_truncate` 917,264 to 32,514, the check row
2,826.6 to 2,729.4 million (-3.4%, the estimate was 6.5%), the
counts and drops 440.7 to 321.9, the parse row -1.7%, the
compile row -3.1%, AT1's synchronous row -0.5%, the image
row -2.7%; the ratios by instructions 6.18,
3.98 and 12.02. The table is in AW4.

**Stage 1b as built (decision AW5, 2026-10-12).** The study's item
(1b): a retain at the caller's `Load` and a release at the callee's
leave for every boxed argument of a direct call, although most
parameters are only read. The flag: `Code::read_only`, a Boolean per
parameter (`file.rs` format 7 to 8, after `locals`; `binary.rs` 4 to
5; `image.rs` image format 9, code format 16; `compiler/bytecode.ry`),
computed from the finished ops by `Code::read_only_params` in
`bytecode.rs` (the Rust emitter's `finish`) and by `read_only_params`
in `compiler/emit.ry`, held equal by the compile judge: no `Store`,
`WithSlot`, `TakeField`, `IterInit`, `Deadline` or `MarkStack` on the
slot; a `LoadMove` is allowed (AU55 makes the last read of nearly
every parameter one; the study's rule, which excluded it, would have
flagged almost nothing). The checker's program: 2,133 of 2,328
parameters read-only, 177 excluded by a `TakeField` (166 by that
alone: a record read once through a field on its last use; a
`TakeField` that clones when the slot is borrowed could admit them
later), 29 by a `WithSlot`. The protocol (`native/codegen.rs`): the
body's signature gains an `i64` after the direct flag, the bits of the
parameters the caller passed borrowed (a trampoline passes zero);
`borrowed_operands` marks, for a `Call` that `direct_call` makes
directly, the plain `Load`s and boxed `Const`s among its last
arguments whose parameter the callee accepts borrowed
(`accepted_params`: boxed, read-only, among the first thirty-two, and
never moved into an owned sink, every `LoadMove` of the slot in the
prepared program an argument of a call through a typed entry or an
operand of a comparison, which read it where it lies; one rule from
the ops and the program's tables alone, so that a caller compiled on
the compile thread or for an image reads the same bits the callee's
compilation does), stepping over a typed push, and the mask per
call; the marked push carries no
reference (as a typed call's, AU1 and AU17), `call_direct` passes the
mask, and the path through `rt_call` (the slow path, and the call
through the VM's path when the emission does not take the direct
call) gives the marked operands a reference first
(`retain_borrowed_arguments`). A moved load (the caller's last use)
moves into the callee as before: the first build borrowed it too and
measured 4.5% more instructions on the check row (2,851.6 million;
the counts and drops 321.9 to 424.4, the natives 355.9 to 385.6), the
callee's copy having two references where the moved value had one, so
every append or update of a list or a record passed by its last use
copied it and the caller's leave freed the old one (kept as the lane's
`study/stage1b_moves_borrowed`). The second build accepted every
boxed read-only parameter and had the callee move conditionally (a
`LoadMove` of such a parameter tested its bit and moved or copied
with a reference); it measured the check row at 2,723.4 million,
-0.2%, and AT1's estimate +0.1% (kept as the lane's
`study/stage1b_conditional_moves`), the conditional moves costing
about half of what the borrows of plain loads and constants saved;
the owner chose the refinement first, then the measure's verdict (the
stage reverted whole if the check row still falls by less than 0.5%
and AT1's estimate does not fall). In the callee a `LoadMove` of an
accepted parameter is one a typed entry or a comparison borrows; the
conditional move remains as the fallback where the analysis finds
such a move owned; the leave in place releases an accepted slot only
when its bit is clear (`owned_slots` unchanged); every way out that
treats the frame as owned calls `rt_retain_params` first (before
`rt_deopt`, `rt_truncate` at the leave, `rt_fail`, `rt_check_failed`,
`rt_unhandled`, and at the interrupt exit). The interpreter is
untouched; the template tier stays on the owned protocol and tells
`borrowed_operands` so (`direct_calls`): without that switch the first
build double-freed on `RENYI_NATIVE_TIER=template` (the template
shares the analysis, pushed the marked operands without a reference
and passed the frame owned), a segmentation fault the same run under a
memory budget did not show. The tests: `crates/renyi_vm/tests/native.rs`
(two thousand turns of ten paths out of frames holding borrowed
parameters, both tiers; the masks of `main`'s direct calls read from
its IR: 1 for a record read by its fields beside a text whose move
goes into an interpolation, 2 for a record beside a list passed by its
last use, 1 for a text whose only move goes into `Text.length`, 0
where the parameter is returned, passed on, constructed into a record
or a failure or updated by `with`, where its last read takes a field,
or where the argument is a temporary or a last use)
and `crates/renyi/tests/sandbox.rs`
(forty thousand turns under sixteen megabytes on both tiers through
the binary, and a crash inside a frame holding a borrowed parameter);
`crates/renyi/tests/compile.rs`'s literal format numbers moved up by
one. The measure: the check row 2,729.4 to 2,719.3 million
(-0.4%, the estimate was 2.6%), the counts and drops 321.9 to
322.2, the generated code 1,014.6 to 1,004.1, the parse row
-0.4%, the compile row +0.2%, AT1's synchronous row
-0.1%, the image row -0.4%; the ratios by instructions
6.16, 3.96 and 12.04. The table is in AW5.

**Stage 1d as built (decision AW6, 2026-10-12).** The study's item
(1d): at every direct call, `push_frame_inline` read the caller's
grant from the frame record on top, checked the stack's capacity for
the callee's locals and wrote the stack's length, and checked the
frames' capacity for one more record. Three changes in
`native/codegen.rs`, the interpreter untouched and the template tier
as it was: the frame's grant travels in the body's signature
(`direct_signature`, an `i32` after the borrowed bits; a direct caller
passes its own from the register it holds it in, or `rt_frame_grant`'s
answer when the callee narrows the grant; the trampoline reads the
record's), so the call site writes the callee's record from a
register; the stack's length is written by the callee's prologue after
the room check it makes anyway (AR4), the call site leaving it at the
arguments, except where a boxed argument moves to a slot past the room
the caller's prologue made (a typed parameter before a boxed one at
the caller's deepest operand), which keeps the check at the site; and
the frames' room for the record a direct call pushes is checked once,
in the prologue of a body that makes a direct call (`has_direct_call`),
the frames' length being the same at every one of its calls. The code
format is 17. The tests: the program of every shape of direct call (a
callee that narrows the grant from `main` and from another that
narrows, a callee that inherits the grant, recursion past the frames'
capacity and the depth limit, typed parameters before a boxed one at
the deepest operand) in `crates/renyi_vm/tests/native.rs`,
`crates/renyi_vm/tests/template.rs` and, through the binary under a
memory budget on both tiers, `crates/renyi/tests/sandbox.rs`. The
measure: the check row 2,719.3 to 2,681.0 million (-1.4%, the
estimate was 1.8%), the generated code 1,004.1 to 966.7, the parse
row -1.1%, the compile row -1.2%, AT1's synchronous row
-1.3% (-1.2% by the estimate), the image row
-1.6%; the ratios by instructions 6.07, 3.92
and 11.90. Stage 1 in all: the check row 2,889.9 after stage
0 to 2,681.0, -7.2%, against the study's 13%. The table is in
AW6.

**The character tables and the hasher as built (decision AW7,
2026-10-12).** The study's items (3b) and (3c). `run_of` and `run_until`
(`natives/prelude.rs`) scan with a table of 128 bits of the set's
members that are texts of one ASCII character, which answers for such
an item exactly and leaves any other to the set; measured alone, with
the table built per call, the check row fell 2,681.0 to 2,621.5
(-2.2%) and `list_run_of_typed` cost 70.0
million over 53,046 calls, so the tables of the last
eight sets scanned are kept in a thread-local list with their sets
(`class_table`, `CLASS_TABLES`; `Vm::drop` clears it). `hashing.rs` is
the hasher of the VM's maps and sets (`ValueHasher`, `ValueHashing`,
the aliases `ValueMap` and `ValueSet`, which every site uses in place
of `IndexMap<Value, Value>` and `IndexSet<Value>`): a folded
multiplication over the words written, a span of bytes as two
overlapping words with its length in the multiplier, the state keyed
once per process from `RandomState`; `vm::hash_of` (`Value.hash()`)
keeps the standard hasher. The measure: the check row 2,681.0 to
2,527.2 (-5.7%), the hashing 220.5 to 55.5, the natives 355.9
to 366.9, the parse row -6.8%, the compile row
-4.7%, AT1's synchronous row -0.3% (-0.7% by the
estimate), the image row -0.4%; the ratios by instructions
5.72, 3.65 and 11.35. Since stage 0 the
check row 2,889.9 to 2,527.2, -12.5%. The table is in AW7. Next
in AW1's order: stage 2, 2a first (`with` by static index with the copy
in place), then 2b (`Done` in the caller's slots) and 2c (the frees in
place).

**Stage 2a as built (decision AW8, 2026-10-12).** The study's item
(2a). `update_by_index` in `native/infer.rs` settles at compile time,
for a `with` whose base the checker typed (the op's annotation, a
record without refinements and not `Date`), the type and the index of
every updated field, the names read from the `Const` ops at their
depths through the analysis's states, packed eight bits each with the
count in the top byte (at most seven updates). Part A: the helpers
`rt_with_by_index` and `rt_with_slot_by_index` (`Vm::op_with_by_index`)
update at the indices with no vector, no lookup and no refinement
check, in place when held once and in a copy otherwise, the general
way for a holder of another kind or a guarded value; both tiers call
them, the template tier for good. Part B: on the Cranelift tier the
update is compiled into the site (`update_in_place` in
`native/codegen.rs`): the tags and the type tested in place, the block
updated where it lies when its count is one and in a copy
(`rt_record_copy`) otherwise, the old field values and the names
released in place, the new values moved in, the helper in a cold
block for the rest; the updates applied in the helper's order (the
last first, so that of two updates of one field the first in the
source wins). The code format is 18. The measure: the check row
2,527.2 to 2,416.7 with the helper alone (-4.4%)
and to 2,370.7 with the update in place (-6.2%; the study's
estimate 4.9%), the records category 206.8 to 79.9, the parse
row -6.3%, the compile row -5.2%, AT1's synchronous row
-1.3% (-0.9% by the estimate), the image row
-3.0%; the ratios by instructions 5.37, 3.42
and 10.76. Since stage 0 the check row 2,889.9 to 2,370.7,
-18.0%. The table is in AW8. Next in AW1's order: stage 2b
(`Done` without its allocation: a two-field record a caller takes
apart at once returned in the caller's slots under a convention per
function), then 2c (the frees in place).

**Stage 2b as built (decision AW9, 2026-10-12).** The study's item
(2b). A function whose declared result is a record of exactly two
fields of a type without refinements and not `Date`, with a code
object, is a pair function (`pair_result` in `native/infer.rs`): on the
Cranelift tier its body answers a direct caller with the two fields in
the first two cells of where its frame was and `D_RETURNED`
(`pair_return` in `native/codegen.rs`), and the trampoline's caller, or
any caller under provenance guards, with the record and `D_BOXED`;
before the `Return` a `Construct` of the type leaves its fields as they
lie, a call of a pair function passes its pair on, a load of a pair
slot its cells, anything else is taken apart (`rt_split_pair`). The
caller keeps the pair as it lies (`Abs::Pair`) when the op after is a
`Store`, a `Field` or a `Return` (`pair_fused`), or when the call ends
the region of an `otherwise` whose end holds such an op
(`otherwise_end`: the success path jumps past the test, which only a
failure then reaches, `handler_only`; `pair_bypass` in the generator),
and makes the record otherwise (`rt_pair_record`); a `Store` of a pair
into a slot stored from pairs alone makes a pair slot
(`SlotKind::Pair`: the second field in a cell past the code object's
locals), whose fields `LoadField` and `TakeField` read or move in place
and `Load` and `LoadMove` push as a pair. The hand-back rebuilds a
pair's record (an owned word per pair slot in the point); an entry at a
loop header checks each pair slot (`rt_pair_fits`, else `D_STAY`),
widens the frame and takes the records apart (`rt_pair_resume`); the
prologue's write of the stack's length before the header dispatch is
gone (it lengthened the stack of an entry refused after it). The
template tier, the expanded callees and the typed test analyse without
pairs. The code format is 19; `tools/image_census.py` and the lane's
`study_map.py` read the pair kinds' type. The measure: the check row
2,370.7 to 2,342.0 with the pair at the call alone
(-1.2%; two pairs of three were made into their record by
the caller after all, most at the `otherwise`) and to 2,301.9 with the
pair past the test (-2.9%; the study's estimate 4.9%), the records
category 79.9 to 57.6, `rt_construct` 677,653 to
410,681 calls, the parse row -3.4%, the compile row
-2.5%, AT1's synchronous row -0.5% (-0.7% by the
estimate), the image row -3.7%; the ratios by instructions
5.21, 3.31 and 10.49. Since stage 0 the
check row 2,889.9 to 2,301.9, -20.3%. The table is in AW9. A
candidate seen on the way, not decided: `rt_resume_range` refuses a
finished range iterator, so a function with two loops in sequence is
never entered at the second header from the interpreter. Next in AW1's
order: stage 2c (the frees in place: the last release of a record walks
its fields in the generated code, `rt_drop_at` only for a nested
collection), then the natives' entry and the glue.

**Stage 2c as built (decision AW10, 2026-10-12).** The study's item
(2c). `release_in_place` in `value.rs` releases a value by its tag, as
the generated code's release in place does (decision AU32), inlined at
every site: a plain value costs a tag test, a shared block has its
count lowered in place, and only the last holder goes out of line
(`release_last`): of a record or a variant to `Composite::free_block`,
which releases the fields the same way and frees the block; of a list
to its elements released the same way and the vector freed empty; of
anything else, and a big Integer, to `drop_in_place`. `rt_drop_at` and
`Pinned::truncate_values` go through it (the generated code of both
tiers, every frame's leave); the interpreter's other drops and the
natives' stay Rust's drop glue, whose remaining calls are stage 3's.
The walk is in the VM's Rust rather than in the generated code, as the
study had it: one loop for every site, no code grown, the same cost per
value. The study's numbers came from the checker's run on `bodies.ry`
instrumented for the day (the classes of the fields and the elements
the frees drop; in AW10). No test is added: every program frees its
values, and the corpus, the conformance suite on both tiers, the
sandbox's memory budget and the four judges hold the change. The
measure, and what it found: by instructions the walk costs nearly what
the glue cost, the check row 2,301.9 to 2,296.2 (-0.2%; the
study's estimate 3.2%), the glue of `Value` 11,647,789 to
6,030,437 calls but the counts and drops with the VM's own
307.9 to 302.1 (-1.9%): the frees' cost is
the inspection of every field and element and the deallocation, not
the glue's dispatch, which the study had taken for it; what the walk
removes is the glue's indirect branch per field, the synchronous row's
indirect branches 17.3 to 13.3 million on the
self-check and AT1's estimate 8,299.5 to 8,192.5 (-1.3%),
which keeps the stage by AT1's rule and by the owner's rule for a
marginal one. The parse row -0.2%, the compile row
-0.7%, the image row -0.4%; the ratios by instructions
5.20, 3.30 and 10.42. Since stage 0 the
check row 2,889.9 to 2,296.2, -20.5%; stage 2 in all 2,527.2 to
2,296.2. The table is in AW10. A first build, one function out of
line for every value, cost the check row 3.4% more (its prologue on
every plain field) and is kept in the lane as `renyi_stage2c_v1`. Next
in AW1's order: stage 3, the natives' entry and the glue (a second
entry per native answering in place through AK1's registration API,
the standard library's hot natives first; `status` and `into_plain` off
the path; the glue's 6,030,437 remaining calls, from `rt_compare`,
`call_pure_from_stack`, `Vm::binary`, `rt_iter_init`, `rt_call_typed`
and the lists Rust's drops still free, each dropping its operands
through Rust).

**Stage 3a as built (decision AW11, 2026-10-12).** The study's item
(3a), the owner's answer 6 of AW1: a second entry per native answering
in place, through AK1's registration API. `InPlace` in `extension.rs`
(`shim`, an `extern "C"` function `fn(vm, at, count, borrowed, pc) ->
status`, and `takes`), the macro `in_place!` that writes the shim from
a typed entry (`in_place!(Int: list_length_typed)`, the four kinds) or
from a function that takes its arguments (`in_place!(takes
list_append_in_place)`, `fn(&mut [Value], u32) -> Option<Value>` with
the mask of the arguments the caller still holds, `take_or_clone` in
`natives`), the shim's body `in_place_typed` and `in_place_taking`
with `answered_in_place` (the arguments the call owns released in
place, the answer written over the first, the memory budget tested),
`Native::with_in_place`, the registry's lookup and check extended
(`Found`; an in-place entry on a function with `needs` refused as a
typed one is). The VM: `Tables` and `tables_of` in `vm.rs` (the
natives, the pure flags, the typed and the in-place entries and the
call kinds of a program under a registry, once), `CallKind::Pure {
entry }` and `Typed { kind, read, entry }` with `Entry::Reads` or
`Takes` and `ReadInPlace::ListAt` (AU34's read in place; its two
`contains` through lean helpers go, with `rt_text_contains`,
`rt_list_contains` and `lean_answer`), `Vm::call_in_place` before the
typed entry in the interpreter's `call!`, `NativeState::in_place` and
the JIT's table of shims per function (`Jit::new` takes the entries).
The Cranelift tier: `call_in_place` in `codegen.rs` (the shim's address
loaded from the table, `call_indirect` with the signature `ppwww -> i`
imported once, the height stored for the answer and a failure's
landing, a scalar answer read from the boxed value in place by the
declared kind, `entry_kind`, the typed call in a cold block when the
entry declines, the bad block for a failure or an interrupt),
`typed_entry` true for a pure native with an entry (its operands lent),
`taking_entry` (a moved operand of an entry that takes is owned by the
call, not lent: `moves_borrowed` off and the push not `lent`). The
template tier: `call_in_place` with the arguments in the convention's
registers and the shim called through the table (`STATE_IN_PLACE`), the
length stored, `check` for the landing, `rt_call_typed` when declined.
The prelude: `read_in_place` (ListAt alone), `integer_quotient_typed`,
`plain_count`, `list_slice_typed`, `list_append_in_place`,
`list_with_at_in_place`, `list_append_all_in_place`, and 26
registrations through `with_in_place`. Code format 20. Built twice:
the first build's shim tested a guard and the budget for every kind
and the generated code stored the length twice and tested the status
twice, and `strings` (1,188,895 calls of `Text.contains`) rose 12.2%
(the shim 72 instructions a call against the lean helper's 20); the
second gives a scalar entry that reads its operands the lean helper's
shape (`release_owned` over the bits set, no guard, no budget, one
test after the call) and keeps the failure test and the budget for a
value answered or operands taken (`Entry::Takes` told to both tiers).
The interpreter's `Vm::call_in_place` declines a call of no argument
or of more than 32, as the generated code does. The test
`an_in_place_entry_answers_on_both_tiers_and_takes_what_the_call_owns`
in `tests/extension.rs` (`grow`, a native with a taking entry: a list
the call owns grown in place to a thousand items on both tiers, a list
the caller keeps copied twice; `twice` through an entry made from its
typed one) and the check's refusal of an in-place entry on `shout`;
`docs/extensions.md` has the section. The measure: the check row
2,296.2 to 2,203.9 (-4.0%), the ratios by instructions
4.99, 3.18 and 10.04; `rt_call_typed`
527,824 to 1,908 calls,
`call_pure_from_stack` 341,454 to 10,448,
`status` 1,567,934 to 1,236,928, the glue of `Value`
6,030,437 to 3,933,657; the natives category 365.0
to 307.3 million, the helper glue 85.3 to 74.0; AT1's
synchronous estimate 8,192.5 to 8,019.3 (-2.1%). The table
is in AW11. Next in AW1's order: stage 3b, `status` and `into_plain`
off the path (the study's item (3d): the helpers that cannot fail
answering without `Result`, `rt_construct`, `rt_construct_variant` and
`rt_list_push` among them, 1,236,928 calls of `status` left,
and `into_plain` inlined with its fast path a tag test,
816,882 calls from `op_list_push`, `iterate` and
`iterator_of`), then stage 4.

**Stage 3b as built (decision AW12, 2026-10-12).** The last item of
stage 3 in AW1's order: `status` and `into_plain` off the path. In
`value.rs`, `into_plain` and `guarded` test in place (`#[inline]`) and
keep the unwrapping and the wrapping out of line (`unguarded`,
`with_origins`); `origins` and `plain` are inlined. In `vm.rs`,
`plain_record_type` (a record type without refinements, not the date)
and `plain_variant_type` (no refinements), which `construct_from_top`
and `construct_variant_from_top` use too. In `native/runtime.rs`,
`rt_construct_plain` and `rt_construct_variant_plain` (the value built
in place of its fields and pushed, no `Result`, no status, no pc; the
origins lifted under a guard as the general helpers lift them),
`rt_list_push` pushing a plain item onto a plain list where it lies
(one tag test each, `op_list_push` for the rest), `rt_compare`'s
general path pushing its boxed answer itself, and `iterator_of` taking
a plain list as it is; the two helpers in `HELPERS`. In
`native/codegen.rs`, the two helpers in `SIGNATURES` (no result) and
`STACK_SAFE`, and `Op::Construct` and `Op::ConstructVariant` (with
fields) calling them without a status test when the type is plain; the
same in `native/template/mod.rs` through `plain`. Code format 21. The
corpus, the conformance suite on both tiers, the four judges,
`tests/guards.rs` and a new test in `tests/native.rs`, `plain_builders_lift_the_origins_of_guarded_fields_as_the_general_helpers_do`, which runs a guarded program (records and variants with fields built from the secret, collected lists, loops over a guarded list and a plain one) on machine code against the interpreter hold the paths. The measure:
the check row 2,203.9 to 2,099.1 (-4.8%), the ratios by
instructions 4.75, 3.09 and 9.66; `status`
1,236,928 to 108,008 calls, `into_plain`
816,882 to 0; the helper glue category
74.0 to 3.7 million; AT1's synchronous estimate 8,019.3 to
7,800.2 (-2.7%). The table is in AW12. Next in AW1's
order: stage 4 (two values of a field-less variant type compared in
place, `match` by a table, the iterator in two slots), then the
checker's own cuts, stages 5 and 6.

**Stage 4a as built (decision AW13, 2026-10-12).** The first item of
stage 4 in AW1's order. In `native/runtime.rs`, `compare_in_place`
answers `is` and `is not` on any operands as structural equality
(`Value::eq` on the plain values) unless the left operand's type
declares its own `equals`, which the general path keeps running
(`has_declared_equals` inlined); an ordering is answered for two small
Integers, texts or Booleans as before. In `native/infer.rs`,
`sum_compared`: whether the checker noted both boxed operands as one
variant type without its own `equals` and with at most 64 variants,
and then the tags that name a variant with fields, one bit per tag. In
`native/codegen.rs`, `compare_sum_variants`, which `compare_boxed`
emits for `is` and `is not` on two such operands: both tags tested for
a variant (a guarded value takes the helper), the type and the tag of
the blocks compared, different ones answered, the same tag answered
when it names a variant without fields and left to the helper
otherwise, the Boolean to its register, the operands not borrowed
released in place; the same in `native/template/mod.rs`
(`sum_compared_at`, `compare_sum_variants`, the answer kept in the out
slot while the operands are released, then boxed where the left
operand lay; `bt_rr`, the 64-bit bit test, in `x64.rs`). No code
format change. The first build compared only the types none of whose
variants has a field, which the checker hardly compares (`Ty` and
`TokenKind` have fielded variants): 238 of the check row's comparisons
took the path, the check row 2,099.1 to 2,049.9 (-2.3%) from the
helper's structural equality alone; the second build is the one
measured. The corpus, the conformance suite on both tiers, the four
judges, AU13's comparison test and the new test
`variants_compare_in_place_by_tag_and_fielded_ones_through_the_helper`
in `tests/native.rs` and `tests/template.rs` hold the paths. The
measure: the check row 2,099.1 to 1,995.8 (-4.9%), the ratios
by instructions 4.52, 2.91 and 8.92;
`rt_compare` 521,364 to 43,402 calls
(125.8 to 5.1 million inclusive),
`binary_values` 309,995 to 30,193; the
comparisons category 131.6 to 34.5 million; AT1's
synchronous estimate 7,800.2 to 7,546.0 (-3.3%). The
table is in AW13. Next in AW1's order: the rest of stage 4 (the
iterator in two slots, 4b; `match` by a table, 4c), then the checker's
own cuts, stages 5 and 6.

**Stage 4b as built (decision AW14, 2026-10-12).** The second item of
stage 4 in AW1's order: a loop's iterator in two consecutive slots of
the frame. Both emitters allocate them (`loop_slots` in
`compile/mod.rs`, at every `IterInit` of `compile/stmt.rs` and
`compile/query.rs`, and in `compiler/emit.ry`): the first holds the
source listed (`iterator_of` in `vm.rs`: a list as it is, the items of
anything else as a list, a range of small Integers as its own iterator
value, as before), the second the position as a small Integer. The
interpreter's `IterInit` writes both and `IterNext` reads the item at
the position and advances it (`iterator_next`); `rt_iter_init` does
the same on the helpers' path, and the template tier calls the helpers
as before. The analysis (`native/infer.rs`) marks the second slot an
Integer, so the Cranelift tier keeps the position in a register: its
`IterInit` moves a plain list from the stack into the slot where it
lies, the slot's old value released, and sets the register to 0
(anything else through the helper); its `IterNext` tests the slot for
a list, reads the items' address and their count through the list's
block as `List.at` does (AU34), compares the position, copies the item
with one more reference and advances the register, a range iterator
value in the slot through the helper as before; a hand-back boxes the
position as any Integer slot and an entry at the loop header reads it
back. `Native::Iterator` and `ListIter` are gone from `value.rs` with
their layout constants; the bytecode file is format 9 (`file.rs`,
`compiler/emit.ry`, `tests/compile.rs`), the image code format 22
(`native/image.rs`). The corpus, the conformance suite on both tiers,
the four judges (the compile judge on the slots of every loop), the
loop tests and the new test
`loops_in_two_slots_keep_their_list_and_position_across_a_hand_back`
in `tests/native.rs` and `tests/template.rs` hold the paths. The
measure: the check row 1,995.8 to 1,930.3 (-3.3%), the
ratios by instructions 4.37, 2.91 and
8.63; `rt_iter_init` 238,534 to
0 calls, the native's drops 238,534 to
0, `mi_malloc` 1,978,955 to 1,740,421;
the iterators category 25.3 to 0.0 million; AT1's
synchronous estimate 7,546.0 to 7,481.6
(-0.9%). The table is in AW14. Next in AW1's order: the last
item of stage 4, `match` by a table (4c), then the checker's own cuts,
stages 5 and 6.

**Stage 4c as built (decision AW15, 2026-10-12).** The last item of
stage 4 in AW1's order: a `match` by a table. A new op, `JumpByTag {
slot, table }` (`bytecode.rs`, kind 55), with the jump tables of a
code object beside its ops, `Code.tables`: the op peeks at the frame
slot and, for a variant whose tag is below the table's length, jumps
to the table's entry; anything else (a guarded value, a `Nothing`, a
failure, a tag past the table) falls through to the arms' own tests,
which stay as they were. Both emitters emit it after the subject's
store when at least three arms name a variant (`TABLE_ARMS`,
`match_table` and the emission in `compile/expr.rs` with
`variant_tag_of` in `compile/pattern.rs`; `arm_tag`, `named_arms`,
`match_table`, `target_of` and `emit_match` in `compiler/emit.ry`):
for every tag up to the highest an arm names, the first arm in order
that can match it, entered past its own tag test (three ops) when it
names the tag and at its start for any other pattern, else the
`otherwise` or the crash after the arms. The interpreter's arm is in
`vm.rs`; the Cranelift tier (`native/codegen.rs`) reads the slot's
kind and the block's tag in place and enters the arm's block through
a jump table (`br_table`), the edges coerced as a branch's are; the
template tier (`native/template/mod.rs`) compares the tag with each
entry in turn, the entries block starts; the analysis follows the
targets among the flows (`native/infer.rs`, both passes), as do the
last reads (`liveness.rs`) and the inliner (`native/inline.rs`, the
tables appended with the targets shifted). The bytecode file is
format 10 (`file.rs`: the op, the tables, their checks;
`compiler/bytecode.ry`: `OpJumpByTag`, `tables`; the literals of
`tests/compile.rs`), the binary encoding 6 (`binary.rs`), the image
code format 23 (`native/image.rs`). The corpus, the conformance suite
on both tiers, the four judges (the compile judge on the tables of
every `match`) and the new test
`matches_by_a_table_enter_the_arm_by_the_tag_and_fall_through_otherwise`
in `tests/native.rs` and `tests/template.rs` hold the paths. The
measure: the check row 1,930.3 to 1,896.4 (-1.8%), the
ratios by instructions 4.29, 2.89 and
8.47; the generated code 1,000.9 to 962.7 million; the
indirect branches of AT1's synchronous row 10.6 to
10.5 million (-0.7%), its mispredictions 31.7
to 30.4; AT1's synchronous estimate 7,481.6 to
7,395.6 (-1.1%). The table is in AW15. Stage 4 is done.
Next in AW1's order: the checker's own cuts between the stages (the
next paragraph), then stages 5 and 6.

**The checker's own cuts as built (decision AW16, 2026-10-12).** The
item of AW1's order after stage 4: the cuts in the compiler written in
Renyi, each gated by `renyi check`, the canonical layout and the four
judges byte-equal and kept by the lane's quick measure (the check row
by `perf stat` with the images rebuilt from the edited sources;
`gate_cut.sh`, `gate_native.sh` and `quick_check_row.sh` in the lane).
In `compiler/declare.ry`: `index_variants` and `variants_of` at the end
of `resolve_all` fill the `variants` field of `ModuleInfo`, and
`lookup_variant` is one map read (-5.1% by the quick measure). In
`compiler/parser.ry`: `token` in `Cursor`, the looked-at token;
`at_position` computes the look and the token together and updates the
cursor in its slot; `moved` changes the position alone; 39 sites read
`cursor.token`; `parse` sets the comments aside in one pass;
`bracketed` looks the closing symbol's code up once (-2.7% and part of
-0.5%). In `compiler/lexer.ry`: `previous_stop` and `previous_code`
through `lex_range`, `next_step`, `lex_word`, `lex_text` and
`camel_case_word`; `lex_reserved` takes the word's text; every run's
text through `slice_text` (-5.3% and -3.4%). In `compiler/bodies.ry`:
`resolve` in one match per step, `zonk` resolving once, `source_text`
through `slice_text`, the scopes and the variables updated in the slot
by `with_at` (-0.8%, part of -0.5%, -0.4%). The library primitive
`slice_text(self: List of Text, start, stop) returns Text` in the
sketch, `library/std/prelude.ry`, the checker's copy and
`natives/prelude.rs`, with a test in `tests/semantics.rs`; the cheat
sheet does not list it (its budget stands at 2,997 of 3,000 tokens).
The flat list of bindings with the scope starts was tried and dropped
(+0.9%). The measure: the check row 1,896.4 to 1,586.1
(-16.4%), the ratios by instructions 3.59, 2.50
and 7.64; the generated code 962.7 to 800.8 million,
the natives 297.2 to 255.8, the texts and copies
96.8 to 79.5, the counts and drops 152.2 to
131.6; AT1's synchronous estimate 7,395.6 to
7,381.0 (-0.2%). The table is in AW16. Next in AW1's
order: stage 5a (the next paragraph), stage 5b, stage 6.

**Stage 5a as built (decision AW17, 2026-10-12).** The first half of
stage 5 of AW1's order, the load: AU17's pass ran on a copy of the
program at every run (37.5 million instructions on the checker's image
run, 2.4%); now `liveness::prepare` runs it once where a program enters
the VM and marks it (`Program::prepared`, kept by the binary encoding,
encoding 7, not by the bytecode file), `liveness::prepared` borrows a
prepared program (`Cow`) and copies another, `image::build` encodes the
prepared program in the image, `compile_cached` prepares on every path
and stores the prepared encoding in the cache keyed by the sources, and
the runner and the sandbox take a prepared program as it is.
`tests/build.rs` holds the image's program to the bytecode file's
prepared; a test in `tests/native.rs` holds the pass to once. The
measure: the check row 1,586.1 to 1,547.9 (-2.4%), the
ratios by instructions 3.51, 2.46 and
7.50; the load 50.4 to 26.1 million; AT1's
synchronous estimate 7,381.0 to 7,332.3
(-0.7%), the image row 3,374.3 to 3,328.7
(-1.4%), the cached run 3,464.0 to 3,408.8
(-1.6%). The table is in AW17. Next in AW1's order: stage 5b
(the next paragraph), then stage 6.

**Stage 5b as built (decision AW18, 2026-10-12).** The second half of
stage 5 of AW1's order, the load: the program of an image or of a cache
entry was decoded whole at the load (32.2 million instructions on the
checker's image run, 2.1%); now a code object's head is decoded with the
program and its body (`Body` in `bytecode.rs`: the ops, the spans, the
types, the tables) at its first read, through a `OnceLock` from the
shared bytes (`binary::Encoded`, `decode_shared`, encoding 8, the body a
length-prefixed block after the head), checked then (`file::check_body`,
`file::Limits`); every read goes through `Code::body`, `ops`, `spans`,
`types` and `tables`, every write through `body_mut` and `set_body`, and
the interpreter's loop binds the body once per frame. A test in
`tests/native.rs` holds the bodies undecoded after the decode and
decoded by their first read. The measure: the check row 1,547.9 to
1,528.5 (-1.3%), the ratios by instructions 3.46, 2.45 and 7.44; the
load 26.1 to 10.0 million; AT1's synchronous estimate 7,332.3 to 7,479.8
(+2.0%, the instruction cache's: the same instructions in the same
functions with more misses, the code's placement; the instructions
+0.1%), the image row 3,328.7 to 3,311.8 (-0.5%), the cached run 3,408.8
to 3,392.8 (-0.5%), the interpreter 11,065.9 to 10,942.5 (-1.1%). The
table is in AW18.
Stage 5 is done. Next in AW1's order: stage 6, AU50's expansion
measured again under the new convention (the next paragraph).

**Stage 6 as built (decision AW19, 2026-10-12).** The last item of
AW1's order: AU50's expansion of small callees measured again on stage
5b's binary with the images built under `RENYI_NATIVE_INLINE=1`
(`probe_inline.sh`, `study/5b_inline/`): the check row 1,528.5 to
1,585.3 (+3.7%), the parse row 696.2 to 718.6 (+3.2%), the compile
row 2,719.7 to 2,784.0 (+2.4%); the generated code 800.8 to 789.0
million, the counts and drops 131.0 to 148.1, mimalloc 76.8 to 91.7,
the VM's own 79.6 to 107.4 (`free_block` 637,423 to 1,060,403 calls,
`release_last` 737,168 to 1,114,125, `rt_pair_record` 67,784 to
252,115): an expanded callee loses the borrowed parameters of AW5 and
the pair return of AW9. The expansion stays off; no code changes. Every
stage of AW1's order is done: the ratio by instructions stands at
3.46 on the check row against the gate's 3, the road from here
the owner's decision.

## The representation items: the design study (session 11)

**What the owner ordered (AU20)**: after the template tier's follow-ups
(AU21), the representation items of AU1's step v (small texts inline,
`Int` flattened into `Value`, lists of unboxed Integers, values in
registers across ops); the study with counts first, then the owner's
questions in one batch.

**How it was measured**: a census build, never committed (counters at
the constructors of `value.rs`, in `binary_values`, `iterator_of` and
`op_list_push`, a clone of every value by its tag, the allocator by
size; printed at the end of a run under `RENYI_CENSUS`; the session's
scratchpad held the script that applied it to a worktree of fb15b1c),
run on the self-check and the four benchmarks on the interpreter (the
program's own work) and on the JIT; callgrind on the self-check's
synchronous JIT run of AU21's release binary (8.31 billion
instructions); `renyi run --profile` for the mix of ops (99 million ops,
5.4 million calls, 3.5 million primitive calls).

**The self-check, by the census** (interpreter):
- Texts: 281,224 made (2.3 MB), 78% of them at most 7 bytes, 92.6% at
  most 15, 95.2% at most 22; 947,627 one-character texts more from the
  ASCII table without an allocation; 9.5 million clones of a text (4.0
  million on the JIT run); 2.76 million `is` and `is not` on two texts
  (the lexer's `ch is " "`), against 0.98 million comparisons of two
  Integers.
- Records: 1,256,474 made (72% with two fields); variants 532,673 (80%
  with one field); each is two allocations (the `Rc` with the type, the
  tag and the `Pinned` header, and the buffer of the fields): 3.6
  million of the run's 7.18 million allocations. 745,769
  `ConstructVariant` of a fieldless variant clone the cached value
  (AG6) through `rt_construct_variant`, no allocation but about 120
  instructions each.
- Lists: 741,923 made, 435,470 of them empty (the lexer's `Step(tokens:
  [], diagnostics: [], next: ...)` for every blank), each an `Rc<Vec>`
  allocation; 3,249 hold only small Integers, 4.4% of the items
  iterated.
- Integers: 2 big ones made; 1.91 million `+`, 0.09 million `-`, 0.98
  million comparisons.
- The run allocates 7.18 million blocks (614 MB) on the interpreter,
  7.94 million (1.1 GB) on the JIT, Cranelift's work the difference.

**The benchmarks**: `records` makes 200,000 records of two fields
(400,000 of its 430,000 allocations); `json_round_trip` 304,000 texts
(all at most 15 bytes but 50) and 102,000 records of three or four
fields; `strings` 200,000 texts of 2 to 15 bytes and 2.58 million text
clones; `primes` no list at all (ranges and Integers, in registers on
the JIT). No program of the five makes lists of Integers worth
unboxing.

**The self-check's synchronous JIT run, by callgrind**:
- the front end that compiles the checker program
  (`compile_with_sources`: parse, check, bytecode) 717 million
  instructions (8.6%); the Cranelift compilation 419 million (5.0%);
  the templates 114 million (1.4%); the program's run the rest;
- the values' lifecycle: `drop_glue<Value>` 614 million (7.4%),
  `Value::clone` 316 million (3.8%, 57% of it under `rt_retain_at`'s
  15.9 million calls), `rt_retain_at` 95 million, `rt_drop_at` 59
  million (14.7 million calls);
- the allocator: `mi_malloc_aligned` and its paths about 370 million
  (7.45 million calls), `mi_free` 167 million (7.6 million calls), the
  copies of `realloc` 151 million (140,000 calls, lists grown by
  `append`): 8.3% together;
- the comparisons: `rt_compare` 400 million (4.8%; 4.1 million calls at
  97 instructions, plus the drops of the operands it consumes),
  `Value::eq` 182 million;
- the constructions: `rt_construct`, `Vm::construct`,
  `rt_construct_variant`, `construct_variant`, `plain_all`,
  `Value::guarded` and `refinement_violation` 660 million (7.9%), of
  which `plain_all` and `guarded`, the guard bookkeeping of decision P3
  in a run whose grant guards nothing, 202 million (2.4%);
- the typed natives' path: `rt_call_typed` 181 million and
  `typed_value_answer` 146 million (3.9%); `status` 207 million (2.5%,
  6.1 million calls).

**The gap of AU18's goal**: in wall-clock on one hardware thread (best
of seven), the front end alone on the checker program (`renyi compile`)
takes 155 ms, the JIT run 1,456 ms, the image 1,212 (+20%): the front
end is about two thirds of the gap, Cranelift and the templates the
rest; without the front end the JIT run would stand about 7% above the
image. A representation item makes the run and the image faster alike
and leaves the front end as it is, so it widens the ratio; the goal as
stated needs a faster front end. The run from the bytecode file is no
shortcut: loading the checker program's JSON `.ryc` (9.7 MB) takes 865
million instructions, more than compiling the sources.

**The items, with what the counts estimate** (in instructions of the
self-check's JIT run, and the benchmarks they move):
1. **Small texts inline**: a text of at most 15 bytes held in the value
   itself (a tag of its own, appended so that no other tag moves; the
   length and the bytes in the 16 bytes past the tag, zero-padded, so
   that two such values are equal exactly when their words are); a
   longer text as today. Buys: no allocation for 92.6% of the texts
   made; no count on their clones and drops (4 million each on the JIT
   run); `is` on two small texts compared in the generated code in a
   few instructions where `rt_compare` costs about 110 with its drops
   (2.7 million of its 4.1 million calls). Estimate: 4 to 5% of the
   self-check, the most of any item; `strings` and `json_round_trip`
   by the texts they build. Touches: `Value` (the variant, the
   accessors `as_text`, `natives::text` and `plain_text`, the 27
   places that name `Value::Text`), equality and hashing (by content), the
   recordings, the generated code's comparison and text paths, the
   image's code format. Risk: moderate; the text accessors are few.
2. **Records and variants in one allocation**: the fields after the
   header in the counted block (a thin pointer, a count-and-copy of its
   own for the update in place), the field read one load shorter. Buys:
   1.95 million allocations and frees fewer (of 7.45 million), the copy
   from the operand stack into a separate buffer gone. Estimate: 2 to
   3% of the self-check, `records` the most (half its allocations).
   Touches: `Record`, `Variant`, the 48 field accesses, `with` and
   `WithSlot`, the field reads of both tiers (`RECORD_FIELDS`), the
   natives that build records, the code format. Risk: moderate (an
   unsized allocation by hand).
3. **`Int` flattened into `Value`** (the small Integer at the payload, a
   big one under its own tag): one comparison and branch fewer at every
   Integer read of the interpreter, the helpers and the generated code;
   below 1% of the self-check by the counts, whose Integers live in
   registers on the Cranelift tier; the step a 16-byte `Value` would
   need (with thin texts), which would cap item 1 at 7 bytes. Risk:
   moderate (77 sites name `Int::Small` or `Int::Big`).
4. **Lists of unboxed Integers**: no program of the five makes them; no
   measure to judge them by.
5. **Values in registers across ops**: the scalars already live in
   registers on the Cranelift tier (AR1); boxed values in registers
   with their counts elided where the consumer borrows is AU14's
   skipped item generalized; on the template tier, a cache of the
   stack's top. Not estimated; the largest and riskiest.

**Cuts the counts found outside the list** (small and local, each a
stage by AT1's rule): the guard bookkeeping skipped in a run whose
grant guards nothing (`plain_all` and `guarded`, 2.4%); a fieldless
variant read from the cache by the generated code instead of through
`rt_construct_variant` (745,769 calls, about 1%); one shared empty list,
as the ASCII table shares the one-character texts (435,470
allocations, about 0.4%).

**The questions put to the owner** (one batch, the recommended option
first): the order of the items; lists of unboxed Integers and the
flattened `Int`; the cuts outside the list; AU18's goal, given the
front end's share of the gap. The owner took every recommended option
(decision AU22).

## The template tier: the design study (session 10)

The owner took every recommended option of this study (decision AU18),
and the tier as built is decision AU19 (the section "The typed round",
"The template tier is in"); the study stays as the record of the
reasons and of the gap measured before the code.

**What the owner ordered (AU16)**: a first tier of machine-code
templates per op without Cranelift, which would serve the machine with
one hardware thread (where the compile thread brings nothing) and
shorten the wait for the code everywhere; the next large item; the
design put to the owner before the code.

**The gap it can close**, measured on the binary of AU17 (d8ee092)
with `tools/bench.py --runs 5` (the `image` column is the program run
with every code object compiled ahead of time by `renyi build`, which
is where a tier that compiled everything at no cost would land; the
JIT column is the cold run as a user gets it):

| hardware threads | self-check, JIT / image | primes | records | strings | json_round_trip |
|------------------|-------------------------|--------|---------|---------|-----------------|
| one (`taskset -c 0`) | 1,647 / 1,213 ms (+36%) | 43 / 32 (+34%) | 54 / 43 (+26%) | 76 / 70 (+9%) | 92 / 85 (+8%) |
| two | 1,473 / 1,221 (+21%) | 38 / 32 (+19%) | 51 / 47 (+9%) | 83 / 80 (+4%) | 95 / 86 (+10%) |
| four | 1,467 / 1,214 (+21%) | 39 / 35 (+11%) | 48 / 42 (+14%) | 77 / 74 (+4%) | 97 / 92 (+5%) |

On one thread the JIT compiles 54 code objects of the self-check
(factor 8000, 90 ms of Cranelift) and 510 code objects that are called
stay cold; 176,735 calls from generated code go to the interpreter. On
four threads (factor 100 on the compile thread) 338 code objects are
compiled, 1.2 s of the thread's time, and 57,803 calls from generated
code go to the interpreter. Compiling everything at its first call on
the interpreter's thread (`RENYI_NATIVE_HOT=0 RENYI_NATIVE_SYNC=1`)
takes 2.8 to 3.4 s, 1.4 to 1.5 s of it Cranelift's, so the running
part is about the cold run on four threads and a quarter of a second
above the image row (the frames entered late, through `rt_call` and
the trampolines, and the thread's share of the machine). The short
programs of an agent are another matter: `examples/hello.ry` and four
of the five starter workflows run in 7 to 25 ms, nearly all of it the
front end (`renyi check` alone takes 8 to 10 ms), so no tier changes
what a user sees there; the tier matters from about a tenth of a
second of running upwards, and most on the machine with one thread,
which an agent's sandbox often is.

**What the tier can reach**: not the image row, since template code
keeps every value boxed and calls the helpers for most ops (the
Cranelift tier's small Integers, Booleans and Floats in registers are
what make `primes` 25 times faster than the interpreter), but a good
part of the gap on boxed code: in the interpreter's profile of the
self-check, `run_frames` (the dispatch: the match on the op, the
bounds check, the counting, and the inline ops) is 46% of the
instructions and the helpers' work the rest; template code removes the
dispatch and keeps the ops' work, so it should run at about two thirds
of the interpreter's instructions on boxed code, which puts the
self-check on one thread from 1,647 ms towards 1,350, and on four
threads a smaller gain (the ops that run on the interpreter today while
the compile thread works, and the cold code objects).

**The design proposed to the owner:**

1. *The code.* A code object the analysis of `infer.rs` settles gets
   machine code emitted by a small x86-64 assembler of the VM's own
   (`native/template/x64.rs`, the dozen instruction forms the sequences
   need; `native/template/mod.rs`, the sequence per op), with every
   operand boxed on the VM's stack in the interpreter's layout (the
   locals, then the operands; the stack's length `base + locals +
   depth` known statically from the analysis, as AT4 has it). Each op
   is a fixed sequence: the trivial ops inline (`Const`, `Nothing`,
   `Load`, `LoadMove`, `Store`, `Pop`, `Dup`, the jumps, the tag tests
   `IsNothing`, `IsFailure`, `JumpIfAbsent`, `JumpIfFailure`, the
   Boolean branches), every other op a call to the helper the Cranelift
   tier calls for boxed operands (`rt_binary`, `rt_call`,
   `rt_call_typed` with the mask of `borrowed_operands`, `rt_call_pure`,
   `rt_load_field`, `rt_construct`, ...) followed by the status check:
   `CONTINUE` falls through, `FAILURE` goes to the landing of the
   innermost handled region (`rt_settle_handler`, then the target, as
   the Cranelift tier's landings do; `rt_unhandled` without one), any
   other status leaves the function with it. The code holds no address
   (the helpers, the constants and the direct table are reached through
   `NativeState`, as AS1 requires), keeps the VM, the frame's base and
   the stack's pointer in callee-saved registers, reloads the pointer
   after a helper that may grow the stack (the `STACK_SAFE` list), and
   follows the platform's C convention: System V and Windows x64 differ
   in the argument registers, the shadow space and the callee-saved
   set, and both are needed, the owner's machine being Windows.
2. *The entry and the exit.* The function has the `Entry` signature
   (the VM, the base, the pc) to a status, as a trampoline does: `pc`
   is 0 or a loop header, and since every op has a block, an entry at
   a loop header fills no registers and refuses nothing; the exit
   statuses are the Cranelift tier's (`RETURNED` with the value on the
   caller's stack after `rt_return`, `INTERRUPT`); there is no
   hand-back, a frame being in the interpreter's layout at every op,
   and the depth limit of native frames applies as today.
3. *The calls.* `Op::Call` goes through `rt_call` (the callee's frame
   pushed by the VM, then its own code: template, Cranelift through its
   trampoline, or the interpreter), the typed and the pure primitives
   through `rt_call_typed` and `rt_call_pure` as the Cranelift tier
   does; a later stage can give template functions direct calls among
   themselves (a body in the direct table with every parameter boxed)
   when the profile shows `rt_call`'s cost.
4. *The tiers.* A code object is cold (the interpreter), then template
   (at its first call, synchronously, microseconds), then Cranelift
   (promoted). The template code counts the hotness as the interpreter
   does, at its entry and at each loop header, and at its entry, when
   the count is hot, asks for the promotion: on the compile thread
   (factor 100) or, with one hardware thread, synchronously (factor
   8000), as today; the Cranelift code replaces the template entry at
   the next call. Narrated and profiled runs, `--interpret` and the
   code objects the analysis rejects stay with the interpreter; images
   and the cache stay the Cranelift tier's.
5. *The tests.* Every native test and the conformance suite run on the
   template tier too: `RENYI_NATIVE_TIER=template` forbids the
   promotion, so that everything runs on templates (`RENYI_NATIVE_HOT=0`
   keeps compiling everything with Cranelift at the first call).
6. *The platforms.* x86-64 first (Linux, macOS, Windows); aarch64
   (Apple silicon, the Linux arm64 release) keeps the interpreter and
   the Cranelift tier until an aarch64 encoder follows, the sequences
   being shared.
7. *The measure.* AT1's rows (the cold JIT run decides) and the
   wall-clock table above on one, two and four hardware threads; the
   goal: the self-check on one thread within 15% of the image row.
8. *The size.* The checker program has 46,238 ops; at 50 to 100 bytes
   an op the template code of everything called is 2 to 4 MB, placed
   in the code arena at the first call of each code object.

**The alternatives weighed**: copy-and-patch (snippets per op compiled
by Cranelift at build time and copied with their holes patched at run
time) reaches the same code for more machinery (a build step, the
relocations of the holes, a stencil per op and operand shape);
Cranelift with every optimization off and the single-pass register
allocator halves the cost at best (AU14), fifty times short of what
compiling everything at the first call needs; a lower hotness factor
brings nothing on one thread (AU14's sweep). The questions put to the
owner at the end of session 10: the approach (the assembler, or
copy-and-patch, or Cranelift stripped down), when a code object gets
template code (at its first call, or after a few runs, or at load),
the promotion (counted by the template code with today's factors, or
only with the compile thread, or none), and the platforms (x86-64 in
this round with aarch64 after, or both now, or Linux x86-64 only
first).

## The execution model, as the owner asked on 2026-10-09

The owner asked in session 10 whether Renyi is compiled or interpreted,
whether the toolchain written in Renyi is in use, and whether it could
be. The answer given: the model is bytecode with a baseline JIT and an
optional image (a source is checked and emitted to bytecode, the VM
interprets it, a hot code object is compiled to machine code by
Cranelift, `renyi build` compiles every code object ahead of time into
an image, `--exe` a self-contained executable, and the image cache of
AU10 gives every second run of a program the image). The default path
is Rust end to end: the front end of `crates/renyi_syntax`,
`renyi_check` and `renyi_vm/compile`, the VM, the JIT and the natives.
The toolchain written in Renyi (`compiler/`: the lexer, the parser, the
checker and the emitter) is complete and held byte-equal to the Rust
one by the judges, and compiles itself, but no command of the binary
runs it: it runs in the tests and by hand (`renyi run
compiler/checker.ry <file>`). The front end could be switched (a
decision, the dispatch of `check` and `compile` to the compiler's
embedded image, the bootstrap chain exists); the runtime cannot, by
H1 and W2. The cost, measured on this machine: checking
`compiler/bodies.ry` (7,774 lines) takes 0.083 s with the Rust front
end and 1.87 s with the Renyi checker on the cold JIT tier, 22 times
more, which is what the typed round's second goal (the self-check a
quarter faster, AU1) and the owner's stages 6 to 8 (AU9) address. The
recommendation given: decide the switch after this round; the session
offered to put it to the owner as a decision question at the round's
end.

## The size of the generated code (decisions AT1 to AT7; session 9, 2026-10-08 and 09)

The owner's four answers of 2026-10-08 after `renyi build` closed
(decision AT1): the round's rule is KCachegrind's estimate of the
cycles over cachegrind's counts on the self-check (`Ir + 10 (I1m + D1m)
+ 10 Bm + 20 Bi + 100 LLm`), deterministic, with the wall-clock at the
end of each stage; the cuts that cost no instruction first; the image's
bytecode in a binary encoding (the `.ryc` stays JSON); the record gains
a tag word so that a record and a variant share the prefix the field
read reads.

- **The tools.** `tools/measure_size.sh <binary>` runs cachegrind with
  `--cache-sim=yes --branch-sim=yes` on the self-check three times (the
  JIT run, `--interpret`, the image built under `valgrind --tool=none`
  since valgrind hides CPU features) and prints the counts and the
  estimate, then the census; about ten minutes a binary.
  `tools/image_census.py <file.ryi> [--summary]` reads an image and
  prints the bytes of machine code per code object and per op, the op
  kinds, and (with numpy) a least-squares attribution of the bytes to the
  op kinds over the code objects. The census that opened the round, on
  the compiler's image of AS4: 17.5 MB, of which 9.0 MB the bytecode as
  JSON and 8.3 MB the machine code (955 code objects, 46,193 ops, 179
  bytes of body per op, 8.7 KB per code object; the trampolines 151 KB);
  the attribution (r² 0.98): `LoadField` 462 bytes each (22% of the
  bodies), `Load` 173 (21%), `Call` 440 (21%), `Store` 171 (11%),
  `Const` 148 (7%), `Return` 223 and `ReturnNothing` 363 (10%), a deopt
  point 384 (4%). Loading the compiler's image costs 152 ms against
  172 ms for compiling it from source (`renyi run <file> /nonexistent`,
  which fails before `main`), almost all of it the parse of the 9 MB of
  JSON (`renyi run selfcheck.ryc` costs 160 ms), which is why the binary
  encoding of stage 2 belongs to the round; since AT3 and AT5 the image
  of the compiler loads in 16 ms, its code section mapped from the file
  (most of the 69 ms measured after AT3 was the checker loading the
  standard library before failing on the missing file, the program's
  work).
- **The baseline** (the binary of AS4, 71965aa):

  | run | instructions | I1 misses | D1 misses | LL misses | mispredicts (cond, ind) | estimated cycles |
  |-----|-------------:|----------:|----------:|----------:|------------------------:|-----------------:|
  | JIT run | 11,181,306,636 | 116,596,885 | 75,149,511 | 2,173,057 | 35,445,492, 85,031,817 | 15,371,167,556 |
  | `--interpret` | 12,956,694,773 | 11,959,382 | 64,644,504 | 1,981,283 | 38,216,065, 185,330,643 | 18,009,635,443 |
  | image (`speed`) | 9,070,866,423 | 160,217,080 | 53,100,744 | 3,388,570 | 26,140,638, 41,528,543 | 12,634,878,903 |

  The compiler's image: 17,455,597 bytes, 8,261,937 of machine code in
  the bodies (178.9 per op), 825 deopt points.
- **Stage 1, the cuts at no instruction cost (decision AT2).**
  `codegen.rs`: the prologue writes `Nothing` to every local that is not
  a boxed parameter (once per callee; `push_frame_inline` wrote them at
  every call site), the `LoadField` fast path reads a record and a
  variant on one path (`RECORD_TAG`, the shared prefix, the holder test
  `tag - TAG_RECORD < 2`, no range check), `deopt_block` and
  `emit_deopts` share one block per pc and operand state (`deopt_unless`
  takes it; `deopt_blocks` on `Gen`), `call_direct` goes to the slow
  path when the table has no body (the ask path and `rt_direct_entry`
  are gone, with `Vm::direct_entry_of` and `Jit::direct`), and the call
  counter is gone (`NativeState` has `depth`, `direct_table`, `helpers`,
  `constants`; `Jit::report` takes the hotness alone and says how many
  calls went to the interpreter). `value.rs`: `Record { ty, tag, fields
  }` with `tag` always `usize::MAX`, `RECORD_TAG`, and the three
  compile-time assertions of the shared prefix. `CODE_FORMAT` 2.
  Measured:

  | run | instructions | I1 misses | D1 misses | LL misses | mispredicts (cond, ind) | estimated cycles |
  |-----|-------------:|----------:|----------:|----------:|------------------------:|-----------------:|
  | JIT run | 11,019,336,948 | 106,417,090 | 74,824,738 | 2,281,534 | 35,221,233, 85,881,206 | 15,129,745,078 (-1.6%) |
  | `--interpret` | 12,847,784,410 | 13,096,267 | 65,085,528 | 2,139,424 | 39,819,316, 185,723,809 | 17,956,214,100 (-0.3%) |
  | image (`speed`) | 8,876,981,034 | 155,552,936 | 53,227,555 | 3,348,443 | 28,440,844, 41,641,494 | 12,416,868,564 (-1.7%) |

  The compiler's image: 17,019,001 bytes, 7,829,707 of machine code in
  the bodies (-5.2%; 169.5 per op), 709 deopt points. Kept by the rule.
- **Stage 2, the image's program in binary (decision AT3).**
  `crates/renyi_vm/src/binary.rs` (new): `encode(&Program) -> Vec<u8>`
  and `decode(&[u8]) -> Result<Program, String>`, the content and order
  of the bytecode file in bytes (LEB128 integers, length-prefixed
  texts, one byte for an option or a variant, the maps in key order;
  `FORMAT` 1 first; a count past the end is refused before anything is
  allocated; `file::check` runs on the result as on a loaded file);
  `image.rs`: `Image { header, code_hash, program, codes }`,
  `IMAGE_FORMAT` 2, `build` stores `sha256_of(file::render(program))`
  and `binary::encode(program)`; `lib.rs` (the binary): `Hashed::Given`
  for the image's hash (`code_hash`, `compile_with_sources`,
  `run_embedded`), `load_image_bytes` decodes the program;
  `tools/image_census.py` takes the `.ryc` beside the image for the
  ops (`tools/measure_size.sh` compiles it); `tests/build.rs`: the
  compiler's program round-trips through the encoding and renders as
  its bytecode file, an image's hash is the bytecode file's, the
  decoded program renders the same, and a recording made from the
  `.ryc` reproduces against the image and the other way round.
  Measured: the compiler's image 17,019,001 to
  8,705,877 bytes (the program 8,989,252 of JSON to 676,056), its load
  152 to 69 ms against 166 ms for compiling the compiler from source
  (`renyi run <file> /nonexistent`); the rule on the self-check as an
  image: 8,126,378,865 instructions, 151,181,468 I1 misses, 50,981,874
  D1 misses, 2,204,948 LL misses, 21,727,334 and 39,288,677
  mispredicts, 11,371,553,965 estimated cycles (-8.4% against stage 1);
  the JIT run 15,147,420,418 (+0.1%, the binary's own layout: the
  generated code is untouched) and the interpreter 17,897,196,056
  (-0.3%); `hello` as an image 42 KB and 7 ms (235 KB and 11 before,
  8 ms from the source); the self-check in wall-clock 1448 ms as an
  image against 1723 on the JIT run (best of five). Kept: the stage
  targets the image, whose estimate fell.
- **Stage 3a, the static stack height and the frame pointer (decision
  AT4).** `codegen.rs`: `height()` is the frame's part of the stack by
  the state (`locals + boxed_depth(state)`), `length_at(height)` and
  `store_height(height)` compute and store the VM's length from `base`,
  `frame_var` holds the stack's pointer plus `base24` (defined by
  `reload_frame` once after the prologue's room check and after every
  helper call in `Gen::call` and after the direct call's `call_indirect`),
  and `address_at(height)`, `top_address()` and `last_address()` address
  the slots and the operands from it; `stack_len`, `set_stack_len`,
  `stack_ptr` and `slot_address` are gone, `item_address` remains for a
  record's fields; `push_typed_value` and `push_nothing_at` take the
  height (the state's for a push, `0` for a result left where the frame
  was); the frame push computes the callee's base as `height - boxed`.
  The one subtlety: after `box_top(1); state.pop()` the value lies at
  `height()`, not `height() - 1`, which the boxed return, the typed
  return of a boxed value and `IsNothing`/`IsFailure` say explicitly.
  Measured: the rule on the self-check, JIT run:
  10,867,362,765 instructions, 102,196,693 I1 misses, 74,887,436 D1
  misses, 2,355,761 LL misses, 34,816,885 and 85,118,789 mispredicts,
  14,924,324,785 estimated cycles (-1.5% against stage 2); the image:
  7,994,955,436 instructions, 142,581,697 I1 misses, 11,145,328,756
  estimated cycles (-2.0%); the interpreter 17,902,234,378 (unchanged);
  the machine code 7,829,707 to 7,909,162 bytes (+1.0%: the frame
  pointer's reloads and block parameters, against fewer instructions and
  misses); the self-check 1678 against 1780 ms in wall-clock, back to
  back on a busy machine. Kept by the rule. A step not taken: a flag per
  helper for whether it can grow the stack (most can, through the user
  code they run), so that `Gen::call` skips the reload after `rt_drop_at`
  and the few others that cannot.
- **The load (decision AT5).** The profile (`strace -T`, the JIT's own
  `placing` clock, a Python experiment on fresh against touched buffers):
  the system calls of a load take 4 ms; 44 ms went to reading the 8.7 MB
  file into a fresh buffer and 49 ms to copying the 7.8 MB of code into
  the arena's fresh pages, first-touch page faults that this Firecracker
  VM makes heavy (about 6 µs a page) and every machine pays in
  proportion. The fix: `image.rs` lays the bodies and trampolines
  sixteen-aligned in one code section at an offset that is a multiple of
  `SECTION_ALIGN` (16 KB), the table before it holding each
  `Placement { offset, len }`; `Image::open` and `Image::open_at` map
  the file read-only (`memmap2`), parse the table and the program from
  the mapping, and leave the section in the file (`MappedSection { file,
  offset }`) when its offset is a multiple of the page size, else copy
  it; `CodeArena::map_section` maps it executable (`Pages::Mapped`
  beside `Pages::Owned`), and `Jit::load_image` falls back to reading
  and placing the section when the system refuses (a `noexec` mount);
  `Image::read` (bytes in memory) copies the section as before, for the
  tests and for a file that cannot be mapped. `exe.rs` pads the
  executable to the alignment before the image, so that the embedded
  image's section maps from the executable's own file; the report says
  "the section mapped from the file" or "copied", which `tests/build.rs`
  asserts for a `.ryi` run and for the executable. Since a2c4685
  (session 14) the file is opened with execute access on Windows, where
  the mapping had been refused and the section copied on every run, and
  the report names a refused mapping with the system's reason. `IMAGE_FORMAT` 3;
  `tools/image_census.py` reads the table and the section's offset.
  Measured: a run that exits at once (`renyi run <image>`
  without arguments) 16 ms with the image mapped, 27 with it copied and
  the check as it was, 105 from the source, 86 from the `.ryc`; with the
  library loaded 46 against 167 ms; `hello` 6 against 8 ms; the load's
  instructions 62 to 32 million (`file::check` 32 to 1 million: it
  formatted a message per op before knowing the check's outcome, built
  on failure now; the decode about 10 million; `Vm::new` and the
  registry under 2); the compiler's image 8.8 MB, its executable 26.6
  MB. The profile's tool: `valgrind --tool=callgrind` on `renyi run
  <image>` with no arguments, then `callgrind_annotate --inclusive=yes`;
  with a file argument the checker loads the standard library first,
  which is the program's work, not the load's. A note on the gates run
  in a worktree under the scratchpad's long path: `a_run_is_narrated`
  (`crates/renyi_vm/tests/recording.rs`) fails there because the
  narration truncates long arguments, and passes from the main tree;
  it is the path, not the code.
- **Stage 3b, the reference counts as calls (decision AT6).** Three
  variants on the binary of AT4, by the rule: (1) the frame pointer not
  reloaded after a helper that cannot move the stack (`rt_drop_at`, the
  grant, the frame growth, the typed takes and resumes, the tag tests):
  the pointer then lives across those calls and Cranelift saves and
  restores it around each, so the code grew 0.7% and the JIT run's
  estimate 1.0%, rejected; (2) on top of it `retain` and `release` as
  calls to `rt_retain_at` (a clone's count kept) and `rt_drop_at` (the
  value dropped whatever its count) in place of the inline sequences of
  AR4: the machine code 7.91 to 5.62 MB (-29%), the JIT run's estimate
  -0.7% (instructions -1.7%, instruction misses -15%, indirect
  mispredicts +16%), the image's +0.3% (misses -32%, indirect
  mispredicts +75%, the model's charge for the calls); (3) the calls
  without (1): the JIT run 14.85 billion and the
  image 11.21, a fifth of a percent behind (2), which stays by the rule
  (the code equal, 5.62 MB). In wall-clock against the binary of AT3,
  best of five: the self-check 1886 to 1775 ms on the JIT run, 1369 to
  1351 as an image, the interpreter, records and primes level. `codegen.rs`: `retain` and `release` are
  the two calls, `rc_of` and the inline bodies are gone, the helpers
  `rt_retain_at` and `rt_drop_at` (`runtime.rs`), `CODE_FORMAT` 3.
- **The round's close (decision AT7).** The owner closed it on
  2026-10-09 with every stage of the plan measured and in: the cuts at
  no cost (AT2), the image's program in binary (AT3), the static stack
  height (AT4), the image's load (AT5) and the reference counts as
  calls (AT6). Against the binary the round began from (71965aa): the compiler's machine code 8.26 to 5.62 MB
  (-32%), its image 17.5 to 6.5 MB, the self-check's estimate 15.37 to
  14.83 billion cycles on the JIT run (-3.5%) and 12.63 to 11.18 as an
  image (-11.5%), the image's load 152 to 16 ms, the self-check in
  wall-clock 1841 to 1775 ms on the JIT run and 1511 to 1351 as an
  image. What is left for a later round: `RELEASES_INLINE` (four calls
  against one `rt_truncate` on a return) is unmeasured since the calls
  replaced the sequences; the indirect call through the helper table is
  what the model now charges most (99 million mispredicts on the JIT
  run, a tenth of the estimate), which a direct `call` to a helper
  would remove at the price of a relocation per site, against AS1's
  address-free code; and `Int` flattened into `Value` would make a
  clone or a drop one compare, which the owner did not pick.

## `renyi build` as it exists (decisions AS1 to AS4; session 9, 2026-10-08)

The owner's four answers of 2026-10-08 after the baseline JIT's round
closed (decision AS1): an image file `.ryi` (the bytecode with the
machine code of every code object), later `build --exe`; the generated
code without addresses, so that the image is a plain block of bytes;
every code object compiled, the optimisation level measured; a
mismatched image refused with the fix.

- **Stage A, the address-free code (decision AS2).** `vm.rs` has
  `NativeState` (`repr(C)`: `depth`, `direct_table`, `helpers`,
  `constants`; `calls` went in decision AT2), a field of the VM the
  generated code reaches
  through the VM pointer (`codegen.rs`: `native_state_offset`, the
  `STATE_*` offsets, `Gen::native_state`, `Gen::call` through the
  helper table with `helper_index` into `SIGNATURES` and a `SigRef`
  per helper, the constants of a code object through the table, the
  trampoline's call of the body through the table of bodies);
  `codegen::compile` returns `Compiled { body, trampoline, deopts,
  headers, stats }` as bytes from Cranelift's `Context::compile`
  (`machine_code`: a relocation refuses the function; none occurs), and
  `mod.rs` places them (`CodeArena::place`, `place_many`: `region` pages
  written, flushed with the icache crate `cranelift-jit` used, made
  executable once). `cranelift-jit` and `cranelift-module` are gone;
  `Jit::new(program, opt_level)`, `Jit::isa`, `Jit::host_target`,
  `compile_code`, `compile_everything`, `load_image`, `state_pointers`.
  Measured against AR5's binary: the self-check 11.13 to 11.18 billion
  instructions on machine code (+0.46%), records 451 to 452 million,
  the typed call per turn 130 to 135: the price of the image.
- **Stage B, the image (decision AS3).** `native/image.rs`: `MAGIC`,
  `IMAGE_FORMAT` (the file), `CODE_FORMAT` (what the code assumes of the
  VM: bump it with any change to the layout, the helpers, the statuses),
  `Header { renyi, code_format, target, opt_level }` with
  `Header::mismatch` (the message with the fix), `Image { header,
  code_hash, program, codes }` with `write` and `read` (little-endian,
  lengths first; a unit test round-trips one; the program in the binary
  encoding of `binary.rs` with its bytecode file's hash since decision
  AT3, the `.ryc` text before), `ImageCode { body, trampoline,
  headers, deopts }`, `build(program, opt_level)`, `target_of(isa)` (the
  triple and every ISA flag), `is_image`, `EXTENSION`. `Options.image`
  (`vm.rs`): `Vm::new` loads it into the JIT after creating it (a load
  that fails says so on stderr and compiles as before). The binary
  (`lib.rs`): `build_command` (`--to`, `--opt none|speed`; a `.ry` or a
  `.ryc`; the message counts the code objects and the bytes),
  `load_image_file` (read, `Header::mismatch` against
  `Jit::host_target`, the program from the embedded bytecode),
  `compile_with_sources` returns the image beside the program and the
  hash (`Hashed::Given`, the bytecode file's, stored in the image since
  AT3), and `run`, `record`, `test`
  and `reproduce` pass it to the options; `tests/build.rs` (an image
  runs, records and reproduces as the source does and compiles nothing;
  tests run from one and find their fixtures; a mismatched target, a
  truncated file and a text are refused naming the fix; `build` of an
  image and a bad `--opt` are refused; both levels run). Measured on the
  self-check built as an image: the self-check 11.18 billion
  instructions on the JIT run to 9.23 as an image at `none` (-17.5%) and
  9.07 at `speed` (-18.9%; no compilation at run time, no cold code on
  the interpreter), `bench/records.ry` 452 million to 423 and 418; the
  compiler's image is 17.6 MB (955 code objects, the checker with every
  module it imports) and builds in 3.7 s at `none` and 4.4 s at `speed`;
  `speed` is the default by AR1's rule, compile time being the build's
  (decision AS3). The cachegrind runs use images built under valgrind,
  which hides AVX-512 from the CPU-feature detection, so a native image
  is refused there by the header check, as designed.
- **Stage C, the executable (decision AS4).** `crates/renyi/src/exe.rs`:
  `TRAILER_MAGIC` (`RENYIEXE`), `TRAILER_LEN` (24 bytes: the image's
  offset, its length, the magic; little-endian), `embedded_image` (the
  binary's own file opened, its last 24 bytes read, the image read when
  the trailer is there and consistent with the file's length, `None`
  otherwise: every command of `renyi` pays one open and one small read)
  and `write` (this binary's bytes, the image, the trailer; executable
  on Unix; on macOS `codesign --force -s -` is run and the command is
  the message when it is not at hand). `main_with` (`lib.rs`) asks
  `embedded_image` before dispatching and runs an embedded image through
  `run_embedded`: the program from `load_image_bytes` (the mismatch
  check as for a file, the executable's name in the messages), the
  whole command line as the arguments, the default flags, through
  `run_loaded`, the tail of `run_command` split out (the visibility
  notices, the sandbox refusal, the replay, the manifest, the run, the
  recording, the exit status). `build --exe` (`build_command`): the
  default name is the program's stem with the platform's suffix
  (`<name>.exe` on Windows); the message says "a self-contained
  executable" with the counts. `tests/build.rs`: the executable runs the
  program with its command line (`greet build` prints `Hello, build!`:
  no command of `renyi` is read from it), compiles nothing (the report
  under `RENYI_NATIVE_REPORT`), starts with this binary's bytes and ends
  with the magic; the default name. The compiler's executable is 35 MB:
  the binary (17.7 MB) and its image (17.5 MB).
- **Measured in wall-clock** (the release build on the quiet machine,
  the best of five runs in milliseconds, the median in brackets; the
  images built by the same binary at both levels):

  | program | JIT run | `--interpret` | image `none` | image `speed` |
  |---------|---------|---------------|--------------|---------------|
  | the self-check (`compiler/checker.ry` on `bodies.ry`) | 1841 (1974) | 1950 (2032) | 1691 (1736) | 1511 (1583) |
  | `bench/records.ry` | 62 (76) | 84 (94) | 54 (56) | 54 (57) |
  | `bench/primes.ry` | 42 (52) | 1088 (1156) | 36 (44) | 35 (37) |
  | `examples/hello.ry` | 10 (10) | 8 (9) | 11 (12) | 11 (12) |

  The image at `speed` runs the self-check 18% faster than the JIT run
  (the compile time, the warm-up and the cold code gone), records 13%,
  primes 17%; the two levels tie on the small programs, and a program
  as short as `hello` pays a millisecond for loading its 235 KB of code
  (thirteen code objects of 18 KB each) and gains nothing: an image is
  for a program that runs long enough to compile. The executable of
  `--exe` runs as its image does. The cache simulation (cachegrind with
  `--cache-sim=yes --branch-sim=yes` on images built under valgrind):
  `none` 9.23 billion instructions and 167.6 million first-level
  instruction misses, `speed` 9.07 and 158.0, `speed_and_size` 9.07 and
  158.0 with an image nine bytes larger, so the level does not shrink
  the code. To measure again: `renyi build --opt <level> --to x.ryi
  compiler/checker.ry`, then time `renyi run x.ryi compiler/bodies.ry`
  against `renyi run compiler/checker.ry compiler/bodies.ry`; `tools/bench.py`
  prints the image column for the benchmark programs.
- **What an image and an executable are not yet.** Not portable: the
  target string holds every CPU feature Cranelift detected, so an image
  moves only between machines with the same features (a build on an
  older CPU would run on a newer one, and is refused anyway: equality is
  the rule for now); the executable refuses a mismatch at startup with
  the same message. Not signed or hashed: the run manifest's code hash
  is the bytecode's, the machine code is trusted as the file is. The
  executable is not smaller than the toolchain (the whole `renyi`
  binary comes first, its commands unreachable rather than removed) and
  not compressed; a `.ryi` already built is not an input of `build
  --exe` (it builds from the sources or the bytecode, as `build` does);
  on macOS the copy needs the ad hoc signature the build applies when
  `codesign` is at hand. The image of the compiler is large (17.6 MB for
  955 code objects, 18 KB a code object; its size is what the
  instruction-cache misses of the section below point at), the inline
  sequences of AR4 being what they are; Cranelift's `speed_and_size`
  level does not shrink it (the same code as `speed` within nine bytes
  and the same cache misses on the self-check), so the size is a
  question for the sequences `codegen.rs` emits.

## The baseline JIT (decisions AR1 to AR6; session 9)

The owner's four answers of 2026-10-08 (decision AR1): the baseline JIT
of AG5 (iii) in three stages, each measured on the compiler's
self-check by its instruction count (the rule of AG6: a step stays when
it cuts the count, the round ends when a step brings under 2% or the
self-check is twice as fast as AG6 left it), with `bench/records.ry`
and four micro-benchmarks beside it; fused operations in the bytecode
itself; direct calls between generated functions with register
arguments; the layout of `Value` pinned and the value operations inline.

- **Why.** Micro-benchmarks of the two tiers (instructions per turn of a
  loop, `cachegrind`, the release build of commit 2fcab99; machine code
  against the interpreter): a call with an Integer parameter 668 against
  1133; a field of a local 1043 against 818; a call with a record
  parameter that reads a field of it 1535 against 1183; a record built
  and a field of it read 1357 against 1480. The generated code calls a
  helper per boxed op on the VM's stack, so on the operations ordinary
  programs are made of it is the interpreter less the dispatch plus the
  helper calls, and it lost the field read in the slot that AG6 gave the
  interpreter. The four programs are twenty lines each: a loop of
  `rounds` turns (the first argument, `1000000` by default) around one
  statement (`change total to total + add_one(index)`; `change total to
  total + point.east + index` on a `Point` in a local; `change total to
  total + first_of(point) + index` with `first_of(point: Point) returns
  Integer` reading `point.east`; `let point be Point(east: index,
  north: index)` then `change total to total + point.east`), run at two
  sizes and the counts' difference divided by the size's; they are
  `bench/micro/call_integer.ry`, `field_read.ry`, `call_record.ry` and
  `record_build.ry`, with `call_integer_1000000.ry` and
  `call_integer_200000.ry` the typed call with a literal bound (the
  owner's answer of 2026-10-08: into the repository, held like
  `bench/*.ry` by CI), and `tools/measure_native.sh <binary>` runs the
  whole set.
- **Stage 1, done: `LoadField`.** `Op::LoadField { slot, name, site }`
  is `Load(slot)` followed by `Field { name, site }`: the field read in
  the slot, the holder neither cloned nor dropped. Both emitters emit it
  for `x.field` and `self.field` on a local that is not a move receiver
  (`compile/expr.rs` `member`, `compiler/emit.ry` `emit_member` with
  `local_holder`) and for a field of a variant pattern
  (`compile/pattern.rs`, `field_pattern`); the interpreter's arm reads
  through the site cache (`Vm::op_load_field` in `vm.rs`) and the look
  at the next op that AG6 had put in `Op::Load` is gone; the generated
  code calls `rt_load_field` (one helper where it called two) and
  unboxes a scalar field as it did after `rt_field`; the analysis
  (`infer.rs`) treats it as a load of a boxed slot that pushes
  `abs_of_field`; the file (`file.rs`) writes and reads `OpLoadField`
  (`slot`, `name`, `site`), checks its three indices, and is format 4
  (`compiler/bytecode.ry` has the variant; `emit.ry` writes 4). The
  static count in `checker.ry`'s bytecode: 3,859 of 4,480 `Field` ops
  followed a `Load`; dynamically on the self-check `Load` was 29% and
  `Field` 6% of 101 million ops. Measured against the binary of
  commit 2fcab99 (`cachegrind`, the release build of each): the
  self-check 13.14 to 12.51 billion instructions on machine code
  (-4.7%) and 12.84 to 12.63 billion on the interpreter (-1.6%);
  `bench/records.ry` 571 to 516 million (-9.7%) and 655 to 653 million;
  per turn of the micro-benchmarks, machine code then the interpreter:
  the field of a local 1043 to 902 and 818 to 811, the call with a
  record parameter 1535 to 1394 and 1183 to 1161, the record built and
  read 1357 to 1216 and 1480 to 1464, the Integer call 668 unchanged
  and 1133 to 1104. Kept by the rule of AG6. The profiler's new table
  of operation pairs on the self-check after it (`Load Load` 6.1
  million, `Load Call` 6.0, `Load Const` and `Const Binary` 4.7, `Store
  Load` 3.8, `JumpIfFalse Load` 3.0, `Binary JumpIfFalse` 2.4, `Store
  LoadField` 2.1, `Binary Store` 1.6, the `or` short circuit `Binary
  Dup JumpIfTrue` 1.6, the `match` dispatch `Load IsVariant
  JumpIfFalse` 1.1, of 96 million) says each further fusion saves a
  dispatch and a push or a pop, about a percent of the self-check
  apiece: under the 2% a step must bring, so the fusions stop here
  (decision AR2).
- **Stage 2, done: direct calls (decision AR3).** Every code object
  compiles to two Cranelift functions (`codegen.rs`): the body
  (`renyi_direct_*`; `direct_signature`: the VM, the base, the pc, a
  flag for a direct entry, then the typed parameters in slot order;
  back come a status of `runtime::D_*` and the bits of a typed result)
  and the trampoline (`renyi_code_*`, the `Entry` signature the VM
  calls; `trampoline()`: at the start the typed parameters from the
  slots through `rt_param_*` with `DEOPT` when one does not fit, at a
  loop header zeros for the body to overwrite, the statuses mapped; the
  body's `Return` leaves a typed result in a register only when the
  flag says a direct caller waits, else on the stack through
  `rt_return_*` as before, so that a call from the interpreter costs
  what it cost). A call site (`call_direct`)
  loads the callee's body from the JIT's table (`Jit::direct_table`,
  by code object, filled at compilation; its address is a constant of
  the generated code, the box the JIT lives in keeps it fixed), asked
  `rt_direct_entry` when the table had none yet (which compiled a hot
  callee; since decision AT2 the call goes through the interpreter
  instead, which compiles a hot callee on the way), reads the count of
  generated frames (`Jit::depth`, by its
  address) and goes through `call_through_helper`, the `rt_call` path
  as before, when there is no body or the count is at `DEPTH_LIMIT`;
  else it counts the frame on, boxes the operands of the boxed
  parameters in place, has `rt_direct_frame` (`Vm::push_frame_direct`)
  push the frame with the boxed arguments moved into their slots by a
  mask, calls the body with the typed operands from registers, counts
  the frame off, takes a typed result from the payload or a boxed one
  from the stack, and sends a failure, a handed-back frame (run by the
  interpreter to its end) or an interrupt (its frame abandoned)
  through `rt_direct_after`. The body's returns: `rt_leave_typed` (no
  push), `rt_leave_unbox` (a boxed value where the declared result is
  typed: the bits when it fits, else boxed), `rt_leave_boxed`,
  `rt_left_status` after a helper left the frame. A site is direct
  only when every typed parameter has a typed operand in the caller;
  `infer.rs` holds a parameter's slot to its declared kind
  (`abs_of_params`; a function that stores another kind there stays
  with the interpreter, none in the compiler), so the caller derives
  the callee's signature from the declared types alone. `run_generated`
  counts the trampoline's frame on and off as before, on `Jit::depth`
  now (`Vm::native_depth` is gone). On the self-check with every code
  object compiled: 555 compiled, none rejected, 5.23 million calls from
  generated code, none to the interpreter, no deopt; the default
  tiering compiles 112 and sends 126 thousand of 4.1 million calls to
  cold callees. A first form of the stage asked `rt_direct_entry` on
  every call and carried the depth in the VM, moved by the helpers: it
  measured 12.51 to 12.53 billion instructions on the self-check (no
  gain; `Jit::ready` and the helper cost what the old path had cost),
  which is why the table and the count moved into the generated code.
  Measured against the binary of stage 1
  (`cachegrind`, the release build of each): the self-check 12.51 to
  12.32 billion instructions on machine code (-1.5%) and 12.63 to 12.91
  billion on the interpreter (+2.2%: the code layout, the loop is
  untouched); `bench/records.ry` 516 to 518 million; per turn on machine
  code: the typed call with its argument in a register 542 to 305
  (-44%), the same call with a boxed argument through `rt_call` 668 to
  719 (+8%, the trampoline), the record call 1394 to 1236 (-11%), the
  field and the record built unchanged. Kept by the rule of AG6; under
  the 2% that ends the round by AR1, which decision AR3 flags for the
  owner, the session going on to stage 3 as the round's purpose.
- **Stage 3, done: the pinned layout (decision AR4).** `Pinned<T>`
  (`crates/renyi_vm/src/pinned.rs`): a vector as three words in a fixed
  order, the pointer, the length and the capacity, with `push`, `pop`,
  `truncate`, `insert`, `extend`, `split_off`, `drain_into`, `resize`,
  `room` and `grow` on the words themselves and `with_vec` (a `Vec` made
  of the parts, used, taken apart) for what remains; `Deref` to a slice,
  `From<Vec<T>>` and into one. `Vm.stack`, `Vm.frames`, `Vm.handlers`
  and `Vm.field_cache` are pinned, `Record.fields` and `Variant.fields`
  too, `Vm`, `Frame`, `FieldSite`, `Record` and `Variant` are `repr(C)`,
  `Value` and `Int` are `repr(C, u8)` (24 and 16 bytes as before, X3).
  `value::layout` holds the constants (the tag at 0 in declaration
  order, the payload at 8, `RC_TAGS` the thirteen tags with an `Rc`
  payload, the `Int` tag at 8 and its payload at 16, `RC_VALUE` 16 into
  an allocation, the record's and the variant's offsets) and
  `layout_tests` holds them to the types (the strong count is the first
  word of an `Rc` allocation; an `Rc<str>` is a fat pointer whose
  address is the allocation). In `codegen.rs`: the stack section
  (`stack_vector`, `stack_len`, `stack_ptr`, `item_address`,
  `slot_address`, `copy_value`, `tag_at`, `rc_of` (one branch: the tag's
  bit in the mask or a big `Int`, both payload words read and one
  selected), `retain`, `release` (`rt_drop_at` when the count was one),
  `push_copy`, `write_typed`, `push_typed_value`); `Load`, `LoadMove`,
  `Store`, `Pop`, `Dup`, `Const`, `Nothing`, `IsNothing`, `IsFailure`,
  `JumpIfAbsent` and `JumpIfFailure` in place; `box_position` pushes a
  typed top in place (`rt_insert_*` below the top as before),
  `unbox_top` and `take_bool_top` read the tags (a value that does not
  fit deopts as before, a non-Boolean condition crashes through
  `rt_take_bool`); the prologue checks the stack's capacity once for the
  frame's locals and its deepest operand stack (`rt_room`), so no push
  checks; the frames section (`frames_vector`, `handlers_vector`,
  `frame_address`, `push_nothing_inline`, `push_frame_inline`: the boxed
  arguments moved to their slots by the static mask (every other local's
  tag `Nothing` was written here until decision AT2 moved it to the
  callee's prologue, once per callee), the record written with the
  caller's grant or `rt_frame_grant`'s when the callee narrows,
  `rt_room` and `rt_grow_frames` for the capacities (the count of calls
  at `Addresses::calls` went in AT2); `leave_frame_inline`: the holders
  among the locals
  and the boxed operands under the result released, up to
  `RELEASES_INLINE` (4) of them, else `rt_truncate`, the handlers cut,
  the frame popped; `leave_boxed_inline`: the result moved to the
  frame's first slot, `D_FAILURE` by its tag); every `Return` arm and
  `ReturnNothing` through them (`rt_direct_frame`, `rt_leave_*`,
  `rt_return_*`, `kind_code` and `Vm::push_frame_direct` are gone;
  `Addresses { depth, direct_table, calls }` replaces the two address
  arguments of `compile`); `LoadField` reads the site's cache entry and
  the holder in one path for a record and a variant (the type first in
  both, the tag and the fields selected by the holder's tag,
  `usize::MAX` the record's tag as the cache has it) and copies the
  field with one more reference, else `rt_load_field` as before;
  `RENYI_NATIVE_REGALLOC` selects Cranelift's register allocator (a
  development aid, `mod.rs`). Measured as AR1 says, each sub-stage
  against the one before (the stage-2 binary first): 3a+3b the
  self-check 12.32 to 12.10 billion instructions on machine code
  (-1.8%), the interpreter 12.91 to 12.90, `bench/records.ry` 518 to 459
  million (-11.5%), per turn the typed call 305 to 311, the boxed call
  719 to 633, the field 901 to 731, the record call 1236 to 1055, the
  record built 1215 to 1109; 3c the self-check 12.10 to 11.69 (-3.4%),
  the interpreter 12.94 (+0.3%), records 459 to 464 (+1.1%, which is
  Cranelift's time on the larger IR, the run-time code equal), the typed
  call 311 to 119 (-62%), the boxed call 633 to 587, the record call
  1055 to 873; 3d as first written the self-check 11.69 to 11.70 (+0.1%:
  the IR had grown from 131 to 236 thousand Cranelift instructions in 19
  to 34 thousand blocks since 3b, and Cranelift's own work from 5.6% to
  12.5% of the self-check, which ate the run-time gain), the interpreter
  12.94 to 12.90, records 464 to 429 (-7.5%), the field 730 to 648, the
  record call 873 to 790, the record built 1109 to 1005; made compact
  (the reference-count test one branch with both payload words read and
  one selected, the stack's capacity checked once in the prologue for
  the frame's locals and its deepest operand stack instead of at every
  push, one path for a record and a variant in `LoadField` with the type
  at one offset in both and the rest selected, `RELEASES_INLINE` 4): the
  self-check 11.69 to 11.66 (-0.25%), the interpreter 12.90, records 464
  to 437 (-5.8%), per turn the typed call 119 to 131, the boxed call 587
  to 605, the field 730 to 677, the record call 873 to 849, the record
  built 1109 to 1041; the IR 200 thousand Cranelift instructions in 15
  thousand blocks, Cranelift 10.3% and the generated code 16.6% of the
  self-check (the first form was faster per turn and slower on the
  self-check: the compact test and the selected path cost at run time
  what the compile time pays back, so a branchy test for a value without
  a reference, cheap at run time and small in IR, is a form still to
  try). The register allocator: `single_pass` halves Cranelift's time
  (658 to 339 ms on the self-check) and loses more in the code (the
  self-check 11.70 to 12.19 billion, +4.2%; records +11%, the record
  call per turn 789 to 1025), so `backtracking` stays.
- **The tiering (decision AR5), the owner's choice for the round's next
  step.** The hotness factor swept on the self-check with the stage-3
  binary (`RENYI_NATIVE_HOT`, the same binary for every value): 500
  13.15 billion instructions (205 code objects compiled, Cranelift 1.1
  s), 1000 12.09, 2000 11.66 (112 compiled), 4000 11.24, 6000 11.16,
  8000 11.13 (51 compiled, 1,477 ops, Cranelift 146 ms), 12000 11.23 (40
  compiled), 16000 11.39, 32000 11.78; `bench/records.ry` the other way,
  433 million at 500 to 437 at 2000, 451 at 8000 and 469 at 16000 (one
  hot loop in `main`: every doubling is more turns interpreted before
  the loop-header entry). `HOT_FACTOR` is 8000: the self-check -4.5%,
  records +3.2%, by the rule. A second factor for the loop-header entry
  (`LOOP_FACTOR` 500 beside the call factor, so that a running loop
  compiles sooner) was built and measured and lost: with the call factor
  2000 the self-check 11.66 to 12.05 billion (137 compiled, the loops of
  twenty-five more code objects whose turns did not pay), with 4000
  11.24 to 11.77; the code was reverted, the numbers are in AR5. The
  shape to try next, if the round goes on: a cheaper first tier (the IR
  without the inline sequences of AR4 for a code object that just became
  warm, the full one when it stays hot), so that the cost of being wrong
  about a function falls instead of the threshold rising; the count of
  runs so far predicts the runs to come poorly for the self-check's
  medium functions.
- **The round's end (decision AR6).** The owner closed the round on
  2026-10-08 after the tiering: against the binary AQ left, the
  self-check 13.14 to 11.13 billion instructions on machine code
  (-15.3%), `bench/records.ry` 571 to 451 million (-21%), the typed
  call per turn 668 to 130, the field 1043 to 677, the record call
  1535 to 849, the record built 1357 to 1041 (`tools/measure_native.sh`
  on the release build of d26dfac; the interpreter within 3% of where
  it was). A later round starts from the profile below and the cheaper
  first tier of AR5.
- **After stage 3.** The profile of the self-check on machine code
  (`cg_annotate`, the scratch directory): the generated code 16.6%,
  `run_frames` 8.7% (`main` and the 443 code objects called but cold),
  Cranelift and regalloc2 10.3%, `drop_glue::<Value>` 5.1%,
  `call_from_stack` 3.0%, `status` 2.9%, `call_primitive` 2.4%,
  `Value::clone` 2.2%, `binary_values` 1.8%, `rt_binary` 1.8%,
  `push_frame_in_place` 1.7%, `Value::eq` 1.7%, mimalloc about 3%,
  `refined_record` 0.8%, `rt_construct_variant` 0.8%, `op_load_field`
  0.7% (the cache misses), `Jit::ready` 0.7% (the IR built), `op_with`
  0.7%. The IR's size is now a cost the round must count: every inline
  sequence is Cranelift time at each of the 112 compilations, so a
  change is measured on the self-check with both effects, and the next
  steps that remain are the primitive boundary (`status`,
  `call_from_stack`, `call_primitive`, `run_primitive`, about 9%: the
  `Result` moved, the arguments drained into the scratch vector, the
  grant and budget checks of a pure primitive), the interpreter's own
  share (`run_frames` 8.7%: `main` and the 443 code objects called but
  cold under `HOT_FACTOR`, against the compile time of each), the clone
  and the drop that remain in the helpers and the primitives
  (`drop_glue` 4.9%, `clone` 2.1%), and the boxed binary operations
  (`rt_binary` and `binary_values`, 3.6%).
- **Measuring here.** `tools/measure_native.sh <binary>`: `valgrind
  --tool=cachegrind --cache-sim=no <binary> run [--interpret]
  compiler/checker.ry compiler/bodies.ry` for the self-check (about
  four minutes a run), the same on `bench/records.ry`, and the micro
  programs of `bench/micro/` at two sizes; the binary before a step
  kept beside the binary after it (the release build of each, 1.97.0
  here). `RENYI_NATIVE_REPORT=1` prints the compile statistics (the
  IR's size in Cranelift instructions and blocks, the time by phase,
  the cold callees), `RENYI_NATIVE_HOT=<n>` sets the hotness factor and
  `RENYI_NATIVE_REGALLOC=<algorithm>` the register allocator for one
  run, and `cg_annotate` on the self-check shows Cranelift's own
  share, which the IR's size moves.

## The profile-guided round on strings and JSON as it exists (decision AQ; session 9, 2026-10-08)

The first of the interspersed performance items the owner set on
2026-10-07 (next steps, item 5), measured on the two benchmarks the
positioning compares with CPython. Session 8 profiled the two programs
on the owner's machine and paused with the change written as a script
(commit 44763c4); session 9, the first in the cloud environment,
applied it, reviewed the diff, ran the gates and measured it. The entry
AQ1 of the decisions has the profiles and the change in full.

- **Where the code is.** `crates/renyi_vm/src/natives/json.rs`: `Path`
  (the position as a chain of borrowed segments, rendered only for a
  `Mismatch` or `Constraint`), `field_type` (a `Cow`: the declared type
  borrowed unless the type has parameters), `Naming::key` (a `Cow` too),
  `decode_at` over `vm.program` borrowed, the `Sink` trait with
  `TreeSink` (the tree of `encode`, kept by the recording,
  `http.post_json`, `server.ok_json` and the Python bridge) and
  `TextSink` (the text of `render`, `render_indented` and `render_with`,
  no tree in between), `walk` (the one encoder over a value) and the
  unit test that holds the text sink equal to `write_json` on both
  layouts. `crates/renyi_json/src/lib.rs`: the reader keeps the text
  beside its bytes and `string` copies a run of plain bytes at once;
  `write_string` copies the runs between escapes; both it and `newline`
  are `pub` for the sink. `crates/renyi_vm/src/value.rs`:
  `Value::character` and the per-thread `ASCII` table;
  `natives/prelude.rs`: `characters(text)`, used by `characters()`,
  `split("")` and `Vm::iterate` over a text (`vm.rs`);
  `natives/regex.rs`: `CACHE` and `CACHE_LIMIT` (256, then emptied);
  `native/runtime.rs`: `op_concat` joins the pieces on the stack, the
  text sized first, an Integer written straight into it.
- **Conformance case 54**, `tests/conformance/programs/json_paths.ry`
  with `expected/json_paths.out`: the differential program of the
  session, printed by the binary before the change and held to by the
  binary after it on both tiers and by the development build. It
  covers the path of every `Mismatch` and `Constraint` error (indices
  under keys, the variant's `kind`, a map's key, a missing field, a
  refined subtype), a document with control characters, escapes and
  non-ASCII text on both layouts, the three namings there and back,
  `characters()`, `split("")`, a loop over a text, every kind of value
  in a hole (a big Integer, a Float, a Decimal, a Boolean, a record, a
  variant, a text) and the pattern cache (a thousand matches of one
  pattern, three hundred of different ones, past the cache's limit).
- **The numbers** (2026-10-08, the cloud environment: Linux, four
  cores, the release build with rustc 1.97.0, the two binaries built
  from the same tree with and without the change; `cachegrind` counts
  user-space instructions, `tools/bench.py --runs 5` the wall-clock
  time, the two binaries back to back):

  | program | instructions, machine code | instructions, interpreter | wall-clock, machine code | wall-clock, interpreter | CPython |
  |---|---|---|---|---|---|
  | `strings` | 1.74 to 1.39 billion (-20%) | 1.97 to 1.62 billion (-18%) | 282 to 226 ms | 284 to 262 ms | 106 ms |
  | `json_round_trip` | 2.34 to 1.00 billion (-57%) | 2.36 to 1.01 billion (-57%) | 314 to 126 ms | 303 to 125 ms | 230 ms |
  | `checker.ry` on `bodies.ry` | 13.40 to 13.14 billion (-2.0%) | 13.12 to 12.84 billion (-2.1%) | 2895 to 2679 ms | 2572 to 2496 ms | |

  `primes`, `records` and `hello` are unchanged within the machine's
  noise, which moved a repeated measurement by a tenth (CPython's
  `primes` twin by a fifth between the two rounds): a wall-clock
  difference under a tenth says nothing here, and the instruction
  counts are the measure, as in AG6. Against the owner's targets: JSON
  is past CPython (1.8 times its speed, from 0.7); strings stands at
  half of CPython's speed (from four tenths); the self-check moved by
  two percent. The numbers session 8 wrote down to beat (CI's
  development build of a2b0702, one run: `strings` 179 ms and
  `json_round_trip` 386 ms against CPython's 43 and 89 ms) are of
  another machine and another build, and CI prints the new ones on the
  next push.
- **Where the rest goes** (`cachegrind` on the interpreter after the
  change, `cg_annotate`): on `strings`, 40% in the interpreter loop, 30%
  in the primitive boundary once per character (`call_primitive`, the
  move of the arguments into the scratch buffer, `run_primitive`,
  `text_contains`, the drops, `guarded`, `effect_of`), 7% dropping and
  4% cloning values, 5% the `contains` search itself, 2% `characters`,
  2% `op_concat`, the allocator under 2%; the page faults of the 29 MB
  list of one-character texts (now one allocation at its exact length)
  are kernel time outside the count, which is why the interpreter's
  wall-clock gain is smaller than its instruction gain. On
  `json_round_trip`, 21% in the reader, 8.5% in `write_string`, 8% in
  `decode_at`, 4% in `walk`, 7% growing vectors and texts, 9% in the
  allocator, 5% in the interpreter loop, 3% in the keys, 2% parsing
  Integers, 1% finding a field by name: the plain cost of reading and
  writing the format. The profiles are `cg_annotate` output of the
  session; nothing of them is in the repository.
- **Not done, on purpose**: `Text` stays `Rc<str>` (decision X3), so
  `change out to "{out}{piece}"` in `compiler/` still copies the
  accumulated text per piece (the lexer's `scan_segment` stays
  quadratic in a token's length); the pure primitive boundary stays as
  AG6 left it, and it is now the largest item after the loop on
  `strings`; `Text.matches` is not on the compiler's path (three calls
  in `project.ry` and `refine.ry`), so the cache changes nothing for
  the self-check; `http.post_json` and `server.ok_json` still build the
  tree and write it, which `TextSink` could replace in a line each
  (not on any benchmark).
- **How it was gated**: `cargo fmt`, `cargo clippy --all-targets -D
  warnings` and `cargo test` (315 tests) with rustc 1.94.1 (the CI
  toolchain, installed beside the environment's 1.97.0 so that the
  lints agree), the conformance suite by both runners (54 cases), the
  corpus canonical, `compiler/*.ry`, `bench/*.ry` and the starter pack
  checked, formatted and tested, the lint. The release binaries and
  the measurement used 1.97.0 (both binaries alike).
- **How to measure here**: build the previous commit's release binary
  first and keep a copy (`cargo build --release`, then copy
  `target/release/renyi` out of `target/`), apply the change, build
  again; `valgrind --tool=cachegrind --cache-sim=no <binary> run
  [--interpret] <program>` prints `I refs`, deterministic under load;
  `python tools/bench.py <binary> --runs 5` needs a quiet machine (no
  build or judge running). On the owner's machine the lane's
  `ab_stat.sh` (`perf stat`) does the same with cycles beside the
  instructions.

## The VM as it exists (`crates/renyi_vm`)

- **The loop** is described in the section above (decision X3): one
  loop over the ops with the running frame's state in locals, the
  arguments of a declared function left in place, one handler stack,
  the field cache per site, the reusable argument buffer of the
  primitives. A new op that can fail must leave through `try_op!` or
  `crash!`, never `?`, or a crash is located at the last synchronised
  pc of the frame.
- **Pipeline.** `renyi run file.ry` reads the file and its imports,
  `check_project`s them (an error stops here, printed as `check` prints
  it), `compile_project` lowers every non-library body to bytecode, and
  `run_main` runs `main` with the command-line arguments. `renyi test`
  compiles the same way and runs every `example:` line and `test` block in
  source order, one line each, then `N passed, M failed[, K skipped]`;
  exit 1 when any fails. A failing `main` prints `main failed with <error>`
  (exit 1); a crash prints the message and `file:line` (exit 2);
  `environment.exit(code)` exits with the code.
- **Compiler** (`compile/`). One `Code` per function body, test, constant,
  `example:` line (one for the call, one for the expected value or the
  `fails with` pattern) and refinement condition. Names are never resolved
  by the compiler: it reads the references the checker recorded
  (`Target::Function`, `AbilityMethod`, `Constant`, `Type`, `Variant`,
  `Number` for the type of every numeric literal and of every `sum`,
  `Result` for a call whose result type only the context decides, such as
  `json.parse`). Locals are slots on the value stack; `change x to
  x.method(...)` loads the receiver with `LoadMove` when the arguments do
  not read `x`, so lists, maps and sets grow in place (decision O1).
  `Op::GroupFold` folds `sum`, `count`, `first`, `any` and `all` per group
  after `group by` (decision M1).
- **Failures.** A fallible call that fails leaves a `Failure` value. The
  value of an `otherwise` and the subject of a `match` with
  `success`/`failure` arms are *handled regions* (`PushHandler` /
  `PopHandler`); a failure inside one unwinds the operand stack to the
  region's start and jumps to the handler, where the fallback outcome runs
  (`otherwise fail` passes the same failure on). Outside any region a
  failure is the result of an example or test, and a crash
  (`unhandled failure: ...`) in a function, which the checker should have
  prevented. Constructing a refined type runs its compiled conditions;
  one that does not hold gives `Failure(ConstraintViolation(type_name,
  detail))` with the condition's source text as the detail.
- **Values** (`value.rs`). `Integer` is `i64` with a `BigInt` spill
  (decision B9), `Decimal` an emulation of IEEE decimal128 (34 digits,
  round half even, exponent kept, so `9.50 + 5.00` prints `14.50`; division
  exact when it terminates, else rounded; decision J16, R5: equality by
  value), `Float` is `f64`. `maybe` is the value or `Nothing`. Records,
  variants, lists, maps and sets (insertion order, K9) are `Rc`; a subtype
  value (`Path`, `Permission`) is its base value. `Date` is the library's
  record, `Instant` and `Duration` are milliseconds.
- **Derived abilities** (`render.rs`). `ToText`: a declared `to_text` of
  the value's type (a user implementation or a library primitive such as
  `Date`'s) when there is one, else a variant as its bare name, a record in
  constructor form with nested text quoted, lists `[1, 2]`, maps `{k: v}`,
  sets `[a, b].to_set()`, `nothing`. `Compare`: a declared `compare`, else
  records by their `can Compare by` fields or every field, variants by
  position then fields; `Equal` and `Hash` are structural.
- **Natives** (`natives/`). All of the prelude; `std.console`,
  `std.environment`, `std.time` (civil-date arithmetic, ISO instants),
  `std.random` (xorshift, seeded from the clock), `std.filesystem`,
  `std.json` (own reader and writer; decoding by the context type with the
  derivation rules of `04-stdlib-sketch.md` section 7, `as` names and
  `Naming`; `Constraint` errors from refinements), `std.csv` (RFC 4180
  cells, rows with line numbers), `std.regex` and `Text.matches` on the
  `regex` crate; `std.http` (`natives/http.rs`) on `ureq` 3 with rustls:
  a 30-second limit per request, redirects followed one hop at a time with
  every host checked against the grant (`HostNotAllowed` for a hop outside
  it, at most ten hops), 303 and a redirected POST become GET, a non-2xx
  answer is `Status(url, status, body)`, duplicate headers joined with a
  comma; `std.sqlite` (`natives/sqlite.rs`) on `rusqlite` with SQLite
  compiled in: a `Connection` is a native value (`<connection PATH>` in
  text and in recordings; its path is the scope of every call on it),
  parameters bind by type (`Decimal` and an `Integer` past `i64` as text,
  `Boolean` as 0 or 1), `query` decodes each row into the record of the
  context type by column name (`as` names honoured) and fails with
  `Mismatch(column, expected, found)` when a cell does not fit the field
  or a row does not satisfy the record's refinement; `std.server`
  (`natives/server.rs`) hand-written over `std::net::TcpListener`:
  HTTP/1.1, one request at a time, `PortInUse` and `PermissionDenied`
  from `bind`, `PermissionDenied` and `OverBudget` from the boundary
  (decision Y1), the handler called through `Vm::call_function` with a
  `Request` record (lower-cased header names, percent-decoded query), the
  answer sent with `content-length` and `connection: close`.
- **The primitive boundary** (`Vm::call_native`, `grant.rs`,
  `recording.rs`; `06-runtime-guarantees.md` sections 1, 2 and 4). A
  primitive with `needs` passes one function. Its *effect* is its declared
  capability plus the scope its argument names (the normalised path, the
  host of the URL, the variable). `begin_run` sets the run's grant: the
  `needs` of `main` or of the test, narrowed by `--deny` (which refuses to
  start when any function needs the capability), `--allow-host`,
  `--allow-read`, `--allow-write` (an ancestor such as `filesystem` is
  split into its children first), with one counter per `at most` budget,
  declared or added by `--at-most`. An effect outside the grant fails with
  `FileError.PermissionDenied`, `HttpError.HostNotAllowed`,
  `DbError.PermissionDenied` or `StartError.PermissionDenied`, past a
  budget with `OverBudget`, and crashes when the primitive cannot fail
  (decision Y1). A replay first checks the recording's `grant` header
  against the grant, then each call (decision Y2). Live, the
  call runs and is appended to the recording (`Call { capability,
  primitive, arguments by name, outcome success/failure, duration_ms,
  at_ms }`, JSON by the `ToJson` rules); replaying, the entry with the
  same primitive and arguments answers it (identical entries in sequence
  order), its outcome decoded by the declared result and error types
  (`FunctionMeta.returns`/`fails`; a type-parameter result uses the
  context type the checker records for effectful primitives), nothing is
  written or sent, and a call the recording lacks crashes naming the call
  and the nearest recorded one. `--explain` narrates on stderr: `Purpose.
  (module.name, param: value)` on entry, `-> value` on exit, and each
  effect as `console "text"` or `capability name(args) -> result[, N
  ms]`, indented by call depth.
- **The grant stack** (decision Q1; `grant::within`, `Vm::frame_grant`).
  Every frame carries its effective grant: the enclosing frame's (the
  run's for `main` and for a test body), narrowed by the `needs` of the
  function the frame runs. On a line the needs mention, a granted
  capability takes the scope both allow (`filesystem.read("work")` inside
  a function declared `filesystem.read("work/data")` becomes the latter;
  a parent such as `filesystem` is split into its children first); a line
  the needs do not mention passes through, since the function can reach
  it only through a function value whose effects its caller covered (the
  sketch: effects of a function-typed parameter flow to the call site).
  `call_native` and the redirect check of `std.http` test the innermost
  frame's grant; a denial that crashes names the function whose `needs`
  narrowed the grant. The narrowed grant is remembered per function while
  the enclosing grant is the same `Rc`, so a function called in a loop
  intersects once (open item R7-4). Budgets stay the run's.
- **The run manifest and `renyi reproduce`** (decisions Q2 and S5;
  `recording::Manifest`, `runner::reproduce`). `renyi record` writes into
  the recording's header the toolchain (`renyi 0.0.1`), the source path,
  `code` (the content hash of `main` from the project map, which covers
  everything `main` reaches), the arguments, the environment variables
  read (`environment.get`, each with the hash of its value, `<redacted>`
  kept, `null` when unset), the outcome (`finished`, `failed with ...`,
  `exited with N`, `crashed: ...`) and `output` (the SHA-256 and length
  of the standard output, hashed by a writer wrapped around stdout while
  the run writes). `renyi run --manifest` prints the same header on
  stderr and writes no file. `renyi reproduce FILE [program.ry]` loads
  the recording (refused without a `code`), compiles the program named by
  the argument or by `source`, refuses when `main` hashes differently,
  warns when the toolchain differs, then replays under the recorded
  arguments with the console output written (`Options::replay_output`)
  and reports every difference: the outcome, the output hash and length,
  and recorded calls never reached; the replay itself names the first
  call that differs. Hand-written fixtures have no manifest and need
  none.
- **Not yet.** `run concurrently` and `concurrently` queries run their
  tasks one after the other by decision S2 (`within` sets a deadline that
  is checked between statements or items and fails with `TimedOut`);
  green threads would be an improvement within that decision, not a
  reversal.

## The embedding API as it exists (decisions AP1 and AP2; session 8, 2026-10-08)

The fourth and last step of M5 in the owner's order: `renyi::Sandbox`,
the grant as a `needs` clause, the memory budget, `renyi run --sandbox`.
The guide is `docs/embedding.md`.

- **The API.** `crates/renyi/src/sandbox.rs`, exported by the crate:
  `Grant` (`capabilities`, `memory` in bytes; `parse` reads the `needs`
  spelling through `renyi_syntax::parse_grant` and checks each
  capability is known, takes its scope and names known sinks;
  `from_json` and `from_file` read `{"grant": ..., "memory": "256
  megabytes"}` strictly; `parse_memory`, `bytes_text`), `Sandbox`
  (`load(path, grant)` through `compile_sources`, a `.ryc` refused;
  `load_source(name, text, grant)` through the new `compile_file`; the
  public functions of the main module from `renyi_index::index_files_in`
  with the tool schemas from `tools_of_in`; `module`, `grant`,
  `functions`, `warnings`; `call(name, args)`: the name must be a
  public function, the arity must match, every `need` must be covered
  by the grant, then a `Vm` of its own with `Narrowing { sandbox }` and
  `Options.memory`, `begin_run(function.needs)`, the budgets of the
  previous call resumed (`Vm::budgets`, `resume_budgets`), the result
  mapped to `CallError::{Refused, Failed, Crashed, Exited, OverMemory}`;
  `guarded(value, capability)` tags a value with the bit of the grant's
  guard), `Function` (name, signature, purpose, parameters, needs,
  `exposed_as_tool`, `input_schema` as a JSON text).
- **The VM.** `memory.rs`: `Counting<A>`, a `GlobalAlloc` wrapper with
  the statics `INSTALLED`, `ACTIVE`, `NET`, `PEAK`, `LIMIT`, `OVER`
  (one load per allocation while inactive; `begin(limit)`, `end()`,
  `over()`, `peak()`, `installed()`). `Options.memory`, `Vm.memory` and
  `Vm.counting`; `begin_run` starts the count (refused without the
  allocator, or while another budget is in force) and `Drop for Vm`
  ends it; `memory_check` at the `call!` macro, at a backward `Jump`
  and at `call_primitive`; a budgeted run is the interpreter's.
  `Interrupt::OverMemory { limit, used }` and `RunOutcome::OverMemory`.
  `Narrowing.sandbox`: `effective` intersects the declared capabilities
  with it (`within`, then only what the sandbox covers) and adds its
  budgets; `begin_run` adds its guards after the declared ones.
- **The binary.** `renyi::Allocator` (mimalloc under `Counting`)
  declared in `src/main.rs`, no longer in the library, so that a host
  links its own; `Counting` and `Value` re-exported. `--sandbox
  <grant.json>` among the flags of `run`, `record` and `serve`
  (`sandbox::apply_flag` reads the file into `Narrowing.sandbox` and
  returns the memory; `sandbox::refusal` refuses when `main` needs a
  capability the grant does not cover; `test` refuses the flag);
  `exit_of` exits 2 on `OverMemory` with the limit and the peak in
  words. `renyi_syntax::parse_grant` parses a `needs` clause alone.
- **Held equal.** `crates/renyi/tests/sandbox.rs` (its own counting
  allocator): a module of eight public functions loaded from a text,
  their list with purpose, parameters, needs and signature; `add`,
  a private function refused, the arity refused, `shout` under
  `console`, a failure rendered, a crash with its location; `shout`
  refused under an empty grant and `add` still answered; a scope
  (`filesystem.read` of one directory, another refused at the boundary
  with `PermissionDenied`); `at most 2 per run` counting across three
  calls; a guarded value printed but refused into a file with
  `Guarded`, the file not written, a plain value written; a memory
  budget of 8 megabytes stopping a list of four million items with
  `OverMemory` and the next call answered; and the binary with
  `--sandbox`: `hello` under `console, environment` with a memory
  budget, refused under `console` alone naming `environment`, a grant
  with an unknown capability refused, `test --sandbox` refused. The
  unit tests of `sandbox.rs` cover the grant's spellings and the
  memory's.
- **Not covered.** The console output of a sandboxed call goes to the
  process's streams, not to the host; the count is the process's, so a
  multi-threaded host's other threads count during a budgeted call and
  one budget is in force at a time; a bytecode file cannot be loaded
  (no visibility in the format); each call builds a VM, which costs
  the setup of the natives table and the field cache per call; the
  cost of the counting allocator on the benchmarks is not measured
  (one relaxed load per allocation and deallocation by design); no C
  API yet (decision A5).

## The watch of `serve` as it exists (decision AO1; session 8, 2026-10-08)

The third step of M5 in the owner's order: `renyi serve --watch`, a
reload between two requests on the resident world of AN1.

- **The command.** `crates/renyi/src/serve.rs`: `renyi serve` parses
  the flags of `run` (`--watch` is one of them now; `run` and `test`
  refuse it); without `--watch` it is `run_command`. With it, the first
  version is compiled as `run` compiles (its errors end the command), a
  `Workspace` in canonical mode is opened on the file's project (the
  nearest `renyi.json` above it, else its directory) and refreshed once
  for the map, and `main` runs in a loop: every run gets
  `Options.watch`, a closure over the watch state, and
  `Options.listener`, the socket the previous run left; a run that
  ends with `RunOutcome::Reload` takes the version the watch prepared
  and starts again, any other outcome is `run`'s exit status
  (`exit_of`, shared with `run`). The watch's `poll`:
  `Workspace::refresh`, nothing to do unless the world was declared
  again; then `compile_sources` (errors: "the new version has errors;
  still serving the last good one:" with the diagnostics); the
  program's sources compared text for text with the serving version (a
  change elsewhere in the directory is no new version); `--deny`
  checked as at start; the map built and diffed with
  `renyi_index::diff`; the message "reloading <file>: 2 definitions
  changed: app.handle (body changed); app.version (added)", a signature
  change as "signature `old` -> `new`". `--watch` takes `--deny`,
  `--allow-*`, `--at-most`, `--explain` and `--interpret`; `--replay`,
  `--manifest`, `--profile`, `--to`, `--redact` and a `.ryc` file are
  refused.
- **The VM.** `Options.watch: Option<Box<dyn FnMut() -> bool>>` and
  `Options.listener: Option<TcpListener>`, copied into the `Vm`;
  `Interrupt::Reload` and `RunOutcome::Reload` ("stopped for a new
  version" in a manifest); `Run.listener` carries the socket out.
  `natives/server.rs`: `serve` continues on the handed socket when its
  port is the one asked for, else binds; under a watch the listener is
  non-blocking, `accept` is tried every 20 ms and the watch asked every
  500 ms (`WATCH_INTERVAL`, `WATCH_SLEEP`); when it answers true the
  listener goes back into `vm.listener` and the native returns
  `Err(Interrupt::Reload)`; an accepted stream is set blocking again.
  A request that arrives while a new version compiles waits in the
  backlog.
- **Held equal.** `crates/renyi/tests/serve.rs` runs the binary on a
  one-file service under `target/serve/`: "v1" answered; the body
  edited and the next request answered "v2", the message naming
  `app.handle (body changed)`; a version with an unknown name reported
  with its diagnostic while "v2" is still served; a definition added
  and the handler changed, "v3" with both named; the error reported
  once, no crash; and the refusals: a first version with errors (exit
  1, the diagnostics on the standard output), `run --watch` ("`--watch`
  is an option of `renyi serve`"), and `serve` without the flag as
  `run` on `examples/hello.ry`. The sequence of three reloads takes
  3.6 s in the test, the poll interval included.
- **Not covered.** `main` is run again whole, so what it did before
  `server.serve` (a print, a database opened) is done again; a file
  imported from outside the project's directory is compiled but not
  stat-ed, so a change to it alone is not noticed; the watch does not
  look while a request is being handled, so a reload waits for the
  request in flight; `serve_limit` stays the tests' business.

## The language server as it exists (decisions AN2 and AN3; session 8, 2026-10-07)

The second step of M5 in the owner's order: `renyi lsp`, a standard
server over standard input and output on the resident world of AN1,
and the client in the VS Code extension.

- **The command.** `crates/renyi/src/lsp.rs`: the base protocol's
  framing (`Content-Length`, then the JSON body), one `Workspace` in
  as-written mode over the folder `initialize` names
  (`workspaceFolders[0]`, else `rootUri`, else `rootPath`, else the
  working directory), the documents the editor opens as overlays
  (`didOpen`, `didChange` with whole texts, `didClose`), the
  diagnostics of every file of the folder published after each change
  for the files whose diagnostics changed (the workspace's plus
  `check_layout`, sorted, each with its code, `renyi` as the source
  and the fix after the message), `shutdown` and `exit` (exit status 0
  after a shutdown, 1 otherwise), `-32002` before `initialize`,
  `-32601` for a method it does not serve. `lsp/describe.rs`: what is
  under a position (the innermost reference there, else the
  declaration whose name is there), described by its signature
  (`renyi_index::function_signature` and `type_signature`, now public)
  and its purpose, located at its declared name; the outline (functions,
  types with their fields or variants, abilities with their methods,
  implementations with theirs, constants, tests); positions as
  zero-based lines and UTF-16 code units. Hover on a library definition
  reads the library's declaration files; definition answers null for
  it. URIs: `file:` with percent-decoding, a drive letter's leading
  slash dropped, an open document's URI reused as the editor spelled
  it.
- **The workspace** gained `as_written()` (no formatting; the tree,
  the diagnostics and the map over the text on disk or in the overlay,
  the mode of `renyi check`), the lazy map (`index(header)` builds it
  when the last refresh changed anything, `index_built()` reads it;
  `refresh()` no longer takes the header), `file`, `files`, `module_of`
  and `file_of`. `tests/refresh.rs` holds the as-written diagnostics of
  the corpus and the compiler equal to `check_project_in` on the files
  as loaded.
- **The client.** `editors/vscode/src/extension.js` starts `renyi lsp`
  through `vscode-languageclient` for the `renyi` language; the setting
  `renyi.path` names the binary (`renyi` on the PATH by default); a
  server that does not start is one warning. `npm run build` bundles
  it with its library into `out/extension.js` (esbuild); `package.json`
  carries `main`, the activation on the language, the setting and the
  scripts; `.vscodeignore` leaves `src/`, `node_modules/` and the
  lockfile out of the `.vsix`; `package-lock.json` is committed for
  `npm ci`. CI builds the bundle on every push and the release
  workflow before packaging. The repository ignores `node_modules/`
  and `out/` under the extension.
- **Held equal.** `crates/renyi/tests/lsp.rs` drives the binary over
  pipes on a two-file project under `target/lsp/`: the handshake, the
  empty diagnostics of both files, a buffer opened with an unknown name
  in a text hole after an emoji (the error at its UTF-16 column, the
  fix in the message), the buffer restored (empty again), hover on a
  call (the signature and the purpose), definition into the other file
  at the function's name, hover on `console.print` (the library's
  declaration and purpose), the outline of a file (a record with its
  field, a function), hover on nothing, an unknown method, the clean
  exit after `shutdown`; and a request before `initialize`, refused,
  with the input closing an unclean exit. Unit tests cover the framing,
  the URIs and the positions.
- **Measured** (`time_lsp.py`-style driver: initialize, the first
  diagnostics, then a keystroke inside a body followed by a hover; a
  development build on Windows): `compiler/` 1200 ms to
  the first diagnostics, 92 to 135 ms per keystroke;
  `examples/` 80 ms, then 15 to 22 ms.
- **Not covered.** No completion, references, rename or formatting
  through the server (the editor's `renyi format` is the command line);
  the client has not been run in a VS Code window here, only bundled;
  incremental synchronization is not offered (whole texts); a
  workspace with several folders gets a server per folder from the
  client library's default.

## The resident world as it exists (decision AN1; session 8, 2026-10-07)

The first step of M5 in the order the owner set on 2026-10-07 (the
resident `World`, then the LSP, then `serve --watch`, then the
embedding API; decision AN1 records the four answers, AN2 the LSP's
shape): a crate of its own, `crates/renyi_workspace/`, that `renyi mcp`
holds between calls.

- **The crate.** `Workspace::new(root, &library)` parses the library's
  declaration files once and reads nothing; `as_written()` switches
  it to the editor's mode (the section above); `refresh()` does the
  work and reports it (`Refresh`: files read, files parsed, whether
  the world was declared again, items checked, items reused);
  `index(header)` builds the map when the last refresh changed
  anything and `index_built()` reads it; `checked()`, `file(name)`,
  `files()`, `canonical_text(file)`, `diagnostics(file)`,
  `module_of(name)` and `file_of(module)` answer from the last
  refresh; `set_overlay(name, text)` and `clear_overlay(name)` make a
  text stand in for a file on disk (an editor's unsaved buffer). A
  file is named relative to the root or as the map names it.
- **A refresh.** The `.ry` and `.renyi` files under the root (the
  store `.renyi/` left out) are listed and stat-ed. A file whose size
  and modification time are those kept is skipped, unless it was
  stamped within two seconds of its last write (file systems round the
  time; such a file is read and its text compared). A file read with
  the same text keeps its tree and its checks. A file with a new text
  is tagged (`renyi_check::tagged`), formatted and parsed as `renyi
  index` does (`canonical_files` now carries the `foreign` and
  `python` tags through, not only `package`), its declarations
  fingerprinted, its items' texts kept; when the fingerprint is the
  old one, the old items' checks are inherited where an item's text is
  unchanged, their spans moved to where the item now sits. Otherwise,
  and when a file appeared, disappeared or stopped parsing, or
  `renyi.json` or `renyi.lock.json` at the root changed (the files are
  tagged again), every item is checked again, and only then are the
  dependencies resolved again (`imported_files`). The world is
  declared again from the kept trees (the library's modules cloned,
  then the own files in path order and the dependencies in the order
  brought, as `load_project` orders them); each module's items are
  checked with `check::check_item` or reused; the module's diagnostics
  are assembled as `check_project_in` assembles them (parse,
  `module-name`, the world's, the items', the module's
  `purpose-missing`, sorted by position); the map is built with
  `renyi_index::index_checked` from the canonical files and the
  checked project, when `index` is asked for.
- **The fingerprint** is the text of everything a body elsewhere can
  see: the module's head (the text before the first item: the name,
  the docs, the imports), each function's signature and docs (the text
  before its body; the docs because `deprecated` warns at call sites),
  an implementation's head and its methods' heads, a test's name,
  needs and recording, and a type, an ability or a constant whole. A
  change to a body costs one check; a change to a signature costs them
  all.
- **The checker** gained `check::check_item(world, module, index)` and
  `check::module_purpose_diagnostic`, over which
  `check_module_with_references` is now written (`Checker::new` is a
  pure constructor, so an item checked alone is checked as in the
  loop); `module_name_mismatch` is public. The index gained
  `index_checked` (the second half of `index_files_in`) and
  `canonical_files` public.
- **Held equal.** `tests/refresh.rs`: the corpus, `compiler/` and
  `starter/workflows` give the map and every module's diagnostics of a
  fresh `index_files_in` and `check_project_in`, and a second refresh
  parses, declares and checks nothing; a two-file project is taken
  through a body edit (one item checked, three reused), a signature
  edit (all four, the caller now wrong), the caller mended, a file
  added and removed, a file broken and mended, the same text written
  again (read, not parsed), an overlay set (its text is what is
  checked) and cleared, each state equal to a fresh build. Unit tests
  cover the fingerprint, the names and the stamp's margin.
- **Measured** (a script driving `renyi mcp` over pipes: the legacy
  handshake, then `project_map` four times; a development build on
  Windows): `compiler/` 1453 ms for the first call, 12 to 13 ms for
  each later one (the map of 287 thousand characters rendered each
  time); `examples/` 152 ms, then 5 to 6 ms.
- **`renyi mcp`** holds a `Workspace` over `.` (it enters the served
  directory first) and refreshes it before `project_map`, `definition`,
  `effects` and `diff`; `Map` and its whole-directory rebuild are
  gone; `definition` reads the canonical text from the workspace.
- **Not covered.** The manifest is watched at the root only (a
  `renyi.json` above the served directory is read by the resolver, but
  a change to it is not noticed until a file changes); the resolver's
  problems (a missing package, a manifest that does not read) are not
  in the map, as they were not before; a dependency's files are read
  again only when a declaration or the manifest changed.

## The MCP server as it exists (`crates/renyi/src/mcp.rs`)

- `renyi mcp [path]` enters the directory (the current one by default),
  reads one JSON-RPC message per line from standard input, writes one per
  line to standard output, logs only to standard error, and exits when the
  input closes. Messages are read and written with the VM's JSON reader
  and writer (`renyi_vm::natives::json`, decision T2).
- Both eras of the protocol (decision T1): a request whose `_meta` names
  `io.modelcontextprotocol/protocolVersion` is answered as revision
  2026-07-28 says (`server/discover`; `resultType: "complete"` and the
  server's identity in every result; `ttlMs` and `cacheScope` on the tool
  list and the discovery; `-32022` with the supported list for another
  version; `-32602` for a request without the client's capabilities), and
  `initialize` opens the handshake of 2024-11-05 to 2025-11-25 (the
  requested version is echoed when it is one of those, else 2025-11-25).
  `ping` answers in both. A legacy `tools/call` is served without an
  `initialize` first.
- Ten tools, in `tools/list` order: `cheat_sheet` (the cheat sheet is
  compiled into the binary), `library_lookup` (every top-level declaration
  of `library/std`, scanned by line into module, name, receiver type, head
  with its clauses, purpose; every word of the query must occur, entries
  named by a word rank first, 25 at most), `project_map` (`module`,
  `json`), `definition` (the record as `renyi index --json` prints it, then
  the canonical source lines), `effects` (the call tree under a
  definition: each project definition with declared and transitive effects
  and failures, library primitives with their declarations, a definition
  shown before marked and not expanded), `check` (`source`, `path`),
  `format`, `run` (arguments, `deny`, `allow_host`, `allow_read`,
  `allow_write`, `at_most`, `replay`, `explain`: the program's output, its
  standard error under a heading, then `--- finished ---` or the outcome;
  a run that did not finish or exited non-zero is a tool error),
  `run_tests` (`strict`, `explain`), `diff` (`base`: a saved map file
  under the served directory or a git revision; `json`; the same text or
  JSON as `renyi index --diff`). A missing or ill-typed argument and a
  failing tool are tool execution errors (`isError: true`); an unknown
  tool or method is a protocol error.
- The map comes from the resident world (`Server::refresh` on a
  `renyi_workspace::Workspace` over the served directory, decision AN1;
  the section above): the files read again where their stamps changed,
  the world declared again when any text did, the items checked again
  where their text changed; the compiled program of `run` and
  `run_tests` is built per call.
  `main.rs` holds the non-printing `compile_sources`, `diagnose`,
  `read_source` and `toolchain` that the commands and the server share;
  `maps.rs` loads the base of a diff.
- `crates/renyi/tests/mcp.rs` drives the binary over pipes: the legacy
  handshake, the tool list, the cheat sheet, a run of `hello.ry`, a
  `check` of source text, an unknown tool, `ping`; the modern discovery, a
  `definition` with its version, an unsupported version, missing
  capabilities, an unknown method, a malformed line, the tool list's cache
  hints; and the lookup, map, effects, format, tests, a denied run, the
  JSON map, an unknown definition and a `diff` against `HEAD`.

## The semantic diff as it exists (`crates/renyi_index/src/diff.rs`)

- `renyi index --diff <base> [--json] [path]` (and the `diff` tool)
  compares the project with a base: a map file `renyi index --json`
  wrote, read with the VM's JSON reader (`text_hash` optional, so maps
  from before this session load), or a git revision, whose `.ry` files
  (`git ls-tree`, `git show`; for a single file, its import closure at
  the revision) are indexed afresh with the same toolchain
  (`crates/renyi/src/maps.rs`). An argument that is neither is an error.
- Definitions are matched by qualified name; a name gone whose content
  hash (`id`, decision D5) is back under another name is one `renamed`
  entry and its callers are untouched. Per matched definition the changes
  are `signature` (the head and its clauses, so `needs` and `or fails
  with` count), `visibility`, `effects` widened or narrowed (on every
  definition whose transitive effects differ, callers included),
  `failures` added or removed, and `body` when the definition's own text
  hash or its `calls`/`uses` edges (old names translated through the
  renames) changed with the signature unchanged; `added` and `removed`
  otherwise. Each entry lists what it reaches: every transitive caller
  through `calls` and `uses` (in the old map for a removed definition).
  The bump of decision G1: a public definition removed, its signature
  changed, made private or renamed is `major`; one added or made public
  is `minor`; else `none`.
- `text_hash` (decision T5, `hash::own_text_hash`) is the SHA-256 of the
  definition's canonical text with its own name and every reference to a
  project definition blanked, and nothing appended, so it changes only
  when the definition's own text does; `id` also changes when a
  dependency does. Both are in every record (`renyi index --json`,
  `definition`, section 4 of `05-agent-tooling.md`). A map without
  `text_hash` falls back to `id` for the body test.
- Tests: `crates/renyi_index/tests/diff.rs` (a body change reported once
  and reaching its callers, a public signature change forcing a major
  bump, a rename by hash, added and removed definitions, an effect gained
  below widening the callers), `crates/renyi/tests/index.rs` (a saved
  map as the base, with `--json` and against itself; `HEAD` as the base
  for the corpus and for one file; a bad base), the `diff` call in
  `tests/mcp.rs`, and a unit test of `own_text_hash`.

## Done in session 14 (the study, AW1, the Windows mapping fix, stage 0; on the owner's machine)

- The design study of stage 29 (the section "Stage 29, the new
  backend: the design study (session 14)"), with the lane's scripts
  `study_callgrind.sh`, `study_rust_rows.sh`, `study_table.py` and
  `study_map.py`, and the binaries `renyi_head` and `renyi_au34`
  measured under WSL; the owner's eight answers as decision AW1;
  commit d43c07f.
- The image's file opened with execute access on Windows (a2c4685):
  every Windows run of an image had copied its code section since AT5,
  the mapping refused with "access denied" for want of
  `GENERIC_EXECUTE` on the handle; `image::open_file`, the executable
  of AS4 opening itself the same way, and the report naming a refused
  mapping with the system's reason; found by the two tests of
  `crates/renyi/tests/build.rs` that assert "mapped from the file",
  which had been failing on the owner's machine.
- Stage 0 (decision AW2, the commit after a2c4685): the allocator
  entered by mimalloc's plain functions; the literal constants known
  to the JIT from `Jit::new` and the pure constants an image builds;
  the test; the measure in AW2 and in "Stage 0 as built"; the lane's
  `measure_stage.sh`, `size_rows.sh`, `bench_stat.sh`, `bench_both.sh`,
  `ratio_wsl.sh` and `check_env.sh`.
- The cloud branch `origin/ccr-69f89985-9k2lfo`, merged into `main`,
  deleted at the owner's word; `main` is the only branch again.
- Stage 1a (decision AW3, the commit after 2632cde): the slow calls
  after stage 0 attributed by `study_map.py`, which now ranks the calls
  through the trampolines; a typed parameter's boxed operand taken
  from the stack at the call site (`noted_kind` in `native/infer.rs`,
  the direct condition and `call_direct` in `native/codegen.rs`); the
  test; the measure in AW3 and in "Stage 1a as built".
- Stage 1c (decision AW4, the commit after 7bb927b): the frame's
  leave by the static ownership state (`owned_slots` in
  `native/infer.rs`, `leave_frame_inline` in `native/codegen.rs`,
  `RELEASES_INLINE` eight); the two tests, one under a memory budget;
  the measure in AW4 and in "Stage 1c as built".
- Stage 1b (decision AW5, the commit after 1cfcd7d): the read-only
  flag per parameter (`Code::read_only`, bytecode format 8, binary
  encoding 5, image format 9, code format 16, both emitters, the
  judges), the borrowed protocol of direct calls (`direct_call`,
  `accepted_params`, `borrowed_operands` with its `direct_calls`
  switch, the mask in `direct_signature`, `retain_params` and
  `retain_borrowed_arguments` in `native/codegen.rs`,
  `rt_retain_params` in `native/runtime.rs`);
  the two tests, one under a memory budget; the measure in AW5 and in
  "Stage 1b as built".
- Stage 1d (decision AW6, the commit after 2367c63): the frame's grant
  in the body's signature (`direct_signature`, the trampoline reading
  the record's), the stack's length and the frames' room in the
  callee's prologue (`has_direct_call`), `push_frame_inline` without
  the grant's loads and the checks (code format 17); the three tests,
  one under a memory budget; the measure in AW6 and in "Stage 1d as
  built".
- The character tables and the hasher (decision AW7, the commit after
  ddbf36a): `run_of` and `run_until` through a 128-bit table of the
  set's one-character members (`ascii_members`, `class_table` and the
  thread-local `CLASS_TABLES` in `natives/prelude.rs`, cleared by
  `Vm::drop`), and `hashing.rs` (`ValueHasher`, `ValueHashing`,
  `ValueMap`, `ValueSet`) in place of SipHash for every map and set of
  values; the measure in AW7 and in "The character tables and the
  hasher as built".
- Stage 2a (decision AW8, the commit after 94089f9): `update_by_index`
  in `native/infer.rs`, the helpers `rt_with_by_index` and
  `rt_with_slot_by_index` (`Vm::op_with_by_index` in
  `native/runtime.rs`) on both tiers, `update_in_place` in
  `native/codegen.rs` with `rt_record_copy` (`Composite::copy_block`)
  on the Cranelift tier (code format 18); the two tests; the measure in
  AW8 and in "Stage 2a as built".
- Stage 2b (decision AW9, the commit after 1ad4399): `pair_result`,
  `pair_fused`, `otherwise_end` and `handler_only` with `Abs::Pair` and
  `SlotKind::Pair` in `native/infer.rs`, `pair_return`, the pair slots
  and `pair_bypass` in `native/codegen.rs`, the helpers
  `rt_pair_record`, `rt_split_pair`, `rt_pair_fits` and
  `rt_pair_resume` with the hand-back's rebuild of a pair in
  `native/runtime.rs` (code format 19; `tools/image_census.py` reads
  the pair kinds' type); the two tests; the measure in AW9 and in
  "Stage 2b as built".
- Stage 2c (decision AW10, the commit after e87f3e1): `release_in_place`,
  `release_last` and `Composite::free_block` in `value.rs`, `rt_drop_at`
  in `native/runtime.rs` and `Pinned::truncate_values` in `pinned.rs`
  through them; the measure in AW10 and in "Stage 2c as built".
- Stage 3a (decision AW11, the commit after f48282f): `InPlace`,
  `in_place!`, `in_place_typed`, `in_place_taking` and `with_in_place`
  in `extension.rs`, `Tables`, `CallKind` with `Entry` and
  `ReadInPlace`, `Vm::call_in_place` and `NativeState::in_place` in
  `vm.rs`, the JIT's table in `native/mod.rs`, `call_in_place` with
  `typed_entry`, `taking_entry` and `entry_kind` in `native/codegen.rs`
  and `call_in_place` in `native/template/mod.rs`, the lean helpers
  gone from `native/runtime.rs`, the entries and their registrations in
  `natives/prelude.rs`, `take_or_clone` in `natives/mod.rs`, code format
  20, the test in `tests/extension.rs`, the section of
  `docs/extensions.md`; the measure in AW11 and in "Stage 3a as built".
- Stage 3b (decision AW12, the commit after 9828e2e): `into_plain`,
  `guarded`, `unguarded` and `with_origins` in `value.rs`,
  `plain_record_type` and `plain_variant_type` in `vm.rs`,
  `rt_construct_plain`, `rt_construct_variant_plain`, `rt_list_push`,
  `rt_compare` and `iterator_of` in `native/runtime.rs`, the helpers
  and the two ops in `native/codegen.rs` and `native/template/mod.rs`,
  code format 21; the measure in AW12 and in "Stage 3b as built".
- Stage 4a (decision AW13, the commit after 1b25b85):
  `compare_in_place` and `has_declared_equals` in `native/runtime.rs`,
  `sum_compared` in `native/infer.rs`, `compare_sum_variants` in
  `native/codegen.rs` and with `sum_compared_at` in
  `native/template/mod.rs`, `bt_rr` in `native/template/x64.rs`, the
  test in `tests/native.rs` and `tests/template.rs`; the measure in
  AW13 and in "Stage 4a as built".
- Stage 4b (decision AW14, the commit after cf06592):
  `loop_slots` in `compile/mod.rs` with its sites in `compile/stmt.rs`
  and `compile/query.rs` and in `compiler/emit.ry` (format 9 there and
  in `file.rs`, the literals of `tests/compile.rs`); `iterator_of` and
  `iterator_next` in `vm.rs`, `rt_iter_init` in `native/runtime.rs`,
  the position slot in `native/infer.rs`, `IterInit` and `IterNext` in
  `native/codegen.rs`, code format 22 in `native/image.rs`;
  `Native::Iterator` and `ListIter` gone from `value.rs` and
  `render.rs`; the test in `tests/native.rs` and `tests/template.rs`;
  the measure in AW14 and in "Stage 4b as built".
- Stage 4c (decision AW15, the commit after 8eed964):
  `JumpByTag` and `Code.tables` in `bytecode.rs`, the interpreter's
  arm in `vm.rs`, the op and the tables in `file.rs` (format 10) and
  `binary.rs` (encoding 6), `TABLE_ARMS`, `match_table` and the
  emission in `compile/expr.rs` with `variant_tag_of` in
  `compile/pattern.rs`, the flows in `native/infer.rs`, the last reads
  in `liveness.rs`, the jump table in `native/codegen.rs`, the
  comparisons in `native/template/mod.rs`, the tables of an expanded
  callee in `native/inline.rs`, code format 23 in `native/image.rs`;
  `OpJumpByTag` and `tables` in `compiler/bytecode.ry`; `arm_tag`,
  `named_arms`, `match_table`, `target_of` and `emit_match` in
  `compiler/emit.ry`; the literals of `tests/compile.rs`; the test in
  `tests/native.rs` and `tests/template.rs`; the measure in AW15 and
  in "Stage 4c as built".
- The checker's own cuts (decision AW16, the commit after
  805ea3a): `index_variants`, `variants_of`, the `variants` field
  of `ModuleInfo` and `lookup_variant` in `compiler/declare.ry`;
  `token` in `Cursor`, `at_position`, `moved`, `with_nesting`,
  `skip_newlines`, `peek`, the 39 `cursor.token` sites, `parse`'s one
  pass, `text_of`, `bracketed` and `element_after_comma` in
  `compiler/parser.ry`; `lex_range`, `next_step`, `lex_word`,
  `lex_reserved`, `lex_text`, `camel_case_word` and the 19 `slice_text`
  sites in `compiler/lexer.ry` (`slice`, `last_of` and
  `previous_is_adjacent_dot` gone); `resolve` (`is_bound_var` gone),
  `zonk`, `source_text`, `set_var_binding`, `set_var_kind`,
  `update_binding`, `mark_all_used`, `bind` and `retag_count` in
  `compiler/bodies.ry`; `slice_text` in `docs/design/04-stdlib-sketch.md`,
  `library/std/prelude.ry`, `crates/renyi_check/library/std/prelude.ry`
  and `crates/renyi_vm/src/natives/prelude.rs` (`list_slice_text`,
  `list_slice_text_typed`), its test in `tests/semantics.rs`; the
  measure in AW16 and in "The checker's own cuts as built".
- Stage 5a (decision AW17, the commit after c7b8f6c):
  `Program::prepared` in `compile/mod.rs`, `file.rs` and `binary.rs`
  (encoding 7); `prepare` and `prepared` as a `Cow` in `liveness.rs`;
  the runner's two uses in `runner.rs`, `image::build` (the prepared
  program encoded) in `native/image.rs`, the sandbox in
  `crates/renyi/src/sandbox.rs`, `compile_cached` in
  `crates/renyi/src/lib.rs`; the tests in `tests/native.rs` and
  `tests/build.rs`; the measure in AW17 and in "Stage 5a as built".
- Stage 5b (decision AW18, the commit after 16a0dd6): `Body`,
  `body`, `ops`, `spans`, `types`, `tables`, `decoded_body`, `body_mut`
  and `set_body` in `bytecode.rs`; `Encoded`, `decode_shared`,
  `write_body`, `read_body` and encoding 8 in `binary.rs`; `check_body`
  and `Limits` in `file.rs`; the body bound per frame in `vm.rs`;
  `move_last_reads` in `liveness.rs`; the readers in
  `native/codegen.rs`, `native/infer.rs`, `native/template/mod.rs`,
  `native/runtime.rs`, `native/mod.rs`, `native/inline.rs` and
  `native/image.rs` (the image's program as an `Arc`), the writers in
  `compile/mod.rs` and `compile/expr.rs`; `check_image` in
  `crates/renyi/src/lib.rs` and the cache's lookup in `cache.rs`; the
  tests in `tests/native.rs`, `tests/typed.rs` and
  `tests/semantics.rs`; the measure in AW18 and in "Stage 5b as built".
- Stage 6 (decision AW19, the commit after 0f17f67): no code;
  the probe `probe_inline.sh` in the lane with `study/5b_inline/`; the
  measure in AW19 and in "Stage 6 as built".
- The owner's choice of road (B) (decision AW20, the commit after
  549341e) and the design study of stage 31 (the section "Stage 31,
  the new code generator (road (B)): the design study (session 14)"),
  with the lane's `study_b_quick.sh` (the interpreter's profile and
  the IR dumps), `micro_b.sh` and `micro_b/` (the four micro programs
  per turn), `measure_native_b.sh`, `sync_cg.sh` (a cachegrind row
  kept and annotated by cache misses) and `annotate_lines.sh`; no
  code.
- The owner's answers after the study (decision AW21, the commit after
  4632f66): no code.
- Stage B2 (decision AW22, the commit after 0db6523): `Abs::Held`,
  `is_held`, `is_value`, `spilled`, `push!`, `hold`, the `held`
  argument of `analyse` and its clean run in `native/infer.rs`;
  `held_vars`, `held_words`, `push_held`, `push_held_state`,
  `unload_top`, `word_byte`, `operand_byte`, `operand_word`,
  `retain_words`, `release_words`, `retain_operand`,
  `release_operand`, `spill_below`, `spill_all_held`,
  `spill_all_held_by_index`, `insert_held`, `exit_for_state`,
  `emit_exits`, `unbox_held_top`, `take_bool_held`,
  `leave_held_inline`, `branch_on_count_tag`, the `scratch_slot`, the
  landings keyed by the whole operand stack and the op arms in
  `native/codegen.rs`; `rt_insert_held`, `value_of_words` and the held
  kind in `rt_deopt` in `native/runtime.rs`; image format 10 in
  `native/image.rs` and `tools/image_census.py`; the callers of
  `analyse` in
  `native/template/mod.rs`, `native/inline.rs`, `native/mod.rs` and
  `tests/typed.rs`; the test in `tests/native.rs`; the measure in AW22
  and in "Stage B2 as built".
- Stage B3 (decision AW23, built and measured, left off, no code
  merged): the diff in the lane's `scratch_b3/b3_as_built.diff`, with
  the lane's `after_measure_b3.sh`, `measure_after_tests_b3.sh`,
  `study/b3/`, `study/b3_inline/` and the scratchpad's scripts copied to
  `scratch_b3/`; the measure in AW23 and in "Stage B3 as built".
- The leak of stages 2b to B2 (decision AW23, the commit after 2758464):
  `owned_slots` in `native/infer.rs` passing the state through an op
  without one; the test
  `the_owned_analysis_reaches_past_an_otherwise_region` in
  `tests/native.rs`; the lane's `bisect/` (the probes' inputs,
  `regions.ry`, the dumps), `build_b2fix.sh`, `measure_b2fix.sh`,
  `chain_b2fix.sh`, `census_b2.sh b2fix`, `renyi_b2fix` and
  `study/b2fix/`; the probes (`probe_frees.py`, `probe_hist.py`,
  `probe_skip.py`, `probe_dump2.py`, `bisect_leak.py`, `ops_of.py`,
  `fix_owned.py`, `add_owned_test.py`) in `scratch_b3/`.
- Stage B1 (decision AW24, the commit after 1048e84): `FrameNode`,
  `NativeState::head`, `unrecorded_head`, `innermost_grant_index` and
  `record_frames` and `record_chain` in `vm.rs`, with `effective_grant`,
  `frame_grant`, `frame_grant_under`, `denied` and `push_frame_in_place`
  reading or making the records; `interrupt`, `rt_deopt`, `sync_pc` and
  `leave_frame` in `native/runtime.rs`; `link_node`, `unlink_node`, the
  node slot, the leave's conditional pop and `push_frame_inline` without
  the record in `native/codegen.rs`, the frames' room check gone from
  the prologue; code format 24 in `native/image.rs`; the test
  `direct_calls_make_their_frame_records_only_when_a_helper_reads_them`
  in `tests/native.rs`; the lane's `after_measure_b1.sh`, `chain_b1.sh`,
  `renyi_b1`, `study/b1/` and `micro_b_b1.log`, and the scratchpad's
  scripts (`apply_b1.py`, `fix_b1_slots.py`, `add_b1_tests.py`,
  `refine_b1.py` (folded into `apply_b1.py`), `refine2_b1.py`,
  `refine3_b1.py`, `refine4_b1.py`, `aw24_template.md`,
  `render_aw24.py`, `apply_docs_b1.py`, `update_memory_b1.py`,
  `msg_b1.txt`, `numbers_b1.json`) copied to `scratch_b1/`; the measure
  in AW24 and in "Stage B1 as built".
- Stage B4 (decision AW25, the commit after d664c2d): `jump_targets`,
  `pushes_before`, `simple_pushes` with the guard, `borrowed_operands`
  returning `Borrowed` (the ops, the masks, the indices, the dynamic
  bits, the bits below a helper's push), the variable `slow_owned`,
  `runtime_mask`, `note_dynamic_push`, `retain_or_note`,
  `release_operand_if_owned`, `release_if_owned`,
  `retain_operand_if_lent`, `check_status_over_borrowed` and
  `release_in_slot` in `native/codegen.rs`, the field, constant, store
  and consumer sites through them; the template tier's call with both
  flags off in `native/template/mod.rs`; the test binary
  `tests/leaks.rs`; the tests
  `fields_and_constants_borrowed_by_their_consumers_print_as_the_interpreter_prints`
  and
  `the_borrowed_walk_stops_where_another_path_enters_and_finds_the_field_reads`
  in `tests/native.rs`; the lane's `after_measure_b4.sh`, `chain_b4.sh`,
  `gates_then_chain_b4.sh`, `renyi_b4`, `study/b4/` and
  `micro_b_b4.log`, the probe `scratch_b4/leakprobe.ry` with
  `leakprobe.sh`, and the scratchpad's scripts (`apply_b4.py`,
  `add_b4_tests.py`, `leaks.rs`, `aw25_template.md`, `render_aw24.py`
  (reused), `apply_docs_b4.py`, `update_memory_b4.py`, `msg_b4.txt`,
  `numbers_b4.json`) copied to `scratch_b4/`; the measure in AW25 and in
  "Stage B4 as built".
- Stage 32 (decision AW26, the commit after b4c057b): the four cuts in
  `compiler/parser.ry`, `compiler/bodies.ry` and `compiler/declare.ry`
  (the scripts `cursor_census.py`, `hoist_fields.py`,
  `let_with_to_change.py`, `cut3_parser.py`, `cut4_inline.py`; measured
  and not kept `unpin_moves.py` and `kill_rule.py` with
  `kill_rule_experiment.diff`), the attribution `copy_callers.py`, the
  lane's `gate_kill.sh`, `chain_cuts2.sh`, `chain_cuts2_wsl.sh`,
  `study/cut1/`, `study/cut3/` and `study/cuts2/`, the scratchpad's
  `numbers_cuts2.py`, `numbers_cuts2.json`, `aw26_template.md`,
  `render_cuts2.py`, `apply_docs_cuts2.py`, `apply_docs_aw27.py` and
  `msg_cuts2.txt`, all copied to `scratch_cuts2/`; the measure in AW26
  and in "Stage 32 as built": the check row 1,437.8 to 1,307.5 (-9.1%),
  the ratio 2.96.
- The owner's decision after stage 32 (AW27, the commit after 0ee25f2):
  the switch of the front end deferred, the Rust front end kept, the
  speed round closed at the gate; the roadmap's milestone row, stage
  32's close, track D and "what is next", this handoff's summary and
  start-here item 1, CLAUDE.md's decisions cell.
- The owner's choice of the next stages (AW28, the commit after the
  AW27 commit): the WASM target, the language server's completion and
  the C API in that order, the showcase game in parallel; the roadmap's
  stages 33 and 34, its M6 row and "what is next", this handoff's
  summary and start-here item 1, CLAUDE.md's decisions cell; the game
  subagent dispatched with the contract of AW28 (4) and the survey of
  the wasm32 blockers started.
- The showcase game, the subagent's part (b8f700e): eleven modules
  and a README under `showcase/raycaster/`, checked with the
  repository's binary (`check --strict` and `format --check` clean,
  `test` 94 passed, the demo reaching the exit), the CI step that
  holds it, CLAUDE.md's showcase row.
- The design study of stage 33 (the commit after b8f700e): the
  section "Stage 33, the WASM target: the design study (session
  14)"; the wasm lane `D:\Projects\.worktrees\Renyi\wasm\` with
  the trial build's `check_asis.log` (the front end's crates build,
  the VM stops in `getrandom`); the `wasm32-unknown-unknown` target
  installed on the owner's machine; the survey of the crates by an
  exploring agent folded in (the 32-bit layout bug of the record
  block among its findings); the scratchpad's `study33.md`,
  `apply_game.py` and `apply_docs_study33.py`; the roadmap's stages
  33 and 34, this handoff's summary and start-here item 1; the four
  questions put to the owner.

## Done in session 13 (A2, B5, B1 to B4, the owner's choice of a new backend, and the icon, in the cloud environment)

- **Decision AU44** (b765636): the cache keyed by the sources as built, the rest
  of A2 after session 12's first piece: `cache::SourceEntry` and the
  `.rys` entry, `compile_cached` for `run`, `record`, `test` and
  `build --cache`, `Hashed::Kept` for the manifest of a hit, the
  main file's read noted, `RENYI_CACHE_REPORT`, the pruning over both
  kinds of entry; two tests in `crates/renyi/tests/build.rs` and one
  of the entry's rendering in `cache.rs`; the reference's appendix B,
  the usage text, CLAUDE.md, the cargo config's and the tools'
  comments. The first build's key hashed the library's text at every
  run and cost more than a hit of `hello` saved; the key takes the
  modules' names and sizes instead. The measure: a hit of `hello` 7.6
  to 5.8 ms (-24%, -33% in instructions), the self-check's second run
  822 to 762 ms, AT1's `cached run` row -6.8%, its cold rows unmoved.
- **Decision AU45**: B5, two boxed values the checker typed Integer
  compared in place in both tiers when both are small, a literal bound
  in a register beside a boxed one included (`is_integer`,
  `compare_small_integers` on each tier, `comparison_cc`), code format
  13; a test on each tier; `bench/micro/scan_codes.ry` and its row in
  `tools/measure_native.sh`; CLAUDE.md. The measure: `scan_codes` -48% in instructions a turn and -55% in time, the
  self-check's synchronous row -0.55% in instructions and -0.77% by the
  estimate, its time -2% to -6%, the Renyi checker 806 to 756 ms, the
  gate ratio 10.0 against AU34's binary.
- **Decision AU46**: B1, the Renyi lexer rewritten for speed: one
  loop, the step as a sum, scans without a call per character, the
  word lists as sets, the plain runs of text literals sliced; the
  lexer alone 983 to 514 million instructions per lex of `bodies.ry`
  (-48%), byte-equal by the judges; `compiler/lex_only.ry`, the
  measuring driver; CLAUDE.md. The measure: the Renyi checker 756 to
  649 ms on `compiler/bodies.ry`, the gate ratio 10.0 to 8.9 against
  AU34's binary, the parse row 313 to 258 ms.
- **Decision AU47**: B2's first cuts, the parser's keyword and symbol
  tests without a variant built per test, `token_at` as an
  `otherwise`, `peek` reading its token itself; the parser alone
  1,197 to 1,039 million instructions per parse of `bodies.ry`
  (-13%), byte-equal by the judges; `compiler/parse_only.ry`, the
  driver; CLAUDE.md. The measure: the Renyi checker 649 to 630 ms on
  `bodies.ry`, the gate ratio 8.9 to 8.6 against AU34's binary.
- **Decision AU48**: the rest of B2, a look as an index with no record
  per look (`peek_at`, `moved`, `bump`, `past_word`, `past_symbol`,
  `second_token`; `peek` left at 11 rare sites) and the eight
  expression levels as one loop over their levels (`binary`,
  `binary_operator`, `level_of`, a ceiling for the one comparison, the
  one `with` and the `not` head); the parser alone 1,039 to 998.6 to
  809.6 million instructions per parse of `bodies.ry` (-22%),
  byte-equal by the quick judge, the four judges and 24 probes of the
  levels' edges. The measure: the Renyi checker 630 to 579 ms, parsing
  264 to 232, compiling 910 to 873; the gate 6.4 on this run (the
  reference binary took 90 ms where it took 73 at AU47's run; 7.9 by
  that time). The keywords as variants without fields not done,
  bounded by `rt_compare`'s 6.6%; the variant test in place planned
  as the next cut.
- **Decision AU49**: the variant test in place on both tiers
  (`Op::IsVariant` reads the tags where the value lies, releases it and
  answers a Boolean; a guarded value takes `rt_is_variant` as the cold
  path; `TAG_GUARDED`; code format 14; a test per tier with a guarded
  JSON document); the self-check's synchronous run -0.9%, the image's
  run -1.1%, the image +1.7%, the parser alone -1.4%; the ratio's
  check 546 ms, 7.5 times AU34's binary; `measure_size.sh` with a
  `JIT run, sync` row and `measure_native.sh` synchronous (the
  compile thread's share varies under valgrind). The handoff's "The
  front end's round" has it as built and B3 planned.
- **Decision AU50** (fb0cc2d, 2cc361c and the documents after them): B3,
  small callees expanded into their callers on the Cranelift tier, in
  the shape the owner chose (`native/inline.rs`: a `Call` to a declared
  function of at most 20 ops without a loop, a `fail` or `needs` becomes
  a `Jump` into a region appended after the caller's own ops, a caller
  growing by at most half, the smallest candidates first, nested once
  more; the frames rebuilt at a hand-back inside a region; the image
  carries the regions, format 8; `tests/inline.rs`; CLAUDE.md). The
  measure: the expanded code is level with the calls it replaces in the
  steady state (+0.3% on the parser alone after the frame leave was
  fixed to release the caller's own locals alone) and the compile costs
  more (the parser alone +5.8% at the default hotness, the cold
  self-check +0.6% in instructions; the rules loosened +68% and +9.1%),
  while a typed Integer call saves 33 instructions and a record call
  101; off unless `RENYI_NATIVE_INLINE=1`. The ratio: 7.4 with the
  expansion off and 8.4 with it on, the reference binary 78.8 and 68.7
  ms in the two tables. The handoff's "The front end's round" has it as
  built and the evaluation the owner asked for after B3; the roadmap
  closes AU36's list short of the gate.
- **The site's fourth round** at the owner's request ("polish the whole
  appearance, beautify the visual effects"), by a fresh Opus subagent
  under a brief that held it to the site layer (`tools/site.py`,
  `docs/index.md`, `docs/how-it-works.md`), to the positioning's and the
  README's wording, to no external request and no dependency beyond the
  `markdown` package, every page whole without JavaScript; reviewed
  against screenshots of the site before and after at 1440 and 390
  px in both colour schemes (the comparison images in the session's
  scratchpad, `site_cmp/`): one palette with a single accent in light
  and dark and a theme switch, every code block highlighted at build
  time (Renyi by the token classes of reference section 1 and the
  reserved words of its section 17, the grammar as EBNF, the other
  languages by a lexicon each), the front page's headline beside the
  signature card with the four promises as cards and the terminal as
  real text played once in view, the How it works page's diagrams
  redrawn as one family with section chips, the document pages with a
  group label, a contents column that follows the reading, previous
  and next links, copy buttons, the corpus index as cards and the
  example pages with line numbers; contrast at WCAG AA in both modes,
  no horizontal scroll at 390 px, no console errors, the 134 code
  blocks byte-identical to before. The project map's row for the site
  says what it is now.
- **Decision AU51** (404233c): the owner's four answers after B3, every
  recommended option: the gate stays at 3, the next lever is the
  compiler's own code, the expansion of AU50 stays off by default, the
  site is final; stage 28 opened in the roadmap, B4.
- **Decision AU52** (4d8cccc, e91cec0): B4's cuts (i), (ii) and (iv),
  the checker's line starts and layout check over lines (the checker's
  run -2.0%), the parser's look kept in the cursor (the parser alone
  -0.8%), the lexer's seven scans as two list primitives, `run_of` and
  `run_until` (the sketch, the library and the checker's copy, the
  natives with typed entries, the cheat sheet within its budget, a test
  of the edges; the lexer alone -1.4%, the checker's run -3.0% for the
  three); the ratio 7.5, unchanged within the noise. The natives table
  of e91cec0 gave `index_of`'s typed entry to `run_until`, which
  `tests/extension.rs` refused on CI; fixed with AU53.
- **Decision AU53**: B4's cut (iii), the reserved words, phrases and
  symbols as Integer codes (`compiler/words.ry`, the lexer's `Token`
  with `code`, the parser's tests on the code in place, 153 sites
  rewritten by a scanner), with the VM's part, a module constant read in
  place on both tiers from the table of built constants the JIT owns
  (`GlobalSlot`, `NativeState::globals`, `Jit::note_global`; a small
  Integer a literal in the Cranelift code from the compile request's
  snapshot; code format 15; a test per tier), and the lexer's one lookup
  where the set test stood. The measure: the lexer alone -2.4%, the
  parser alone -3.5%, the checker's run -1.2% against the
  numbers before the cut; AT1's synchronous row -0.2% in instructions and -0.5% by the estimate, the image row -0.6%; the ratio
  7.2 and 8.1 on two runs (7.5 before the cut, within the wall-clock's noise). CLAUDE.md's rows for `compiler/` and the VM.
- **Decision AU54**: the fifth cut of B4, `find_binding` and the
  emitter's `lookup` by index, the lexer's end-of-file append in AU11's
  form, and the natives `append` and `append_all` copying a shared list
  once with room (the census build that named the sites is described in
  the entry); the checker's run -2.8% for the VM's part alone and -3.1%
  with the Renyi parts, the lexer alone -0.6%; AT1's synchronous row
  -2.2% in instructions and -2.3% by the estimate, the image row -2.9%.
- **Decision AU55**: the sixth cut of B4, the liveness rule AU11
  deferred: the last read of a local moves it (`LoadMove`, `TakeField`
  for a field) in both emitters, `compile/liveness.rs` and its mirror in
  `emit.ry` (a write is no mention; a block ending in `return`, `fail`
  or `crash` never falls through); a taken field read in place on both
  tiers; the checker's code reshaped by a script where it defeated the
  rule (stale reads of the parameter, `Done` values read after their
  checkers); the checker's run -1.6%, the parser alone +0.4%, the lexer
  alone +1.4% against AU54; AT1's synchronous row +0.4% in instructions
  and +0.5% by the estimate (the frozen compiler keeps the old shape);
  the ratio 6.9 (the Renyi checker 557.9 ms, from 589.8 and 606.6 after
  AU53).
- **Decision AU56**: the seventh cut of B4, the frame leave dropping
  only the slots whose tag says they hold a counted block
  (`Pinned::truncate_values`); the checker's run -0.8%, the parser alone
  -0.6%, the lexer alone -0.9% against AU55; AT1's synchronous row -0.6%
  in instructions and -0.2% by the estimate, the image row -1.0% and
  -1.1%. The cheat sheet's gate (3,001 tokens by the tokenizer CI runs)
  kept CI red from d172887 to fa27d73; fixed in 81c5ef9.
- **Decision AU57**: the eighth cut of B4, `List.with_at(index, item)`,
  a library addition (the sketch, the library and the checker's copy,
  the native, the cheat sheet, a test), on which `lists.replace_at`
  is one call where it built three lists; the checker's run -0.9%
  against AU56 (4,391.6 million), the stage -10.3%. Its other half, a
  small text hashed as two words, measured slower under SipHash (the
  lexer alone +4.8%) and is left out; a hasher of the VM's own is the
  lever it names. The cheat sheet counted with the gate's real
  tokenizers, offline (start-here item 5): 2,997 and 2,992.
- **Decision AU58**: the owner's four answers after B4, every
  recommended option: a new backend for the VM, its design study
  first, the gate kept at 3; AU57 pushed; the measure kept, a table by
  category added; the first stage the calling convention. The entry
  corrects the question's distance (seven times, against this binary's
  own front end) to the gate's: the checker's image run 3,080.0 million
  instructions against three times AU34's `renyi check` of the same
  file (442.8 million), 2.3 times. Stage 28 closed, stage 29 opened in
  the roadmap; the session ended at the owner's word before the study.
- **Decision AV1, the icon and the wordmark**, at the owner's request
  after AU58 (the R on the accent tile read as the R language's):
  five rounds of rendered sheets sent to the owner and put as
  AskUserQuestion (the character 人; six divergent directions with
  eight dropped; the name in forty-five script fonts, eight shown with
  four companion marks; R and Ry built from a nib and other objects;
  Ry on grids and five objects). The owner chose a war hammer on the
  accent tile and the name signed in Ms Madi (OFL 1.1). `tools/brand.py`
  holds both (the mark's full and plain drawings, the signature as one
  path, the colours, which `tools/site.py` checks against its own) and
  writes `docs/assets/` (the mark, the README's light and dark logos,
  the social preview, 1,280 by 640) and `editors/vscode/images/` (the
  extension's 128-pixel icon, the `.ry` file's icon in both themes;
  `package.json` names them, and `vsce package` was run on it to check).
  The site's header shows the mark and the signature in both themes
  (screenshots at 1,440 and 390 pixels, no console error), its tab the
  plain mark; the README opens with the logo in a `picture` element.
  How the PNG files are made: `python tools/brand.py --png
  /opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell`
  in this container (Inter is installed here for the preview's text).
  The fonts were fetched from npm (`@fontsource/<name>`, reachable
  through the proxy), shaped with `uharfbuzz` and drawn with
  `fontTools` (`pip install uharfbuzz fonttools brotli`), which only a
  change of the signature needs again.
- **The README's opening** at the owner's request (68c5cd2): the design
  philosophy as the selling points, five points a reviewer can check,
  the comparison with Python, TypeScript, Rust and shell for the one
  job of a program an agent wrote and a person must trust, and the gap
  the language closes; all of it derived from `08-positioning.md`,
  nothing claimed beyond what the implementation keeps; the status
  paragraph points at the roadmap.
- **The site redesigned** at the owner's request ("minimal, modern,
  premium; few short pages"), by an Opus subagent at medium effort
  under a brief that held it to the positioning's wording and the
  README's, with the build unchanged (`python tools/site.py`, the
  `markdown` package alone, every page at its path, 50 pages): a new
  stylesheet in `tools/site.py` (the system font stack, one accent, a
  44rem measure for the documents, light and dark through
  `prefers-color-scheme`, every table in its own scroll container,
  heading links, a sticky header with the navigation and the
  repository, a footer with the licence) and a new front page,
  `docs/index.md`, written as HTML blocks: the one line as the
  heading, the three sentences, the code sample beside "what a program
  may do is in its signature", the four points as a grid, the gap, the
  install one-liners, six links. Reviewed with screenshots from the
  container's Chromium through `playwright-core` installed in the
  scratchpad (phone, tablet, laptop, desktop, dark): no page wider than
  its viewport; two fixes after the review, the desktop navigation
  (one row, scrolling inside itself when it does not fit, with its
  first entry visible) and the sample's long `purpose:` line wrapped.
  Round two the same day at the owner's request ("cut the words,
  strengthen the visuals; Opus again, you review"), af8bee4: the lead
  paragraph gone, the four points a title of a few words and one
  sentence each, the gap two lines, the install three labelled
  commands, the links labels alone; the heading larger, the code
  sample the page's picture with its signature lines tinted and ruled
  in the accent (`.sig`, `.sig.needs`, the tint a variable per
  scheme), large muted numerals, more space between the sections. The
  review measured the point titles with Playwright (the third wrapped
  to two lines in the four columns at every desktop width): the titles
  a little smaller, the column gap 2rem and the four columns from
  1200px, every title on one line; the five CSS lines over 100
  characters rewrapped.
  Round three the same day ("the other pages too: visuals, less text;
  animation, embeds, SVG"), 84b3390, after three answers in one batch
  (the site layer only, the documents unchanged; four inline SVG
  concept diagrams; a terminal demo as a CSS-animated SVG with subtle
  motion): the navigation in four groups (Learn, Reference, Extend,
  Project; a row with hairlines from 1200px, a Menu disclosure below
  it, the groups in the footer), a lead per page (`LEADS` in
  `tools/site.py`), a table of contents from the headings for every
  page with four sections or more (sticky beside the text from
  1100px, a disclosure below), the decisions page's lettered sections
  as `details` with the last open, the four diagrams small beside the
  front page's points and in full on the new `docs/how-it-works.md`
  with a paragraph each, and a real session (`renyi check` refusing a
  program that reads a file its `main` does not declare, the fix,
  `check` clean, `record`, `run --replay --explain` with the input
  deleted, run with `target/renyi-au49`; `SESSION` in `tools/site.py`
  holds the transcript) rendered as an SVG whose lines appear in turn
  by CSS keyframes, on the front page beside the gap's two lines and
  on the new page; the signature lines highlight in turn; every
  animation off under `prefers-reduced-motion`; no script, no request.
  Reviewed against the reference's appendix B, the recording's header
  (`recording.rs`, decision Q2's manifest) and the package commands
  before the commit; screenshots at 390 and 1440 in both schemes.
- **`docs/ROADMAP.md`** at the owner's request ("a complete roadmap
  file, numbered, grouped by track"): the milestones, the 26 stages in
  order with their decisions and status, the eight tracks, what is
  next; linked from the site's navigation, the front page and
  CLAUDE.md, which asks the session that closes a stage to update it.
- The gates with the 1.94.1 toolchain (fmt, clippy, the tests, the
  conformance suite) before the commit.

## Done in session 12 (the counts in place on both tiers and the decoder's records, in the cloud environment)

- **Decision AU28**, the owner's answers to the three questions after
  AU27, every recommended option taken: the retain and the release in
  place in template code next; the natives building a record in its
  block after it, as a small stage, the decoder of `std.json` first;
  the front end's speed a round of its own after this one.
- **Decision AU29** (8b642fd): the counts raised and lowered in place in
  template code, a helper called only for an Integer or the last
  reference; two assembler forms more (`bt`, the short conditional
  jump); the template code alone -5.2% and the synchronous JIT run
  -1.9% by AT1's estimate, the self-check -5.6% in wall-clock on one
  hardware thread; the run part of AU22 within AU18's 15% on this boot.
- **Decision AU30**: the decoder's fields on the VM's stack and its
  records moved into their blocks with no vector between;
  `json_round_trip` -3.9% in instructions; a test of the decoder's
  refined records, which no test had run; the lesson of its first two
  builds on the inliner (the section "Start here", item 2).
- **Decision AU31** (6836ee0), the owner's answers after AU30, every
  recommended option taken.
- **Decision AU32**: the counts raised and lowered in place in
  Cranelift's code, four builds measured, code format 11; the compile
  thread stopped after the compilation under way when the VM is
  dropped; the self-check -9% to -15% in wall-clock, `strings` -13% to
  -16%, the image's run part -16%.
- **Decision AU33** (4951fec), the owner's answers after the study of
  the typed calls; the run part measured on a second boot.
- **Decision AU34** (44782aa): `List.at` in place with the list's
  layout probed at startup, `Text.contains` and `List.contains` through
  lean helpers, code format 12; the synchronous JIT run -4.7% by the
  estimate.
- **The progress report and decision AU35**: the typed round closed,
  the front end's round opened with a study, the Rust front end kept
  until the Renyi compiler is within 3 times its time.
- **The front end's study** (62ff0fe) and the owner's question why the
  Renyi compiler is eleven times slower, answered with the cost of
  each kind of operation; **decision AU36**, the owner's order.
- **Decision AU37** (195798a): every file parsed once by the Rust front
  end; `check compiler/bodies.ry` -44% in instructions, -35% in time.
- **Decision AU38** (d79c6fc): the parser's token comparisons by
  variant, `bump` for the dropped advances, `expect` answering a span;
  the same check -23% more.
- **Decision AU39** (f1ba01b): the module trees shared by a count in the
  world, no copy of the item lists; the same check -17% more, an empty
  program's check -21%.
- **Decision AU40**, the owner's answers after the first part of A4;
  **decision AU41** (57a9b7c), the self-check frozen under
  `bench/selfcheck/`; **decision AU42** (c4b43bf), a program declaring
  the prelude, `std.json` and the library modules it imports, in both
  front ends and the resident world, the fixes that name a module not
  imported reading the whole library on their error's path; an empty
  program's check -35% in instructions, the thirty examples -11%,
  `hello`'s bytecode 231 to 125 KB.
- **Decision AU43**, what a hit of A2's cache loads; the first piece of
  A2, the reads a compile makes noted (`renyi_package::reads`), with
  `tools/ratio.py` and `tools/front_end_compare.py`.
- The gates with the 1.94.1 toolchain before every commit that touched
  `crates/`, the conformance suite and the judges on the template tier
  among them; CI green on both jobs after every push.

## Done in session 11 (the template tier's direct calls, in the cloud environment)

- **Decision AU20**, the owner's answers to the four questions of
  session 10, every recommended option taken but the first, where the
  owner chose both follow-ups in order ("1+2"): the template tier's
  follow-ups, then the representation items of AU1's step v; a Windows
  job in CI (`.github/workflows/ci.yml`, the job `windows`, `cargo test
  -p renyi_vm` on `windows-latest`), which passed on its first push (CI
  run 78, commit d2dec3a); the goal of AU18 kept; the tier on by
  default.
- **Decision AU21**, the first part: direct calls between template
  functions through the entry table, the field read through the site's
  cache in place, the rare paths after the body, the promotion at a
  template's start, code format 7; `tests/direct.rs`; the self-check
  -12.4% in wall-clock on one hardware thread, -2.6% on two and -1.3%
  on four against AU19's binary; by AT1's estimate the synchronous
  JIT run -9.0% and the template code alone -16.0%; the goal of AU18 at
  +25.7% (from +42%).
- **The design study of the representation items** (4640d03): a
  census build (never committed) and callgrind; the owner's answers,
  **decision AU22** (a895c90), with `RENYI_NATIVE_REPORT`'s last line,
  the run part of a run.
- **Decisions AU23 to AU27**, each a stage measured on one boot against
  the binary before it: the guard bookkeeping skipped (2abc849), a
  variant without fields read from its slot (f26c1ca, code format 8),
  one shared empty list (1c27240, neutral), small texts in the value
  (5be5e60, code format 9), records and variants in one block (14ce593,
  code format 10). Every push green on CI, the Windows job included.
- The gates with the 1.94.1 toolchain (fmt, clippy, test, the
  conformance runner), the conformance suite and the judges on the
  template tier, before every commit that touched the generated code.

## Done in session 10 (stages 6 to 9 and the template tier of the typed round, in the cloud environment)

- **Decision AU11, stage 6 of the typed round**: the ops `WithSlot` and
  `TakeField`, bytecode format 7, binary encoding 4, image format 6,
  code format 4; the three emitter rules in `compile/stmt.rs` and
  `compiler/emit.ry` (the update in the slot with the takes or the move,
  the argument form of O1's move, `pins` with the escapes), the VM's
  `with` through `Rc::make_mut`; the native test and the conformance
  case `record_update`; measured on this machine against AU10's binary
  built from its commit: the cold JIT run -0.5% by the estimate, -0.1%
  in instructions; the counts that explain the small gain (the
  checker's records are shared at nearly every update) and the two
  rules that would change that, put to the owner (the section "The
  typed round", "What is next").
- **Decision AU12**, the owner's answers after stage 6 (stage 7 now, the
  copies left to the representation round; the round's goal judged on
  the cold JIT run, the round continuing by the profile after stage 8;
  the self-hosted front end decided after the round), and **decision
  AU13, stage 7**: the comparisons on borrowed operands in the generated
  code (`rt_compare`, `borrowed_operands` for `Binary`, `compare_boxed`;
  code format 5; the native test of every shape), -2.2% on the cold JIT
  run, -3.4% on the image, the interpreter unchanged after the three
  fast paths were inlined by force; a lesson on the measurement: the
  simulated cache misses change between boots of the container, so a
  stage's two binaries are measured on one boot.
- **Decision AU14**, the owner's answers after stage 7 (the borrowed
  parameters skipped; the compile cost measured: 42 microseconds an op,
  two thirds register allocation, no factor below 8000 pays on the
  interpreter's thread; the compile thread chosen, measured by
  wall-clock), and **decision AU15, stage 8**: the JIT's compilation on
  a thread of its own (`Worker`, `State::Queued`, `HOT_FACTOR_BACKGROUND`
  100, `RENYI_NATIVE_SYNC`; the tests synchronous under
  `RENYI_NATIVE_HOT=0`), the self-check -11.6% in wall-clock against
  AU13's binary, `primes` and `records` -8%.
- **Decision AU16**, the owner's answers after stage 8 (the template
  tier to be built as the next large item; the liveness pass as the
  stage now), and **decision AU17, stage 9**: the last read of a slot
  as a move, a liveness pass in the VM (`crates/renyi_vm/src/liveness.rs`,
  applied to a copy of the program by `runner.rs`, `native/image.rs`
  and the sandbox; the bytecode file, the image's program, the cache's
  key and the judges untouched), the moved loads borrowed by typed
  calls and comparisons in the generated code; the cold JIT run -2.2%
  by the estimate, the interpreter -3.0%, the image -3.3%, the
  self-check -4.9% in wall-clock on four hardware threads and -3.3% on
  one, the interpreter -7.7%.
- **Decision AU18**, the owner's answers on the template tier's design
  (the design study written into this file first, the four questions
  in one batch, every recommended option taken), and **decision AU19,
  the template tier as built** (`native/template/`: the assembler and
  the sequences per op; the tiers in `native/mod.rs`; `rt_osr`; the
  tests `template.rs` and `promotion.rs`), with the defect of the
  Cranelift tier's hand-back inside a handled region fixed (image
  format 7, the native test); the synchronous JIT run -6.2% by the estimate,
  the self-check -1.3% to -4.8% in wall-clock on one hardware thread,
  -3.1% on two, neutral on four; the goal of AU18 (within 15% of the
  image on one thread) not reached, at +42%. Commit bbea69b, CI green
  (run 76); the earlier commits of the session (aaa44f6, da926cc,
  30db0f3, d8ee092, 42d7ed5) green too.
- **The owner's question on the execution model** answered with the
  numbers of this machine (the section "The execution model, as the
  owner asked on 2026-10-09").
- The gates with the 1.94.1 toolchain (fmt, clippy, test, the
  conformance runner) and the judges; the environment set up again in
  a fresh container (the 1.94.1 toolchain installed beside 1.97.0, a
  `target-1.94.1/` for its gates).

## Done in session 9 (the profile-guided round on strings and JSON and the baseline JIT, in the cloud environment)

Eleven commits on `main` (2fcab99, cf4c912, 02380ea, 5eeb28e, 176cfc2,
d26dfac and 5e4f1d0, the commit of AR6; then 4af6311, the handoff,
7094e1d and efa63a2, `renyi build` with AS1 to AS3, and 71965aa,
`build --exe` with AS4), each gated as in session 8 (rustc 1.94.1, the CI toolchain,
installed beside the environment's 1.97.0 for `cargo fmt`, `cargo
clippy --all-targets -- -D warnings` and `cargo test`; the conformance
suite by both runners; the corpus canonical; `compiler/*.ry`,
`bench/*.ry` and the starter pack checked, formatted and tested; the
lint). For most of the session the pushes were refused (403) until
the owner installed the Claude GitHub App on the `renyi-lang`
organisation (https://github.com/apps/claude/installations/select_target;
a cloud session needs it, and the patches were handed to the owner as
a bundle meanwhile); the seven then went up in one push and CI passed
on the head (run 51 on 5e4f1d0), and the four after them went up one
by one, CI green on each (runs 52 to 55; 55 on 71965aa).

1. the round (decision AQ; the section "The profile-guided round on
   strings and JSON as it exists"): `crates/renyi_vm/src/natives/json.rs`
   (`Path`, `field_type`, `Naming::key` as a `Cow`, the `Sink` trait,
   `TreeSink`, `TextSink`, `walk`, the unit tests),
   `crates/renyi_json/src/lib.rs` (the reader by runs, `write_string`
   by runs, `newline` and `write_string` public), `value.rs`
   (`Value::character`, the `ASCII` table), `natives/prelude.rs`
   (`characters`), `vm.rs` (the loop over a text), `natives/regex.rs`
   (the cache), `native/runtime.rs` (`op_concat`); conformance case 54
   (`tests/conformance/programs/json_paths.ry`,
   `expected/json_paths.out`, the manifest); `tools/apply_perf_round.py`
   deleted; the decisions (section AQ), `docs/GAPS.md` (the performance
   entry of section 4, the status at the end of section 7), `CLAUDE.md`
   (the decisions row), this file.
2. the baseline JIT, stage 1 (decisions AR1 and AR2; the section "The
   baseline JIT"): `Op::LoadField` in `crates/renyi_vm/src/bytecode.rs`
   (kind 52), emitted by `compile/expr.rs` (`member`) and
   `compile/pattern.rs` and by `compiler/emit.ry` (`emit_member`,
   `local_holder`, `field_pattern`); the interpreter's arm and
   `Vm::op_load_field` in `vm.rs`, the look at the next op gone from
   `Op::Load`; `infer.rs`, `codegen.rs` and `runtime.rs`
   (`rt_load_field`); `file.rs` (format 4, the op written, read and
   checked) with `tests/file.rs` and `crates/renyi/tests/compile.rs`;
   `compiler/bytecode.ry` (the variant, format 4 in `emit.ry`); the
   profiler's table of operation pairs (`profile.rs`,
   `tests/semantics.rs`, `05-agent-tooling.md`); the decisions (section
   AR), `docs/GAPS.md`, `CLAUDE.md`, this file.
3. the baseline JIT, stage 2 (decision AR3; the section "The baseline
   JIT"): `codegen.rs` (`direct_signature`, the body as
   `renyi_direct_*`, `trampoline`, `call_direct`, `call_through_helper`,
   `return_direct`, `return_with`, `payload_bits`, `push_payload`, the
   `Return` arms, the exit), `runtime.rs` (the `D_*` statuses,
   `kind_code`, `rt_direct_entry`, `rt_direct_frame`, `rt_direct_after`,
   `rt_leave_typed`, `rt_leave_unbox`, `rt_leave_boxed`,
   `rt_left_status`), `mod.rs` (`State::Ready.direct`, `Jit::direct`,
   `Jit::depth`, `Jit::direct_table`), `vm.rs` (`direct_entry_of`,
   `push_frame_direct`, `run_generated` on the JIT's count), `infer.rs`
   (`abs_of_params`, the rule on parameter slots); the decisions
   (AR3), `docs/GAPS.md`, this file.
4. the baseline JIT, stage 3 (decision AR4; the section "The baseline
   JIT"): `crates/renyi_vm/src/pinned.rs` (new: `Pinned<T>`),
   `value.rs` (`repr(C, u8)` on `Value`, `repr(C)` on `Record` and
   `Variant` with pinned fields, the module `layout`, `layout_tests`),
   `integer.rs` (`repr(C, u8)` on `Int`), `vm.rs` (`repr(C)` on `Vm`,
   `Frame` and `FieldSite`; the stack, the frames, the handlers and the
   field cache pinned; `frame_grant` crate-visible; `push_frame_direct`
   gone), `native/codegen.rs` (the stack and the frames sections,
   `Addresses`, the arms in place, the prologue's room check, the
   merged `LoadField`), `native/runtime.rs` (`rt_room`,
   `rt_grow_frames`, `rt_frame_grant`, `rt_drop_at`; `rt_direct_frame`,
   `rt_leave_*`, `rt_return_*` and `kind_code` gone), `native/mod.rs`
   (`Addresses`, `RENYI_NATIVE_REGALLOC`), `lib.rs` (`pub mod pinned`);
   the decisions (AR4), `docs/GAPS.md`, `CLAUDE.md` (the `renyi_vm`
   row), this file.
5. the micro-benchmarks into the repository (the owner's answer):
   `bench/micro/` (six programs), `tools/measure_native.sh`, the CI
   step, `CLAUDE.md`, this file; then the tiering (decision AR5):
   `HOT_FACTOR` 8000 in `crates/renyi_vm/src/native/mod.rs`, the
   decisions (AR5), `docs/GAPS.md`, this file; then the round closed
   (decision AR6: the totals), the plan's item 4, this file.
6. `renyi build` (decisions AS1 to AS3; the section "`renyi build` as
   it exists"): `crates/renyi_vm/src/vm.rs` (`NativeState`,
   `Options.image`, the load in `Vm::new`), `native/codegen.rs` (the
   address-free code: the helper table, the state's offsets, the
   constants and the bodies through tables, `Compiled` as bytes,
   `machine_code`), `native/mod.rs` (`CodeArena`, `Jit::new` with the
   level, `isa`, `host_target`, `compile_code`, `compile_everything`,
   `load_image`, `state_pointers`; `cranelift-jit` and
   `cranelift-module` gone, `region` and the icache crate in),
   `native/image.rs` (new), `native/infer.rs` (the kinds' bytes),
   `native/runtime.rs` and `runner.rs` (the counters), `Cargo.toml`;
   `crates/renyi/src/lib.rs` (`build_command`, `load_image_file`, the
   image through `compile_with_sources` to `run`, `record`, `test` and
   `reproduce`, the usage), `crates/renyi/tests/build.rs` (new); the
   decisions (section AS), `docs/reference.md` (appendix B),
   `docs/GAPS.md`, `CLAUDE.md`, this file.
7. `renyi build --exe` (decision AS4; the section "`renyi build` as it
   exists", stage C): `crates/renyi/src/exe.rs` (new), `lib.rs`
   (`run_embedded`, `run_loaded`, `load_image_bytes`, `--exe` in
   `build_command`, the usage), `tests/build.rs` (the executable's
   test); the decisions (AS4), `docs/reference.md` (appendix B),
   `docs/GAPS.md`, `CLAUDE.md`, this file; the cache simulation's
   finding on `speed_and_size` and the wall-clock picture of the image
   against the JIT, in the section.
8. the size round, stage 1 (decisions AT1 and AT2; the section "The
   size of the generated code"): `crates/renyi_vm/src/native/codegen.rs`
   (the prologue's locals, the field read's one path, `deopt_block` and
   `emit_deopts`, the direct call without the ask path, no call
   counter), `value.rs` (`Record::tag`, `RECORD_TAG`, the assertions),
   `vm.rs` (`NativeState` without `calls`, `direct_entry_of` gone),
   `native/runtime.rs` (`rt_direct_entry` gone, no count), `native/mod.rs`
   (`report`, `Jit::direct` gone), `native/image.rs` (`CODE_FORMAT` 2),
   `runner.rs`; `tools/measure_size.sh` and `tools/image_census.py`
   (new); the decisions (section AT), `docs/GAPS.md`, `CLAUDE.md`, this
   file.
9. the size round, stage 2 (decision AT3; the section "The size of the
   generated code"): `crates/renyi_vm/src/binary.rs` (new), `lib.rs`
   (the module), `file.rs` (`check` shared), `native/image.rs` (the
   image's program and hash, format 2); `crates/renyi/src/lib.rs`
   (`Hashed::Given`, the decode), `tests/build.rs` (the round trip and
   the cross-reproduction); `tools/image_census.py` and
   `tools/measure_size.sh` (the `.ryc` beside the image); the decisions
   (AT3), `docs/reference.md` (appendix B), `docs/GAPS.md`,
   `CLAUDE.md`, this file.
10. the size round, stage 3a (decision AT4; the section "The size of the
    generated code"): `crates/renyi_vm/src/native/codegen.rs` (the
    static height, the frame pointer); the decisions (AT4),
    `docs/GAPS.md`, this file.
11. the image's load (decision AT5; the same section): `native/image.rs`
    (the code section, `Placement`, `MappedSection`, `Image::open`,
    `Image::open_at`, format 3), `native/mod.rs` (`Pages`,
    `CodeArena::map_section`, `Jit::load_image` with the fall-back,
    `Jit::mapped`, the report), `file.rs` (the check's messages on
    failure only), `Cargo.toml` (`memmap2`); `crates/renyi/src/lib.rs`
    (`check_image`, `Image::open`), `exe.rs` (the padding, the file
    handed over), `tests/build.rs`; `tools/image_census.py`; the
    decisions (AT5), `CLAUDE.md`, this file.
12. the size round, stage 3b (decision AT6; the same section):
    `crates/renyi_vm/src/native/codegen.rs` (`retain` and `release` as
    calls, `rc_of` and the inline sequences gone, `STACK_SAFE`),
    `native/runtime.rs` (`rt_retain_at`, `rt_drop_at`'s role),
    `native/image.rs` (`CODE_FORMAT` 3); the decisions (AT6),
    `docs/GAPS.md`, this file.
13. the round's close (decision AT7, the owner's answer of 2026-10-09,
    with the benchmark table against CPython of that day): the
    decisions, `docs/GAPS.md`, this file.

## Done in session 8 (stage 2: the grammar, the reference, the front end in Renyi)

Three commits on `main`, each gated as in session 7:

1. `fa40315` the formal grammar `docs/grammar.ebnf` (W3C EBNF over the
   lexer's tokens, two start symbols, 76 rules) and
   `crates/renyi_syntax/tests/grammar.rs`, an interpreter of the file
   that runs it over the corpus, the library declarations, the
   conformance programs, every text hole and two lists of corner
   programs, and fails when it and the parser disagree. Decision V12: a
   line break after a comma carries no meaning (the formatter folds a
   wide `needs` clause), `is` and `fails` continue a line everywhere,
   one spelling per construct (a trailing comma, a variant's empty
   parentheses, a space after a dot, an `otherwise` arm before the last
   of a `match` expression, a head and `end` on one line, a hole in a
   plain text and `public` on a method are errors with fixes;
   `public-method` is the new code), a line break inside brackets
   before `.` or `(` is accepted. Conformance case 37, `one_spelling.ry`.
2. `17509df` the reference: `docs/reference.md` (sections 0 to 17 after
   the sketch's, appendix A the diagnostic codes with their severities
   and sections, appendix B the commands and exit statuses) and
   `crates/renyi_syntax/tests/reference.rs` (every quoted rule equals
   the grammar file's and every rule is quoted once; appendix A equals
   the codes the crates emit, found at every call named `error` or
   `warning`, with their severities); `CLAUDE.md`, `README.md`,
   `docs/GAPS.md`, the sketch's status line and its section 17 follow.
   Every claim of the reference was read back from the lexer, the
   parser, the checker, the VM and the library before it was written.
3. `63aa5ba` the front end in Renyi (decisions W1 to W4 from the
   owner's batch: the Renyi sources in `compiler/`, a Rust integration
   test as the judge, the lexer and the parser first, the JSON defined
   by the Renyi types with the Rust encoder changed to match).
   `compiler/ast.ry`, `lexer.ry`, `parser.ry`, `parse.ry`, `tokens.ry`
   as described above; `crates/renyi_syntax/src/json.rs` rewritten to
   the derived shape (character offsets through a byte-to-character
   table; its unit tests and `tests/corpus.rs` follow); `renyi parse
   --declarations`; `crates/renyi/tests/selfhost.rs`; the formatter's
   rule for a wide `exposing` list (`format.rs`, a unit test, reference
   section 16); CI's `renyi check compiler/*.ry` step; `CLAUDE.md`,
   `README.md`, `docs/GAPS.md` (status at the end of session 8, with
   the first measurement), appendix B of the reference. The parser was
   written in one pass as a transcription of `parser.rs` and agreed
   with it on the first comparison; the lexer needed one round of
   fixes from `renyi check` (reserved words as names, `"{"` as a hole,
   `\r` as an escape) and one performance fix (slicing by characters).
   Part of this commit (the JSON encoder, `tokens.ry`, the CI step and
   appendix B) was written by a second Claude Code session the owner
   ran in parallel; the two sessions split the files by message and
   this session ran the gates and committed.
4. The checker in Renyi (the last commit; decisions W5 to W8 from the
   owner's batch: the checker next, transcribed from
   `crates/renyi_check`; the profile after it; full equality with
   `renyi check --json` as the judge; the references produced and not
   yet judged). `compiler/lists.ry`, `report.ry`, `effects.ry`,
   `suggest.ry`, `types.ry`, `refine.ry`, `declare.ry`, `bodies.ry`
   and `checker.ry` as described above; `regex.problem(pattern)
   returns maybe Text` added to the library (`library/std/regex.ry`,
   the sketch, `natives/regex.rs`) so that `bodies.ry` can report
   `regex-invalid` with the engine's own message; the checker judge in
   `crates/renyi/tests/selfhost.rs` (`judge_all` generic over the case
   type); `CLAUDE.md`, `README.md`, `docs/GAPS.md` (status at the end
   of session 8, continued), the reference's appendix B, section W of
   the decisions. `declare.ry` and `bodies.ry` were each written in one
   pass from a reading of `world.rs` and `check.rs`, then taken through
   `renyi check` (reserved words, qualified type names, `with`, the
   match-arm rules, unused bindings, the body-length and line-width
   limits: the formatter's wrapping pushed six functions over sixty
   lines, each split into named helpers) until clean; the first full
   comparison with the Rust checker found no difference on any of the
   142 cases.
5. `f770f79` the profile of the VM (W6 and decisions X1 to X4
   from the owner's batch: the development profile optimizes the VM,
   `List.slice`, the interpreter loop rewritten next instead of the
   three cheap changes, `renyi run --profile`). `Cargo.toml` (X1),
   `slice` in the sketch, `library/std/prelude.ry`,
   `natives/prelude.rs` and the cheat sheet (X2; a case in
   `tests/semantics.rs`; two sentences of the sheet shortened to pay
   for it), the lexer's `slice`, the checker's own hot spots as
   described above, `docs/GAPS.md` (the profile's numbers), this file.
6. The interpreter loop and the profiler (the last commit; X3 and X4 as
   described in the profile section): `crates/renyi_vm/src/vm.rs`
   (`run_frames`, `settle`, `leave`, `push_frame_in_place`, `field_at`,
   `ability_target`, `derived_ability`, the fast paths), `profile.rs`
   (new), `bytecode.rs` (`Op::Field { name, site }`, `Op::kind`),
   `compile/mod.rs` (`field_sites`, `specials`, `date`,
   `function_codes`), `compile/expr.rs` and `pattern.rs` (the sites),
   `value.rs` (`Decimal` boxed, `plain_in_place`, the iterator over the
   list), every file under `natives/` (the slice signature,
   `natives::take`), `render.rs`, `runner.rs` (the report), `lib.rs`,
   `crates/renyi/src/main.rs` (`--profile`), `tests/semantics.rs` (the
   report's case), the reference's appendix B, `05-agent-tooling.md`
   (section 8), `README.md`, `CLAUDE.md`, `docs/GAPS.md`, this file.
7. The owner's batch after X3 (the last commit): the emitter writes a
   bytecode file (X5), `mimalloc` (X6, measured and kept), the
   development profile stays as X1 set it, the residue of stage 1
   before the emitter; `crates/renyi/Cargo.toml`, `main.rs`,
   `Cargo.lock`, section X of the decisions, `docs/GAPS.md`,
   `README.md`, this file.
8. The residue of stage 1 (decisions Y1 to Y4, the section "The residue
   of stage 1" above): `library/std/server.ry` and the sketch
   (`StartError.OverBudget`), `vm.rs` (`denied`, `over_budget`,
   `replay_with`, the two stack ops), `bytecode.rs`, `compile/mod.rs`,
   `compile/stmt.rs`, `check.rs` and `compiler/bodies.ry`
   (`refinement-field`), `tests/{semantics,recording,network}.rs`,
   `crates/renyi_check/tests/rules.rs`, conformance case 38, the
   reference (sections 4, 5, 7, 11, 14, appendix A), the decisions
   (section Y), `06-runtime-guarantees.md`, the sketch's R3-2,
   `docs/GAPS.md`, this file.
9. The bytecode file (decisions Z1 to Z4, the section "The bytecode
   file" above): `compiler/bytecode.ry` (new), `compiler/types.ry` and
   `effects.ry` (`can ToJson`), `crates/renyi_vm/src/file.rs` (new),
   `lib.rs`, `compile/mod.rs` (spans in characters, `SourceLines`),
   `runner.rs`, `crates/renyi/src/main.rs` (`compile`, `Hashed`, the
   `.ryc` loading), `tests/file.rs` and `tests/compile.rs` (new),
   `tests/conformance.rs` (the rerun), the reference's appendix B, the
   decisions (section Z), `README.md`, `CLAUDE.md`, `docs/GAPS.md`,
   this file.
10. The rename of `set` to `change` (decision AA1, the section "The
    rename of `set` to `change`" above): the grammar, the reference, the
    cheat sheet, the decisions (section AA), both front ends, the
    corpus, the conformance programs, the crates' tests, the lint,
    `docs/GAPS.md`, this file.
11. The compiler written in Renyi (decision Z3, the section "The
    compiler written in Renyi" above): `compiler/project.ry`, `emit.ry`
    and `compile.ry` (new), `checker.ry` (the shared pieces moved out),
    `bodies.ry` (four helpers made public), `crates/renyi_check/src/
    lib.rs` (`imported_files` joins with `/`), the judge in
    `crates/renyi/tests/selfhost.rs`, `README.md`, `CLAUDE.md`,
    `docs/GAPS.md`, this file.
12. Constraints with type arguments (decision AB1, the section
    "Constraints with type arguments" above): `parser.rs`, `world.rs`,
    `check.rs`, `vm.rs`, `natives/prelude.rs`, `library/std/prelude.ry`,
    `compiler/parser.ry`, `declare.ry`, `bodies.ry` and `types.ry`,
    `rules.rs`, `tests/library.rs`, conformance cases 39 to 42, the
    grammar, the reference, the cheat sheet, the sketch, the library
    sketch, the decisions (section AB), `CLAUDE.md`, `docs/GAPS.md`,
    this file.
13. The judges run from bytecode (the section "The judges run from
    bytecode" above): `crates/renyi/tests/selfhost.rs` (`front_end`,
    the driver's file passed to every judge, the fixed point of
    `compile.ry`), `CLAUDE.md`, `docs/GAPS.md`, this file.
14. Packages, the first commit (decision AC1, the section "Packages"
    above): `crates/renyi_json` and `crates/renyi_package` (new),
    `renyi_syntax` (`Package`, `SourceFile.package`), `renyi_check`
    (`world.rs`, `check.rs`, `lib.rs`, `rules.rs`), `renyi_index`
    (`load_project`), the binary (`compile_sources`, the judges'
    `PROGRAM_DIRECTORIES`), `renyi_vm` (`natives/json.rs` re-exports,
    `Bytes.sha256`), `library/std/prelude.ry` and the library sketch,
    `compiler/project.ry`, `declare.ry`, `bodies.ry`, `checker.ry` and
    `compile.ry`, the fixture `tests/conformance/packages/` with
    conformance cases 43 to 47 and `renyi_package/tests/fixture.rs`,
    the reference (sections 2 and 11, appendix A), the decisions
    (section AC), `CLAUDE.md`, `docs/GAPS.md`,
    `tests/conformance/README.md`, `.gitignore`, this file.
15. Packages, the second commit (decision AC1, the section "Packages"
    above): `crates/renyi_package` (`select.rs`, `Project::of_directory`,
    `resolve_in`), `crates/renyi/src/packages.rs` (the five commands) and
    `main.rs` (the dispatch, the usage, the run manifest's dependencies,
    `reproduce`, the budgets of the manifest), `renyi_vm/recording.rs`
    (`Dependency`), `renyi_index` (`package` on modules and definitions,
    the canonical copies keep the tag), `maps.rs`, `budgets.rs`,
    `crates/renyi/tests/packages.rs`, the reference (appendix B),
    `README.md`, `07-system-design.md` (2.3, 3, 6),
    `05-agent-tooling.md`, `CLAUDE.md`, `docs/GAPS.md`, this file.
16. The diagnostics of the front end in Renyi (decision AD1, the
    section "The diagnostics of the front end in Renyi" above):
    `compiler/lexer.ry` (rewritten around `Lexed`), `parser.ry`
    (rewritten around `Cursor.diagnostics` and `Halt`), `parse.ry`,
    `tokens.ry`, `project.ry` (`Front`), `checker.ry`, `report.ry`
    (`json_string`), the three judges of `selfhost.rs`, conformance
    cases 48 and 49, the decisions (section AD), `CLAUDE.md`,
    `README.md`, `docs/GAPS.md`, this file.
17. `std.process` (decision AE1, the section "The process module"
    above): `library/std/process.ry`, section 13 of the library
    sketch, `natives/process.rs`, the two failure arms of `vm.rs`,
    `effects.rs` and `effects.ry` (`foreign` alone unavailable), the
    library lists of `lib.rs` and `project.ry`, `tests/library.rs`,
    `tests/rules.rs`, `crates/renyi/tests/process.rs`, conformance cases
    20 and 50, the
    reference (the module list, the capability table, the static rule,
    the run-time errors), the cheat sheet (`process` and the module
    back on the sheet, within the budget), the decisions (section AE),
    `CLAUDE.md`, `README.md`, `docs/GAPS.md`, this file.
18. The foreign function interface (decision AF1, the section "The
    foreign function interface" above): `library/std/foreign.ry`,
    section 14 of the library sketch, `crates/renyi_check/src/foreign.rs`
    and the three diagnostics of `world.rs`, the `foreign` section of
    `manifest.rs` and the tagging of `resolve.rs`, `SourceFile.foreign`,
    `FunctionMeta.foreign` and `FORMAT` 2 of the bytecode file,
    `natives/foreign.rs` with the generated `foreign_abi.rs` and
    `tools/gen_foreign_abi.py`, `Vm::run_primitive`, `bind.rs`, the run
    notice, the `publish` refusal, `capability-unavailable` removed; the
    mirrors in `project.ry`, `declare.ry`, `effects.ry`, `bodies.ry`,
    `bytecode.ry` and `emit.ry`; `crates/renyi/tests/foreign.rs`, the
    fixtures `tests/conformance/foreign/` and `foreign_bad/` with cases
    20 and 51, `tests/fixture.rs`, `tests/library.rs`, `tests/rules.rs`,
    the judges' directories; the reference (the module list, the
    manifest, the capability table, the static rules, the run time,
    appendices A and B), the library sketch, the cheat sheet, the
    decisions (section AF), `CLAUDE.md`, `README.md`, `docs/GAPS.md`,
    the conformance `README.md`, this file.

19. the Python bridge (decisions AJ2, AJ3 and AL1 to AL4; the section
    "The Python bridge as it exists"): `library/std/python.ry`,
    `crates/renyi_check/src/python.rs` and the two diagnostics, the
    `python` section of `manifest.rs` and the tagging of `resolve.rs`,
    `SourceFile.python`, `FunctionMeta.python` and `FORMAT` 3 of the
    bytecode file, `natives/python.rs` with `python_worker.py`,
    `Vm::run_primitive`, the run notice, the `publish` refusal; the
    mirrors in `project.ry`, `declare.ry`, `effects.ry`, `bytecode.ry`
    and `emit.ry`; `crates/renyi/tests/python.rs`, the fixtures
    `tests/conformance/python/` and `python_bad/` with cases 21 and 53,
    `tests/fixture.rs`, `tests/library.rs`, `tests/rules.rs`,
    `tests/file.rs` and `tests/compile.rs` (the format numbers), the
    judges' directories, `ci.yml` (Python before `cargo test`); the
    guide `docs/python.md`, the reference, the library sketch, the
    syntax sketch, the runtime and system documents, the cheat sheet
    and its two copies, the decisions (section AL), `CLAUDE.md`,
    `README.md`, the conformance `README.md`, this file.

20. the Python binder (decisions AM1 and AM2; the section "The Python
    binder as it exists"): `--python` in `crates/renyi/src/bind.rs` with
    the shared `write_module`, `bind/python.rs` and
    `bind/python_inspect.py`, `interpreters` public in
    `natives/python.rs`, `renyi_json` a dependency of `renyi`, the usage
    text; the unit tests and the binary test in `tests/python.rs`; the
    decisions (section AM), the reference, `docs/python.md`, the library
    sketch, `README.md`, `CLAUDE.md`, this file.
21. the resident world (decision AN1; the section "The resident world
    as it exists"): `crates/renyi_workspace/` (`src/lib.rs`,
    `tests/refresh.rs`), `check::check_item` and
    `check::module_purpose_diagnostic` with
    `check_module_with_references` written over them,
    `module_name_mismatch` public, `index_checked` and
    `canonical_files` public with every tag carried through, `renyi
    mcp` on a `Workspace`; the workspace manifest and `Cargo.lock`; the
    decisions (section AN, with AN2 for the LSP),
    `05-agent-tooling.md` (R5-5 closed), `docs/GAPS.md`,
    `docs/RELEASE.md` (the publish order), `CLAUDE.md`, this file.
22. the language server (decisions AN2 and AN3; the section "The
    language server as it exists"): `crates/renyi/src/lsp.rs` and
    `lsp/describe.rs`, the `lsp` command, `tests/lsp.rs`; the
    workspace's as-written mode, lazy map and accessors with their
    test; `function_signature` and `type_signature` public in
    `renyi_index`, the MCP server's JSON helpers shared; the VS Code
    client (`editors/vscode/src/extension.js`, `package.json`,
    `package-lock.json`, `.vscodeignore`, the README and the
    changelog), the client's build in CI and in the release workflow,
    `.gitignore`; the decisions (AN3), the reference (appendix B),
    `README.md`, `docs/RELEASE.md`, `docs/GAPS.md`, `CLAUDE.md`, this
    file.
23. the watch of `serve` (decision AO1; the section "The watch of
    `serve` as it exists"): `crates/renyi/src/serve.rs`, the `serve`
    command with `--watch` among the flags of `run` (`exit_of` shared),
    `tests/serve.rs`; `Options.watch` and `Options.listener`,
    `Interrupt::Reload`, `RunOutcome::Reload`, `Run.listener` and the
    polling `serve` native in `crates/renyi_vm`; the decisions (section
    AO), the reference (appendix B), `07-system-design.md` (section 5
    as implemented, R7-3 closed), `docs/GAPS.md`, `README.md`,
    `CLAUDE.md`, this file.
24. the embedding API (decisions AP1 and AP2; the section "The
    embedding API as it exists"): `crates/renyi/src/sandbox.rs`
    (`Grant`, `Sandbox`, `CallError`, `Function`, `--sandbox` for
    `run`, `record` and `serve`), `Allocator` in the binary,
    `compile_file`, `tests/sandbox.rs`; `crates/renyi_vm/src/memory.rs`,
    `Options.memory`, `Interrupt::OverMemory`, `RunOutcome::OverMemory`,
    `Narrowing.sandbox`, `Vm::budgets` and `resume_budgets`;
    `renyi_syntax::parse_grant`; the guide `docs/embedding.md`, the
    decisions (section AP), the reference (appendix B),
    `07-system-design.md` (section 4 as implemented, R7-1 closed),
    `docs/GAPS.md`, `README.md`, `docs/extensions.md` (the allocator
    line), `docs/index.md` and `tools/site.py` (the guide in the
    navigation), `CLAUDE.md`, this file.
25. `44763c4` the profile-guided round on strings and JSON, paused by
    the owner to finish it elsewhere: the change as the script
    `tools/apply_perf_round.py` (applied, judged and deleted in session
    9, decision AQ), the two profiles and the numbers to beat in this
    file.

## Done in session 7 (stage 1 of the gap audit)

Fourteen commits on `main`, each gated by fmt, clippy, the tests, the
canonical corpus, the lint, the token gate and the conformance suite:

1. `8c775cd` decisions V1 to V8 (the road to self-hosting; indentation
   carries no meaning, continuation lines included, so the parser and
   the lexer lost their indentation rules).
2. `8bc1534` the checker enforces V5 (`nesting-depth`, `body-length`),
   V6 (`deprecated:` tiers 1 and 2: the `deprecated` warning with the
   replacement as its fix, an error under `renyi check --strict`, the
   index drops the definition) and V7 (`task-independence`).
3. `7ade49a` `renyi tools [path]` prints the tool manifest of decision
   D6 (`crates/renyi_index/src/tools.rs`); `tool-type` rejects a
   parameter or result JSON cannot carry.
4. `71ad9d3` the `only to` guards of P3 run: origins as a bit set on
   values (`Value::Guarded`), refusals at the boundary with the
   prelude's `Guarded(origin, sink)`, the `guard-no-sink` warning
   (`crates/renyi_vm/tests/guards.rs`).
5. `d141e57` CI on every push and the conformance suite of V8.
6. `c952ed9` the runtime keeps its promises: an expired `within`
   deadline is the function's failure, `with` runs the refinements
   again, `filesystem.copy` and `move` check the target against the
   write scope, SQLite fails with `PermissionDenied` and `OverBudget`, a
   declared `equals` decides `is`, a `Float` past its range is a crash,
   `Text` and `List` edge cases, `module.constant`
   (`crates/renyi_vm/tests/semantics.rs`).
7. `e0023c4` the checker keeps the module rules: privacy for functions,
   methods, constants and abilities (`private-name`), constants across
   modules, an implementation carries the ability's signature
   (`method-signature`), methods at home (`method-module`, K1), a
   higher-order function declares only its own effects (B1, Q1), type
   parameters in scope for body annotations, a let-bound literal takes
   its own type.
8. `56f13da` exhaustiveness is the usefulness check over the shapes of
   the arms (`check.rs`, `missing`).
9. `b997b93` one spelling of the range loop (`range-loop`), the module
   name equals the path (`module-name`, G3), `.renyi` read everywhere.
10. `7127dd8` the literal rules: a literal pattern checked by the engine
    (`regex-invalid`, K3), `Url` and `Path` literals (`invalid-literal`,
    N2), a `Date` from literals and from run-time values, a variant
    field named `kind` under the JSON abilities (`kind-field`, K10),
    `see also:` existence (`unknown-reference`, C8b), `ability X where
    self can Y` (`missing-ability`), the `count` fix of M2; `Iterable`
    struck from the core abilities (open item R3-1).
11. `ccab15e` every diagnostic carries a fix (D3): the fix-less helpers
    are gone from the lexer, the parser and the checker; the parser's
    `expected` names the Renyi spelling of a foreign token (C4); the
    lexer keeps `userName` as one name (C5); an unknown name suggests
    the closest one in scope or the Renyi name of a foreign one
    (`crates/renyi_check/src/suggest.rs`); both conformance runners
    require a `fix:` line after every diagnostic.
12. `fb798a6` the documents of `docs/GAPS.md` section 5 corrected, the
    four clippy lints of GitHub's toolchain fixed.
13. `dc58bb3` `Iterable` is one method, `to_list` (V10: the prelude
    ability, the item type from the implementation's type argument, the
    VM's `IterInit` calling `to_list`, no `needs` on an ability's
    method); V9 records the runtime grant of a passed function; CI
    pinned to rustc 1.94.1.
14. The last commit: the freeze entry V11; the sketch's status line,
    `CLAUDE.md`, `README.md`, `docs/GAPS.md` and this handoff follow.

## Done in session 6

1. **`crates/renyi_vm`** (about 8300 lines with tests): `bytecode.rs`,
   `value.rs`, `integer.rs`, `decimal.rs`, `types.rs`, `compile/{mod, expr,
   stmt, pattern, query}.rs`, `vm.rs`, `render.rs`, `natives/{mod, prelude,
   system, time, filesystem, json, csv, regex}.rs`, `runner.rs`;
   `tests/corpus.rs` runs the ten Predict programs of the readability
   manifest against `tests/readability/reference/*.out` (all ten match: the
   hand-derived references are confirmed), every `example:` and `test` of
   the corpus, and a failing and a crashing `main`.
2. **Checker changes the VM needed**, each recorded as a reference so the
   map is unaffected: `Target::Number(NumberKind)` on every numeric literal
   and on a `sum` query (the literal takes the type its context expects, so
   `Number(value: 4)` carries a Decimal); `Target::Result(Ty)` at a call
   whose return type mentions a type parameter no argument mentions
   (`json.parse`, `sqlite.query`), resolved at the end of the body;
   `Target::Variant`/`Target::Type` on variant patterns;
   `BodyLocation::Example` for the references of `example:` lines and
   `BodyLocation::Condition` for refinement conditions.
3. **Refinement conditions are now checked** (`check_type_conditions`): a
   subtype's `where value ...` and every field condition of a record or a
   variant is checked as a small body over the fields (previously they were
   not checked at all; a condition calling an unknown method passed). The
   corpus and the library needed no change; `rules.rs` has a test.
4. **`renyi run` and `renyi test`** in `crates/renyi/src/main.rs`.
5. `CLAUDE.md`, `README.md` and the status line of `06-runtime-guarantees.md`
   say what runs.
6. **The primitive boundary** (second commit of the session): `grant.rs`
   (effective grant, budget counters, `effect_of`, `parse_capability`,
   path and host normalisation), `recording.rs` (the format of section
   1.1 with `at_ms`, parsing, the replay's matching), `Vm::call_native`
   with the scope check, the budgets, recording, replay and narration;
   `runner.rs` runs `main` and each test under its own grant, loads a
   `replays` fixture beside the test's source file, `--strict` fails a
   test that leaves recorded calls unused, `--refresh NAME` re-records one
   test's fixture (`TestOutcome::Recorded`); the binary gained `record`,
   `--replay`, `--explain`, `--deny`, `--allow-host`, `--allow-read`,
   `--allow-write`, `--at-most`, `--strict`, `--refresh`.
   `tests/recording.rs` records and replays a run with console, time,
   random and filesystem effects, stops a replay at an edited entry,
   refuses a recording the grant does not cover, enforces `at most 2 per
   run` and a path scope, writes and replays a `replays` fixture offline,
   and checks the narration of a run. Checker: `records_result` also
   records the context type of an effectful library call whose result
   mentions a type parameter (`random.choice`), so a replay can decode it;
   `effects::scope_contains` is public. Library: `OverBudget` added to
   `FileError` and `HttpError` (sketch and `library/std`).
7. **The HTTP client, the HTTP server and SQLite** (third commit). The
   owner chose the dependencies (decision S1: `ureq` 3, `rusqlite` with
   SQLite compiled in, a hand-written server over `std::net`) and kept
   the tasks sequential (decision S2; sketch section 12 says so).
   `natives/{http, sqlite, server}.rs` as described above;
   `tests/network.rs` runs a server on a loopback port and a client
   against it in one process (`Options::serve_limit` stops the server
   after N requests), records and replays a SQLite program with the
   database deleted between the two runs, and checks a row outside a
   refinement. The checker records `Target::Otherwise { fallible }` at
   every `otherwise` and the compiler emits the new `Op::JumpIfFailure`
   for a fallible subject, so `otherwise` after a void fallible call
   (`server.serve(...) otherwise fail`) no longer mistakes the `Nothing`
   of success for an absent value (a bug the server test found).
   `renyi test --refresh` recorded live fixtures for `weather` (Berlin,
   14.0 C, wind 5.5 km/h), `currency_tool` (USD to EUR at 0.89254; the
   service moved from `frankfurter.app` to `api.frankfurter.dev` and the
   example follows) and `concurrent_fetch` (example.com, 577
   characters); the three `replays` tests pin those values and pass
   offline with `--strict`. `--at-most` now rejects a budget on a
   capability that takes none. `examples/README.md`, `README.md`,
   `CLAUDE.md`, the status line of `06-runtime-guarantees.md` and the
   crate doc of `renyi_vm` say so.
8. **The grant stack** (fourth commit): `grant::within` and
   `Vm::frame_grant` as described above; `tests/recording.rs` shows a
   helper declared `filesystem.read(".../data")` denied a file that
   `main` reads directly before and after the call, and the crash of a
   primitive that cannot fail naming the helper.
9. **Recordings for the rest of the corpus and `--redact`** (fifth
   commit): `pagination` and `assistant` carry hand-written fixtures (two
   pages; one chat answer with the `Authorization` header redacted),
   `inventory_db` a `replays` test that seeds its own table and was
   recorded live with `--refresh` from a scratch directory (the database
   file is not committed); every program that reaches the network or
   SQLite now runs offline. `renyi record --redact NAME` (decision S3,
   open item R6-3) replaces the argument, map entry or environment
   variable of that name with `<redacted>` in the recording, and a replay
   matches the placeholder against anything (`recording::redact`,
   `arguments_match`); open item R6-1 is settled by decision S4 (bodies
   stay inline as base64).
10. **The run manifest and `renyi reproduce`** (sixth commit; the owner
    chose the recording's header over a separate file or the source file
    itself, the standard output's hash and length as the comparison,
    hashed environment values, and a warning on a toolchain mismatch;
    decision S5). `tests/recording.rs` records a run with an argument and
    a variable, checks the manifest, reproduces it with the same output,
    and sees an edited recording reported on the outcome, the output and
    the unused call. `renyi reproduce` was also run by hand on `hello`:
    reproduced; refused after the program was edited; refused on a
    fixture without a manifest.
11. **Decision L2 carried out, and a VM bug it found** (seventh commit).
    `tests/readability/run.py` lets the VM judge Complete and Write
    (`renyi test` on the spliced or whole program after the lint and the
    checker; `score` keeps the summary and the failing items; a program
    with nothing to run still waits for `judgement.json`), and `compare`
    sets the VM's verdict against every judged sample. Live round: 179 of
    182 agree, the other 3 (gpt, `stacks`) are rejected today by
    `ignore-pure` (R6) before any judge, as the subagents had rejected
    them. Pre-test: 27 of 40 agree, the other 13 rejected today by the
    checker, the parser or the lint's foreign-keyword rule, four of them
    `while` loops judged correct under the old grammar. Before the fix
    the VM disagreed on every `sales_report` sample that wrote `for each
    sale in sales group by sale.region sum sale.amount` (ten): the
    compiler summed the whole list where decision M1 says per group.
    `Op::GroupFold` now folds `sum`, `count`, `first`, `any` and `all`
    per group (`tests/queries.rs`), and `totals_by_region` in the corpus
    is written in that form with a second example. Both rounds were
    re-scored with the cached Explain grades (no API call): the live
    round's rates are unchanged; agent-sonnet in the pre-test loses the
    four `while` samples. Each round's `notes.md` has a section on it.
12. **`renyi mcp`** (eighth commit; decisions T1 to T4, asked as a batch
    of four): `crates/renyi/src/mcp.rs` as described above, its tests,
    `renyi_index::definition_json` made public, the shared helpers in
    `main.rs`; `05-agent-tooling.md` (status, section 7 as implemented,
    section 8, open item R5-5), `README.md`, `CLAUDE.md`.
13. **The semantic diff** (ninth commit, `c1fb558`; decision T5,
    derived): `crates/renyi_index/src/diff.rs`, `hash::own_text_hash` and
    the `text_hash` field, `crates/renyi/src/maps.rs`, `renyi index
    --diff`, the `diff` tool, the tests listed above;
    `05-agent-tooling.md` (status, sections 1, 4, 6, 7, 8), `README.md`,
    `CLAUDE.md` (a `crates/renyi_index/` row).
14. **Readability round 2** (tenth commit). The owner chose all four
    models (Sonnet 5.5 and gpt-5.5 at their default temperature, the
    gating models of R1 and R8; Haiku 4.5 and gpt-5.4-mini at 0), the
    graders of round 1 and in-session adjudication; `run.py prepare`
    wrote `tests/readability/2026-10-05-ed37120/` (69 prompts; the cheat
    sheet of `ed37120`). Collection ran through the vendor APIs for
    everything but 45 Write samples of gpt-5.5, at which point the owner
    ruled that the pay-per-token keys are not spent by default
    (subscription quota instead: Claude Code subagents and the Codex
    CLI); the 45 came through `codex exec` on the ChatGPT subscription and
    `run --provider file` (provenance and session ids in the output
    records), and Explain was graded by subagents (Sonnet 5.5 first, Opus
    5.5 second, batches of fifty, reasons kept in `grades.json`), merged
    into the grade cache under `agent:` buckets so `score` ran offline.
    Eleven adjudications (`judgement.json`). A stale reference was found
    and fixed (`currency_tool` named the old host; twenty samples
    re-graded). The subagent graders are about one point more lenient
    than the API graders on the 204 samples both graded, flipping 30
    floor-model samples from fail to pass and none the other way; the
    notes say so and ask the owner which grader kind the protocol keeps.
    Results in the table above and in `notes.md` (the five-point rule:
    Sonnet's Write fell 40 to 30 on one item, two samples missing an
    import, noise at the model's default temperature; everything else
    held or rose). Status lines of `03-readability-test.md` and
    `tests/readability/README.md`.
15. **Decisions U1 to U4 applied; round 1 re-graded** (twelfth commit,
    after the handoff commit). The owner answered the four questions: M4
    stands and the cheat sheet says it in a sentence (U1);
    `rounded(places: 2)` in the cheat sheet, the library unchanged (U2);
    every library method listed with its parentheses, the `length of`
    form declined as a second spelling (U3); the subagent graders from
    here on and round 1 re-graded with them (U4). `docs/cheatsheet.md`
    at 2994 tokens after trimming elsewhere (the `deprecated` and
    `expose as tool` sentences, "No `async`/`await`", the `trimmed` call
    example, "Short signatures stay on the head line"). `run.py prepare`
    snapshots `reference/` into the run and `score` reads it from there
    (`reference_file`); both runs got their snapshot (round 1's
    `currency_tool.explain.txt` with the old host). Round 1's 450 Explain
    samples were graded by 18 subagents (nine Sonnet 5.5, nine Opus 5.5,
    batches of fifty, 2.07 million tokens of quota) and merged into its
    `grades.json` by `tests/readability/merge_agent_grades.py` (now in
    the repository); its API-graded tally moved to `scores-api.json` and
    `scores-strict-api.json`; `score` and `report` re-ran offline. The
    two subagents never disagreed by more than one point, so the hand
    verdicts of session 5 stand and nothing was adjudicated anew.
    Results and the caveats in round 1's `notes.md`, section 8; round
    2's notes, `README.md` and `03-readability-test.md` updated.
16. **Decisions U5 to U7** (thirteenth commit): the owner's answers to
    the re-grade's three questions. U5 applied to both rounds: fourteen
    verdicts in `judgement.json`, each naming the originating verdict and
    the sentence (round 1: Haiku's `inventory_db.1` to `.4`,
    `shipping_rules.0`, `invoice.2` and `.4`, the last two withdrawing
    session 5 verdicts of 4; Sonnet's `shipping_rules.1` and `retry.3`;
    round 2: Haiku's `inventory_db.0` to `.4`), both rounds re-scored
    offline (Haiku's Explain 27 of 30 in round 1, 26 in round 2; nothing
    else moved). `03-readability-test.md` amended (the Explain row, the
    sampling channel, the five-point rule), `README.md`, the notes of
    both rounds, section U of `01-decisions.md`.
17. **Round 3 through the CLIs** (fourteenth commit). `run.py` gained
    `--provider claude` (the Claude Code CLI on its subscription login:
    `claude -p` with the cheat sheet as `--system-prompt-file`, the
    dynamic system-prompt sections excluded, no tools, JSON output, no
    session persistence, `CLAUDE_CONFIG_DIR` pointed at a directory
    holding only the login, the keys removed from the environment),
    `--provider codex` (round 2's `codex exec` recipe moved into the
    harness), `--parallel`, and a `source` record per output file with
    the channel, the session ids and, for the Claude CLI, the thinking
    and output tokens of every session (thinking cannot be switched off;
    every switch was tried). `tests/readability/grade_batches.py` writes
    the subagent graders' batch files. The owner chose the two gating
    models on all four tasks; `prepare` wrote
    `tests/readability/2026-10-06-3e7c45a/`. Sonnet's 345 samples came
    with no failure (335 in the main run, 2333 seconds of session time
    four prompts at a time); gpt-5.5 stopped at 72 (Predict complete, 22
    Explain) when the Codex CLI reported the ChatGPT login's usage limit
    ("try again at Nov 4th, 2026"), and the owner chose to score Sonnet
    now. Explain graded by six subagents (three batches, two graders),
    merged, five adjudications (`file_tree.1` to `.4` graded 4 as in
    rounds 1 and 2, `config.0` graded 5), the U5 search found no
    recurrence. Results: Sonnet Predict 90, Explain 100, Complete 68,
    Write 40; gpt-5.5 Predict 100. The notes name the defect (item 18).
    A probe after the round found a second defect, in the channel: run
    from the repository, the CLI loads the project's `CLAUDE.md`, and
    the account's claude.ai connectors (Gmail, Claude Docs) bring 38
    tool definitions that `--tools ""` does not remove; round 3's samples
    had both in context. Fixed before round 4: the CLI runs in an empty
    `--work-dir` with `--strict-mcp-config` and no servers, and a probe
    through the harness's own command shows only the CLI's fixed frame
    (identity line, environment block, date, the email line) around the
    system text.
18. **Decision U8 and the corrected cheat sheet** (same commit). The
    round's Complete fell 89 to 68 because the sheet listed
    `rounded(places: 2)` (a named single argument, against M4) and
    `split()` (the separator hidden), both Claude's rendering of U2 and
    U3 and neither run through the checker before the round; Sonnet
    copied them in every sample of `invoice`, `shapes` and
    `compound_interest` and three of `word_count`. Asked as a batch of
    three; the owner chose the list with parameter names
    (`split(separator)`, `rounded(places)`, `replace(old, new)`; every
    entry checked against `library/std/prelude.ry`) plus the sentence
    "One parameter is positional, more are named" with three calls,
    round 3 as a defect round with round 4 on the corrected sheet
    through the same channel, and `count` kept reserved. Paid for by
    shortening the phrase-token list, dropping the nesting and body
    limits, the reserved-word count and "(explicit)" after `Float`:
    2998 tokens. `01-decisions.md` (U8), `03-readability-test.md`,
    `README.md`.
19. **Round 4** (fifteenth commit). The channel fix of item 17 in
    `run.py` (`call_claude_cli` takes `--work-dir`, runs there, passes
    `--strict-mcp-config --mcp-config '{"mcpServers":{}}'`; the
    `source` record names both), probed through the harness's own
    function before the run (0 tools, no instruction file, no MCP
    instructions). `prepare` wrote `tests/readability/2026-10-06-c696747/`;
    345 Sonnet samples in 2350 seconds of session time, no failure;
    Explain graded by six subagents, merged, two adjudications
    (`expression_tree.0` and `.4`, the program's own purpose line, graded
    5 as round 3's graders did), the U5 search found no recurrence.
    Results in the table above and in the round's `notes.md` (sections 1
    and 6 carry the analysis and the three questions). Round 3's notes,
    the README and this file corrected for the channel (the project
    `CLAUDE.md` and the claude.ai connectors were in round 3's context).
    A memory note on the channel recipe was saved outside the repository.
20. **Decision U9** (sixteenth commit): the owner's answers to round 4's
    questions. The sheet says when a refined construction takes
    `otherwise` ("A literal the checker can evaluate needs no
    `otherwise` (`Port(8080)`); a variable needs one even after a
    check"), paid for by the phrase-token sentence, the `# flatten`
    comment and "`renyi run` enforces it", 2991 tokens; the five-point reading stays in the notes;
    gpt-5.5 deferred. `01-decisions.md`, `03-readability-test.md`, round
    4's notes.

## Done in session 5 (condensed)

Branch consolidation into `main`; the Windows build (LF pinned by
`.gitattributes`); `renyi index` with metrics, content hashes and
`--budgets`; the first live readability round (harness fixes 1 to 9, 182
judged Complete and Write samples, 49 adjudicated Explain samples, the
results table above and the analysis in the round's `notes.md`); cheat
sheet clarifications (derived `ToText`, `purpose:`, `ignore`, `is` on
numbers); the `weight_steps` purpose line; decisions R1 to R8 applied (the
protocol gates on large models; J3 and M4 stand; `purpose:` stays; `is` on
numbers compares values of one type; `ignore` only on calls with effects,
the checker's `ignore-pure`; budgets 10 / 5 / 7; gpt-5.5 joins round 2);
`ce.toml` at the repository root (untracked, in `.git/info/exclude`) sets
CodeEraser's guard to `warn` because its 750-line budget refused appends to
`01-decisions.md` and `02-syntax-sketch.md` (owner's decision; recreate it
on a fresh clone).

## Owner actions pending

- **The social preview** (decision AV1): upload
  `docs/assets/social-preview.png` in the repository's settings
  (Settings, General, Social preview); GitHub offers no other way to set
  it. The VS Code Marketplace, if the extension is published there,
  takes its icon from `editors/vscode/package.json` already.
- **Release 0.1.0 is out** (2026-10-07). Left to the owner: the
  announcement with the texts of `08-positioning.md` section 6; the
  VS Code Marketplace publisher `renyi-lang` and `vsce publish` from
  `editors/vscode/` if the extension is to be found by search (the
  `.vsix` on the release page installs by hand). The domain
  renyi-lang.org is in front of the site (AI4, 2026-10-07, late):
  the zone is on Cloudflare, its records name GitHub Pages, the
  Pages setting names the domain and HTTPS is enforced; the session
  did it with a one-hour token minted from a credential of the
  owner's that stays on the owner's machine, outside the repository
  (`docs/RELEASE.md` section 5, item 8).
- **Updates and installers** (the owner's question of 2026-10-07,
  evening): the one-line installers overwrite the binary in place and
  `RENYI_VERSION` pins a version; `cargo install renyi` upgrades in
  place too; there is no package-manager manifest (winget, scoop,
  Homebrew) and no update check in the binary. The session's advice,
  not decided: never an automatic check (the toolchain of a language
  whose point is no undeclared effect should not phone home); an
  explicit `renyi upgrade` command and the three manifests are
  candidates after 0.1, for the owner to order.
- **Decision AI5** (the owner's answer at the end of session 8): the
  two crates that embedded files from outside their directories now
  embed copies held equal by tests; nothing remains for the owner on
  it.
- **The residue of stage 1** is done (decisions Y1 to Y4), and open item
  R3-2 (constraints with type arguments) is decision AB1; `docs/GAPS.md`
  section 7 keeps 1.11, 3.4 and 3.6 open.
- **The rename of `set` to `change`** (the owner's decision of
  2026-10-06, asked back as a structured question after the owner raised
  it) is done: decision AA1 and the section "The rename of `set` to
  `change`" above.
- **Stage 2's toolchain in Renyi is complete** (the lexer, the parser,
  the checker and the emitter, W1 to W8 and Z3), R3-2 is decided (AB1)
  and the judges run the front end from its bytecode (the section "The
  judges run from bytecode"): the next piece is the owner's to choose
  between the remaining performance items and stage 3 (M4), which is
  done since: packages, `std.process`, the FFI.
- **Readability**: the scores are no longer the gate (decision V1).
  Round 5 (U9's sentence) and gpt-5.5 run only if the owner asks.
- Session 5 printed the values of `ANTHROPIC_API_KEY` and
  `OPENAI_API_KEY` into a tool result once (not committed); the owner
  said in session 6 that they handle the transcript and the keys
  themselves, so no session needs to raise it again.

## Done in sessions 1 to 4 (condensed)

- Session 1: ~50 design decisions in interactive batches (`01-decisions.md`
  sections 0 to I), the clause grammar and the three verbs, the cheat sheet
  with its token gate, the lint, 14 example programs, the readability
  protocol (`03-readability-test.md`), README and license.
- Session 2: the corpus grown to 30 programs; round-2 decisions J1 to J18;
  the library sketch (`04-stdlib-sketch.md`) with decisions K1 to K11; the
  readability harness with the file provider; M1 started: lexer, parser,
  formatter, `renyi check`, `tokens`, `parse`, corpus conformance tests.
- Sessions 3 and 4: the subagent pre-test (`2026-10-05-e41258c/`) and
  decisions M1 to M10 from it; `renyi parse --json`; the standard library
  as declaration files (`library/std/*.ry`, decisions N1 to N3); **M2,
  `crates/renyi_check`** (world, bodies with bidirectional inference,
  subtyping, implicit `maybe`, generics, abilities, refinements on literals,
  fallible calls, error unions, exhaustiveness, queries, scoped effects);
  decisions O1 to O5 (reference counting with in-place reuse; the project
  map; budgets; the semantic diff; `renyi mcp`), P1 to P4 (recorded runs
  and `replays` tests, budgets in grants, provenance guards, the reserved
  words `only`, `per`, `replays`) and Q1 to Q4 (capability-safe packages,
  reproducibility, in-process sandboxing, checked live update), each with
  its design document.

## Next steps

1. **After the bridge and the binder** (decisions AJ2, AJ3, AL1 to
   AL4, AM1 and AM2 are in; the sections above): pending from the
   release, the announcement (owner), the three measurements of the
   positioning's section 5 on the starter pack, the Marketplace if
   wanted, and the update candidates above (`renyi upgrade`, the
   package-manager manifests), in the order the owner sets; then M5
   (item 5). Candidates around the bridge, none decided: record types
   from dataclasses and `TypedDict`s in the binder; a deadline per
   call; one worker across the tests of a `renyi test` run instead of
   one per VM; keyword arguments by parameter name instead of
   position; how the checker written in Renyi (`compiler/project.ry`,
   a fixed list of the fifteen standard modules under `--library`)
   sees extension modules, which the judges need once the corpus has a
   program that imports one. The behaviour of `renyi tools` on a
   directory of declaration files (it fails on them as programs;
   `renyi index` counts their errors) is settled: the owner struck it
   from this list on 2026-10-07, the files are not programs.
2. **Stage 2 of `docs/GAPS.md`, section 7, continued** (the grammar is
   frozen, V11; the formal grammar is `docs/grammar.ebnf`, V12; the
   language reference is `docs/reference.md`; the lexer and the parser
   in Renyi are `compiler/`, W1 to W4; the checker in Renyi is
   `compiler/declare.ry`, `bodies.ry` and `checker.ry`, W5 to W8; the
   profile and the loop are done, X1 to X6; the residue of stage 1 is
   done, Y1 to Y4; the bytecode file, `renyi compile` and the loader
   are done, Z1 to Z4; the rename of `set` to `change` is done, AA1;
   the emitter written in Renyi is done and judged, Z3; constraints
   with type arguments are done, AB1, which closes R3-2): the toolchain
   in Renyi is complete with the Rust toolchain as stage 0, and the
   judges run the front end from its bytecode. Next, in the order the
   owner chooses: the remaining performance item (the string building
   of `change out to "{out}{piece}"`, which copies the accumulated text
   per piece; the pattern cache of `Text.matches` is done, decision AQ)
   as a later profile calls for it; any change to the loop is measured with `bench.py`
   and `micro.py` against the `base` worktree, the two binaries run
   back to back. The front end in Renyi reports the Rust parser's
   diagnostics (codes, fixes, recovery; decision AD1) and the judges
   are byte-equal on rejected programs too, the step the owner chose
   beside stage 3 ("1+2", 2026-10-07).
3. **Readability, only on request**: round 5 on Sonnet measures U9
   (`run.py prepare`, the Sonnet command at the end of this item after
   its probe, the graders, `score`, the U5 search, `report`, the
   notes; `config` and `todo_cli` are the items to watch); gpt-5.5 on
   the corrected cheat sheet when the owner lifts the deferral and it
   has a channel: `run
   --provider codex --model gpt-5.5 --work-dir <empty dir> --parallel 3
   --run 2026-10-06-c696747` (restartable; the Codex quota returns Nov
   4th, 2026), or the API path with the owner's authorization; then
   `grade_batches.py`, six grader subagents, `merge_agent_grades.py`,
   `score` with `--grader agent:claude-sonnet-5-5 --second-grader
   agent:claude-opus-5-5 --temperature none` (and `--no-format --scores
   scores-strict.json`), the U5 search, adjudication, `report`, the
   notes. Round 3's 72 gpt-5.5 samples can be finished the same way in
   `2026-10-06-3e7c45a` for the record. A Sonnet round through the
   Claude CLI is `run --provider claude --model claude-sonnet-5-5
   --config-dir <login only dir> --work-dir <empty dir outside the
   repository> --parallel 4` after a probe through `call_claude_cli`
   (expect 0 tools, no instruction file, no MCP instructions); the
   login-only directory holds a hard link to the CLI's
   `.credentials.json` and a `.claude.json` of
   `{"hasCompletedOnboarding": true}`.
4. **Readability observations not asked**: `otherwise` binds loosest
   (Sonnet wrote `check f(x) otherwise "" is "y"`); Sonnet reasons before
   the Predict answer on `traffic_light` (0 of 5 with right lines); the
   rubric's reading of "only X and Y" when Z is also needed (round 1 and
   2 read it as a need missing).
5. **M4**: its three slices (packages, decision AC1; `std.process`,
   decision AE1; the FFI, decision AF1; the sections above) are done.
   **M5**, in the order the owner set on 2026-10-07: the resident
   `World` is done (decision AN1; open item R5-5 closed) and the
   language server is done (decisions AN2 and AN3: `renyi lsp` with
   diagnostics, hover, definition and the outline, the client in the
   VS Code extension) and the watch of `serve` is done (decision AO1:
   `renyi serve --watch` runs `main` again between two requests on a
   clean new version, the socket kept open, on the same resident
   world) and the embedding API is done (decisions AP1 and AP2:
   `renyi::Sandbox`, the grant as a `needs` clause with the memory
   budget, `renyi run --sandbox`): M5 is complete. Next, as the owner
   set on 2026-10-07, the interspersed items below, and later the C
   API of decision A5 over the Rust one. Candidates for the server,
   none decided: completion (names in
   scope, the library's), references, rename, formatting through the
   editor, a `renyi.path` prompt when the binary is missing. Interspersed, as the owner asked the same day: the performance
   items by the profile of AG5 (the pattern cache of `Text.matches`,
   the string building, the JSON path; strings and JSON to CPython's
   speed, the compiler's self-check twice as fast): the first round is
   done, decision AQ (the section "The profile-guided round on strings
   and JSON as it exists" below, with the numbers against the targets
   and what it left); then the baseline JIT (AG5, item iii), decided as
   AR1 and under way in three stages (the section "The baseline JIT"
   below: stage 1, `LoadField`, and stage 2, direct calls with
   register arguments, are in; stage 3, the pinned layout of `Value`
   with the value operations inline, is next); and a showcase, a bookmark and reading-list web app
   in a repository of its own under `renyi-lang/` (HTTP for the titles,
   SQLite, an HTML page and a JSON API, tags and search, CSV export,
   one use of the Python bridge, `replays` tests).

## Known gaps and risks

- **The embedding API.** The console output of a sandboxed call goes
  to the process's streams; the memory count is the process's (a
  multi-threaded host's other threads count during a budgeted call, and
  one budget is in force at a time); a bytecode file cannot be loaded
  into a sandbox; each call sets up a VM; the counting allocator's
  cost on the benchmarks is not measured; the `Value` type a host
  builds arguments with is `renyi_vm`'s, re-exported, with no
  conversion from JSON yet (the C API will need one).
- **The watch of `serve`.** A reload runs `main` again from the top,
  so a service that does work before `server.serve` repeats it on
  every reload; a file imported from outside the project's directory
  is not stat-ed, so a change to it alone is not noticed until a file
  of the directory changes; a change to an unrelated file of the
  directory costs a compile; a reload waits for the request in flight;
  the listener is polled every 20 ms under the watch, a cost a service
  without `--watch` does not pay.
- **The language server.** The VS Code client has only been bundled
  here, not run in a window: the first real session will show whether
  `vscode-languageclient` 9 and the server agree on every detail (the
  settings reload, the warning when the binary is missing); the server
  checks after every keystroke with no debounce (fine at the measured
  speed, to be watched on a large folder); the positions assume the
  UTF-16 default and do not negotiate `positionEncoding`.
- **The resident world.** The manifest is watched at the served
  directory's root only; the resolver's problems are not in the map
  (as before); a dependency's files are read again only when a
  declaration or the manifest changed; the stamp of a file written
  within two seconds of a refresh is not trusted, so an editor that
  saves every second makes every refresh read that file (the text is
  compared, not parsed).
- **The binder.** `renyi bind --python` imports the package, so the
  package's top-level code runs on the binder's machine, as it does
  for any Python tool that imports it; the signature in a purpose is
  the interpreter's rendering (Python 3.14 prints `Optional[int]` as
  `int | None`), so a generated file differs in its purposes across
  versions; a class, a dataclass or a `TypedDict` is `JsonValue`, no
  record type is generated; classes, methods and constants are not
  declared; a default is not carried, the Renyi caller passes every
  positional parameter.
- **The bridge.** The worker's standard error is inherited from the
  VM's process, not routed through `Options::stderr`, so a harness
  that captures the VM's standard error does not see what Python
  prints; a `Decimal` crosses as a JSON number, which Python reads as
  a float; a Python function that blocks (reads its standard input,
  waits on a socket) blocks the call with no deadline; the candidates
  of decision AL3 are tried in order, and a stub that prints and exits
  (the Windows Store's `python3`) costs a message on the standard
  error before the next candidate answers; the `tests/python.rs` tests
  and the two conformance cases need a Python 3 on the PATH.
- **Extensions and the checker in Renyi.** `compiler/project.ry`
  reads the fifteen standard modules by name from `--library`; an
  extension's modules are invisible to it, so the self-hosted judges
  cannot compare a program that imports one (none does today). A
  bytecode file compiled with an extension runs on a binary without
  it until the first call, which crashes as a library function this
  build does not implement, rather than being refused at load.
- **Three copies to keep.** `crates/renyi_check/library/std/` and
  `crates/renyi/cheatsheet.md` are copies (decision AI5) that a test
  in each crate holds equal to `library/std/` and `docs/cheatsheet.md`,
  and `starter/skill/renyi/cheatsheet.md` (decision AI3) is one that
  CI's `cmp` holds equal to the document; an edit to the canonical
  file without the copies fails `cargo test` or CI, which is the
  point, but the failure names the file to copy rather than copying
  it.
- **The site is checked as text, not in a browser**: the live pages
  were fetched over HTTP (status, size, titles, the example links,
  the build stamp) and the generator's output inspected as text; no
  one has looked at the rendering. The `markdown` package renders the
  documents' tables and fenced blocks; nothing highlights Renyi.
- **The FFI.** `natives/foreign.rs` and `foreign_abi.rs` are the VM's
  only unsafe code: a declaration that does not match the C side (a
  width, a missing length, a pointer the function keeps) is undefined
  behaviour in C's sense, not a Renyi crash; the signature family stops
  at six words and at integer-class and double-class scalars (no
  `float`, no struct by value, no callback); the libraries are named by
  the manifest per platform, so a project that binds `libc` names three
  names. `renyi format` does not read a foreign module. Nothing in the
  tests calls a library other than the platform's C library.
- **The front end in Renyi.** Its diagnostics are a transcription of
  the Rust lexer's and parser's (decision AD1): a new error site, a
  changed message or fix in `lexer.rs` or `parser.rs` is the same
  change in `lexer.ry` or `parser.ry` in the same commit, or the judges
  fail; the probes that exercise the error sites live in the lane, not
  in the repository (two conformance cases hold a sample).
  Its equality with the Rust parser is exact on every program in the
  repository, by construction of its `peek` (the Rust parser's
  side-effecting peek is simulated, line-break spans included); a new
  layout rule in `parser.rs` must be mirrored in `parser.ry` or the judge
  fails. The run time grows with the file (0.6 s for `parser.ry` in the
  release build after X3; the profile above says where it goes); the
  `"{text}{ch}"` concatenation in the lexer's `scan_segment` is
  quadratic in a token's length (negligible for tokens, visible on a
  long block text; `slice` is one `List.slice` since X2). `json.rs` and
  `ast.ry` must change together (W1); nothing checks that the Rust
  encoder's keys match `ast.ry` except the judge's byte comparison.
- **`std.process`.** One call per run, by decision AE1: no handle, no
  interactive reading or writing, no background process, and the
  child's whole output is held in memory. A limited run polls every
  5 ms. The program is found as `std::process::Command` finds it: on
  Windows a `.exe` on the `PATH`, not a `.cmd` or `.bat`. The scope is
  the program text for text, so `git` and `/usr/bin/git` are two
  grants. A program a signal ended reports 128 plus the signal.
- **The checker in Renyi.** It is a transcription: a change to a
  message, a fix, a rule or the order of checks in `crates/renyi_check`
  is a change to `declare.ry` or `bodies.ry` in the same commit, or the
  judge fails (CLAUDE.md says so). The by-construction differences
  listed above (hash-map ties, the NUL in a `Path`, `debug_quoted`, the
  import path separator) show on no program in the repository and
  have no test. The checker's run time after X3: 2.3 s on `bodies.ry`
  in the release build, 2.7 s in the optimized development build; the
  two judges add about 51 s to `cargo test` on eight threads and more
  on CI's runners. The Rust
  checker's quirk that a `failure(x)` binding's fields are unknown is
  reproduced on purpose (W7: equality first); fixing it is a change to
  both checkers and a conformance case.
- **The bytecode file.** JSON, about 170 KB for `hello.ry` since the
  whole library's metadata travels with every program; the loader's
  consistency pass checks indices, not stack discipline, so a file
  written by hand can still underflow the operand stack (a panic, not
  an exploit: the VM holds no unsafe code). The name of a test's code
  object is Rust's `{:?}` of the test's name, which the Renyi emitter
  reproduces through `bodies.debug_quoted` (quotes, backslashes, `\n`
  and `\t` escaped; other escapes do not occur in the corpus). The
  Renyi emitter's whitespace, when it puts an example's or a condition's
  text on one line, is what `Text.trim` strips (Rust's
  `char::is_whitespace`, as `split_whitespace` uses). A `.ryc` is tied to the toolchain that wrote
  it by nothing but `format`; a change to `Op`, `Program` or the
  format types bumps `FORMAT` and `compiler/bytecode.ry` together.
- **VM.** A declared `equals` decides `is` and `is not` (session 7);
  `contains`, `index_of`, sets and maps keep the derived form, and a
  user `Hash` implementation is never called, by decision Y3 and the
  reference's section 5. A
  value's `IsType` test for a builtin (`when failure(error: Text)`) is by
  kind. `random` is seeded from the clock; a run is reproducible through
  its recording only. `Float.to_text` is Rust's shortest round-trip form.
  `Text.matches` and `std.regex` keep a compiled pattern per thread (256
  at most, then the cache is emptied; decision AQ). Durations print as
  `1.5s` / `250ms` (no decision covers the format). A refinement condition
  on a library type (`Date`, `Port`) runs on construction but its
  references are not recorded (the checker does not walk library bodies);
  only literals and local names occur there today. An untyped `failure(x)`
  pattern on a call declared `or fails with Fault` gives `x` a type on
  which the record's fields are unknown (`unknown-field` on `x.message`);
  a typed pattern `failure(x: Fault)` or a destructuring one
  (`failure(Fault(message))`) works. Not yet reported as a checker
  item; found while writing the compiler, and met again in the checker
  (`checker.ry` destructures).
- **Boundary.** A scope denial of a primitive that cannot fail
  (`filesystem.exists`, `environment.get` under `environment("HOME")`)
  is a crash, since the module has no error to return. A budget on a
  parent (`filesystem at most 10 per run`) stays one counter, but
  `--allow-read` splitting the parent into children copies the budget
  spelling onto each child in the recording's `grant` header. The
  replay matches arguments by their JSON, so a `Float` argument that
  prints differently after a change in `float_text` would not match. A
  replayed `--explain` shows no durations. `renyi record` of a program
  that reads stdin records the lines read (`console.read_line`), as
  designed. The recording's `revision` is `git rev-parse --short HEAD` of
  the program's directory. The grant stack narrows scopes and nothing
  else: a capability kind a function does not declare passes through its
  frame, where only a function value it calls can use it; the static rule
  keeps everything else out. `--redact` works by name: a secret that
  reaches another argument (a printed line, a URL) is recorded as it is.
  The manifest hashes the standard output only: `console.print_error`
  lines and the narration go to stderr and are not compared. `reproduce`
  checks the local program's hash; fetching code by hash needs the
  registry (M4). The toolchain is named by its version only, not by a
  build hash. A `--manifest` run collects its calls in memory as `record`
  does.
- **MCP and the diff.** The map is rebuilt whole when any file changed
  (T4), and every tool call that needs it re-reads the served directory.
  The server does not implement `subscriptions/listen`, pagination,
  progress or cancellation (a `notifications/cancelled` is ignored; a
  running program runs to its end), answers a legacy `tools/call` without
  an `initialize` first, and validates `_meta` only when it carries a
  version. `run` executes effects for real unless `replay` is given; a
  program's relative paths resolve against the served directory, which
  the server enters at start. The library lookup scans the declaration
  files by line, so a declaration inside an `ability` block is not
  listed. The diff matches a rename by content hash only, so a definition
  renamed and edited in one step reads as removed and added; a
  definition moved between modules likewise (its qualified name changes);
  an `effects` change is listed on every definition it reaches, which is
  by design but makes a widened leaf verbose in a deep call graph; a git
  base indexes the revision's files afresh on every call (no cache); the
  revision's import closure for a single-file base follows `import`
  lines only (no `std` resolution needed). Open item R5-2 (local names in
  the content hash) stands.
- **Network, server, SQLite.** The server is single-threaded and answers
  one request at a time, binds `0.0.0.0`, speaks plain HTTP/1.1 without
  TLS, reads a head of at most 64 KiB and a body of at most 16 MiB, and
  has `Options::serve_limit` only so that a test can stop it. The client
  reads at most 10 MB of body (ureq's default; past it the call fails as
  `Unreachable` with ureq's message), follows at most ten redirects and
  sends ureq's own `User-Agent` (`ureq/<version>`). SQLite binds a
  `Decimal` parameter as text, so SQL that compares it with a REAL column
  compares text; the type mapping is exercised by `tests/network.rs`
  against a real database and by the `inventory_db` recording (its test
  seeds its own table). `renyi test --refresh` prints the fixture path with mixed
  separators on Windows (`examples\fixtures/weather.json`): the relative
  name is joined onto the source directory as given.
- **Checker.** First-generation (local inference, covariant type arguments,
  structural ability checks, capabilities checked at call sites only; no
  `example:` literal check, no unreachable-pattern detection). Since
  session 7 exhaustiveness is the usefulness check, `see also:` names are
  checked, `deprecated:` warns, literal patterns, URLs, paths and dates
  are checked, and every diagnostic carries a fix. The `ignore-pure` rule counts a call
  with effects anywhere inside the ignored expression. A field's
  condition binds that field alone since decision Y4 (`refinement-field`
  for another field of the type); the VM's condition code still takes
  every field.
- **Readability.** Explain is graded by subagents in both rounds (round
  1 re-graded, U4). They are about a point more lenient than the API
  graders, let three wrong statements pass in round 1 that the hand
  verdicts call wrong (round 1's `notes.md`, section 8), and read a
  boundary word strictly in one batch and loosely in another. Since U5 a
  hand verdict extends to every sample repeating the sentence, but a
  wrong statement neither grader catches and no hand verdict names still
  passes (a hand pass over every agreed grade was declined). The floor
  model's Explain number carries that; the gating models' failing samples are wrong statements
  both grader kinds mark 2. Sonnet 5.5 and gpt-5.5 cannot be sampled at temperature 0, so
  their rates carry sampling noise of about one item. gpt-5.5's Write
  samples came through the Codex CLI (its own system prompt around ours,
  reasoning effort `medium`), the rest through the API; round 3's Sonnet
  samples came through the Claude Code CLI with adaptive thinking on
  (tokens recorded per session; Predict `permissions` failed only in the
  sessions without thinking), the CLI's fixed frame (identity line,
  environment block, date, a one-line note of the account's email
  address) and, found by a probe afterwards, the project's `CLAUDE.md`
  and the claude.ai connectors' tool definitions (kept out since round
  4 by an empty `--work-dir` and `--strict-mcp-config`). Round 4's
  Complete (79) is ten points under round 2's with the targeted errors
  gone: four items fell on slips the notes attribute to the default
  temperature and to a sheet gap on refined construction and
  `otherwise`; the five-point rule's reading of that is with the owner. A call form written into the cheat sheet must
  be run through `renyi check` before a round: round 3 was lost to
  `rounded(places: 2)` and `split()`. The grant clauses
  (`at most`, `only to`, `replays` in a prompt) have not been through a
  round. `reference/*.explain.txt` must be re-read against the corpus
  before a round (one went stale this session; `prepare` now snapshots
  them into the run, so an old run keeps the text its samples were
  graded against).
- **Library.** The standard library sketch is a first draft from the
  corpus; the JSON derivation rules are exercised by the VM's decoder on
  the three recorded responses; the SQLite type mapping is exercised by one
  test and the HTTP defaults (the 30-second limit, redirect handling) by
  the three recordings and the loopback test only.
- 88 reserved words include common identifiers; the rounds measured their
  cost and the owner kept them (R2). `tools/lint_examples.py` is
  regex-based. Decision O1 rests on the value graph being acyclic.

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (deriving comparisons from `is`; typed comparison for `is`
  on numbers; keeping J3 and M4 against the measured cost).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches of four, recommended
  option first, over prose; answers within minutes.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews. `main` is the only branch. In session 8 the owner ran two
  Claude Code sessions on the repository at once; they split the files
  by cross-session message, and one ran the gates and committed. A
  session that finds the working tree changing under it should list the
  peer sessions and ask before editing a shared file. In session 9 the
  owner moved from the local machine to the cloud environment
  (claude.ai/code), whose session names a branch of its own
  (`claude/...`); asked, the owner kept the rule: the commit goes to
  `main`, and the session's branch is deleted. The cloud environment
  has rustc 1.97.0; the CI toolchain 1.94.1 is installed beside it
  (`rustup toolchain install 1.94.1 -c clippy -c rustfmt`) for the
  gates, with a target directory of its own.
- On 2026-10-11 the owner asked, after a progress list and two
  explanations (what the front end and the VM are, why the VM stays in
  Rust and why there is one), for the design philosophy on the README
  as the selling points, with the comparison to Python and Rust and
  the gap filled, and for the site rewritten by an Opus subagent at
  medium effort, "minimal, modern, premium, few short pages"; and to
  go on stage by stage, pushing each. A roadmap file numbered and
  grouped by track was asked for the same day (`docs/ROADMAP.md`).
- The pay-per-token API keys are not spent by default (ruled 2026-10-06):
  subscription quota first (Claude Code subagents, the Codex CLI), the
  keys only when the owner says so in the same request, with the volume
  named. The readability harness drives the two CLIs itself (`run
  --provider claude`, `--provider codex`) and takes grades from the
  subagents' files (`grades.json` buckets) for that reason.
