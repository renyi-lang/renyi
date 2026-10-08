//! `renyi bind <header.h> --module <name> --library <name>[,<name>...]
//! [--to <directory>]` (decision AF1): the prototypes of a C header, as far
//! as the boundary carries them, written as a foreign module of the
//! project: the declaration file `<directory>/<name>.ry`, and the module's
//! entry in the manifest `renyi.json` of that directory (written when the
//! manifest exists, printed otherwise). A prototype the boundary cannot
//! carry (a pointer other than `char *`, a struct, a `float`, a `long`, a
//! variadic function) is left in the file as a comment with the reason.
//! `renyi bind --python <package> [--module <name>] [--to <directory>]`
//! (decisions AM1 and AM2, `python.rs`) does the same for a Python package
//! through the interpreter of decision AL3.

use std::path::Path;
use std::process::ExitCode;

use renyi_check::foreign::WIDTHS;
use renyi_package::{Manifest, MANIFEST_FILE};
use renyi_syntax::{ForeignModule, PythonModule, Word};

mod python;

/// What `renyi bind` makes of a header.
pub struct Binding {
    pub declarations: String,
    pub entry: ForeignModule,
}

pub fn bind_command(args: &[String]) -> ExitCode {
    let mut header: Option<String> = None;
    let mut python: Option<String> = None;
    let mut module: Option<String> = None;
    let mut libraries: Vec<String> = Vec::new();
    let mut directory = ".".to_string();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--python" => match rest.next() {
                Some(value) => python = Some(value.clone()),
                None => return usage("`--python` takes the package to import"),
            },
            "--module" => match rest.next() {
                Some(value) => module = Some(value.clone()),
                None => return usage("`--module` takes a name"),
            },
            "--library" => match rest.next() {
                Some(value) => libraries.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_string),
                ),
                None => return usage("`--library` takes a name, or names separated by commas"),
            },
            "--to" => match rest.next() {
                Some(value) => directory = value.clone(),
                None => return usage("`--to` takes a directory"),
            },
            other if other.starts_with("--") => return usage(&format!("unknown option `{other}`")),
            other => {
                if header.is_some() {
                    return usage("one header at a time");
                }
                header = Some(other.to_string());
            }
        }
    }
    if let Some(package) = python {
        if header.is_some() {
            return usage("`--python` binds a package, not a header");
        }
        if !libraries.is_empty() {
            return usage("`--library` is for a C header; a Python package needs none");
        }
        return python::bind_command(&package, module, &directory);
    }
    let Some(header) = header else {
        return usage("a header file is required, or `--python <package>`");
    };
    let Some(module) = module else {
        return usage("`--module <name>` is required");
    };
    if libraries.is_empty() {
        return usage("`--library <name>` is required: the shared library the symbols are in");
    }
    if !is_module_name(&module) {
        return usage(&format!(
            "`{module}` is not a module name; write lower-case segments separated by dots"
        ));
    }
    let text = match std::fs::read_to_string(&header) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("renyi: cannot read {header}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let header_name = Path::new(&header)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| header.clone());
    let binding = bind(&header_name, &text, &module, libraries);
    write_module(
        &directory,
        &module,
        &binding.declarations,
        ManifestEntry::Foreign(binding.entry),
    )
}

fn usage(message: &str) -> ExitCode {
    eprintln!(
        "renyi: {message}\nusage: renyi bind <header.h> --module <name> --library <name>[,<name>...] [--to <directory>]\n       renyi bind --python <package> [--module <name>] [--to <directory>]"
    );
    ExitCode::FAILURE
}

/// A module's entry for the manifest: its section says what kind of
/// module it is.
enum ManifestEntry {
    Foreign(ForeignModule),
    Python(PythonModule),
}

impl ManifestEntry {
    fn kind(&self) -> &'static str {
        match self {
            ManifestEntry::Foreign(_) => "foreign",
            ManifestEntry::Python(_) => "Python",
        }
    }

    /// The entry into its section of the manifest, in place of an entry
    /// of the same name.
    fn write_into(self, manifest: &mut Manifest, module: &str) {
        match self {
            ManifestEntry::Foreign(entry) => {
                manifest.foreign.retain(|(name, _)| name != module);
                manifest.foreign.push((module.to_string(), entry));
                manifest.foreign.sort_by(|a, b| a.0.cmp(&b.0));
            }
            ManifestEntry::Python(entry) => {
                let modules = &mut manifest.python.modules;
                modules.retain(|(name, _)| name != module);
                modules.push((module.to_string(), entry));
                modules.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }
    }

    /// The section as the manifest spells it, for a directory without one.
    fn json(&self, module: &str) -> String {
        match self {
            ManifestEntry::Foreign(entry) => {
                format!("\"foreign\": {}", entry_json(module, entry))
            }
            ManifestEntry::Python(entry) => format!(
                "\"python\": {{\"modules\": {}}}",
                python::entry_json(module, entry)
            ),
        }
    }
}

