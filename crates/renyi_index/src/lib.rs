//! The project map (`renyi index`; design document 05, decisions O2 to O4):
//! one record per definition and one per module, projected from the checked
//! program. Every number has the definition given in section 2 of the
//! document, so that two runs on the same canonical text agree exactly.
//!
//! The map is computed on the canonical text of every file (the formatter's
//! output), so that line counts and hashes do not depend on layout; a module
//! record says whether its file was already canonical. A file that does not
//! parse is indexed as far as it checks and its module record carries the
//! error count.

pub mod budgets;
pub mod diff;
mod drafts;
mod edges;
pub mod hash;
pub mod metrics;
pub mod render;
pub mod tools;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use renyi_check::effects::Capability;
use renyi_check::{
    check_project_in, BodyLocation, CheckedProject, FunctionId, Library, ModuleId, Target,
};
use renyi_syntax::ast::Item;
use renyi_syntax::{format, SourceFile, Span};

use drafts::{Draft, Key};

pub use budgets::{over_budget, Budgets};
pub use diff::{diff, diff_json, render_diff, Bump, Change, Diff, Entry};
pub use render::{definition_json, to_json, to_text};
pub use tools::{manifest_json, tools_of, tools_of_in, Tool};

/// What a definition is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Function,
    Method,
    Type,
    Ability,
    Implementation,
    Constant,
    Test,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Function => "function",
            Kind::Method => "method",
            Kind::Type => "type",
            Kind::Ability => "ability",
            Kind::Implementation => "implementation",
            Kind::Constant => "constant",
            Kind::Test => "test",
        }
    }

    /// The kind spelled by `name`, as the map's JSON spells it.
    pub fn parse(name: &str) -> Option<Kind> {
        [
            Kind::Function,
            Kind::Method,
            Kind::Type,
            Kind::Ability,
            Kind::Implementation,
            Kind::Constant,
            Kind::Test,
        ]
        .into_iter()
        .find(|kind| kind.name() == name)
    }
}

/// The six metrics of section 2, with the library calls counted apart from
/// the fan-out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub lines: usize,
    pub depth: usize,
    pub branches: usize,
    pub effects: usize,
    pub fan_in: usize,
    pub fan_out: usize,
    pub library_calls: usize,
}

/// The ability and target of an implementation, qualified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Implements {
    pub ability: String,
    pub target: String,
}

/// One definition record (section 1 of the design document).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    /// `sha256:` and the content hash (section 3).
    pub id: String,
    /// `sha256:` and the hash of the definition's own canonical text with
    /// its name and its references blanked: unlike `id`, it does not change
    /// when a dependency changes (section 6).
    pub text_hash: String,
    pub module: String,
    /// The name inside the module: a function's name, `Type.method` for a
    /// method, `Ability for Target` for an implementation, `test:` and the
    /// name for a test.
    pub name: String,
    pub kind: Kind,
    pub public: bool,
    /// The head and signature clauses on one line, as the formatter spells
    /// them.
    pub signature: String,
    pub purpose: Option<String>,
    pub tags: Vec<String>,
    pub see_also: Vec<String>,
    pub exposed_as_tool: bool,
    /// The `needs` clause, and the union over everything the body reaches
    /// (library primitives included; an ability method reaches every
    /// implementation).
    pub effects_declared: Vec<String>,
    pub effects_transitive: Vec<String>,
    /// The `or fails with` types, and the union of the declared failure
    /// types over everything the body reaches.
    pub fails_declared: Vec<String>,
    pub fails_transitive: Vec<String>,
    /// The functions and ability methods it calls or passes by name,
    /// qualified (`module.name`, `module.Type.method`,
    /// `module.Ability.method`), library ones included.
    pub calls: Vec<String>,
    /// The types, constants and abilities it mentions, qualified.
    pub uses: Vec<String>,
    pub implements: Option<Implements>,
    /// The tests whose bodies refer to it, qualified.
    pub tested_by: Vec<String>,
    pub metrics: Metrics,
    /// The number of `example:` lines.
    pub examples: usize,
    /// The number of tests in `tested_by`.
    pub tests: usize,
    pub file: String,
    pub line: usize,
    pub end_line: usize,
    /// `<name> <version>` of the dependency the definition belongs to
    /// (decision AC1); none for the project's own.
    pub package: Option<String>,
}

impl Definition {
    /// `module.name`, as edges spell it.
    pub fn qualified(&self) -> String {
        format!("{}.{}", self.module, self.name)
    }
}

