//! The resolution of a program's imports (decision AC1): its project (the
//! nearest `renyi.json` in its file's directory or above it, else the
//! file's directory, decision J17), its own files from the project root,
//! and the files of its dependencies from the registry or the store, each
//! verified against the lockfile. The front end written in Renyi does the
//! same in `compiler/project.ry`, and the judges hold the two equal: the
//! order of the files, the names of the files and the problems reported
//! are the same on both sides.

use std::collections::HashSet;

use renyi_syntax::{parse, Diagnostic, ForeignModule, Package, SourceFile, Span};

use crate::manifest::{Lock, Manifest, PackageFile, LOCK_FILE, MANIFEST_FILE, PACKAGE_FILE};
use crate::registry::{hash_of, is_absolute, join, Registry};
use crate::version::Version;

/// A problem the resolver found, for the file named: a manifest or a
/// lockfile that cannot be read, a package that is missing or differs
/// from the lockfile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub diagnostic: Diagnostic,
}

/// The project a file belongs to.
#[derive(Clone, Debug)]
pub struct Project {
    /// The directory holding `renyi.json`, or the file's directory; empty
    /// for the working directory.
    pub root: String,
    pub manifest: Option<Manifest>,
    pub lock: Option<Lock>,
    pub registry: Option<Registry>,
    /// What went wrong reading the manifest or the lockfile, as
    /// `manifest-invalid` diagnostics without a position.
    pub problems: Vec<Diagnostic>,
}

/// The file, tagged as the foreign module it declares when the manifest of
/// its project names it (decision AF1); any other file as it is. The
/// commands tag the file they are given before parsing it, since a foreign
/// module is parsed as declarations.
pub fn tagged(file: SourceFile) -> SourceFile {
    if file.foreign.is_some() {
        return file;
    }
    match Project::of(&file.name).foreign_of_file(&file.name) {
        Some(binding) => file.in_foreign(binding),
        None => file,
    }
}

/// The files of a program with its imports, and the problems found.
#[derive(Clone, Debug, Default)]
pub struct Resolved {
    /// The main file first, then every import in the order found.
    pub files: Vec<SourceFile>,
    pub problems: Vec<Problem>,
}

/// The directory part of a path as text, empty for a bare name.
pub fn directory_of(file_name: &str) -> String {
    match file_name.rfind(['/', '\\']) {
        Some(cut) => file_name[..cut].to_string(),
        None => String::new(),
    }
}

fn manifest_invalid(file: &str, detail: &str, span: Span) -> Diagnostic {
    Diagnostic::error("manifest-invalid", format!("{file}: {detail}"), span)
        .with_fix("fix the file, or write it again with `renyi add`")
}

impl Project {
    /// The project of a file: its directories walked up, textually, to the
    /// nearest `renyi.json`; a relative path stops at the working
    /// directory, an absolute one at its root.
    pub fn of(file_name: &str) -> Project {
        Project::walk(directory_of(file_name))
    }

    /// The project of a directory: the nearest `renyi.json` in it or above
    /// it, else the directory itself (`.` and an empty text are the
    /// working directory).
    pub fn of_directory(directory: &str) -> Project {
        let own = directory.trim_end_matches(['/', '\\']);
        let own = if own == "." {
            String::new()
        } else {
            own.to_string()
        };
        Project::walk(own)
    }

    fn walk(own: String) -> Project {
        let mut directory = own.clone();
        loop {
            let manifest_path = join(&directory, MANIFEST_FILE);
            if let Ok(text) = std::fs::read_to_string(&manifest_path) {
                return Project::at(directory, &manifest_path, &text);
            }
            if directory.is_empty() {
                break;
            }
            let parent = directory_of(&directory);
            if parent.is_empty() && is_absolute(&directory) {
                break;
            }
            directory = parent;
        }
        Project {
            root: own,
            manifest: None,
            lock: None,
            registry: None,
            problems: Vec::new(),
        }
    }

    fn at(root: String, manifest_path: &str, manifest_text: &str) -> Project {
        let mut problems = Vec::new();
        let manifest = match Manifest::read(manifest_text) {
            Ok(manifest) => Some(manifest),
            Err(detail) => {
                problems.push(manifest_invalid(manifest_path, &detail, Span::new(0, 0)));
                None
            }
        };
        let lock_path = join(&root, LOCK_FILE);
        let lock = match std::fs::read_to_string(&lock_path) {
            Ok(text) => match Lock::read(&text) {
                Ok(lock) => Some(lock),
                Err(detail) => {
                    problems.push(manifest_invalid(&lock_path, &detail, Span::new(0, 0)));
                    None
                }
            },
            Err(_) => None,
        };
        let registry = manifest
            .as_ref()
            .and_then(|manifest| manifest.registry.as_deref())
            .map(|spec| Registry::parse(spec, &root));
        Project {
            root,
            manifest,
            lock,
            registry,
            problems,
        }
    }