/// The declaration file written as `<directory>/<name>.ry`, and the entry
/// into the manifest of the directory when it has one, printed otherwise.
fn write_module(
    directory: &str,
    module: &str,
    declarations: &str,
    entry: ManifestEntry,
) -> ExitCode {
    let file = Path::new(directory).join(format!("{}.ry", module.replace('.', "/")));
    if let Some(parent) = file.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!("renyi: cannot create {}: {error}", parent.display());
            return ExitCode::FAILURE;
        }
    }
    if let Err(error) = std::fs::write(&file, declarations) {
        eprintln!("renyi: cannot write {}: {error}", file.display());
        return ExitCode::FAILURE;
    }
    let manifest_path = Path::new(directory).join(MANIFEST_FILE);
    match std::fs::read_to_string(&manifest_path) {
        Ok(source) => {
            let mut manifest = match Manifest::read(&source) {
                Ok(manifest) => manifest,
                Err(detail) => {
                    eprintln!("renyi: {}: {detail}", manifest_path.display());
                    return ExitCode::FAILURE;
                }
            };
            let kind = entry.kind();
            entry.write_into(&mut manifest, module);
            if let Err(error) = std::fs::write(&manifest_path, manifest.render()) {
                eprintln!("renyi: cannot write {}: {error}", manifest_path.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "renyi: wrote {} and the {kind} module `{module}` of {}",
                file.display(),
                manifest_path.display()
            );
        }
        Err(_) => {
            eprintln!(
                "renyi: wrote {}; no {MANIFEST_FILE} in {directory}, so add to it:\n  {}",
                file.display(),
                entry.json(module)
            );
        }
    }
    ExitCode::SUCCESS
}

/// `{"libc": {"library": [...], "symbols": {...}}}` as the manifest spells it.
fn entry_json(module: &str, entry: &ForeignModule) -> String {
    let libraries: Vec<String> = entry
        .libraries
        .iter()
        .map(|name| format!("{name:?}"))
        .collect();
    let mut inner = format!("\"library\": [{}]", libraries.join(", "));
    if !entry.symbols.is_empty() {
        let symbols: Vec<String> = entry
            .symbols
            .iter()
            .map(|(renyi, c)| format!("{renyi:?}: {c:?}"))
            .collect();
        inner.push_str(&format!(", \"symbols\": {{{}}}", symbols.join(", ")));
    }
    format!("{{{module:?}: {{{inner}}}}}")
}

fn is_module_name(name: &str) -> bool {
    name.split('.').all(renyi_package::is_package_name)
}

// ------------------------------------------------------------ the header

/// One prototype of the header as the file will hold it.
enum Entry {
    Function {
        name: String,
        symbol: String,
        params: Vec<(String, &'static str)>,
        returns: Option<&'static str>,
        prototype: String,
    },
    Skipped {
        prototype: String,
        reason: String,
    },
}

/// The declaration file and the manifest entry for a header's text.
pub fn bind(header_name: &str, header: &str, module: &str, libraries: Vec<String>) -> Binding {
    let mut entries = Vec::new();
    for statement in statements(header) {
        if let Some(entry) = entry_of(&statement) {
            entries.push(entry);
        }
    }
    let mut widths: Vec<&str> = Vec::new();
    let mut symbols: Vec<(String, String)> = Vec::new();
    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for entry in entries {
        match entry {
            Entry::Function {
                name,
                symbol,
                params,
                returns,
                prototype,
            } => {
                let mut name = name;
                while taken.contains(&name) {
                    name.push('_');
                }
                taken.push(name.clone());
                if name != symbol {
                    symbols.push((name.clone(), symbol));
                }
                for (_, ty) in &params {
                    note_width(&mut widths, ty);
                }
                if let Some(ty) = returns {
                    note_width(&mut widths, ty.trim_start_matches("maybe "));
                }
                functions.push((name, params, returns, prototype));
            }
            Entry::Skipped { prototype, reason } => skipped.push((prototype, reason)),
        }
    }
    symbols.sort();
    let mut text = format!(
        "module {module}\n  purpose: Bindings generated by `renyi bind` from {header_name}.\n"
    );
    if !widths.is_empty() {
        widths.sort_by_key(|name| WIDTHS.iter().position(|(width, _)| width == name));
        text.push_str(&format!(
            "\nimport std.foreign exposing {}\n",
            widths.join(", ")
        ));
    }
    if !skipped.is_empty() {
        text.push('\n');
        for (prototype, reason) in &skipped {
            text.push_str(&format!("# skipped: {prototype} ({reason})\n"));
        }
    }
    for (name, params, returns, prototype) in &functions {
        text.push('\n');
        text.push_str(&declaration(name, params, *returns, &["needs foreign"]));
        text.push_str(&format!("  purpose: `{prototype}`.\n"));
    }
    Binding {
        declarations: text,
        entry: ForeignModule { libraries, symbols },
    }
}

fn note_width<'a>(widths: &mut Vec<&'a str>, ty: &'a str) {
    if WIDTHS.iter().any(|(width, _)| *width == ty) && !widths.contains(&ty) {
        widths.push(ty);
    }
}

