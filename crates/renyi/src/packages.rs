//! The package commands (decision AC1): `renyi add`, `update`, `audit`,
//! `fetch` and `publish`. They act on the project of the working directory
//! (its `renyi.json` and `renyi.lock.json`) and on its registry: a
//! directory, read in place, or an `http://` or `https://` base whose
//! packages are fetched into the store `.renyi/packages/` under the
//! project root. Every package is verified when it is taken: each file
//! against the hash its `package.json` names, and the effect manifest
//! against the one its sources have (decision Q1).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use renyi_index::{index_files_in, Bump, Header, Index, Kind};
use renyi_package::{
    hash_of, is_absolute, is_package_name, join, resolve_in, select, Effect, Lock, Locked,
    Manifest, PackageFile, Project, PythonSection, Registry, Requirement, Version, Versions,
    LOCK_FILE, MANIFEST_FILE, PACKAGE_FILE, STORE, VERSIONS_FILE,
};
use renyi_syntax::diagnostics::render_text;
use renyi_syntax::SourceFile;

use crate::toolchain;

type Fallible<T> = Result<T, String>;

/// A package taken from the registry: where its files are, its package
/// file and the file's text.
struct Fetched {
    root: String,
    text: String,
    file: PackageFile,
}

/// A program of the project with a `main`: its file, the capabilities
/// `main` declares (the kind of each) and the packages it reaches.
struct MainProgram {
    file: String,
    declared: Vec<String>,
    reaches: BTreeSet<String>,
}

impl MainProgram {
    fn covers(&self, kind: &str) -> bool {
        self.declared
            .iter()
            .any(|base| kind == base || kind.starts_with(&format!("{base}.")))
    }
}

// ---------------------------------------------------------- the commands

pub(crate) fn add_command(args: &[String]) -> ExitCode {
    report(add(args))
}

pub(crate) fn update_command(args: &[String]) -> ExitCode {
    report(update(args))
}

pub(crate) fn fetch_command(args: &[String]) -> ExitCode {
    report(fetch(args))
}

pub(crate) fn publish_command(args: &[String]) -> ExitCode {
    report(publish(args))
}

pub(crate) fn audit_command(args: &[String]) -> ExitCode {
    match audit(args) {
        Ok((text, covered)) => {
            print!("{text}");
            if covered {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(message) => {
            eprintln!("renyi: {message}");
            ExitCode::FAILURE
        }
    }
}

fn report(result: Fallible<String>) -> ExitCode {
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("renyi: {message}");
            ExitCode::FAILURE
        }
    }
}

/// `renyi add <name> [<version>]`: the dependency in the manifest (the
/// version given, else the highest the registry has), the versions chosen
/// for every requirement, every package fetched and verified, the effects
/// of the one added printed, both files written.
fn add(args: &[String]) -> Fallible<String> {
    let (name, requested) = match args {
        [name] => (name.as_str(), None),
        [name, version] => (name.as_str(), Some(Version::parse(version)?)),
        _ => return Err("usage: renyi add <name> [<version>]".to_string()),
    };
    if name == "std" {
        return Err("`std` is the standard library, not a package".to_string());
    }
    if !is_package_name(name) {
        return Err(format!(
            "`{name}` is not a package name; write lower-case letters, digits and `_`, starting with a letter"
        ));
    }
    let project = project_here()?;
    let registry = registry_of(&project)?;
    if Path::new(&join(&project.root, name)).is_dir() {
        return Err(format!(
            "`{name}` is a directory of the project; a dependency of that name would hide it"
        ));
    }
    let versions = versions_of(&registry, name)?;
    let requirement = match requested {
        Some(version) => version,
        None => *versions
            .last()
            .ok_or_else(|| format!("the registry has no package `{name}`"))?,
    };
    let mut manifest = manifest_of(&project).clone();
    manifest.dependencies.retain(|(other, _)| other != name);
    manifest.dependencies.push((name.to_string(), requirement));
    manifest.dependencies.sort();
    let chosen = choose(&registry, &manifest)?;
    let (lock, fetched) = fetch_all(&project.root, &registry, &chosen)?;
    write_text(&join(&project.root, MANIFEST_FILE), &manifest.render())?;
    write_text(&join(&project.root, LOCK_FILE), &lock.render())?;
    let added = fetched
        .iter()
        .find(|package| package.file.name == name)
        .expect("the package added is among the chosen");
    let mut out = format!(
        "added `{name}` {} ({MANIFEST_FILE} requires {requirement}: the same major, at least that version)\n",
        added.file.version
    );
    out.push_str(&effects_text(&added.file.effects));
    let before: BTreeSet<&str> = project
        .lock
        .iter()
        .flat_map(|lock| lock.packages.iter().map(|locked| locked.name.as_str()))
        .collect();
    for package in &fetched {
        if package.file.name != name && !before.contains(package.file.name.as_str()) {
            out.push_str(&format!(
                "also `{}` {}, which it needs\n",
                package.file.name, package.file.version
            ));
        }
    }
    Ok(out)
}