/// One module record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub file: String,
    pub purpose: Option<String>,
    pub imports: Vec<String>,
    pub definitions: usize,
    pub public: usize,
    pub lines: usize,
    /// The union of its definitions' transitive effects.
    pub effects: Vec<String>,
    pub ids: Vec<String>,
    /// The number of errors the checker reported for the file.
    pub errors: usize,
    /// Whether the file was already in canonical layout; when it was not,
    /// lines and hashes refer to the formatted text.
    pub canonical: bool,
    /// `<name> <version>` of the dependency the module belongs to
    /// (decision AC1); none for the project's own.
    pub package: Option<String>,
}

/// The map header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub project: String,
    pub revision: String,
    pub toolchain: String,
}

/// The whole map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub header: Header,
    pub modules: Vec<Module>,
    pub definitions: Vec<Definition>,
}

// ---------------------------------------------------------------- loading

/// The files of a project: every `.ry` or `.renyi` file under a directory
/// (recursively, in path order, the store `.renyi` left out) with the
/// files of the dependencies they import, or one file with its imports
/// and dependencies.
pub fn load_project(path: &Path) -> Result<Vec<SourceFile>, String> {
    if path.is_dir() {
        let mut paths = Vec::new();
        collect_sources(path, &mut paths)?;
        paths.sort();
        let mut files: Vec<SourceFile> = paths
            .iter()
            .map(|path| read(path).map(renyi_check::tagged))
            .collect::<Result<_, _>>()?;
        let own = files.clone();
        for file in &own {
            for resolved in renyi_check::imported_files(file) {
                if resolved.package.is_some() && !files.iter().any(|f| f.name == resolved.name) {
                    files.push(resolved);
                }
            }
        }
        Ok(files)
    } else {
        let file = renyi_check::tagged(read(path)?);
        let mut files = vec![file.clone()];
        files.extend(renyi_check::imported_files(&file));
        Ok(files)
    }
}

fn collect_sources(directory: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == ".renyi") {
                continue;
            }
            collect_sources(&path, out)?;
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ry" | "renyi")
        ) {
            out.push(path);
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<SourceFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(SourceFile::new(path.display().to_string(), text))
}

