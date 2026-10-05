//! Name resolution, type checking and effect checking (milestone M2). The
//! standard library's declarations are compiled in from `library/std/*.ry`;
//! a program's own imports are read from the directory of its file.

pub mod check;
pub mod effects;
pub mod refine;
pub mod types;
pub mod world;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use renyi_syntax::{parse, parse_declarations, Diagnostic, SourceFile, Span};

pub use check::{NumberKind, Reference, Target};
pub use types::{AbilityId, FunctionId, ModuleId, TypeId};
pub use world::{BodyLocation, World};

/// The standard library, one declaration file per module.
pub const LIBRARY: &[(&str, &str)] = &[
    (
        "std.prelude",
        include_str!("../../../library/std/prelude.ry"),
    ),
    (
        "std.console",
        include_str!("../../../library/std/console.ry"),
    ),
    (
        "std.environment",
        include_str!("../../../library/std/environment.ry"),
    ),
    ("std.time", include_str!("../../../library/std/time.ry")),
    ("std.random", include_str!("../../../library/std/random.ry")),
    (
        "std.filesystem",
        include_str!("../../../library/std/filesystem.ry"),
    ),
    ("std.json", include_str!("../../../library/std/json.ry")),
    ("std.http", include_str!("../../../library/std/http.ry")),
    ("std.server", include_str!("../../../library/std/server.ry")),
    ("std.csv", include_str!("../../../library/std/csv.ry")),
    ("std.sqlite", include_str!("../../../library/std/sqlite.ry")),
    ("std.regex", include_str!("../../../library/std/regex.ry")),
];

/// A world with the standard library declared.
pub fn library_world() -> World {
    let mut world = World::new();
    for (name, source) in LIBRARY {
        let parsed = parse_declarations(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "the library file {name} does not parse: {:?}",
            parsed.diagnostics
        );
        world.add_module(parsed.module, true);
    }
    world
}

/// One file of a project after `check_project`.
pub struct CheckedModule {
    /// The index of the file in the list given.
    pub file: usize,
    /// The module declared from the file, or `None` when the file does not
    /// parse.
    pub id: Option<ModuleId>,
    /// Parse, declaration and body diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// A project checked as a whole: the world with every module declared, one
/// entry per file, and every reference the bodies make, for tools such as
/// the project map.
pub struct CheckedProject {
    pub world: World,
    pub modules: Vec<CheckedModule>,
    pub references: Vec<(ModuleId, BodyLocation, Reference)>,
}

/// Declare and check every file of a project together. A file that does not
/// parse keeps its parse diagnostics and declares no module, so a module
/// that imports it sees an unknown module.
pub fn check_project(files: &[SourceFile]) -> CheckedProject {
    let mut world = library_world();
    let mut modules = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let parsed = parse(&file.text);
        let id = if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            None
        } else {
            Some(world.add_module(parsed.module, false))
        };
        modules.push(CheckedModule {
            file: index,
            id,
            diagnostics: parsed.diagnostics,
        });
    }
    world.resolve_all();
    let mut references = Vec::new();
    for module in &mut modules {
        let Some(id) = module.id else {
            continue;
        };
        module.diagnostics.extend(
            world
                .diagnostics
                .iter()
                .filter(|(m, _)| *m == id)
                .map(|(_, d)| d.clone()),
        );
        let (body_diagnostics, body_references) = check::check_module_with_references(&world, id);
        module.diagnostics.extend(body_diagnostics);
        module.diagnostics.sort_by_key(|d| d.span.start);
        references.extend(
            body_references
                .into_iter()
                .map(|(body, reference)| (id, body, reference)),
        );
    }
    CheckedProject {
        world,
        modules,
        references,
    }
}

/// Check a parsed program whose imports are given as sources, for tests and
/// tools that hold everything in memory. The main module comes first; an
/// import with errors is summarized as one diagnostic of the main module.
pub fn check_sources(main: &SourceFile, imports: &[SourceFile]) -> Vec<Diagnostic> {
    let mut files = Vec::with_capacity(imports.len() + 1);
    files.push(main.clone());
    files.extend(imports.iter().cloned());
    let checked = check_project(&files);
    let main_module = &checked.modules[0];
    if main_module.id.is_none() {
        return main_module.diagnostics.clone();
    }
    let mut diagnostics = main_module.diagnostics.clone();
    for (import, module) in imports.iter().zip(&checked.modules[1..]) {
        if module.id.is_none() {
            continue; // reported as an unknown module by the resolver
        }
        let count = module.diagnostics.iter().filter(|d| d.is_error()).count();
        if count > 0 {
            diagnostics.push(Diagnostic::error(
                "import-errors",
                format!(
                    "the imported module `{}` has {count} error{}",
                    import.name,
                    if count == 1 { "" } else { "s" }
                ),
                Span::new(0, 0),
            ));
        }
    }
    diagnostics.sort_by_key(|d| d.span.start);
    diagnostics
}

/// The non-library modules a file imports, transitively, read from its
/// directory (decision J17: the file's directory is the project root). A
/// file that cannot be read is left out; the resolver then reports an
/// unknown module.
pub fn imported_files(file: &SourceFile) -> Vec<SourceFile> {
    let directory = Path::new(&file.name)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let parsed = parse(&file.text);
    let mut imports = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: Vec<Vec<String>> = parsed
        .module
        .imports
        .iter()
        .map(|i| i.path.iter().map(|n| n.text.clone()).collect())
        .collect();
    while let Some(path) = queue.pop() {
        if path.first().map(String::as_str) == Some("std") {
            continue;
        }
        let name = path.join(".");
        if !seen.insert(name.clone()) {
            continue;
        }
        let mut file_path = directory.clone();
        for segment in &path {
            file_path.push(segment);
        }
        file_path.set_extension("ry");
        let Ok(text) = std::fs::read_to_string(&file_path) else {
            continue;
        };
        let imported = parse(&text);
        for import in &imported.module.imports {
            queue.push(import.path.iter().map(|n| n.text.clone()).collect());
        }
        imports.push(SourceFile::new(file_path.display().to_string(), text));
    }
    imports
}

/// Check a file on disk together with the imports read from its directory.
pub fn check_file(file: &SourceFile) -> Vec<Diagnostic> {
    check_sources(file, &imported_files(file))
}
