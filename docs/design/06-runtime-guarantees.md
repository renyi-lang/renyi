# Renyi Runtime Guarantees: Recorded Runs, Budgets and Provenance Guards

Status: design accepted (decisions P1 to P4); syntax in the grammar. The
VM (`crates/renyi_vm`) implements sections 1 and 2: `renyi record` (the
run manifest of decision Q2 in its header), `renyi run --replay`, `renyi
reproduce`, `--explain`, `replays` tests with `renyi test --strict` and
`--refresh`, budgets with `--at-most`, and the scope check of
section 11 of the syntax sketch against the grant stack of decision Q1
(each function's `needs` narrow the grant inside it), all at one primitive
boundary (`Vm::call_native`, section 4). Each recorded call also carries
`at_ms`, its time since the run began, which a replay's budget check uses.
The HTTP client, the server and SQLite exist (decision S1); every corpus
program that reaches the network or SQLite carries a `replays` test with
its recording under `examples/fixtures/` (`pagination` and `assistant`
hand-written, the latter with a redacted header, decision S3). Guards
(section 3) are M4. Date: 2026-10-06. Companion to `02-syntax-sketch.md`
sections 11 and 14.

The three capabilities in this document are the ones the owner chose as the
language's signature: each is new as a language feature, each is practical
for the first use case (agents writing and running API glue), and each
costs one clause of syntax because it rests on a guarantee the effect system
already gives: **every effect of a Renyi program happens through a declared
capability and one of a small set of library primitives.** There is no
ambient I/O, no reflection, no FFI outside the `foreign` capability, and no
shared mutable state. The runtime therefore sees every effect, and can
record it, count it, or tag the data that flows through it.

---

## 1. Recorded runs (decision P1)

### 1.1 Recording

`renyi record program.ry [arguments]` runs the program under the grant its
`main` declares, exactly as `renyi run` would, and writes a **recording**: a
JSON document with one entry per primitive call made under a capability,
in the order the calls completed. It goes to `--to FILE`, by default
`<program>.recording.json` in the working directory.

```json
{
  "program": "weather",
  "revision": "f520dc0",
  "toolchain": "renyi 0.0.1",
  "source": "examples/weather.ry",
  "code": "sha256:9fc7216c...562cf1b",
  "recorded_at": "2026-10-05T18:42:11Z",
  "grant": ["console", "network.http(\"api.open-meteo.com\")"],
  "outcome": "finished",
  "output": {"stdout": "sha256:3b1a0c2e...9d4e", "bytes": 36},
  "calls": [
    {
      "sequence": 1,
      "capability": "network.http(\"api.open-meteo.com\")",
      "primitive": "std.http.get",
      "arguments": {"url": "https://api.open-meteo.com/v1/forecast?latitude=52.52&longitude=13.41"},
      "outcome": {"success": {"status": 200, "headers": {"content-type": "application/json"}, "body": "{...}", "bytes": "e30="}},
      "duration_ms": 184
    },
    {
      "sequence": 2,
      "capability": "console",
      "primitive": "std.console.print",
      "arguments": {"text": "Berlin: 21.5 degrees, wind 12 km/h"},
      "outcome": {"success": null}
    }
  ]
}
```

- Arguments and outcomes are encoded with the `ToJson` rules of the library
  sketch (section 7), so a recording is readable and editable by a person or
  an agent, and a replay decodes it with the `FromJson` rules, which checks
  it against the declared types: a response whose shape no longer fits the
  program's types fails the replay the way it would fail in production.
- `time.now()`, `time.today()` and the `std.random` functions are effects
  and are recorded like any other, so a replay is deterministic.
- Output effects (`console.print`, `filesystem.write_text`, `http.post`)
  are recorded with their arguments; on replay the arguments are compared
  with the recording and a difference is reported (see 1.3). Nothing is
  written or sent during a replay.
- A failure outcome records the error value; the replay raises the same
  failure.
- `run concurrently` tasks complete in an order the scheduler decides, so
  calls are matched to entries by primitive and arguments first, and by
  sequence only when several entries match (two identical requests).
- The recording names the grant, and a replay checks that the recording
  does not exceed the grant of the test or program it is replayed under.

`renyi record --redact NAME` keeps a secret out of a recording: the
argument, the map entry (a header) or the environment variable of that
name is written as `<redacted>`, and a replay matches the placeholder
against any value (decision S3). The `assistant` fixture of the corpus
carries a redacted `Authorization` header.

`renyi record` also writes the run manifest of decision Q2 into the
header (decision S5): `toolchain`, `source`, `code` (the content hash of
`main`, which covers everything it reaches), `arguments`, `environment`
(each variable read with the hash of its value, `<redacted>` kept, `null`
when unset; empty lists and maps are left out), `outcome` and `output`
(the SHA-256 and length of the standard output). `renyi run --manifest`
prints that header without writing a file. `renyi reproduce FILE
[program.ry]` refuses when `main` hashes differently, warns when the
toolchain differs, replays the recording under its arguments with the
console output written, and reports a different outcome or output and
any recorded call not reached.

### 1.2 Replaying in tests

```
test "the forecast for Berlin is read" needs network.http replays "fixtures/weather/berlin.json"
  let forecast be weather.fetch(city: "Berlin") otherwise fail
  check forecast.temperature is 21.5
end
```

`renyi test` runs a `replays` test with every effectful call answered from
the recording, so the test is offline and deterministic. A call the
recording does not hold fails the test with the primitive, its arguments
and the nearest recorded call; an entry the test never reaches is reported
as unused when `renyi test --strict` is given. The test still declares its
`needs`, which the recording's grant must fit.

Recordings live with the tests (`fixtures/`), are committed, and are
refreshed with `renyi record`; `renyi test --refresh "name"` re-records one
test's fixture by running the test body live under its `needs`.

### 1.3 Replaying for debugging

`renyi run --replay recording.json program.ry` re-executes the program with
its effects answered from the recording: the same run, offline, as often as
needed. Combined with `--explain` (1.4) it is the debugging story: an agent
that saw a failure in production records once and then reads the narrated
replay. `--replay` stops at the first call that differs from the recording
and shows both.

### 1.4 Narrated runs: the self-narrating layer

`renyi run --explain` (also with `--replay`, and `renyi test --explain`)
prints, as the program runs, one line per call to a definition that has a
`purpose:` clause: the purpose, the arguments and the result in the
`ToText` rendering (truncated past a width), and the effects the call used,
indented by call depth:

```
Download today's report and print a summary. (main)
  Fetch the forecast for a city. (weather.fetch, city: "Berlin")
    network.http GET https://api.open-meteo.com/v1/forecast?... -> 200, 184 ms
    -> Forecast(temperature: 21.5, wind_speed: 12)
  console "Berlin: 21.5 degrees, wind 12 km/h"
```

The narration is derived, not written: `purpose:` is syntax, so every public
definition has one, and effects are declared, so the runtime knows which
lines are effects. Private helpers without a purpose are narrated by name.
A failure is narrated with the error value and the `otherwise` that caught
it. The narration is the trace an agent reads instead of a stack trace.

### 1.5 What makes this possible

Each of the following is already in the language: effects only through
declared capabilities (B1, J11) and library primitives (K1, 04-stdlib);
`time` and `random` as capabilities (B1); purity by default, so everything
outside the recorded calls is a deterministic function of them; values
with `ToJson`/`FromJson` derivations (J14, K10); `purpose:` as syntax (C8a).
The VM intercepts primitives in one place (M3), which is where recording,
replay and counting sit.

### 1.6 Open items

- R6-1: settled by decision S4: bodies stay inline as base64, no side
  file.
- R6-2: whether `--explain` narrates query clauses (`for each ... where ...`)
  as English sentences; cheap to do, possibly noisy.
- R6-3: settled by decision S3: `renyi record --redact NAME` (section
  1.1).

---

## 2. Budgets in grants (decision P2)

```
public function main() or fails with AppError
  needs console, network.http("api.example.com") at most 60 per minute
```

A capability in the grant of `main` or of a `test` may carry `at most COUNT
per UNIT`. The runtime keeps a counter per budgeted capability and scope:
a sliding window for `second`, `minute`, `hour` and `day`, a plain count for
`run`. A primitive call under the capability that would exceed the budget
fails with the module's error type, `HttpError.OverBudget(host: Text)`,
`FileError.OverBudget(path: Path)`, `ProcessError.OverBudget(program:
Text)`; the program handles it like any failure (`otherwise`, a `match`, a
retry). Budgets apply to `network`, `process` and `filesystem` and their
children; `console`, `time`, `random`, `environment` and `foreign` take
none.

The checker rejects a budget on any function other than `main` or a test
(`grant-clause`): a budget is a property of the whole run, like the grant.
`renyi run --at-most network.http=60/minute program.ry` narrows a budget
from the command line, as `--allow-host` narrows a scope; it can only
tighten. A `replays` test ignores budgets (nothing is sent) but checks that
the recording stayed within them.

Why in the grant: an agent's program that loops over an API is the usual
way to get a key banned or a bill run up; a budget in the signature of
`main` is reviewable before the run and enforced during it, and the
reviewer reads it in the same clause as the host.

Open item R6-4: budgets as data (`at most limit per minute` where `limit`
is a constant) once top-level constants can be read in grants.

---

## 3. Provenance guards (decision P3)

```
public function main() or fails with AppError
  needs console,
    filesystem.read("secrets") only to network.http("api.example.com"),
    network.http("api.example.com")
```

### 3.1 Meaning

A capability in the grant may carry `only to SINK or SINK`. Every value
that enters the program through that capability (the result of a primitive
call under it: the text of a file, the body of a response, an environment
variable) carries the **origin** of the capability; every value computed
from an origin-carrying value carries the same origin (interpolation,
record construction, list and map operations, arithmetic, conversions). A
value may leave the program through an **outgoing primitive** (a `console`
print, a `filesystem.write`, an `http` request's URL, headers or body, a
`process` argument, a `foreign` call) only when the sink is one of those
listed, or a sub-scope of one (`network.http("api.example.com")` is covered
by `network.http`); otherwise the primitive fails with the built-in error
`Guarded(origin: Text, sink: Text)` before anything leaves. A capability
without `only to` is unguarded: its data flows anywhere the grant allows.

In the example, the contents of the `secrets` directory may go to the API
and nowhere else: a prompt injection that talks the program into printing a
key, writing it to a file or posting it to another host fails at the
primitive, whatever the program's logic was made to do.

### 3.2 Implementation

Origins are tracked at run time, not in the type system. A program has few
guarded capabilities, so an origin set is a small bit set stored with each
heap value (scalars that are results of guarded primitives are boxed with
their set; literals and pure computations over unguarded data carry none).
Propagation rules:

- a primitive call under a guarded capability tags its result with the
  capability's origin;
- every operation tags its result with the union of its operands' origins
  (this includes `ToText`, interpolation, JSON rendering and decoding, and
  collections: a list carries the union of its items' origins as items are
  added);
- a `match` or `if` on a guarded value does not tag the branch taken (no
  implicit flows are tracked; see 3.4);
- an outgoing primitive computes the union of the origins of every
  argument and checks it against its own capability.

The cost is one set-union per heap allocation and one check per outgoing
primitive, both constant time for sets of at most 64 origins (R6-5 raises
the limit if a program ever needs more guards).

Recordings (section 1) store origins with each recorded value, so a replay
enforces the same guards.

### 3.3 Checker

The checker verifies that every sink after `only to` is a known capability
and that the clause sits on `main` or a test (`grant-clause`), and warns
when a guarded capability has no sink that the grant itself allows (the
data could never leave, which is sometimes intended and often a mistake).
A future static pass can prove some flows safe and skip the runtime check;
the runtime check is the guarantee.

### 3.4 Limits, stated

Guards track explicit data flow. They do not track implicit flow (a program
that prints "yes" when a secret starts with "a" leaks one bit per run), nor
timing, nor the size of what it sends. They stop the wholesale exfiltration
that prompt injection aims at, not a determined covert channel; the budget
of section 2 bounds the covert channel's rate. The document says so, so that
no one reads more into a guard than it gives.

### 3.5 Open items

- R6-5: more than 64 guarded origins (a wider set, or a dictionary).
- R6-6: whether `environment` variables should be guarded by default
  (`environment("API_KEY") only to network.http(...)` is the common case;
  a default would make the usual program safe without a clause).
- R6-7: a library function that deliberately declassifies (`guard.release(
  value)`), needing its own capability so that it shows in the grant.

---

## 4. Order of work

1. M3: the VM's primitive boundary carries the recorder, the replayer, the
   budget counters and the narration hook from the first version.
2. M3: `renyi record`, `renyi run --replay`, `--explain`, `renyi test` with
   `replays`, budgets with `--at-most`; the corpus programs that reach the
   network (`weather`, `concurrent_fetch`, `pagination`, `assistant`,
   `currency_tool`) get recordings and `replays` tests.
3. M4: origin tracking and the `only to` check; `environment` default
   (R6-6) decided then; the readability round measures the three clauses.
