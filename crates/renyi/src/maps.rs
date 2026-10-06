//! The base of `renyi index --diff` and of the `diff` tool of `renyi mcp`:
//! a map saved by `renyi index --json`, or the project as it was at a git
//! revision, indexed from the files `git show` gives.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use renyi_index::{Definition, Header, Implements, Index, Kind, Metrics, Module};
use renyi_syntax::{parse, SourceFile};
use renyi_vm::natives::json::{read_json, Json};

/// The map named by the argument: a file holding a saved map, else a git
/// revision of the project at `path`.
pub(crate) fn load_base(base: &str, path: &Path, toolchain: &str) -> Result<Index, String> {
    if Path::new(base).is_file() {
        let text = std::fs::read_to_string(base)
            .map_err(|error| format!("cannot read {base}: {error}"))?;
        return index_from_json(&text).map_err(|detail| format!("{base}: {detail}"));
    }
    index_at_revision(path, base, toolchain)
}

// --------------------------------------------------------------- saved map

/// A map as `renyi index --json` printed it. A field a newer toolchain adds
/// (such as `text_hash`) is empty when the file lacks it.
pub(crate) fn index_from_json(text: &str) -> Result<Index, String> {
    let json = read_json(text).map_err(|(detail, line)| format!("line {line}: {detail}"))?;
    let header = Header {
        project: text_field(&json, "project")?,
        revision: text_field(&json, "revision")?,
        toolchain: text_field(&json, "toolchain")?,
    };
    let modules = array_field(&json, "modules")?
        .iter()
        .map(module_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let definitions = array_field(&json, "definitions")?
        .iter()
        .map(definition_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Index {
        header,
        modules,
        definitions,
    })
}

fn module_from_json(json: &Json) -> Result<Module, String> {
    Ok(Module {
        name: text_field(json, "name")?,
        file: text_field(json, "file")?,
        purpose: optional_text_field(json, "purpose")?,
        imports: texts_field(json, "imports")?,
        definitions: number_field(json, "definitions")?,
        public: number_field(json, "public")?,
        lines: number_field(json, "lines")?,
        effects: texts_field(json, "effects")?,
        ids: texts_field(json, "ids")?,
        errors: number_field(json, "errors")?,
        canonical: boolean_field(json, "canonical")?,
    })
}

fn definition_from_json(json: &Json) -> Result<Definition, String> {
    let kind_name = text_field(json, "kind")?;
    let kind = Kind::parse(&kind_name).ok_or_else(|| format!("unknown kind `{kind_name}`"))?;
    let effects = object_field(json, "effects")?;
    let fails = object_field(json, "fails")?;
    let metrics = object_field(json, "metrics")?;
    let coverage = object_field(json, "coverage")?;
    let location = object_field(json, "location")?;
    let implements = match field(json, "implements") {
        None | Some(Json::Null) => None,
        Some(object) => Some(Implements {
            ability: text_field(object, "ability")?,
            target: text_field(object, "target")?,
        }),
    };
    Ok(Definition {
        id: text_field(json, "id")?,
        text_hash: optional_text_field(json, "text_hash")?.unwrap_or_default(),
        module: text_field(json, "module")?,
        name: text_field(json, "name")?,
        kind,
        public: boolean_field(json, "public")?,
        signature: text_field(json, "signature")?,
        purpose: optional_text_field(json, "purpose")?,
        tags: texts_field(json, "tags")?,
        see_also: texts_field(json, "see_also")?,
        exposed_as_tool: boolean_field(json, "exposed_as_tool")?,
        effects_declared: texts_field(effects, "declared")?,
        effects_transitive: texts_field(effects, "transitive")?,
        fails_declared: texts_field(fails, "declared")?,
        fails_transitive: texts_field(fails, "transitive")?,
        calls: texts_field(json, "calls")?,
        uses: texts_field(json, "uses")?,
        implements,
        tested_by: texts_field(json, "tested_by")?,
        metrics: Metrics {
            lines: number_field(metrics, "lines")?,
            depth: number_field(metrics, "depth")?,
            branches: number_field(metrics, "branches")?,
            effects: number_field(metrics, "effects")?,
            fan_in: number_field(metrics, "fan_in")?,
            fan_out: number_field(metrics, "fan_out")?,
            library_calls: number_field(metrics, "library_calls")?,
        },
        examples: number_field(coverage, "examples")?,
        tests: number_field(coverage, "tests")?,
        file: text_field(location, "file")?,
        line: number_field(location, "line")?,
        end_line: number_field(location, "end_line")?,
    })
}

fn field<'a>(json: &'a Json, key: &str) -> Option<&'a Json> {
    match json {
        Json::Object(fields) => fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

fn required<'a>(json: &'a Json, key: &str) -> Result<&'a Json, String> {
    field(json, key).ok_or_else(|| format!("the map lacks `{key}`"))
}