/// `renyi update [--accept-effects]`: every dependency to the highest
/// version its requirement allows; a version whose effects widen is refused
/// without the flag, and with it when a `main` that reaches the package
/// does not declare the new capability.
fn update(args: &[String]) -> Fallible<String> {
    let accept = match args {
        [] => false,
        [flag] if flag == "--accept-effects" => true,
        _ => return Err("usage: renyi update [--accept-effects]".to_string()),
    };
    let project = project_here()?;
    let registry = registry_of(&project)?;
    let chosen = choose(&registry, manifest_of(&project))?;
    let before = project.lock.clone().unwrap_or_default();
    let (lock, fetched) = fetch_all(&project.root, &registry, &chosen)?;
    let mut out = String::new();
    let mut widened_all: Vec<(String, Version, Vec<String>)> = Vec::new();
    for package in &fetched {
        let name = &package.file.name;
        match before.get(name) {
            Some(locked)
                if locked.version == package.file.version
                    && locked.hash == hash_of(package.text.as_bytes()) => {}
            Some(locked) => {
                out.push_str(&format!(
                    "`{name}` {} -> {}\n",
                    locked.version, package.file.version
                ));
                let previous = package_file_of(&registry, name, locked.version)
                    .map(|(_, file)| kinds_of(&file.effects))
                    .unwrap_or_default();
                let widened: Vec<String> = kinds_of(&package.file.effects)
                    .difference(&previous)
                    .cloned()
                    .collect();
                if !widened.is_empty() {
                    out.push_str(&format!(
                        "  needs {}, which {} did not\n",
                        quoted(&widened),
                        locked.version
                    ));
                    widened_all.push((name.clone(), package.file.version, widened));
                }
            }
            None => out.push_str(&format!("`{name}` {} (new)\n", package.file.version)),
        }
    }
    if !widened_all.is_empty() {
        if !accept {
            return Err(format!(
                "{out}the effects widen; run `renyi update --accept-effects` to take the new versions (nothing was written)"
            ));
        }
        for main in mains_of(&project, &lock)? {
            for (name, version, widened) in &widened_all {
                if !main.reaches.contains(name) {
                    continue;
                }
                let uncovered: Vec<String> = widened
                    .iter()
                    .filter(|kind| !main.covers(kind))
                    .cloned()
                    .collect();
                if !uncovered.is_empty() {
                    return Err(format!(
                        "{out}`main` of {} does not declare {}, which `{name}` {version} needs; add it to its `needs` first (nothing was written)",
                        main.file,
                        quoted(&uncovered)
                    ));
                }
            }
        }
    }
    if out.is_empty() {
        out.push_str(
            "nothing to update: every dependency is at the highest version its requirement allows\n",
        );
    }
    write_text(&join(&project.root, LOCK_FILE), &lock.render())?;
    Ok(out)
}

