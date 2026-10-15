//! `renyi bind --python <package> [--module <name>] [--to <directory>]`
//! (decisions AM1 and AM2): the functions of a Python package, as the
//! interpreter of decision AL3 reports them through the inspection script
//! `python_inspect.py`, written as a Python module of the project: the
//! declaration file `<directory>/<name>.ry` and the module's entry in the
//! manifest's `python` section. A parameter's or a result's type is the
//! Renyi type its annotation maps to, `JsonValue` of `std.json` where the
//! mapping does not reach, with a comment naming what stands as
//! `JsonValue`; a function the bridge cannot call by position (a
//! keyword-only parameter without a default) or whose signature Python
//! cannot give is left in the file as a comment with the reason. A
//! dataclass, a `NamedTuple` or a `TypedDict` the annotations name becomes
//! a record type of the file (decision AW49), its fields by the same
//! mapping, when its fields are Renyi names and its bare name is free;
//! otherwise it stays `JsonValue` and the comment says why.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use renyi_json::{read_json, Json};
use renyi_package::{Manifest, MANIFEST_FILE};
use renyi_syntax::PythonModule;
use renyi_vm::natives::python::interpreters;

use super::{declaration, is_module_name, renyi_name, usage, write_module, ManifestEntry};

const INSPECT: &str = include_str!("python_inspect.py");

/// The prefix a Python name gets when it is a reserved word of Renyi.
const PREFIX: &str = "py_";

/// What a purpose line has room for after `  purpose: ` within the line
/// limit of 100 columns.
const PURPOSE_ROOM: usize = 100 - "  purpose: ".len();

/// The type names a generated record may not take: the prelude's, the
/// JSON module's, the time module's and the bridge's.
const TAKEN_TYPES: [&str; 28] = [
    "Integer",
    "Decimal",
    "Float",
    "Boolean",
    "Text",
    "Bytes",
    "List",
    "Map",
    "Set",
    "Range",
    "Pair",
    "Duration",
    "Ordering",
    "ConstraintViolation",
    "InvalidNumber",
    "InvalidEncoding",
    "TimedOut",
    "Guarded",
    "JsonValue",
    "Naming",
    "JsonError",
    "Instant",
    "Date",
    "Weekday",
    "InvalidDate",
    "PythonError",
    "Equal",
    "Compare",
];

const LISTS: [&str; 4] = [
    "builtins.list",
    "collections.abc.Sequence",
    "collections.abc.MutableSequence",
    "collections.abc.Iterable",
];
const SETS: [&str; 4] = [
    "builtins.set",
    "builtins.frozenset",
    "collections.abc.Set",
    "collections.abc.MutableSet",
];
const MAPS: [&str; 3] = [
    "builtins.dict",
    "collections.abc.Mapping",
    "collections.abc.MutableMapping",
];

pub(super) fn bind_command(package: &str, module: Option<String>, directory: &str) -> ExitCode {
    if !is_import_name(package) {
        return usage(&format!(
            "`{package}` is not a Python import name; write identifiers separated by dots"
        ));
    }
    let module = module.unwrap_or_else(|| package.to_string());
    if !is_module_name(&module) {
        return usage(&format!(
            "`{module}` is not a module name; write lower-case segments separated by dots, or give one with `--module`"
        ));
    }
    let configured = match configured_interpreter(directory) {
        Ok(configured) => configured,
        Err(detail) => {
            eprintln!("renyi: {detail}");
            return ExitCode::FAILURE;
        }
    };
    let report = match report(package, directory, configured.as_deref()) {
        Ok(report) => report,
        Err(detail) => {
            eprintln!("renyi: {detail}");
            return ExitCode::FAILURE;
        }
    };
    let (declarations, entry) = match bind(package, &module, &report) {
        Ok(binding) => binding,
        Err(detail) => {
            eprintln!("renyi: {detail}");
            return ExitCode::FAILURE;
        }
    };
    write_module(
        directory,
        &module,
        &declarations,
        ManifestEntry::Python(entry),
    )
}

/// `{"analysis": {"package": "...", "symbols": {...}}}` as the manifest
/// spells it: the package when it differs from the module's name, the
/// symbols when there are any.
pub(super) fn entry_json(module: &str, entry: &PythonModule) -> String {
    let mut inner = Vec::new();
    if entry.package != module {
        inner.push(format!("\"package\": {:?}", entry.package));
    }
    if !entry.symbols.is_empty() {
        let symbols: Vec<String> = entry
            .symbols
            .iter()
            .map(|(renyi, python)| format!("{renyi:?}: {python:?}"))
            .collect();
        inner.push(format!("\"symbols\": {{{}}}", symbols.join(", ")));
    }
    format!("{{{module:?}: {{{}}}}}", inner.join(", "))
}

/// A Python import name: identifiers separated by dots.
fn is_import_name(name: &str) -> bool {
    name.split('.').all(|segment| {
        let mut chars = segment.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    })
}