/// The short git revision of the directory holding `path`, or `unknown`.
pub fn git_revision(path: &Path) -> String {
    let directory = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    std::process::Command::new("git")
        .arg("-C")
        .arg(&directory)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// The project's name: the directory's or the file's own name.
pub fn project_name(path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let name = if resolved.is_dir() {
        resolved
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
    } else {
        resolved
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
    };
    name.unwrap_or_else(|| "project".to_string())
}

// ---------------------------------------------------------------- building

type BodyKey = (usize, usize, usize, usize);

fn body_key(body: BodyLocation) -> BodyKey {
    match body {
        BodyLocation::None => (0, 0, 0, 0),
        BodyLocation::Item(index) => (1, index, 0, 0),
        BodyLocation::Implementation(item, function) => (2, item, function, 0),
        // the references of `example:` lines are not a body's (they are not
        // calls the definition makes)
        BodyLocation::Example {
            item,
            method,
            example,
        } => (3, item, method.unwrap_or(usize::MAX), example),
        // a refinement's calls belong to the type: tracked with the type's
        // other references once conditions count as a body (not yet)
        BodyLocation::Condition {
            item,
            variant,
            field,
        } => (
            4,
            item,
            variant.unwrap_or(usize::MAX),
            field.unwrap_or(usize::MAX),
        ),
    }
}

/// The package a file belongs to, as the map labels it.
fn package_label(file: &SourceFile) -> Option<String> {
    file.package
        .as_ref()
        .map(|package| format!("{} {}", package.name, package.version))
}

/// Index the files given, in memory, against the standard library.
pub fn index_files(files: &[SourceFile], header: Header) -> Index {
    index_files_in(&Library::standard(), files, header)
}

/// Index the files given, in memory, against the declaration files of
/// the toolchain's extensions (decision AJ1).
pub fn index_files_in(library: &Library, files: &[SourceFile], header: Header) -> Index {
    let (canonical, was_canonical) = canonical_files(files);
    let checked = check_project_in(library, &canonical, &[]);
    index_checked(&canonical, &was_canonical, &checked, header)
}

/// The map of a project checked already: the files in canonical layout
/// (`canonical_files`), whether each one was, the checked project over
/// those files and the header. The resident world of `renyi_workspace`
/// (decision AN1) checks incrementally and builds its map from here.
pub fn index_checked(
    canonical: &[SourceFile],
    was_canonical: &[bool],
    checked: &CheckedProject,
    header: Header,
) -> Index {
    let world = &checked.world;

    let mut body_refs: HashMap<(ModuleId, BodyKey), Vec<(Target, Span)>> = HashMap::new();
    for (module, body, reference) in &checked.references {
        body_refs
            .entry((*module, body_key(*body)))
            .or_default()
            .push((reference.target.clone(), reference.span));
    }
    let mut function_at: HashMap<(ModuleId, BodyKey), FunctionId> = HashMap::new();
    for (id, function) in world.functions.iter().enumerate() {
        function_at.insert((function.module, body_key(function.body)), id);
    }
    let refs_of = |module: ModuleId, body: BodyLocation| -> Vec<(Target, Span)> {
        body_refs
            .get(&(module, body_key(body)))
            .cloned()
            .unwrap_or_default()
    };

    // one draft per definition, in module and item order
    let mut drafts: Vec<Draft> = Vec::new();
    for checked_module in &checked.modules {
        let Some(module) = checked_module.id else {
            continue;
        };
        let file = checked_module.file;
        let text = &canonical[file];
        let items = &world.modules[module].ast.items;
        for (index, item) in items.iter().enumerate() {
            match item {
                Item::Function(function) => {
                    let body = BodyLocation::Item(index);
                    if let Some(&id) = function_at.get(&(module, body_key(body))) {
                        let refs = refs_of(module, body);
                        drafts.push(drafts::function_draft(
                            world, module, file, id, function, refs, None,
                        ));
                    }
                }
                Item::Type(def) => drafts.push(drafts::type_draft(world, module, file, text, def)),
                Item::Ability(ability) => {
                    drafts.push(drafts::ability_draft(world, module, file, ability));
                }
                Item::Implementation(implementation) => {
                    let Some(impl_index) = world
                        .impls
                        .iter()
                        .position(|i| i.module == module && i.span == implementation.span)
                    else {
                        continue;
                    };
                    let implements = drafts::implements_of(world, impl_index);
                    let mut methods = Vec::new();
                    for (function_index, function) in implementation.functions.iter().enumerate() {
                        let body = BodyLocation::Implementation(index, function_index);
                        if let Some(&id) = function_at.get(&(module, body_key(body))) {
                            let refs = refs_of(module, body);
                            methods.push(drafts::function_draft(
                                world,
                                module,
                                file,
                                id,
                                function,
                                refs,
                                Some(implements.clone()),
                            ));
                        }
                    }
                    drafts.push(drafts::implementation_draft(
                        world,
                        module,
                        file,
                        impl_index,
                        implementation,
                        implements,
                        &methods,
                    ));
                    drafts.extend(methods);
                }
                Item::Constant(constant) => {
                    let refs = refs_of(module, BodyLocation::Item(index));
                    drafts.push(drafts::constant_draft(world, module, file, constant, refs));
                }
                Item::Test(test) => {
                    let refs = refs_of(module, BodyLocation::Item(index));
                    drafts.push(drafts::test_draft(module, file, index, test, refs));
                }
            }
        }
    }

    let key_index: HashMap<Key, usize> = drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| (draft.key.clone(), index))
        .collect();
    let edges: Vec<edges::Edges> = drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| edges::resolve_edges(world, draft, index, &key_index))
        .collect();

    // the functions each project function reaches directly
    let mut callees: HashMap<FunctionId, Vec<FunctionId>> = HashMap::new();
    for (draft, edges) in drafts.iter().zip(&edges) {
        if let Key::Function(id) = draft.key {
            callees.insert(id, edges.callees.clone());
        }
    }

    let mut fan_in = vec![0usize; drafts.len()];
    let mut tested_by: Vec<BTreeSet<String>> = vec![BTreeSet::new(); drafts.len()];
    for (draft, edges) in drafts.iter().zip(&edges) {
        for &target in &edges.out {
            fan_in[target] += 1;
            if draft.kind == Kind::Test {
                tested_by[target].insert(drafts::qualified_name(world, draft));
            }
        }
    }

    let inputs: Vec<hash::Input> = drafts
        .iter()
        .zip(&edges)
        .map(|(draft, edges)| edges::hash_input(&canonical[draft.file], draft, edges))
        .collect();
    let ids = hash::hashes(&inputs, &header.toolchain);
    let text_hashes: Vec<String> = inputs.iter().map(hash::own_text_hash).collect();

    let mut definitions = Vec::new();
    for (index, (draft, edges)) in drafts.iter().zip(&edges).enumerate() {
        // decision C8c, tier 1: a deprecated definition is compiled for its
        // callers but is not discoverable, so the map leaves it out
        if draft.docs.deprecated.is_some() {
            continue;
        }
        let text = &canonical[draft.file];
        let (effects, fails) = edges::transitive(world, &callees, draft, edges);
        let line = text.position(draft.span.start).line;
        let end_line = text
            .position(draft.span.end.saturating_sub(1).max(draft.span.start))
            .line;
        let effect_paths: BTreeSet<&str> = effects
            .iter()
            .map(|effect| effect.split('(').next().unwrap_or(effect))
            .collect();
        definitions.push(Definition {
            id: ids[index].clone(),
            text_hash: text_hashes[index].clone(),
            module: world.modules[draft.module].name.clone(),
            name: draft.name.clone(),
            kind: draft.kind,
            public: draft.public,
            signature: draft.signature.clone(),
            purpose: draft.docs.purpose.clone(),
            tags: draft.docs.tags.clone(),
            see_also: draft.docs.see_also.clone(),
            exposed_as_tool: draft.docs.expose_as_tool,
            effects_declared: draft.needs.iter().map(Capability::spelling).collect(),
            effects_transitive: effects.iter().cloned().collect(),
            fails_declared: draft.fails.iter().map(|ty| world.show(ty)).collect(),
            fails_transitive: fails.iter().cloned().collect(),
            calls: edges.calls.iter().cloned().collect(),
            uses: edges.uses.iter().cloned().collect(),
            implements: draft.implements.clone(),
            tested_by: tested_by[index].iter().cloned().collect(),
            metrics: Metrics {
                lines: end_line - line + 1,
                depth: draft.depth,
                branches: draft.branches,
                effects: effect_paths.len(),
                fan_in: fan_in[index],
                fan_out: edges.out.len(),
                library_calls: edges.library_calls.len(),
            },
            examples: draft.docs.examples.len(),
            tests: tested_by[index].len(),
            file: display_path(&text.name),
            line,
            end_line,
            package: package_label(text),
        });
    }

    let mut modules = Vec::new();
    for checked_module in &checked.modules {
        let file = &canonical[checked_module.file];
        let errors = checked_module
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .count();
        let (name, purpose, imports) = match checked_module.id {
            Some(module) => {
                let info = &world.modules[module];
                let imports = info
                    .ast
                    .imports
                    .iter()
                    .map(|import| {
                        import
                            .path
                            .iter()
                            .map(|n| n.text.as_str())
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .collect();
                (info.name.clone(), info.ast.docs.purpose.clone(), imports)
            }
            None => (display_path(&file.name), None, Vec::new()),
        };
        let own: Vec<&Definition> = definitions
            .iter()
            .filter(|d| checked_module.id.is_some() && d.module == name)
            .collect();
        let mut effects = BTreeSet::new();
        for definition in &own {
            effects.extend(definition.effects_transitive.iter().cloned());
        }
        modules.push(Module {
            name,
            file: display_path(&file.name),
            purpose,
            imports,
            definitions: own.len(),
            public: own.iter().filter(|d| d.public).count(),
            lines: file.line_count(),
            effects: effects.into_iter().collect(),
            ids: own.iter().map(|d| d.id.clone()).collect(),
            errors,
            canonical: was_canonical[checked_module.file],
            package: package_label(file),
        });
    }

    Index {
        header,
        modules,
        definitions,
    }
}

/// Every file in canonical layout, and whether it already was, each with
/// the tags of the file it came from. A file that does not parse is kept
/// as it is.
pub fn canonical_files(files: &[SourceFile]) -> (Vec<SourceFile>, Vec<bool>) {
    let mut canonical = Vec::new();
    let mut was_canonical = Vec::new();
    for file in files {
        match format(file) {
            Ok(text) => {
                was_canonical.push(text == file.text);
                let mut copy = SourceFile::new(file.name.clone(), text);
                copy.package = file.package.clone();
                copy.foreign = file.foreign.clone();
                copy.python = file.python.clone();
                canonical.push(copy);
            }
            Err(_) => {
                was_canonical.push(true);
                canonical.push(file.clone());
            }
        }
    }
    (canonical, was_canonical)
}

fn display_path(name: &str) -> String {
    name.replace('\\', "/")
}