/// `renyi audit`: every locked dependency's effects (the kinds of the
/// capabilities its public functions need), whether each `main` that
/// reaches it declares them, and the capabilities of a `main` that no
/// dependency uses. The second value is whether every reached dependency
/// is covered.
fn audit(args: &[String]) -> Fallible<(String, bool)> {
    if !args.is_empty() {
        return Err("usage: renyi audit".to_string());
    }
    let project = project_here()?;
    let registry = registry_of(&project)?;
    let lock = project.lock.clone().unwrap_or_default();
    if lock.packages.is_empty() {
        return Ok(("no dependency is locked\n".to_string(), true));
    }
    let mains = mains_of(&project, &lock)?;
    let mut out = String::new();
    let mut covered = true;
    let mut used: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for locked in &lock.packages {
        let root = registry.package_root(&project.root, &locked.name, locked.version);
        let package_path = join(&root, PACKAGE_FILE);
        let text = std::fs::read_to_string(&package_path).map_err(|_| {
            format!(
                "`{}` {} is not at {root}; run `renyi fetch`",
                locked.name, locked.version
            )
        })?;
        if hash_of(text.as_bytes()) != locked.hash {
            return Err(format!(
                "`{}` {} at {root} is not the package {LOCK_FILE} names: its hash differs; run `renyi fetch`",
                locked.name, locked.version
            ));
        }
        let file =
            PackageFile::read(&text).map_err(|detail| format!("{package_path}: {detail}"))?;
        let kinds = kinds_of(&file.effects);
        let listed: Vec<String> = kinds.iter().cloned().collect();
        out.push_str(&format!(
            "`{}` {} needs {}\n",
            locked.name,
            locked.version,
            if listed.is_empty() {
                "nothing".to_string()
            } else {
                quoted(&listed)
            }
        ));
        for main in &mains {
            if !main.reaches.contains(&locked.name) {
                continue;
            }
            used.entry(main.file.clone())
                .or_default()
                .extend(kinds.iter().cloned());
            let uncovered: Vec<String> = kinds
                .iter()
                .filter(|kind| !main.covers(kind))
                .cloned()
                .collect();
            if uncovered.is_empty() {
                out.push_str(&format!("  covered by `main` of {}\n", main.file));
            } else {
                covered = false;
                out.push_str(&format!(
                    "  not covered by `main` of {}: {}\n",
                    main.file,
                    quoted(&uncovered)
                ));
            }
        }
    }
    for main in &mains {
        let reached = used.get(&main.file).cloned().unwrap_or_default();
        let unused: Vec<String> = main
            .declared
            .iter()
            .filter(|base| {
                !reached
                    .iter()
                    .any(|kind| kind == *base || kind.starts_with(&format!("{base}.")))
            })
            .cloned()
            .collect();
        if !unused.is_empty() {
            out.push_str(&format!(
                "`main` of {} declares {}, which no dependency uses\n",
                main.file,
                quoted(&unused)
            ));
        }
    }
    Ok((out, covered))
}

/// `renyi fetch`: every locked package taken from the registry and
/// verified, into the store for a URL registry.
fn fetch(args: &[String]) -> Fallible<String> {
    if !args.is_empty() {
        return Err("usage: renyi fetch".to_string());
    }
    let project = project_here()?;
    let registry = registry_of(&project)?;
    let lock = project
        .lock
        .clone()
        .ok_or_else(|| format!("no {LOCK_FILE} here; run `renyi add` first"))?;
    let mut out = String::new();
    let mut fetched = Vec::new();
    for locked in &lock.packages {
        let (text, file) = package_file_of(&registry, &locked.name, locked.version)?;
        if hash_of(text.as_bytes()) != locked.hash {
            return Err(format!(
                "`{}` {} in the registry is not the package {LOCK_FILE} names: its hash differs; run `renyi update` to take it",
                locked.name, locked.version
            ));
        }
        let package = fetch_package(&project.root, &registry, &text, &file)?;
        out.push_str(&format!(
            "`{}` {} at {}\n",
            locked.name, locked.version, package.root
        ));
        fetched.push(package);
    }
    for package in &fetched {
        verify_effects(&project.root, &registry, &lock, package)?;
    }
    Ok(out)
}