/// The interpreter the manifest of the directory names, when it has one.
fn configured_interpreter(directory: &str) -> Result<Option<String>, String> {
    let path = Path::new(directory).join(MANIFEST_FILE);
    let Ok(source) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let manifest =
        Manifest::read(&source).map_err(|detail| format!("{}: {detail}", path.display()))?;
    Ok(manifest.python.interpreter)
}

/// The report of the inspection script, from the first interpreter of
/// decision AL3 that runs it; an import that fails in that interpreter is
/// the answer, not a reason to try the next.
fn report(package: &str, directory: &str, configured: Option<&str>) -> Result<Json, String> {
    let mut tried = Vec::new();
    for interpreter in interpreters(configured) {
        let output = match Command::new(&interpreter)
            .arg("-c")
            .arg(INSPECT)
            .arg(package)
            .arg(directory)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
        {
            Ok(output) => output,
            Err(error) => {
                tried.push(format!("`{interpreter}`: {error}"));
                continue;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let Some(line) = stdout
            .lines()
            .rev()
            .find(|line| line.trim_start().starts_with('{'))
        else {
            tried.push(format!(
                "`{interpreter}`: it ended without a report ({})",
                output.status
            ));
            continue;
        };
        let report = read_json(line.trim())
            .map_err(|(detail, _)| format!("`{interpreter}`: the report is not JSON: {detail}"))?;
        if let Json::Object(fields) = &report {
            if let Some(Json::Object(error)) = field(fields, "error") {
                let kind = text(error, "kind").unwrap_or("error");
                let message = text(error, "message").unwrap_or("");
                return Err(format!(
                    "`{interpreter}` cannot import `{package}`: {kind}: {message}"
                ));
            }
        }
        return Ok(report);
    }
    Err(format!(
        "no Python interpreter answered: {}",
        tried.join("; ")
    ))
}

/// One function of the package as the file will hold it.
enum Entry {
    Function {
        name: String,
        symbol: String,
        params: Vec<(String, String)>,
        returns: Option<String>,
        purpose: String,
        /// what stands as `JsonValue`, for the comment above the function
        notes: Vec<String>,
        json_value: bool,
    },
    Skipped {
        signature: String,
        reason: String,
    },
}

/// A record type of the file, from a class of the report (decision AW49).
struct Record {
    name: String,
    purpose: String,
    fields: Vec<(String, String)>,
    /// what stands as `JsonValue` among the fields
    notes: Vec<String>,
    json_value: bool,
}

/// What a class of the report became: a record type by its name, or
/// `JsonValue` for the reason.
type Outcome = (String, Result<String, String>);

/// A class as the report describes it.
struct Class<'a> {
    qualified: &'a str,
    short: &'a str,
    generic: bool,
    doc: Option<&'a str>,
    /// name, annotation tree, required, taken by the constructor
    fields: Vec<(&'a str, Option<&'a Json>, bool, bool)>,
}

fn class_of(json: &Json) -> Result<Class<'_>, String> {
    let fields = object(json, "a class of the report")?;
    let qualified = text(fields, "name").ok_or("a class of the report has no `name`")?;
    let short = text(fields, "short").unwrap_or(qualified);
    let generic = matches!(field(fields, "generic"), Some(Json::Boolean(true)));
    let doc = text(fields, "doc");
    let mut members = Vec::new();
    if let Some(Json::Array(items)) = field(fields, "fields") {
        for item in items {
            let item = object(item, "a field of the report")?;
            let name = text(item, "name").ok_or("a field of the report has no `name`")?;
            let annotation = field(item, "annotation").filter(|tree| !matches!(tree, Json::Null));
            let required = !matches!(field(item, "required"), Some(Json::Boolean(false)));
            let init = !matches!(field(item, "init"), Some(Json::Boolean(false)));
            members.push((name, annotation, required, init));
        }
    }
    Ok(Class {
        qualified,
        short,
        generic,
        doc,
        fields: members,
    })
}

/// A Renyi type name: PascalCase, letters and digits, not taken.
fn is_type_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_uppercase())
        && chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// A Python field name that is a Renyi value name as it is: the JSON key
/// must stay the Python name, so a name the binder would respell is out.
fn is_field_name(name: &str) -> bool {
    name.chars().count() >= 2 && renyi_name(name, PREFIX) == name
}

/// The classes of the report an annotation tree names.
fn classes_in<'a>(tree: Option<&'a Json>, out: &mut Vec<&'a str>) {
    let Some(Json::Object(fields)) = tree else {
        return;
    };
    if let Some(class) = text(fields, "class") {
        out.push(class);
    }
    for key in ["args", "union"] {
        if let Some(Json::Array(items)) = field(fields, key) {
            for item in items {
                classes_in(Some(item), out);
            }
        }
    }
}