/// A declaration in canonical layout: on one line when it fits, else the
/// clauses (`returns`, then those given: the failure, the needs) on lines
/// of their own.
fn declaration<T: AsRef<str>>(
    name: &str,
    params: &[(String, T)],
    returns: Option<&str>,
    clauses: &[&str],
) -> String {
    let params: Vec<String> = params
        .iter()
        .map(|(name, ty)| format!("{name}: {}", ty.as_ref()))
        .collect();
    let head = format!("public function {name}({})", params.join(", "));
    let mut lines: Vec<String> = returns
        .map(|ty| format!("returns {ty}"))
        .into_iter()
        .collect();
    lines.extend(clauses.iter().map(|clause| clause.to_string()));
    let one_line = format!("{head} {}\n", lines.join(" "));
    if one_line.len() <= 100 {
        return one_line;
    }
    let mut out = head;
    out.push('\n');
    for line in lines {
        out.push_str(&format!("  {line}\n"));
    }
    out
}

/// The statements of the header: comments and preprocessor lines removed,
/// `extern "C"` brackets dropped, split at `;` outside braces.
fn statements(header: &str) -> Vec<String> {
    let text = without_comments(header);
    let text = without_preprocessor(&text);
    let text = text.replace("extern \"C\"", " ");
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                if depth > 0 {
                    depth -= 1;
                    current.push(ch);
                }
                // an unmatched `}` closed an `extern "C" {` bracket
            }
            ';' if depth == 0 => {
                out.push(collapse(&current));
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let block = rest.find("/*");
        let line = rest.find("//");
        // the earlier of the two kinds of comment, and which it is
        let next = match (block, line) {
            (Some(b), Some(l)) if b < l => Some((b, true)),
            (Some(b), None) => Some((b, true)),
            (_, Some(l)) => Some((l, false)),
            (None, None) => None,
        };
        let Some((at, is_block)) = next else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        rest = if is_block {
            out.push(' ');
            match rest[at + 2..].find("*/") {
                Some(end) => &rest[at + 2 + end + 2..],
                None => "",
            }
        } else {
            match rest[at..].find('\n') {
                Some(end) => &rest[at + end..],
                None => "",
            }
        };
    }
}