/// `renyi publish [--to <directory>]`: the project, checked clean, written
/// into a directory registry as a new version with its package file (the
/// files' hashes and the effect manifest); the version must be what the
/// semantic diff against the highest published version demands (decision
/// G1), and a published version is never overwritten.
fn publish(args: &[String]) -> Fallible<String> {
    let to = match args {
        [] => None,
        [flag, directory] if flag == "--to" => Some(directory.clone()),
        _ => return Err("usage: renyi publish [--to <directory>]".to_string()),
    };
    let project = project_here()?;
    let manifest = manifest_of(&project).clone();
    // decision AF1: a package is written in Renyi; native code stays in the
    // program that grants it
    if !manifest.foreign.is_empty() {
        let names: Vec<String> = manifest
            .foreign
            .iter()
            .map(|(name, _)| format!("`{name}`"))
            .collect();
        return Err(format!(
            "the project has foreign modules ({}); a package carries no native code",
            names.join(", ")
        ));
    }
    // decision AL1: nor Python
    if !manifest.python.modules.is_empty() {
        let names: Vec<String> = manifest
            .python
            .modules
            .iter()
            .map(|(name, _)| format!("`{name}`"))
            .collect();
        return Err(format!(
            "the project has Python modules ({}); a package carries no Python",
            names.join(", ")
        ));
    }
    let registry = match to {
        Some(directory) => Registry::parse(&directory, ""),
        None => registry_of(&project)?,
    };
    let Registry::Directory(directory) = &registry else {
        return Err(
            "the registry is a URL; publish into its local copy with `--to <directory>`"
                .to_string(),
        );
    };
    let own = own_files(&project.root)?;
    if own.is_empty() {
        return Err("the project has no source file to publish".to_string());
    }
    let (index, errors) = index_of(&project, &own)?;
    if !errors.is_empty() {
        return Err(format!(
            "the project must check clean before it is published:\n{}",
            errors.join("")
        ));
    }
    let effects = effect_manifest(&index);
    let versions = versions_of(&registry, &manifest.name)?;
    if versions.contains(&manifest.version) {
        return Err(format!(
            "`{}` {} is published already; a published version is never overwritten: raise `version` in {MANIFEST_FILE}",
            manifest.name, manifest.version
        ));
    }
    if let Some(&highest) = versions.iter().max() {
        check_bump(&project, &registry, &manifest, &index, highest)?;
    }
    let files: Vec<(String, String)> = own
        .iter()
        .map(|file| {
            (
                relative_of(&project.root, &file.name),
                hash_of(file.text.as_bytes()),
            )
        })
        .collect();
    let package = PackageFile {
        name: manifest.name.clone(),
        version: manifest.version,
        purpose: manifest.purpose.clone(),
        dependencies: manifest.dependencies.clone(),
        toolchain: toolchain(),
        files,
        effects,
    };
    let target = join(
        directory,
        &format!("{}/{}", manifest.name, manifest.version),
    );
    for file in &own {
        write_text(
            &join(&target, &relative_of(&project.root, &file.name)),
            &file.text,
        )?;
    }
    write_text(&join(&target, PACKAGE_FILE), &package.render())?;
    let mut all = versions.clone();
    all.push(manifest.version);
    all.sort();
    all.dedup();
    write_text(
        &join(directory, &format!("{}/{VERSIONS_FILE}", manifest.name)),
        &Versions { versions: all }.render(),
    )?;
    Ok(format!(
        "published `{}` {} to {directory}: {} file{}, {} public function{}\n",
        manifest.name,
        manifest.version,
        own.len(),
        plural(own.len()),
        package.effects.len(),
        plural(package.effects.len())
    ))
}

