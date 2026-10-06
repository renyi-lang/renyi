//! Every diagnostic of the lexer and the parser carries a fix (decision D3),
//! and a spelling of another language gets the Renyi one as its fix
//! (decision C4).

use renyi_syntax::{lex, parse, Diagnostic};

/// The lexer's and the parser's diagnostics for a source, each once.
fn diagnostics(source: &str) -> Vec<Diagnostic> {
    let parsed = parse(source).diagnostics;
    let lexed = lex(source).diagnostics;
    let included = lexed
        .iter()
        .all(|l| parsed.iter().any(|p| p.code == l.code && p.span == l.span));
    if included {
        parsed
    } else {
        lexed.into_iter().chain(parsed).collect()
    }
}

fn program(body: &str) -> String {
    format!("module demo\n  purpose: Spellings of other languages.\n\n{body}\n")
}

/// The fix of the first diagnostic with this code.
fn fix_of(source: &str, code: &str) -> String {
    let all = diagnostics(source);
    let found = all
        .iter()
        .find(|d| d.code == code)
        .unwrap_or_else(|| panic!("no `{code}` in {all:?}"));
    found
        .fix
        .clone()
        .unwrap_or_else(|| panic!("`{code}` has no fix: {}", found.message))
}

/// Bodies that each trip at least one diagnostic site of the parser.
const BROKEN_BODIES: &[&str] = &[
    "def go() returns Integer\n  return 1\nend",
    "fn go() returns Integer\n  return 1\nend",
    "function go(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  else\n    return 2\n  end\nend",
    "function go(flag: Boolean) returns Integer\n  if flag:\n    return 1\n  end\n  return 2\nend",
    "function go(count_of: Integer) returns Boolean\n  return count_of == 1\nend",
    "function go(count_of: Integer) returns Boolean\n  return count_of < 1\nend",
    "function go(count_of: Integer) returns Boolean\n  return count_of is larger than 1\nend",
    "function go(userName: Text) returns Text\n  return userName\nend",
    "type user_record\n  has name: Text\nend",
    "function go(flag: Boolean) returns Integer\n  while flag\n    return 1\n  end\n  return 2\nend",
    "function go() returns Integer\n  let total = 1\n  return total\nend",
    "function go(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  elif flag then\n    return 3\n  end\n  return 2\nend",
    "function go(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  end if\n  return 2\nend",
    "function go() returns Integer\n  return 1;\nend",
    "function go(items: List of Integer) returns Integer\n  for item in items\n    return item\n  end\n  return 0\nend",
    "function go() returns Integer\n  let total be 1\n  total = 2\n  return total\nend",
    "function go(flag: Boolean) returns Integer\n  if (flag) {\n    return 1\n  }\n  return 2\nend",
    "function go() returns Integer\n  return 1\n",
    "function go() needs network at most ten per minute\n  return\nend",
    "function go() needs network at most 10 per week\n  return\nend",
    "type Shape\n  purpose: A figure.\n  has name: Text\n  foo\nend",
    "type Shape\n  has name: Text as name\nend",
    "public ability ToText for Shape\n  function to_text(self) returns Text\n    return \"x\"\n  end\nend",
    "ability Printable\n  foo\nend",
    "test go\n  check true\nend",
    "function go() returns Text\n  return raw 1\nend",
    "function go(value: Integer) returns Integer\n  match value\n    when + then return 1\n    otherwise return 2\n  end\nend",
    "function go(value: Text) returns Integer\n  return value.(1)\nend",
    "function go(value: Integer) returns Integer\n  return -value\nend",
    "function go(items: List of Integer) returns Integer\n  return for each item in items where item is 1\nend",
    "function go() returns Integer\n  let total be 1\n  total\n  return total\nend",
    "function go(value: maybe Integer) returns Integer\n  let total be value\n    otherwise 0\n  return total\nend",
    "function go() returns Integer\n  until true\n  end\n  return 1\nend",
    "function go(Name: Text) returns Text\n  return \"x\"\nend",
    "function go() returns integer\n  return 1\nend",
    "function go() returns Integer\n  return 1 $ 2\nend",
    "function go() returns Integer\n  return 1\nend\n\nfunction other(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  }\n  return 2\nend",
];

/// Whole sources, for the sites before the first item.
const BROKEN_SOURCES: &[&str] = &[
    "function go() returns Integer\n  return 1\nend\n",
    "module demo\n  purpose: A dot without a name.\n\nimport std.\n\nfunction go() returns Integer\n  return 1\nend\n",
];

#[test]
fn every_diagnostic_carries_a_fix() {
    let sources: Vec<String> = BROKEN_BODIES
        .iter()
        .map(|body| program(body))
        .chain(BROKEN_SOURCES.iter().map(|s| s.to_string()))
        .collect();
    for source in &sources {
        let all = diagnostics(source);
        assert!(!all.is_empty(), "accepted:\n{source}");
        for d in &all {
            assert!(
                d.fix.is_some(),
                "`{}` has no fix: {}\n{source}",
                d.code,
                d.message
            );
        }
    }
}

#[test]
fn a_foreign_spelling_gets_the_renyi_one() {
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[0]), "foreign-keyword"),
        "write `function`"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[1]), "foreign-keyword"),
        "write `function`"
    );
    assert!(fix_of(&program(BROKEN_BODIES[6]), "expected").contains("`is greater than`"));
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[7]), "identifier-shape"),
        "write `user_name`"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[8]), "type-name-shape"),
        "write `UserRecord`"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[12]), "expected"),
        "`end` closes a block by itself; drop the word after it"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[14]), "expected"),
        "write `for each item in items`"
    );
    assert!(fix_of(&program(BROKEN_BODIES[16]), "expected").contains("without braces"));
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[17]), "missing-end"),
        "write `end`"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[33]), "identifier-shape"),
        "write `name`"
    );
    assert_eq!(
        fix_of(&program(BROKEN_BODIES[34]), "type-name-shape"),
        "write `Integer`"
    );
}

#[test]
fn a_camel_case_name_is_one_token() {
    let lexed = lex("getHTTPResponse");
    assert_eq!(lexed.tokens[0].span.end, "getHTTPResponse".len());
    assert_eq!(
        lexed.diagnostics[0].fix.as_deref(),
        Some("write `get_http_response`")
    );
}

#[test]
fn a_token_the_lexer_reported_is_not_reported_again() {
    for body in [BROKEN_BODIES[10], BROKEN_BODIES[4], BROKEN_BODIES[13]] {
        let all = diagnostics(&program(body));
        assert!(all.iter().all(|d| d.code != "expected"), "{all:?}");
        assert_eq!(all.len(), 1, "{all:?}");
    }
}
