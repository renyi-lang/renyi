# Renyi

Renyi is a statically typed programming language whose syntax is regular
English. It is designed to be read and written by LLM agents and reviewed by
people: one spelling per concept, effects in the type system, no exceptions,
no null, no anonymous functions, and documentation that is part of the grammar
so that code can be retrieved by meaning.

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

Source files use the `.renyi` or `.ry` extension; the two are equivalent.

## Status

Design phase (M0). There is no compiler yet. What exists:

- `docs/design/01-decisions.md`: every design decision taken so far, with the
  reasoning.
- `docs/design/02-syntax-sketch.md`: the concrete surface those decisions
  produce, and the open items for the next round.
- `docs/design/03-readability-test.md`: how the grammar is frozen by measuring
  LLM comprehension before the parser is written.
- `docs/cheatsheet.md`: the whole language on one page, kept under 3000 tokens
  (`python3 tools/count_tokens.py`).
- `examples/`: the example corpus, checked by `python3 tools/lint_examples.py`.

Next: grow the corpus to thirty programs, run the readability test, then start
the Rust implementation (lexer, parser, formatter) in M1.

## License

Apache-2.0. See `LICENSE`.
