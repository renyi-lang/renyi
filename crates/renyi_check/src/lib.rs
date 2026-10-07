//! Name resolution, type checking and effect checking (milestone M2). The
//! standard library's declarations are compiled in from `library/std/*.ry`;
//! a program's own imports are read from its project root and its
//! dependencies' files from the registry or the store (decision AC1, the
//! resolver of `renyi_package`).

pub mod check;
pub mod effects;
pub mod refine;
mod suggest;
pub mod types;
pub mod world;

use renyi_syntax::{parse, parse_declarations, Diagnostic, SourceFile, Span};

pub use check::{NumberKind, Reference, Target};
pub use renyi_package::{resolve, Problem, Project, Resolved};
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
    (
        "std.process",
        include_str!("../../../library/std/process.ry"),
    ),
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
        let mut diagnostics = parsed.diagnostics;
        let id = if diagnostics.iter().any(Diagnostic::is_error) {
            None
        } else {
            if let Some(diagnostic) = module_name_mismatch(&file.name, &parsed.module) {
                diagnostics.push(diagnostic);
            }
            let id = world.add_module(parsed.module, false);
            world.set_source_lines(id, &file.text);
            if let Some(package) = &file.package {
                world.set_package(id, package.clone());
            }
            Some(id)
        };
        modules.push(CheckedModule {
            file: index,
            id,
            diagnostics,
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

/// `check_project`, with the resolver's problems (a manifest that cannot be
/// read, a package missing or not the one the lockfile names) added to the
/// diagnostics of the files they belong to.
pub fn check_project_with_problems(files: &[SourceFile], problems: &[Problem]) -> CheckedProject {
    let mut checked = check_project(files);
    for problem in problems {
        let Some(module) = checked
            .modules
            .iter_mut()
            .find(|module| files[module.file].name == problem.file)
        else {
            continue;
        };
        module.diagnostics.push(problem.diagnostic.clone());
        module.diagnostics.sort_by_key(|d| d.span.start);
    }
    checked
}

/// Decision G3: the module name equals the path. The last segment is the
/// file's stem and the segments before it its parent directories, read
/// from the end; the extension is `.ry` or `.renyi`.
fn module_name_mismatch(file_name: &str, module: &renyi_syntax::ast::Module) -> Option<Diagnostic> {
    let segments: Vec<&str> = module.name.iter().map(|n| n.text.as_str()).collect();
    let mut components: Vec<&str> = file_name
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .collect();
    let last = components.pop().unwrap_or("");
    let stem = last
        .strip_suffix(".ry")
        .or_else(|| last.strip_suffix(".renyi"))
        .unwrap_or(last);
    let (name, parents) = segments.split_last()?;
    let matches = stem == *name
        && components.len() >= parents.len()
        && parents
            .iter()
            .rev()
            .zip(components.iter().rev())
            .all(|(segment, component)| segment == component);
    if matches {
        return None;
    }
    let span = match (module.name.first(), module.name.last()) {
        (Some(first), Some(last)) => first.span.join(last.span),
        _ => Span::new(0, 0),
    };
    let expected = format!("{}.ry", segments.join("/"));
    Some(
        Diagnostic::error(
            "module-name",
            format!(
                "the module is named `{}`, but its file is `{last}`; the module name equals the path",
                segments.join(".")
            ),
            span,
        )
        .with_fix(format!(
            "rename the module after the file, or the file to `{expected}` (decision G3)"
        )),
    )
}

/// Check a parsed program whose imports are given as sources, for tests and
/// tools that hold everything in memory. The main module comes first; an
/// import with errors is summarized as one diagnostic of the main module.
pub fn check_sources(main: &SourceFile, imports: &[SourceFile]) -> Vec<Diagnostic> {
    let mut files = Vec::with_capacity(imports.len() + 1);
    files.push(main.clone());
    files.extend(imports.iter().cloned());
    summarize(&check_project(&files), imports)
}

/// The main module's diagnostics of a checked project, with an import
/// that has errors summarized as one diagnostic.
fn summarize(checked: &CheckedProject, imports: &[SourceFile]) -> Vec<Diagnostic> {
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
            diagnostics.push(
                Diagnostic::error(
                    "import-errors",
                    format!(
                        "the imported module `{}` has {count} error{}",
                        import.name,
                        if count == 1 { "" } else { "s" }
                    ),
                    Span::new(0, 0),
                )
                .with_fix(format!(
                    "run `renyi check` on `{}` and fix them first",
                    import.name
                )),
            );
        }
    }
    diagnostics.sort_by_key(|d| d.span.start);
    diagnostics
}

/// The non-library modules a file imports, transitively: its own from the
/// project root (decision J17: the file's directory, when there is no
/// manifest above it) and its dependencies' from the registry or the store
/// (decision AC1), without the problems the resolver found. The path of a
/// file is joined with `/` on every platform, so that a bytecode file,
/// which remembers the paths, does not depend on the machine that wrote it
/// (decision Z3).
pub fn imported_files(file: &SourceFile) -> Vec<SourceFile> {
    resolve(file).files.into_iter().skip(1).collect()
}

/// Check a file on disk together with its imports and dependencies; the
/// resolver's problems count as the main file's diagnostics.
pub fn check_file(file: &SourceFile) -> Vec<Diagnostic> {
    let resolved = resolve(file);
    let checked = check_project_with_problems(&resolved.files, &resolved.problems);
    summarize(&checked, &resolved.files[1..])
}