fn without_preprocessor(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut continued = false;
    for line in text.lines() {
        let directive = continued || line.trim_start().starts_with('#');
        continued = directive && line.trim_end().ends_with('\\');
        if !directive {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Whitespace collapsed to single spaces, `*` spaced from what follows it.
fn collapse(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out.replace(" *", "*").replace('*', " *")
}

const NOISE: [&str; 6] = [
    "extern",
    "__cdecl",
    "__inline",
    "__extension__",
    "__restrict",
    "restrict",
];

/// The statement as a function entry, or `None` when it is not a
/// prototype (a type, a variable, a definition).
fn entry_of(statement: &str) -> Option<Entry> {
    let prototype = statement.to_string();
    if statement.contains('{') {
        return None;
    }
    let mut text = statement.to_string();
    text = without_parenthesized(&text, "__attribute__");
    text = without_parenthesized(&text, "__declspec");
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|word| !NOISE.contains(word))
        .collect();
    let text = words.join(" ");
    let first = words.first().copied().unwrap_or("");
    if matches!(
        first,
        "typedef" | "struct" | "union" | "enum" | "static" | "inline" | ""
    ) {
        return None;
    }
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open || !text[close + 1..].trim().is_empty() {
        return None;
    }
    let head = text[..open].trim();
    let params_text = &text[open + 1..close];
    let name_start = head.rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))?;
    let c_name = &head[name_start + 1..];
    let returns_text = head[..=name_start].trim();
    if c_name.is_empty() || returns_text.is_empty() {
        return None;
    }
    if text.contains("__stdcall") || text.contains("__fastcall") {
        return Some(Entry::Skipped {
            prototype,
            reason: "a calling convention other than C's".to_string(),
        });
    }
    if params_text.contains('(') {
        return Some(Entry::Skipped {
            prototype,
            reason: "a function-pointer parameter".to_string(),
        });
    }
    if params_text.contains("...") {
        return Some(Entry::Skipped {
            prototype,
            reason: "variadic".to_string(),
        });
    }
    let returns = match returns_text {
        "void" => None,
        other => match carried(other, true) {
            Ok(renyi) => Some(renyi),
            Err(reason) => {
                return Some(Entry::Skipped { prototype, reason });
            }
        },
    };
    let mut params = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let pieces: Vec<&str> = params_text
        .split(',')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .collect();
    if !(pieces.len() == 1 && pieces[0] == "void") {
        for (index, piece) in pieces.iter().enumerate() {
            let (ty, name) = split_parameter(piece);
            let carried = match carried(&ty, false) {
                Ok(carried) => carried,
                Err(reason) => return Some(Entry::Skipped { prototype, reason }),
            };
            // a single letter is not a Renyi name (`single-letter-identifier`)
            let mut name = name
                .map(|name| renyi_name(&name, "c_"))
                .filter(|name| name.len() > 1)
                .unwrap_or_else(|| format!("argument_{}", index + 1));
            while names.contains(&name) {
                name.push('_');
            }
            names.push(name.clone());
            params.push((name, carried));
        }
    }
    Some(Entry::Function {
        name: renyi_name(c_name, "c_"),
        symbol: c_name.to_string(),
        params,
        returns,
        prototype,
    })
}

/// `__attribute__((...))` and the like removed, parentheses balanced.
fn without_parenthesized(text: &str, keyword: &str) -> String {
    let mut out = text.to_string();
    while let Some(start) = out.find(keyword) {
        let mut depth = 0usize;
        let mut end = None;
        for (offset, ch) in out[start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + offset + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        match end {
            Some(end) => out.replace_range(start..end, " "),
            None => break,
        }
    }
    out
}

const TYPE_WORDS: [&str; 24] = [
    "void",
    "char",
    "short",
    "int",
    "long",
    "signed",
    "unsigned",
    "float",
    "double",
    "bool",
    "_Bool",
    "size_t",
    "ssize_t",
    "int8_t",
    "uint8_t",
    "int16_t",
    "uint16_t",
    "int32_t",
    "uint32_t",
    "int64_t",
    "uint64_t",
    "intptr_t",
    "uintptr_t",
    "ptrdiff_t",
];

/// A parameter's type and its name, when it has one.
fn split_parameter(piece: &str) -> (String, Option<String>) {
    let words: Vec<&str> = piece.split_whitespace().collect();
    let Some(last) = words.last() else {
        return (String::new(), None);
    };
    let identifier = last.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
        && last
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if words.len() > 1 && identifier && !TYPE_WORDS.contains(last) {
        (
            words[..words.len() - 1].join(" "),
            Some((*last).to_string()),
        )
    } else if let Some(name) = last.strip_prefix('*').filter(|name| !name.is_empty()) {
        // `char *name`: the star belongs to the type
        let mut ty = words[..words.len() - 1].join(" ");
        ty.push_str(" *");
        (ty, Some(name.to_string()))
    } else {
        (words.join(" "), None)
    }
}

/// The Renyi type a C type is carried as, or why it is not.
fn carried(c_type: &str, result: bool) -> Result<&'static str, String> {
    let words: Vec<&str> = c_type
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile"))
        .collect();
    let mut stars = 0;
    let mut base: Vec<&str> = Vec::new();
    for word in words {
        let plain = word.trim_matches('*');
        stars += word.matches('*').count();
        if !plain.is_empty() {
            base.push(plain);
        }
    }
    let base = base.join(" ");
    if stars > 1 {
        return Err(format!("`{c_type}` is a pointer to a pointer"));
    }
    if stars == 1 {
        return match base.as_str() {
            "char" => Ok(if result { "maybe Text" } else { "Text" }),
            "unsigned char" | "uint8_t" | "signed char" | "int8_t" => Err(format!(
                "`{c_type}` is a byte pointer, which needs a length: declare it by hand with `Bytes`"
            )),
            _ => Err(format!("`{c_type}` is a pointer")),
        };
    }
    let renyi = match base.as_str() {
        "char" | "signed char" | "int8_t" => "Int8",
        "unsigned char" | "uint8_t" => "UInt8",
        "short" | "short int" | "signed short" | "signed short int" | "int16_t" => "Int16",
        "unsigned short" | "unsigned short int" | "uint16_t" => "UInt16",
        "int" | "signed" | "signed int" | "int32_t" => "Int32",
        "unsigned" | "unsigned int" | "uint32_t" => "UInt32",
        "long long" | "long long int" | "signed long long" | "int64_t" | "intptr_t"
        | "ptrdiff_t" | "ssize_t" => "Int64",
        "unsigned long long" | "unsigned long long int" | "uint64_t" | "uintptr_t" => "UInt64",
        "size_t" => "Size",
        "double" => "Float",
        "bool" | "_Bool" => "Boolean",
        "long" | "long int" | "unsigned long" | "unsigned long int" | "signed long" => {
            return Err(format!(
                "the width of `{c_type}` differs across platforms; write `int32_t` or `int64_t` in the header"
            ));
        }
        "float" => {
            return Err(
                "`float` is not in the signature family; declare the C side with `double`"
                    .to_string(),
            );
        }
        "void" => return Err("`void` is not a parameter type".to_string()),
        _ => return Err(format!("`{c_type}` is not a type of the boundary")),
    };
    Ok(renyi)
}