/// The version of the manifest against the highest published one: a new
/// major when a public signature went or changed, a new minor when one
/// came, else greater than the previous (decision G1).
fn check_bump(
    project: &Project,
    registry: &Registry,
    manifest: &Manifest,
    index: &Index,
    highest: Version,
) -> Fallible<()> {
    let (_, old_file) = package_file_of(registry, &manifest.name, highest)?;
    let old_root = registry.package_root(&project.root, &manifest.name, highest);
    let old_manifest = Manifest {
        version: highest,
        dependencies: old_file.dependencies.clone(),
        ..manifest.clone()
    };
    let chosen = choose(registry, &old_manifest)?;
    let packages = chosen
        .iter()
        .map(|(name, version)| {
            package_file_of(registry, name, *version).map(|(text, _)| Locked {
                name: name.clone(),
                version: *version,
                hash: hash_of(text.as_bytes()),
            })
        })
        .collect::<Fallible<Vec<_>>>()?;
    let old_project = Project {
        root: old_root.clone(),
        manifest: Some(old_manifest),
        lock: Some(Lock { packages }),
        registry: Some(registry.clone()),
        problems: Vec::new(),
    };
    let old_own = old_file
        .files
        .iter()
        .map(|(path, _)| read_source(&join(&old_root, path)))
        .collect::<Fallible<Vec<_>>>()?;
    let (old_index, _) = index_of(&old_project, &old_own)?;
    let diff = renyi_index::diff(&own_only(&old_index), &own_only(index));
    let version = manifest.version;
    let (enough, least) = match diff.bump {
        Bump::Major => (
            version.major > highest.major,
            Version {
                major: highest.major + 1,
                minor: 0,
                patch: 0,
            },
        ),
        Bump::Minor => (
            version.major > highest.major
                || (version.major == highest.major && version.minor > highest.minor),
            Version {
                major: highest.major,
                minor: highest.minor + 1,
                patch: 0,
            },
        ),
        Bump::None => (
            version > highest,
            Version {
                major: highest.major,
                minor: highest.minor,
                patch: highest.patch + 1,
            },
        ),
    };
    if enough {
        return Ok(());
    }
    Err(format!(
        "version {version} is not enough: the changes since {highest} need a {} version, {least} at least (decision G1)",
        match diff.bump {
            Bump::Major => "new major",
            Bump::Minor => "new minor",
            Bump::None => "greater",
        }
    ))
}

// ------------------------------------------------------------ the project

/// The project of the working directory; its manifest is required.
fn project_here() -> Fallible<Project> {
    let project = Project::of_directory("");
    if let Some(problem) = project.problems.first() {
        return Err(problem.message.clone());
    }
    if project.manifest.is_none() {
        return Err(format!(
            "no {MANIFEST_FILE} in this directory; write one with `name`, `version` and `registry` first"
        ));
    }
    Ok(project)
}

fn manifest_of(project: &Project) -> &Manifest {
    project
        .manifest
        .as_ref()
        .expect("project_here requires a manifest")
}

fn registry_of(project: &Project) -> Fallible<Registry> {
    project.registry.clone().ok_or_else(|| {
        format!("{MANIFEST_FILE} names no `registry`; add one: a directory, or an http:// or https:// URL")
    })
}

/// The project's own source files, every `.ry` or `.renyi` file under the
/// root (the store `.renyi` left out), named from the root with `/`, in
/// path order.
fn own_files(root: &str) -> Fallible<Vec<SourceFile>> {
    let base = if root.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(root)
    };
    let mut relatives = Vec::new();
    collect_sources(&base, "", &mut relatives)?;
    relatives.sort();
    relatives
        .iter()
        .map(|relative| read_source(&join(root, relative)))
        .collect()
}

fn collect_sources(directory: &Path, prefix: &str, out: &mut Vec<String>) -> Fallible<()> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if entry.path().is_dir() {
            if name != ".renyi" {
                collect_sources(&entry.path(), &relative, out)?;
            }
        } else if name.ends_with(".ry") || name.ends_with(".renyi") {
            out.push(relative);
        }
    }
    Ok(())
}

fn read_source(name: &str) -> Fallible<SourceFile> {
    std::fs::read_to_string(name)
        .map(|text| SourceFile::new(name.to_string(), text))
        .map_err(|error| format!("cannot read {name}: {error}"))
}