/// The classes of the report settled into record types, each after the
/// classes its fields name (decision AW49): a generic class, one without
/// fields, one with a field the constructor does not take, a field that is
/// not a Renyi name, a bare name that is not a type name or is taken, and
/// a class that names itself through its fields stay `JsonValue`.
fn settle_records(classes: &[Json]) -> Result<(Vec<Record>, Vec<Outcome>), String> {
    let classes: Vec<Class> = classes.iter().map(class_of).collect::<Result<_, _>>()?;
    let mut records = Vec::new();
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut settling: Vec<&str> = Vec::new();
    for index in 0..classes.len() {
        settle(index, &classes, &mut records, &mut outcomes, &mut settling);
    }
    Ok((records, outcomes))
}

fn settle<'a>(
    index: usize,
    classes: &[Class<'a>],
    records: &mut Vec<Record>,
    outcomes: &mut Vec<Outcome>,
    settling: &mut Vec<&'a str>,
) {
    let class = &classes[index];
    if outcomes.iter().any(|(name, _)| name == class.qualified) {
        return;
    }
    settling.push(class.qualified);
    let outcome = settle_class(class, classes, records, outcomes, settling);
    settling.pop();
    if let Ok(record) = &outcome {
        records.push(Record {
            name: record.name.clone(),
            purpose: record.purpose.clone(),
            fields: record.fields.clone(),
            notes: record.notes.clone(),
            json_value: record.json_value,
        });
    }
    let outcome = outcome.map(|record| record.name);
    outcomes.push((class.qualified.to_string(), outcome));
}

fn settle_class<'a>(
    class: &Class<'a>,
    classes: &[Class<'a>],
    records: &mut Vec<Record>,
    outcomes: &mut Vec<Outcome>,
    settling: &mut Vec<&'a str>,
) -> Result<Record, String> {
    if class.generic {
        return Err("a generic class".to_string());
    }
    if class.fields.is_empty() {
        return Err("no fields".to_string());
    }
    if !is_type_name(class.short) {
        return Err(format!("`{}` is not a type name", class.short));
    }
    if TAKEN_TYPES.contains(&class.short)
        || outcomes
            .iter()
            .any(|(_, outcome)| outcome.as_deref() == Ok(class.short))
    {
        return Err(format!("the name `{}` is taken", class.short));
    }
    for (name, _, _, init) in &class.fields {
        if !init {
            return Err(format!("the constructor does not take `{name}`"));
        }
        if !is_field_name(name) {
            return Err(format!("`{name}` is not a Renyi field name"));
        }
    }
    // the classes the fields name come first; one still settling is this
    // class through its own fields
    let mut named = Vec::new();
    for (_, annotation, _, _) in &class.fields {
        classes_in(*annotation, &mut named);
    }
    for name in named {
        if settling.contains(&name) {
            return Err("a class that names itself".to_string());
        }
        if let Some(position) = classes.iter().position(|other| other.qualified == name) {
            settle(position, classes, records, outcomes, settling);
        }
    }
    let mut mapper = Mapper::new(outcomes);
    let mut fields = Vec::new();
    for (name, annotation, required, _) in &class.fields {
        let mut ty = mapper.map(*annotation, &format!("`{name}`"));
        if !required && !ty.starts_with("maybe ") {
            ty = format!("maybe {ty}");
        }
        fields.push((name.to_string(), ty));
    }
    let purpose = match class.doc {
        Some(doc) => purpose_of(Some(doc), class.short),
        None => format!(
            "The Python class `{}`.",
            fit(class.qualified, PURPOSE_ROOM - 20)
        ),
    };
    Ok(Record {
        name: class.short.to_string(),
        purpose,
        fields,
        notes: mapper.notes,
        json_value: mapper.json_value,
    })
}

/// The declaration file and the manifest entry for a report.
pub(super) fn bind(
    package: &str,
    module: &str,
    report: &Json,
) -> Result<(String, PythonModule), String> {
    let fields = object(report, "the report")?;
    let Some(Json::Array(functions)) = field(fields, "functions") else {
        return Err("the report has no `functions`".to_string());
    };
    let classes: &[Json] = match field(fields, "classes") {
        Some(Json::Array(classes)) => classes,
        _ => &[],
    };
    let (records, outcomes) = settle_records(classes)?;
    let tail = [
        "or fails with PythonError".to_string(),
        format!("needs python({package:?})"),
    ];
    let tail: Vec<&str> = tail.iter().map(String::as_str).collect();
    let mut taken: Vec<String> = Vec::new();
    let mut symbols: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut declared: Vec<String> = Vec::new();
    let mut imports_json = records.iter().any(|record| record.json_value);
    for function in functions {
        match entry_of(function, &mut taken, &outcomes)? {
            Entry::Skipped { signature, reason } => {
                skipped.push(format!("# skipped: {signature} ({reason})\n"));
            }
            Entry::Function {
                name,
                symbol,
                params,
                returns,
                purpose,
                notes,
                json_value,
            } => {
                if name != symbol {
                    symbols.push((name.clone(), symbol));
                }
                imports_json |= json_value;
                let mut text = String::new();
                if !notes.is_empty() {
                    text.push_str(&format!("# as JsonValue: {}\n", notes.join(", ")));
                }
                text.push_str(&declaration(&name, &params, returns.as_deref(), &tail));
                text.push_str(&format!("  purpose: {purpose}\n"));
                declared.push(text);
            }
        }
    }
    symbols.sort();
    let mut text = format!(
        "module {module}\n  purpose: Bindings generated by `renyi bind --python` from {package}.\n\nimport std.python exposing PythonError\n"
    );
    if imports_json {
        text.push_str("import std.json exposing JsonValue\n");
    }
    if !skipped.is_empty() {
        text.push('\n');
        for line in &skipped {
            text.push_str(line);
        }
    }
    for record in &records {
        text.push('\n');
        if !record.notes.is_empty() {
            text.push_str(&format!("# as JsonValue: {}\n", record.notes.join(", ")));
        }
        text.push_str(&format!(
            "public type {}\n  purpose: {}\n",
            record.name, record.purpose
        ));
        for (name, ty) in &record.fields {
            text.push_str(&format!("  has {name}: {ty}\n"));
        }
        text.push_str("  can ToJson\n  can FromJson\nend\n");
    }
    for function in &declared {
        text.push('\n');
        text.push_str(function);
    }
    Ok((
        text,
        PythonModule {
            package: package.to_string(),
            symbols,
            deadline: None,
        },
    ))
}