    /// Where a locked package's files are, when the manifest names a
    /// registry.
    pub fn package_root(&self, name: &str, version: Version) -> Option<String> {
        self.registry
            .as_ref()
            .map(|registry| registry.package_root(&self.root, name, version))
    }

    /// The binding of a foreign module of the project (decision AF1), by
    /// the module's qualified name.
    pub fn foreign_module(&self, name: &str) -> Option<ForeignModule> {
        self.manifest
            .as_ref()?
            .foreign
            .iter()
            .find(|(module, _)| module == name)
            .map(|(_, binding)| binding.clone())
    }

    /// The binding of a file of the project that declares a foreign module:
    /// the file's path from the root, without its extension, is the
    /// module's name.
    pub fn foreign_of_file(&self, file_name: &str) -> Option<ForeignModule> {
        let relative = if self.root.is_empty() {
            file_name
        } else {
            file_name
                .strip_prefix(self.root.as_str())?
                .trim_start_matches(['/', '\\'])
        };
        let stem = relative
            .strip_suffix(".ry")
            .or_else(|| relative.strip_suffix(".renyi"))?;
        self.foreign_module(&stem.replace(['/', '\\'], "."))
    }

    /// The names of the manifest's dependencies.
    pub fn dependencies(&self) -> Vec<String> {
        self.manifest
            .as_ref()
            .map(|manifest| {
                manifest
                    .dependencies
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Where an import is read from: the program, or a package with its root
/// and the names of its own dependencies.
#[derive(Clone, Debug)]
enum Origin {
    Program,
    Package {
        name: String,
        version: String,
        root: String,
        dependencies: Vec<String>,
    },
}

struct Pending {
    path: Vec<String>,
    origin: Origin,
    /// The importing file and the import's span, for a problem.
    importer: String,
    span: Span,
}

/// The imports of a parsed text, without those of the library.
fn imports_of(text: &str) -> Vec<(Vec<String>, Span)> {
    parse(text)
        .module
        .imports
        .iter()
        .filter(|import| import.path.first().map(|n| n.text.as_str()) != Some("std"))
        .map(|import| {
            (
                import.path.iter().map(|n| n.text.clone()).collect(),
                import.span,
            )
        })
        .collect()
}

/// A file with the `.ry` or the `.renyi` extension (decision G3).
fn read_source(base: &str) -> Option<(String, String)> {
    for extension in ["ry", "renyi"] {
        let path = format!("{base}.{extension}");
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Some((path, text));
        }
    }
    None
}

/// The non-library modules a file imports, transitively: its own from the
/// project root, its dependencies' from the registry or the store. The
/// last import of a file is read first, as before packages existed, so
/// that the order of the modules, and of the bytecode, is unchanged. A
/// file that cannot be read is left out, and the checker then reports an
/// unknown module.
pub fn resolve(file: &SourceFile) -> Resolved {
    resolve_in(&Project::of(&file.name), file)
}

/// `resolve` in a project given: the commands read a package's own files
/// from the registry as a project whose dependencies the lock names.
pub fn resolve_in(project: &Project, file: &SourceFile) -> Resolved {
    let mut resolved = Resolved::default();
    let mut main = file.clone();
    if main.foreign.is_none() {
        if let Some(binding) = project.foreign_of_file(&main.name) {
            main = main.in_foreign(binding);
        }
    }
    resolved.files.push(main);
    for diagnostic in &project.problems {
        resolved.problems.push(Problem {
            file: file.name.clone(),
            diagnostic: diagnostic.clone(),
        });
    }
    let program_dependencies = project.dependencies();
    let mut seen: HashSet<String> = HashSet::new();
    let mut reported: HashSet<String> = HashSet::new();
    let mut queue: Vec<Pending> = imports_of(&file.text)
        .into_iter()
        .map(|(path, span)| Pending {
            path,
            origin: Origin::Program,
            importer: file.name.clone(),
            span,
        })
        .collect();
    while let Some(pending) = queue.pop() {
        let Some(first) = pending.path.first() else {
            continue;
        };
        let (dependencies, own_root, own_prefix) = match &pending.origin {
            Origin::Program => (&program_dependencies, project.root.clone(), None),
            Origin::Package {
                name,
                root,
                dependencies,
                ..
            } => (dependencies, root.clone(), Some(name.clone())),
        };
        let (qualified, base, origin, package) = if dependencies.contains(first) {
            // a dependency: the file from the package's root
            let Some((root, package_file, package)) = locate_package(
                project,
                first,
                &pending,
                &mut reported,
                &mut resolved.problems,
            ) else {
                continue;
            };
            let base = if pending.path.len() == 1 {
                join(&root, first)
            } else {
                join(&root, &pending.path[1..].join("/"))
            };
            let origin = Origin::Package {
                name: first.clone(),
                version: package.version.clone(),
                root,
                dependencies: package_file
                    .dependencies
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect(),
            };
            (pending.path.join("."), base, origin, Some(package))
        } else {
            // an own file of the program or of the package
            let dotted = pending.path.join(".");
            let qualified = match &own_prefix {
                Some(prefix) if dotted != *prefix => format!("{prefix}.{dotted}"),
                _ => dotted,
            };
            let package = match &pending.origin {
                Origin::Package { name, version, .. } => Some(Package {
                    name: name.clone(),
                    version: version.clone(),
                }),
                Origin::Program => None,
            };
            (
                qualified,
                join(&own_root, &pending.path.join("/")),
                pending.origin.clone(),
                package,
            )
        };
        if !seen.insert(qualified.clone()) {
            continue;
        }
        let Some((path, text)) = read_source(&base) else {
            continue;
        };
        for (import_path, span) in imports_of(&text) {
            queue.push(Pending {
                path: import_path,
                origin: origin.clone(),
                importer: path.clone(),
                span,
            });
        }
        let mut source = SourceFile::new(path, text);
        if let Some(package) = package {
            source = source.in_package(package);
        } else if let Some(binding) = project.foreign_module(&qualified) {
            source = source.in_foreign(binding);
        }
        resolved.files.push(source);
    }
    resolved
}

/// The root, the package file and the tag of a locked dependency, verified
/// against the lockfile; `None` with a problem reported (once per package)
/// when it is not locked, not there, or cannot be read. A package whose
/// hash differs from the lockfile is reported and read all the same.
fn locate_package(
    project: &Project,
    name: &str,
    pending: &Pending,
    reported: &mut HashSet<String>,
    problems: &mut Vec<Problem>,
) -> Option<(String, PackageFile, Package)> {
    let mut report = |diagnostic: Diagnostic| {
        if reported.insert(name.to_string()) {
            problems.push(Problem {
                file: pending.importer.clone(),
                diagnostic,
            });
        }
    };
    let Some(locked) = project.lock.as_ref().and_then(|lock| lock.get(name)) else {
        report(
            Diagnostic::error(
                "package-missing",
                format!("`{name}` is a dependency of {MANIFEST_FILE} but not in {LOCK_FILE}"),
                pending.span,
            )
            .with_fix(format!("run `renyi add {name}`")),
        );
        return None;
    };
    let Some(root) = project.package_root(name, locked.version) else {
        report(
            Diagnostic::error(
                "package-missing",
                format!("{MANIFEST_FILE} names no `registry` to read `{name}` from"),
                pending.span,
            )
            .with_fix("add `\"registry\"` to the manifest: a directory or a URL"),
        );
        return None;
    };
    let package_path = join(&root, PACKAGE_FILE);
    let Ok(text) = std::fs::read_to_string(&package_path) else {
        report(
            Diagnostic::error(
                "package-missing",
                format!("`{name}` {} is locked but not at {root}", locked.version),
                pending.span,
            )
            .with_fix("run `renyi fetch` to get the locked packages"),
        );
        return None;
    };
    let package_file = match PackageFile::read(&text) {
        Ok(package_file) => package_file,
        Err(detail) => {
            report(manifest_invalid(&package_path, &detail, pending.span));
            return None;
        }
    };
    if hash_of(text.as_bytes()) != locked.hash {
        report(
            Diagnostic::error(
                "package-mismatch",
                format!(
                    "`{name}` {} is not the package {LOCK_FILE} names: its hash differs",
                    locked.version
                ),
                pending.span,
            )
            .with_fix(
                "run `renyi fetch` to get the locked package, or `renyi update` to take this one",
            ),
        );
    }
    let package = Package {
        name: name.to_string(),
        version: locked.version.to_string(),
    };
    Some((root, package_file, package))
}