fn text_field(json: &Json, key: &str) -> Result<String, String> {
    match required(json, key)? {
        Json::Text(text) => Ok(text.clone()),
        _ => Err(format!("`{key}` is not a string")),
    }
}

fn optional_text_field(json: &Json, key: &str) -> Result<Option<String>, String> {
    match field(json, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Text(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("`{key}` is not a string")),
    }
}

fn texts_field(json: &Json, key: &str) -> Result<Vec<String>, String> {
    array_field(json, key)?
        .iter()
        .map(|item| match item {
            Json::Text(text) => Ok(text.clone()),
            _ => Err(format!("`{key}` holds something that is not a string")),
        })
        .collect()
}

fn array_field<'a>(json: &'a Json, key: &str) -> Result<&'a [Json], String> {
    match required(json, key)? {
        Json::Array(items) => Ok(items),
        _ => Err(format!("`{key}` is not a list")),
    }
}

fn object_field<'a>(json: &'a Json, key: &str) -> Result<&'a Json, String> {
    match required(json, key)? {
        object @ Json::Object(_) => Ok(object),
        _ => Err(format!("`{key}` is not an object")),
    }
}

fn number_field(json: &Json, key: &str) -> Result<usize, String> {
    match required(json, key)? {
        Json::Number(text) => text
            .parse()
            .map_err(|_| format!("`{key}` is not a whole number")),
        _ => Err(format!("`{key}` is not a number")),
    }
}

fn boolean_field(json: &Json, key: &str) -> Result<bool, String> {
    match required(json, key)? {
        Json::Boolean(value) => Ok(*value),
        _ => Err(format!("`{key}` is not true or false")),
    }
}

// --------------------------------------------------------------- revision

/// The project at a git revision: every `.ry` file under the directory, or
/// the file with its imports, read with `git show`, named as the working
/// tree names them.
fn index_at_revision(path: &Path, revision: &str, toolchain: &str) -> Result<Index, String> {
    let directory = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(".").to_path_buf())
    };
    if git(
        &directory,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{revision}^{{commit}}"),
        ],
    )
    .is_err()
    {
        return Err(format!(
            "`{revision}` is neither a map file nor a git revision of {}",
            directory.display()
        ));
    }
    let files = if path.is_dir() {
        let listed = git(
            &directory,
            &["ls-tree", "-r", "--name-only", revision, "--", "."],
        )?;
        let mut files = Vec::new();
        for relative in listed.lines().filter(|line| line.ends_with(".ry")) {
            let text = git(&directory, &["show", &format!("{revision}:./{relative}")])?;
            files.push(SourceFile::new(display(&directory.join(relative)), text));
        }
        files
    } else {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .ok_or_else(|| format!("{} is not a file", path.display()))?;
        let text = git(&directory, &["show", &format!("{revision}:./{name}")])?;
        let main = SourceFile::new(display(path), text);
        let mut files = vec![main];
        // the imports, as `renyi_check::imported_files` reads them from the
        // directory, but at the revision
        let mut queue: Vec<Vec<String>> = imports_of(&files[0].text);
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(import) = queue.pop() {
            if import.first().map(String::as_str) == Some("std") || !seen.insert(import.join(".")) {
                continue;
            }
            let relative = format!("{}.ry", import.join("/"));
            let Ok(text) = git(&directory, &["show", &format!("{revision}:./{relative}")]) else {
                continue; // the resolver reports the unknown module
            };
            queue.extend(imports_of(&text));
            files.push(SourceFile::new(display(&directory.join(relative)), text));
        }
        files
    };
    let header = Header {
        project: renyi_index::project_name(path),
        revision: revision.to_string(),
        toolchain: toolchain.to_string(),
    };
    Ok(renyi_index::index_files(&files, header))
}

fn imports_of(text: &str) -> Vec<Vec<String>> {
    parse(text)
        .module
        .imports
        .iter()
        .map(|import| import.path.iter().map(|name| name.text.clone()).collect())
        .collect()
}

fn display(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Run git in the directory and give its standard output.
fn git(directory: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {}: {detail}", arguments.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