/// A function of the report as an entry: the positional parameters in
/// order, each required; `*args` and `**kwargs` left out; a keyword-only
/// parameter with a default left out, one without a default skipping the
/// function (decision AM1).
fn entry_of(
    function: &Json,
    taken: &mut Vec<String>,
    records: &[Outcome],
) -> Result<Entry, String> {
    let fields = object(function, "a function of the report")?;
    let python_name = text(fields, "name").ok_or("a function of the report has no `name`")?;
    if let Some(reason) = text(fields, "skipped") {
        return Ok(Entry::Skipped {
            signature: python_name.to_string(),
            reason: reason.to_string(),
        });
    }
    let signature = text(fields, "signature").unwrap_or(python_name);
    let Some(Json::Array(parameters)) = field(fields, "parameters") else {
        return Err(format!("`{python_name}` in the report has no `parameters`"));
    };
    let mut mapper = Mapper::new(records);
    let mut params: Vec<(String, String)> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for parameter in parameters {
        let parameter = object(parameter, "a parameter of the report")?;
        let name = text(parameter, "name").ok_or("a parameter of the report has no `name`")?;
        let has_default = matches!(field(parameter, "default"), Some(Json::Boolean(true)));
        match text(parameter, "kind").unwrap_or("positional") {
            "positional" => {}
            "keyword" if has_default => continue,
            "keyword" => {
                return Ok(Entry::Skipped {
                    signature: signature.to_string(),
                    reason: format!("a keyword-only parameter without a default: `{name}`"),
                });
            }
            _ => continue,
        }
        let ty = mapper.map(field(parameter, "annotation"), &format!("`{name}`"));
        // a single letter is not a Renyi name (`single-letter-identifier`)
        let mut renyi = renyi_name(name, PREFIX);
        if renyi.chars().count() < 2 {
            renyi = format!("argument_{}", params.len() + 1);
        }
        while names.contains(&renyi) {
            renyi.push('_');
        }
        names.push(renyi.clone());
        params.push((renyi, ty));
    }
    let returns = match field(fields, "returns") {
        Some(Json::Object(tree)) if is_none(tree) => None,
        tree => Some(mapper.map(tree, "the result")),
    };
    let mut name = renyi_name(python_name, PREFIX);
    while taken.contains(&name) {
        name.push('_');
    }
    taken.push(name.clone());
    Ok(Entry::Function {
        name,
        symbol: python_name.to_string(),
        params,
        returns,
        purpose: purpose_of(text(fields, "doc"), signature),
        notes: mapper.notes,
        json_value: mapper.json_value,
    })
}

/// The purpose line: the docstring's first line as a sentence, else the
/// signature as Python prints it; cut to the line width.
fn purpose_of(doc: Option<&str>, signature: &str) -> String {
    let first = doc.and_then(|doc| doc.lines().map(str::trim).find(|line| !line.is_empty()));
    match first {
        Some(line) => {
            let mut text = fit(line, PURPOSE_ROOM - 1);
            if !text.ends_with(['.', '!', '?', ':']) {
                text.push('.');
            }
            text
        }
        None => format!("`{}`.", fit(signature, PURPOSE_ROOM - 3)),
    }
}

/// The text within `room` characters, `...` marking a cut.
fn fit(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_string();
    }
    let cut: String = text.chars().take(room - 3).collect();
    format!("{}...", cut.trim_end())
}

/// The mapping of decision AM2: an annotation tree to the Renyi type the
/// bridge carries, `JsonValue` where the mapping does not reach, with a
/// note of what stands as `JsonValue`; a class settled as a record type
/// (decision AW49) is that type, one that was not carries its reason in
/// the note.
struct Mapper<'a> {
    records: &'a [Outcome],
    json_value: bool,
    notes: Vec<String>,
}