/// A C or Python name as a Renyi name: snake case, never a reserved word
/// (the prefix marks one, `c_` or `py_`) nor starting with a digit.
fn renyi_name(foreign_name: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut previous_lower = false;
    for ch in foreign_name.chars() {
        if ch.is_ascii_uppercase() {
            if previous_lower {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_lower = false;
        } else {
            out.push(ch);
            previous_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    let mut name = String::new();
    for ch in out.trim_matches('_').chars() {
        if ch == '_' && name.ends_with('_') {
            continue;
        }
        name.push(ch);
    }
    if name.is_empty() || name.starts_with(|ch: char| ch.is_ascii_digit()) {
        name = format!("{prefix}{name}");
    }
    if Word::from_spelling(&name).is_some() {
        name = format!("{prefix}{name}");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_becomes_declarations_and_an_entry() {
        let header = "#include <stddef.h>\n/* the length of a string */\nsize_t strlen(const char *s);\nint abs(int value); // absolute\nchar *getenv(const char *name);\nint printf(const char *format, ...);\nvoid SetValue(unsigned long value);\n";
        let binding = bind("string.h", header, "libc", vec!["libc.so.6".to_string()]);
        assert_eq!(
            binding.declarations,
            "module libc\n  purpose: Bindings generated by `renyi bind` from string.h.\n\nimport std.foreign exposing Int32, Size\n\n# skipped: int printf(const char *format, ...) (variadic)\n# skipped: void SetValue(unsigned long value) (the width of `unsigned long` differs across platforms; write `int32_t` or `int64_t` in the header)\n\npublic function strlen(argument_1: Text) returns Size needs foreign\n  purpose: `size_t strlen(const char *s)`.\n\npublic function abs(value: Int32) returns Int32 needs foreign\n  purpose: `int abs(int value)`.\n\npublic function getenv(name: Text) returns maybe Text needs foreign\n  purpose: `char *getenv(const char *name)`.\n"
        );
        assert_eq!(binding.entry.libraries, vec!["libc.so.6".to_string()]);
        assert!(binding.entry.symbols.is_empty());
    }

    #[test]
    fn names_are_snake_case_and_never_reserved() {
        assert_eq!(renyi_name("GetTickCount", "c_"), "get_tick_count");
        assert_eq!(renyi_name("strlen", "c_"), "strlen");
        assert_eq!(renyi_name("count", "c_"), "c_count");
        assert_eq!(renyi_name("_exit", "c_"), "exit");
        assert_eq!(renyi_name("SHA256Init", "c_"), "sha256_init");
    }
}