fn relative_of(root: &str, name: &str) -> String {
    if root.is_empty() {
        return name.to_string();
    }
    name.strip_prefix(&format!("{root}/"))
        .unwrap_or(name)
        .to_string()
}

/// The index of a project's own files with their dependencies, and the
/// diagnostics of the own files that have errors, rendered.
fn index_of(project: &Project, own: &[SourceFile]) -> Fallible<(Index, Vec<String>)> {
    let mut files: Vec<SourceFile> = Vec::new();
    let mut problems = Vec::new();
    for file in own {
        let resolved = resolve_in(project, file);
        for resolved_file in resolved.files {
            if !files.iter().any(|known| known.name == resolved_file.name) {
                files.push(resolved_file);
            }
        }
        for problem in resolved.problems {
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
    }
    let checked = renyi_check::check_project_in(&crate::library(), &files, &problems);
    let mut errors = Vec::new();
    for module in &checked.modules {
        let file = &files[module.file];
        if file.package.is_none() && module.diagnostics.iter().any(|d| d.is_error()) {
            errors.push(render_text(file, &module.diagnostics));
        }
    }
    let header = Header {
        project: String::new(),
        revision: String::new(),
        toolchain: toolchain(),
    };
    Ok((index_files_in(&crate::library(), &files, header), errors))
}

/// The map without the dependencies' modules and definitions.
fn own_only(index: &Index) -> Index {
    Index {
        header: index.header.clone(),
        modules: index
            .modules
            .iter()
            .filter(|module| module.package.is_none())
            .cloned()
            .collect(),
        definitions: index
            .definitions
            .iter()
            .filter(|definition| definition.package.is_none())
            .cloned()
            .collect(),
    }
}

/// Every program of the project with a `main`, with the lock given (the
/// packages it reaches are read under that lock).
fn mains_of(project: &Project, lock: &Lock) -> Fallible<Vec<MainProgram>> {
    let project = Project {
        lock: Some(lock.clone()),
        ..project.clone()
    };
    let own = own_files(&project.root)?;
    let (index, _) = index_of(&project, &own)?;
    let mut mains = Vec::new();
    for module in index.modules.iter().filter(|m| m.package.is_none()) {
        let Some(main) = index.definitions.iter().find(|d| {
            d.package.is_none()
                && d.module == module.name
                && d.name == "main"
                && d.kind == Kind::Function
        }) else {
            continue;
        };
        // own files are named with `/` from the root, as the map names them
        let Some(file) = own.iter().find(|file| file.name == module.file) else {
            continue;
        };
        let reaches = resolve_in(&project, file)
            .files
            .iter()
            .filter_map(|resolved| resolved.package.as_ref().map(|p| p.name.clone()))
            .collect();
        mains.push(MainProgram {
            file: module.file.clone(),
            declared: main
                .effects_declared
                .iter()
                .map(|need| kind_of(need))
                .collect(),
            reaches,
        });
    }
    Ok(mains)
}

// ----------------------------------------------------------- the registry

/// A text of the registry; `None` when the registry does not have it.
fn registry_text(registry: &Registry, relative: &str) -> Fallible<Option<String>> {
    match registry {
        Registry::Directory(directory) => {
            let path = join(directory, relative);
            match std::fs::read_to_string(&path) {
                Ok(text) => Ok(Some(text)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(format!("cannot read {path}: {error}")),
            }
        }
        Registry::Url(base) => {
            let url = format!("{base}/{relative}");
            match ureq::get(&url).call() {
                Ok(mut response) => response
                    .body_mut()
                    .read_to_string()
                    .map(Some)
                    .map_err(|error| format!("cannot read {url}: {error}")),
                Err(ureq::Error::StatusCode(404)) => Ok(None),
                Err(error) => Err(format!("cannot fetch {url}: {error}")),
            }
        }
    }
}

/// The versions the registry has of a package, ascending; none when it
/// has no such package.
fn versions_of(registry: &Registry, name: &str) -> Fallible<Vec<Version>> {
    let relative = format!("{name}/{VERSIONS_FILE}");
    match registry_text(registry, &relative)? {
        Some(text) => Versions::read(&text)
            .map(|versions| versions.versions)
            .map_err(|detail| format!("{relative} of the registry: {detail}")),
        None => Ok(Vec::new()),
    }
}

/// A version's package file from the registry, with its text.
fn package_file_of(
    registry: &Registry,
    name: &str,
    version: Version,
) -> Fallible<(String, PackageFile)> {
    let relative = format!("{name}/{version}/{PACKAGE_FILE}");
    let text = registry_text(registry, &relative)?
        .ok_or_else(|| format!("the registry has no {relative}"))?;
    let file = PackageFile::read(&text)
        .map_err(|detail| format!("{relative} of the registry: {detail}"))?;
    if file.name != name || file.version != version {
        return Err(format!(
            "{relative} of the registry names `{}` {}",
            file.name, file.version
        ));
    }
    Ok((text, file))
}

/// A path a package file names: relative, inside the package, no `..`.
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !is_absolute(path)
        && path
            .split(['/', '\\'])
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// The package's files where the program reads them: in place for a
/// directory registry, fetched into the store for a URL; every file's hash
/// checked against the package file.
fn fetch_package(
    project_root: &str,
    registry: &Registry,
    text: &str,
    file: &PackageFile,
) -> Fallible<Fetched> {
    let root = registry.package_root(project_root, &file.name, file.version);
    let label = format!("`{}` {}", file.name, file.version);
    for (path, hash) in &file.files {
        if !safe_path(path) {
            return Err(format!(
                "{label} names the file {path:?}, which is not a path inside the package"
            ));
        }
        let full = join(&root, path);
        let content = match registry {
            Registry::Directory(_) => {
                std::fs::read(&full).map_err(|error| format!("cannot read {full}: {error}"))?
            }
            Registry::Url(_) => {
                let relative = format!("{}/{}/{path}", file.name, file.version);
                registry_text(registry, &relative)?
                    .ok_or_else(|| format!("the registry has no {relative}"))?
                    .into_bytes()
            }
        };
        if hash_of(&content) != *hash {
            return Err(format!(
                "{full} is not the file {label} names: its hash differs"
            ));
        }
        if registry.is_url() {
            write_bytes(&full, &content)?;
        }
    }
    if registry.is_url() {
        write_text(&join(&root, PACKAGE_FILE), text)?;
    }
    Ok(Fetched {
        root,
        text: text.to_string(),
        file: file.clone(),
    })
}

/// The versions chosen for the manifest's requirements.
fn choose(registry: &Registry, manifest: &Manifest) -> Fallible<Vec<(String, Version)>> {
    let requirements: Vec<Requirement> = manifest
        .dependencies
        .iter()
        .map(|(name, version)| Requirement {
            name: name.clone(),
            version: *version,
            by: MANIFEST_FILE.to_string(),
        })
        .collect();
    let mut available = |name: &str| versions_of(registry, name);
    let mut dependencies_of = |name: &str, version: Version| {
        package_file_of(registry, name, version).map(|(_, file)| file.dependencies)
    };
    select(&requirements, &mut available, &mut dependencies_of)
}

/// Every chosen package fetched and verified, and the lock naming them.
fn fetch_all(
    project_root: &str,
    registry: &Registry,
    chosen: &[(String, Version)],
) -> Fallible<(Lock, Vec<Fetched>)> {
    let mut packages = Vec::new();
    let mut fetched = Vec::new();
    for (name, version) in chosen {
        let (text, file) = package_file_of(registry, name, *version)?;
        packages.push(Locked {
            name: name.clone(),
            version: *version,
            hash: hash_of(text.as_bytes()),
        });
        fetched.push(fetch_package(project_root, registry, &text, &file)?);
    }
    let lock = Lock { packages };
    for package in &fetched {
        verify_effects(project_root, registry, &lock, package)?;
    }
    Ok((lock, fetched))
}

/// The effect manifest a package claims must be the one its sources have,
/// computed as `renyi publish` computes it (decision Q1: the client
/// recomputes until a registry with signatures exists). The package is
/// checked as a project of its own, rooted where its files are; its
/// dependencies are read from the store when the registry is a URL (the
/// store has the layout of a directory registry, and every chosen package
/// is fetched before any is verified).
fn verify_effects(
    project_root: &str,
    registry: &Registry,
    lock: &Lock,
    package: &Fetched,
) -> Fallible<()> {
    let dependencies_from = match registry {
        Registry::Url(_) => Registry::Directory(join(project_root, STORE)),
        directory => directory.clone(),
    };
    let project = Project {
        root: package.root.clone(),
        manifest: Some(Manifest {
            name: package.file.name.clone(),
            version: package.file.version,
            purpose: None,
            dependencies: package.file.dependencies.clone(),
            registry: None,
            budgets: None,
            foreign: Vec::new(),
            python: PythonSection::default(),
        }),
        lock: Some(lock.clone()),
        registry: Some(dependencies_from),
        problems: Vec::new(),
    };
    let own = package
        .file
        .files
        .iter()
        .map(|(path, _)| read_source(&join(&package.root, path)))
        .collect::<Fallible<Vec<_>>>()?;
    let label = format!("`{}` {}", package.file.name, package.file.version);
    let (index, errors) = index_of(&project, &own)?;
    if !errors.is_empty() {
        return Err(format!("{label} does not check:\n{}", errors.join("")));
    }
    let computed = effect_manifest(&index);
    if computed != package.file.effects {
        return Err(format!(
            "{label} claims an effect manifest its sources do not have:\n  claimed: {}\n  computed: {}",
            describe(&package.file.effects),
            describe(&computed)
        ));
    }
    Ok(())
}

/// Every public function of the project's own modules with its transitive
/// effects and failure types, in name order: the effect manifest of
/// decision Q1.
fn effect_manifest(index: &Index) -> Vec<Effect> {
    let mut effects: Vec<Effect> = index
        .definitions
        .iter()
        .filter(|d| {
            d.package.is_none() && d.public && matches!(d.kind, Kind::Function | Kind::Method)
        })
        .map(|d| Effect {
            function: d.qualified(),
            needs: d.effects_transitive.clone(),
            fails: d.fails_transitive.clone(),
        })
        .collect();
    effects.sort_by(|a, b| a.function.cmp(&b.function));
    effects
}

// ----------------------------------------------------------------- text

fn effects_text(effects: &[Effect]) -> String {
    if effects.is_empty() {
        return "  no public function\n".to_string();
    }
    let mut out = String::new();
    for effect in effects {
        out.push_str(&format!(
            "  {} needs {}",
            effect.function,
            if effect.needs.is_empty() {
                "nothing".to_string()
            } else {
                effect.needs.join(", ")
            }
        ));
        if !effect.fails.is_empty() {
            out.push_str(&format!("; fails with {}", effect.fails.join(", ")));
        }
        out.push('\n');
    }
    out
}

fn describe(effects: &[Effect]) -> String {
    if effects.is_empty() {
        return "no public function".to_string();
    }
    effects
        .iter()
        .map(|effect| {
            format!(
                "{} needs {}{}",
                effect.function,
                if effect.needs.is_empty() {
                    "nothing".to_string()
                } else {
                    effect.needs.join(", ")
                },
                if effect.fails.is_empty() {
                    String::new()
                } else {
                    format!(", fails with {}", effect.fails.join(", "))
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The kinds of the capabilities an effect manifest needs: each spelling
/// without its scope, budget or sinks.
fn kinds_of(effects: &[Effect]) -> BTreeSet<String> {
    effects
        .iter()
        .flat_map(|effect| effect.needs.iter())
        .map(|need| kind_of(need))
        .collect()
}

fn kind_of(need: &str) -> String {
    need.split(['(', ' ']).next().unwrap_or(need).to_string()
}

fn quoted(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn write_text(path: &str, text: &str) -> Fallible<()> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &str, bytes: &[u8]) -> Fallible<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
    }
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {path}: {error}"))
}