impl<'a> Mapper<'a> {
    fn new(records: &'a [Outcome]) -> Mapper<'a> {
        Mapper {
            records,
            json_value: false,
            notes: Vec::new(),
        }
    }

    fn map(&mut self, tree: Option<&Json>, what: &str) -> String {
        let Some(tree) = tree.filter(|tree| !matches!(tree, Json::Null)) else {
            return self.fallback(what, "no annotation");
        };
        match self.known(tree, what) {
            Some(ty) => ty,
            None => {
                let mut shown = shown(tree);
                if let Some(reason) = self.reason(tree) {
                    shown = format!("{shown}: {reason}");
                }
                self.fallback(what, &shown)
            }
        }
    }

    /// Why a class of the report is not a record type, when it is one.
    fn reason(&self, tree: &Json) -> Option<&str> {
        let Json::Object(fields) = tree else {
            return None;
        };
        let class = text(fields, "class")?;
        self.records
            .iter()
            .find(|(name, _)| name == class)
            .and_then(|(_, outcome)| outcome.as_ref().err().map(String::as_str))
    }

    fn fallback(&mut self, what: &str, shown: &str) -> String {
        self.json_value = true;
        self.notes.push(format!("{what} ({shown})"));
        "JsonValue".to_string()
    }

    fn any(&mut self, shape: &str) -> Option<String> {
        self.json_value = true;
        Some(format!("{shape} JsonValue"))
    }

    /// The type of a tree the mapping reaches; `None` for the rest.
    fn known(&mut self, tree: &Json, what: &str) -> Option<String> {
        let Json::Object(fields) = tree else {
            return None;
        };
        if let Some(class) = text(fields, "class") {
            return match class {
                "builtins.int" => Some("Integer".to_string()),
                "builtins.float" => Some("Float".to_string()),
                "builtins.str" => Some("Text".to_string()),
                "builtins.bool" => Some("Boolean".to_string()),
                "builtins.bytes" => Some("Bytes".to_string()),
                "builtins.list" => self.any("List of"),
                "builtins.set" | "builtins.frozenset" => self.any("Set of"),
                "builtins.dict" => self.any("Map of Text to"),
                _ => self
                    .records
                    .iter()
                    .find(|(name, _)| name == class)
                    .and_then(|(_, outcome)| outcome.as_ref().ok().cloned()),
            };
        }
        if let Some(Json::Array(members)) = field(fields, "union") {
            let (nones, others): (Vec<&Json>, Vec<&Json>) = members
                .iter()
                .partition(|member| matches!(member, Json::Object(fields) if is_none(fields)));
            if others.len() == 1 && !nones.is_empty() {
                return Some(format!("maybe {}", self.map(Some(others[0]), what)));
            }
            return None;
        }
        if let (Some(origin), Some(Json::Array(args))) =
            (text(fields, "generic"), field(fields, "args"))
        {
            if origin == "typing.Annotated" {
                return args.first().map(|first| self.map(Some(first), what));
            }
            if LISTS.contains(&origin) && args.len() == 1 {
                return Some(format!("List of {}", self.map(Some(&args[0]), what)));
            }
            if SETS.contains(&origin) && args.len() == 1 {
                return Some(format!("Set of {}", self.map(Some(&args[0]), what)));
            }
            if MAPS.contains(&origin) && args.len() == 2 && is_class(&args[0], "builtins.str") {
                return Some(format!("Map of Text to {}", self.map(Some(&args[1]), what)));
            }
        }
        None
    }
}

/// An annotation tree as Python would write it, for a note.
fn shown(tree: &Json) -> String {
    let Json::Object(fields) = tree else {
        return "?".to_string();
    };
    if let Some(class) = text(fields, "class") {
        return short(class);
    }
    if let Some(other) = text(fields, "other") {
        return other.to_string();
    }
    if is_none(fields) {
        return "None".to_string();
    }
    if let Some(Json::Array(members)) = field(fields, "union") {
        let members: Vec<String> = members.iter().map(shown).collect();
        return members.join(" | ");
    }
    if let (Some(origin), Some(Json::Array(args))) =
        (text(fields, "generic"), field(fields, "args"))
    {
        let args: Vec<String> = args.iter().map(shown).collect();
        return format!("{}[{}]", short(origin), args.join(", "));
    }
    "?".to_string()
}

/// A qualified name without the module Python leaves out too.
fn short(qualified: &str) -> String {
    ["builtins.", "typing.", "collections.abc."]
        .iter()
        .find_map(|prefix| qualified.strip_prefix(prefix))
        .unwrap_or(qualified)
        .to_string()
}

fn is_none(fields: &[(String, Json)]) -> bool {
    matches!(field(fields, "none"), Some(Json::Boolean(true)))
}

fn is_class(tree: &Json, name: &str) -> bool {
    matches!(tree, Json::Object(fields) if text(fields, "class") == Some(name))
}

fn object<'a>(json: &'a Json, what: &str) -> Result<&'a [(String, Json)], String> {
    match json {
        Json::Object(fields) => Ok(fields),
        other => Err(format!("{what} is not an object but {}", other.kind())),
    }
}

fn field<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a str> {
    match field(fields, name) {
        Some(Json::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(functions: &str) -> Json {
        read_json(&format!(
            "{{\"version\": \"3.13.0\", \"file\": \"geometry.py\", \"functions\": [{functions}]}}"
        ))
        .unwrap_or_else(|(detail, _)| panic!("{detail}"))
    }

    const AREA: &str = "{\"name\": \"area\", \"signature\": \"area(width: float, height: float = 1.0) -> float\", \"parameters\": [{\"name\": \"width\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"builtins.float\"}}, {\"name\": \"height\", \"kind\": \"positional\", \"default\": true, \"annotation\": {\"class\": \"builtins.float\"}}], \"returns\": {\"class\": \"builtins.float\"}, \"doc\": \"The area of a rectangle\"}";

    #[test]
    fn a_report_becomes_declarations_and_an_entry() {
        let lookup = "{\"name\": \"lookup\", \"signature\": \"lookup(table: dict[str, int], key: str) -> Optional[int]\", \"parameters\": [{\"name\": \"table\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"generic\": \"builtins.dict\", \"args\": [{\"class\": \"builtins.str\"}, {\"class\": \"builtins.int\"}]}}, {\"name\": \"key\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"builtins.str\"}}], \"returns\": {\"union\": [{\"class\": \"builtins.int\"}, {\"none\": true}]}, \"doc\": null}";
        let count = "{\"name\": \"count\", \"signature\": \"count(items: list) -> int\", \"parameters\": [{\"name\": \"items\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"builtins.list\"}}], \"returns\": {\"class\": \"builtins.int\"}, \"doc\": null}";
        let untyped = "{\"name\": \"untyped\", \"signature\": \"untyped(value, x, flag=False, *rest, **options)\", \"parameters\": [{\"name\": \"value\", \"kind\": \"positional\", \"default\": false, \"annotation\": null}, {\"name\": \"x\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"shapes.Point\"}}, {\"name\": \"flag\", \"kind\": \"positional\", \"default\": true, \"annotation\": {\"class\": \"builtins.bool\"}}, {\"name\": \"rest\", \"kind\": \"var_positional\", \"default\": false, \"annotation\": null}, {\"name\": \"options\", \"kind\": \"var_keyword\", \"default\": false, \"annotation\": null}], \"returns\": null, \"doc\": null}";
        let greet = "{\"name\": \"greet\", \"signature\": \"greet(names: Sequence[str], *, loud: bool = False) -> None\", \"parameters\": [{\"name\": \"names\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"generic\": \"collections.abc.Sequence\", \"args\": [{\"class\": \"builtins.str\"}]}}, {\"name\": \"loud\", \"kind\": \"keyword\", \"default\": true, \"annotation\": {\"class\": \"builtins.bool\"}}], \"returns\": {\"none\": true}, \"doc\": \"Greet everyone.\\n\\nLoudly when asked.\"}";
        let weird = "{\"name\": \"weird\", \"signature\": \"weird(*, key: str) -> str\", \"parameters\": [{\"name\": \"key\", \"kind\": \"keyword\", \"default\": false, \"annotation\": {\"class\": \"builtins.str\"}}], \"returns\": {\"class\": \"builtins.str\"}, \"doc\": null}";
        let len = "{\"name\": \"len\", \"skipped\": \"no signature: no signature found for builtin <built-in function len>\"}";
        let (text, entry) = bind(
            "geometry",
            "geometry",
            &report(&[AREA, lookup, count, untyped, greet, weird, len].join(", ")),
        )
        .unwrap_or_else(|detail| panic!("{detail}"));
        assert_eq!(
            text,
            "module geometry
  purpose: Bindings generated by `renyi bind --python` from geometry.

import std.python exposing PythonError
import std.json exposing JsonValue

# skipped: weird(*, key: str) -> str (a keyword-only parameter without a default: `key`)
# skipped: len (no signature: no signature found for builtin <built-in function len>)

public function area(width: Float, height: Float)
  returns Float
  or fails with PythonError
  needs python(\"geometry\")
  purpose: The area of a rectangle.

public function lookup(table: Map of Text to Integer, key: Text)
  returns maybe Integer
  or fails with PythonError
  needs python(\"geometry\")
  purpose: `lookup(table: dict[str, int], key: str) -> Optional[int]`.

public function py_count(items: List of JsonValue)
  returns Integer
  or fails with PythonError
  needs python(\"geometry\")
  purpose: `count(items: list) -> int`.

# as JsonValue: `value` (no annotation), `x` (shapes.Point), the result (no annotation)
public function untyped(value: JsonValue, argument_2: JsonValue, flag: Boolean)
  returns JsonValue
  or fails with PythonError
  needs python(\"geometry\")
  purpose: `untyped(value, x, flag=False, *rest, **options)`.

public function greet(names: List of Text) or fails with PythonError needs python(\"geometry\")
  purpose: Greet everyone.
"
        );
        assert_eq!(entry.package, "geometry");
        assert_eq!(
            entry.symbols,
            vec![("py_count".to_string(), "count".to_string())]
        );
        assert_eq!(
            entry_json("geometry", &entry),
            "{\"geometry\": {\"symbols\": {\"py_count\": \"count\"}}}"
        );
        assert_eq!(
            entry_json(
                "stats",
                &PythonModule {
                    package: "scipy.stats".to_string(),
                    symbols: Vec::new(),
                    deadline: None
                }
            ),
            "{\"stats\": {\"package\": \"scipy.stats\"}}"
        );
    }

    #[test]
    fn the_types_the_bridge_carries_map_and_the_rest_is_json_value() {
        let cases = [
            ("{\"class\": \"builtins.bytes\"}", "Bytes", ""),
            (
                "{\"generic\": \"builtins.list\", \"args\": [{\"generic\": \"builtins.set\", \"args\": [{\"class\": \"builtins.str\"}]}]}",
                "List of Set of Text",
                "",
            ),
            (
                "{\"union\": [{\"none\": true}, {\"class\": \"builtins.float\"}]}",
                "maybe Float",
                "",
            ),
            (
                "{\"union\": [{\"class\": \"builtins.int\"}, {\"class\": \"builtins.str\"}]}",
                "JsonValue",
                "`x` (int | str)",
            ),
            (
                "{\"generic\": \"builtins.dict\", \"args\": [{\"class\": \"builtins.int\"}, {\"class\": \"builtins.str\"}]}",
                "JsonValue",
                "`x` (dict[int, str])",
            ),
            (
                "{\"generic\": \"builtins.list\", \"args\": [{\"class\": \"typing.Any\"}]}",
                "List of JsonValue",
                "`x` (Any)",
            ),
            (
                "{\"union\": [{\"class\": \"shapes.Point\"}, {\"none\": true}]}",
                "maybe JsonValue",
                "`x` (shapes.Point)",
            ),
            (
                "{\"generic\": \"typing.Annotated\", \"args\": [{\"class\": \"builtins.int\"}, {\"other\": \"'positive'\"}]}",
                "Integer",
                "",
            ),
            ("{\"other\": \"Vector\"}", "JsonValue", "`x` (Vector)"),
            ("{\"none\": true}", "JsonValue", "`x` (None)"),
            ("null", "JsonValue", "`x` (no annotation)"),
        ];
        for (tree, expected, note) in cases {
            let tree = read_json(tree).unwrap_or_else(|(detail, _)| panic!("{detail}"));
            let mut mapper = Mapper::new(&[]);
            assert_eq!(mapper.map(Some(&tree), "`x`"), expected, "{tree:?}");
            assert_eq!(mapper.notes.join(", "), note, "{tree:?}");
        }
    }

    #[test]
    fn the_classes_with_fields_become_record_types() {
        // decision AW49: a dataclass, a NamedTuple and a TypedDict as
        // records, the classes a field names first; the rest stays
        // JsonValue with the reason in the note
        let point = "{\"name\": \"shapes.Point\", \"short\": \"Point\", \"kind\": \"dataclass\", \"generic\": false, \"doc\": \"A point on the grid\", \"fields\": [{\"name\": \"across\", \"annotation\": {\"class\": \"builtins.int\"}, \"required\": true, \"init\": true}, {\"name\": \"down\", \"annotation\": {\"class\": \"builtins.int\"}, \"required\": true, \"init\": true}]}";
        let size = "{\"name\": \"shapes.Size\", \"short\": \"Size\", \"kind\": \"namedtuple\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"width\", \"annotation\": {\"class\": \"builtins.float\"}, \"required\": true, \"init\": true}, {\"name\": \"height\", \"annotation\": {\"class\": \"builtins.float\"}, \"required\": true, \"init\": true}]}";
        let options = "{\"name\": \"shapes.Options\", \"short\": \"Options\", \"kind\": \"typeddict\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"label\", \"annotation\": {\"class\": \"builtins.str\"}, \"required\": false, \"init\": true}, {\"name\": \"extra\", \"annotation\": {\"class\": \"typing.Any\"}, \"required\": true, \"init\": true}]}";
        // a box names a point before the point is settled
        let boxed = "{\"name\": \"shapes.Box\", \"short\": \"Box\", \"kind\": \"dataclass\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"corner\", \"annotation\": {\"class\": \"shapes.Point\"}, \"required\": true, \"init\": true}, {\"name\": \"sizes\", \"annotation\": {\"generic\": \"builtins.list\", \"args\": [{\"class\": \"shapes.Size\"}]}, \"required\": true, \"init\": true}]}";
        let node = "{\"name\": \"shapes.Node\", \"short\": \"Node\", \"kind\": \"dataclass\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"next\", \"annotation\": {\"union\": [{\"class\": \"shapes.Node\"}, {\"none\": true}]}, \"required\": true, \"init\": true}]}";
        let xy = "{\"name\": \"shapes.XY\", \"short\": \"XY\", \"kind\": \"namedtuple\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"x\", \"annotation\": {\"class\": \"builtins.int\"}, \"required\": true, \"init\": true}]}";
        let text_class = "{\"name\": \"shapes.Text\", \"short\": \"Text\", \"kind\": \"dataclass\", \"generic\": false, \"doc\": null, \"fields\": [{\"name\": \"value\", \"annotation\": {\"class\": \"builtins.str\"}, \"required\": true, \"init\": true}]}";
        let generic = "{\"name\": \"shapes.Cell\", \"short\": \"Cell\", \"kind\": \"dataclass\", \"generic\": true, \"doc\": null, \"fields\": [{\"name\": \"value\", \"annotation\": {\"other\": \"~T\"}, \"required\": true, \"init\": true}]}";
        let shift = "{\"name\": \"shift\", \"signature\": \"shift(point: Point, by: int) -> Point\", \"parameters\": [{\"name\": \"point\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"shapes.Point\"}}, {\"name\": \"by\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"builtins.int\"}}], \"returns\": {\"class\": \"shapes.Point\"}, \"doc\": null}";
        let odd = "{\"name\": \"odd\", \"signature\": \"odd(a: XY, b: Node, c: Text, d: Cell[int]) -> Box\", \"parameters\": [{\"name\": \"a\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"shapes.XY\"}}, {\"name\": \"b\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"shapes.Node\"}}, {\"name\": \"c\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"class\": \"shapes.Text\"}}, {\"name\": \"d\", \"kind\": \"positional\", \"default\": false, \"annotation\": {\"generic\": \"shapes.Cell\", \"args\": [{\"class\": \"builtins.int\"}]}}], \"returns\": {\"class\": \"shapes.Box\"}, \"doc\": null}";
        let report = read_json(&format!(
            "{{\"version\": \"3.13.0\", \"file\": \"shapes.py\", \"functions\": [{shift}, {odd}], \"classes\": [{boxed}, {point}, {size}, {options}, {node}, {xy}, {text_class}, {generic}]}}"
        ))
        .unwrap_or_else(|(detail, _)| panic!("{detail}"));
        let (text, _) =
            bind("shapes", "shapes", &report).unwrap_or_else(|detail| panic!("{detail}"));
        assert_eq!(
            text,
            "module shapes
  purpose: Bindings generated by `renyi bind --python` from shapes.

import std.python exposing PythonError
import std.json exposing JsonValue

public type Point
  purpose: A point on the grid.
  has across: Integer
  has down: Integer
  can ToJson
  can FromJson
end

public type Size
  purpose: The Python class `shapes.Size`.
  has width: Float
  has height: Float
  can ToJson
  can FromJson
end

public type Box
  purpose: The Python class `shapes.Box`.
  has corner: Point
  has sizes: List of Size
  can ToJson
  can FromJson
end

# as JsonValue: `extra` (Any)
public type Options
  purpose: The Python class `shapes.Options`.
  has label: maybe Text
  has extra: JsonValue
  can ToJson
  can FromJson
end

public function shift(point: Point, py_by: Integer)
  returns Point
  or fails with PythonError
  needs python(\"shapes\")
  purpose: `shift(point: Point, by: int) -> Point`.

# as JsonValue: `a` (shapes.XY: `x` is not a Renyi field name), `b` (shapes.Node: a class that names itself), `c` (shapes.Text: the name `Text` is taken), `d` (shapes.Cell[int])
public function odd(argument_1: JsonValue, argument_2: JsonValue, argument_3: JsonValue, argument_4: JsonValue)
  returns Box
  or fails with PythonError
  needs python(\"shapes\")
  purpose: `odd(a: XY, b: Node, c: Text, d: Cell[int]) -> Box`.
"
        );
    }

    #[test]
    fn purposes_are_sentences_within_the_line() {
        assert_eq!(purpose_of(Some("The mean"), "mean(values)"), "The mean.");
        assert_eq!(purpose_of(Some("  Is it?  "), "x()"), "Is it?");
        assert_eq!(
            purpose_of(None, "mean(values: list[float]) -> float"),
            "`mean(values: list[float]) -> float`."
        );
        let long = "a".repeat(120);
        let purpose = purpose_of(Some(&long), "x()");
        assert_eq!(purpose.len(), PURPOSE_ROOM - 1);
        assert!(purpose.ends_with("..."), "{purpose}");
        let purpose = purpose_of(None, &long);
        assert_eq!(purpose.len(), PURPOSE_ROOM);
        assert!(purpose.ends_with("...`."), "{purpose}");
        assert!(is_import_name("scipy.stats") && is_import_name("_private"));
        assert!(!is_import_name("my-package") && !is_import_name("a..b"));
    }
}
