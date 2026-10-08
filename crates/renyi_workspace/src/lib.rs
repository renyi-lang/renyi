//! The resident world of a project (decision AN1; open item R5-5 of
//! `docs/design/05-agent-tooling.md` closed): the files of a directory
//! and of the dependencies they import, read once and read again only
//! where their stamps changed, each kept with its canonical text, its
//! syntax tree, the fingerprint of its declarations and the check of
//! every item of it. A refresh declares the world again from the kept
//! trees when anything changed, checks again only the items whose text
//! changed (every item, when a declaration changed anywhere) and rebuilds
//! the project map from the results; what it gives is what a fresh
//! `renyi index` and `renyi check` give, byte for byte. `renyi mcp` holds
//! one between calls; the language server holds one per workspace. An
//! overlay stands in for a file's text on disk (an editor's unsaved
//! buffer).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use renyi_check::check::{check_item, module_purpose_diagnostic};
use renyi_check::{
    imported_files, module_name_mismatch, tagged, BodyLocation, CheckedModule, CheckedProject,
    Library, Reference, World,
};
use renyi_index::{index_checked, Header, Index};
use renyi_syntax::ast::{Function, Item, Module};
use renyi_syntax::{
    capabilities_text, format, parse, parse_declarations, Diagnostic, SourceFile, Span,
};

/// What a refresh did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refresh {
    /// Files read from disk: new ones, those whose stamp changed, and
    /// those stamped too soon after their last write to be trusted.
    pub read: usize,
    /// Files whose text changed: formatted and parsed again.
    pub parsed: usize,
    /// The world was declared again: something changed, or this was the
    /// first refresh.
    pub declared: bool,
    /// Items checked again.
    pub checked: usize,
    /// Items whose check was kept.
    pub reused: usize,
}

/// A file stamped within this long of its last write is read again on the
/// next refresh: file systems round modification times (to a tick of the
/// kernel's clock on Linux, to two seconds on FAT), so a write in the same
/// tick as the stamp, of the same length, would otherwise pass for
/// unchanged.
const RACY_MARGIN: Duration = Duration::from_secs(2);

/// A file's size and modification time: what tells a changed file apart
/// without reading it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl Stamp {
    fn of(path: &Path) -> Option<Stamp> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Stamp {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        })
    }

    /// Whether an equal stamp taken later proves the file unchanged: only
    /// when the file was last written well before `taken`, the time this
    /// stamp was taken.
    fn settled(&self, taken: SystemTime) -> bool {
        self.modified.is_some_and(|modified| {
            taken
                .duration_since(modified)
                .is_ok_and(|age| age >= RACY_MARGIN)
        })
    }
}

/// One item's check, kept with the item.
#[derive(Clone)]
struct Checked {
    diagnostics: Vec<Diagnostic>,
    references: Vec<(BodyLocation, Reference)>,
}

impl Checked {
    /// The same check of the same text, after the item moved by `delta`
    /// bytes within its file: every span inside the item's old span moves
    /// with it, a span outside it (a position of the module's head) stays.
    fn moved(&self, old_item: Span, delta: isize) -> Checked {
        let shift = |span: Span| {
            if span.start >= old_item.start && span.end <= old_item.end {
                Span::new(
                    (span.start as isize + delta) as usize,
                    (span.end as isize + delta) as usize,
                )
            } else {
                span
            }
        };
        Checked {
            diagnostics: self
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let mut moved = diagnostic.clone();
                    moved.span = shift(diagnostic.span);
                    moved
                })
                .collect(),
            references: self
                .references
                .iter()
                .map(|(body, reference)| {
                    (
                        *body,
                        Reference {
                            target: reference.target.clone(),
                            span: shift(reference.span),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// A file as the workspace keeps it.
struct Source {
    /// The file as read, or as its overlay says, with its tags.
    file: SourceFile,
    stamp: Option<Stamp>,
    /// When the stamp was taken.
    taken: SystemTime,
    overlaid: bool,
    /// The file in canonical layout, the text the tree, the diagnostics
    /// and the map refer to; the file itself when it does not parse.
    canonical: SourceFile,
    was_canonical: bool,
    module: Module,
    parse_diagnostics: Vec<Diagnostic>,
    /// A foreign or a Python module: declarations, no bodies.
    declares: bool,
    /// The declarations of the module as text.
    fingerprint: String,
    /// Per item of `module`: its canonical text and its check, once it
    /// has one.
    items: Vec<(String, Option<Checked>)>,
}

impl Source {
    /// A file formatted and parsed, as `renyi index` formats and parses
    /// it: a file that does not parse is kept as it is.
    fn new(file: SourceFile, stamp: Option<Stamp>, taken: SystemTime, overlaid: bool) -> Source {
        let declares = file.foreign.is_some() || file.python.is_some();
        let (text, was_canonical) = match format(&file) {
            Ok(text) => {
                let was = text == file.text;
                (text, was)
            }
            Err(_) => (file.text.clone(), true),
        };
        let mut canonical = SourceFile::new(file.name.clone(), text);
        canonical.package = file.package.clone();
        canonical.foreign = file.foreign.clone();
        canonical.python = file.python.clone();
        let parsed = if declares {
            parse_declarations(&canonical.text)
        } else {
            parse(&canonical.text)
        };
        let fingerprint = fingerprint(&canonical.text, &parsed.module);
        let items = parsed
            .module
            .items
            .iter()
            .map(|item| (slice(&canonical.text, item.span()).to_string(), None))
            .collect();
        Source {
            file,
            stamp,
            taken,
            overlaid,
            canonical,
            was_canonical,
            module: parsed.module,
            parse_diagnostics: parsed.diagnostics,
            declares,
            fingerprint,
            items,
        }
    }

    fn parses(&self) -> bool {
        !self.parse_diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Whether the other source declares the same: the same fingerprint,
    /// and both parse or neither does.
    fn declares_as(&self, other: &Source) -> bool {
        self.fingerprint == other.fingerprint && self.parses() == other.parses()
    }

    /// The checks of the old source's items carried over where an item's
    /// text is the same, moved to where the item is now.
    fn inherit(&mut self, old: &Source) {
        if !self.declares_as(old) || old.items.len() != self.items.len() {
            return;
        }
        for index in 0..self.items.len() {
            let (old_text, old_checked) = &old.items[index];
            if self.items[index].0 != *old_text {
                continue;
            }
            let Some(checked) = old_checked else {
                continue;
            };
            let old_span = old.module.items[index].span();
            let delta = self.module.items[index].span().start as isize - old_span.start as isize;
            self.items[index].1 = Some(checked.moved(old_span, delta));
        }
    }
}

/// The declarations of a module as text: the module's head (its name,
/// its docs and its imports: the text before the first item) and each
/// item's head (a function's signature and docs before its body, an
/// implementation's head and its methods' heads, a test's name, needs
/// and recording; a type, an ability or a constant whole). It changes
/// when anything the check of another body could see changes, and not
/// when a body does.
fn fingerprint(text: &str, module: &Module) -> String {
    let mut out = String::new();
    let first = module
        .items
        .first()
        .map_or(text.len(), |item| item.span().start);
    out.push_str(slice(text, Span::new(0, first)).trim());
    for item in &module.items {
        out.push('\u{1f}');
        match item {
            Item::Function(function) => out.push_str(head_of(text, function)),
            Item::Implementation(implementation) => {
                let first_method = implementation
                    .functions
                    .first()
                    .map_or(implementation.span.end, |function| function.span.start);
                out.push_str(
                    slice(text, Span::new(implementation.span.start, first_method)).trim(),
                );
                for function in &implementation.functions {
                    out.push('\u{1e}');
                    out.push_str(head_of(text, function));
                }
            }
            Item::Test(test) => {
                out.push_str("test ");
                out.push_str(&test.name);
                out.push(' ');
                out.push_str(&capabilities_text(&test.needs));
                if let Some(replays) = &test.replays {
                    out.push(' ');
                    out.push_str(replays);
                }
            }
            Item::Type(_) | Item::Ability(_) | Item::Constant(_) => {
                out.push_str(slice(text, item.span()).trim());
            }
        }
    }
    out
}

/// A function's text before its body: its signature and its docs.
fn head_of<'a>(text: &'a str, function: &Function) -> &'a str {
    let end = function
        .body
        .as_ref()
        .map_or(function.span.end, |body| body.span.start);
    slice(text, Span::new(function.span.start, end)).trim()
}

fn slice(text: &str, span: Span) -> &str {
    text.get(span.start..span.end).unwrap_or("")
}

/// A path as the map names it: forward slashes, no leading `./`.
fn normalize(name: &str) -> String {
    let mut name = name.replace('\\', "/");
    while let Some(rest) = name.strip_prefix("./") {
        name = rest.to_string();
    }
    name
}

/// The `.ry` and `.renyi` files under a directory, the store `.renyi`
/// left out, as `renyi index` lists them.
fn collect(directory: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
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
            collect(&path, out)?;
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ry" | "renyi")
        ) {
            out.push(path);
        }
    }
    Ok(())
}

/// The resident world of one directory.
pub struct Workspace {
    root: PathBuf,
    /// The library's declaration files, parsed once.
    library_modules: Vec<Module>,
    /// The project's own files, in path order.
    own: Vec<String>,
    /// The files of the dependencies the own files import, in the order
    /// they were first brought.
    dependencies: Vec<String>,
    sources: BTreeMap<String, Source>,
    /// Texts that stand in for files on disk, by name relative to the
    /// root.
    overlays: BTreeMap<String, String>,
    /// The stamps of the directory's `renyi.json` and `renyi.lock.json`:
    /// a change to either tags the files again and resolves the
    /// dependencies again.
    manifests: (Option<Stamp>, Option<Stamp>),
    checked: Option<CheckedProject>,
    index: Option<Index>,
}

impl Workspace {
    /// A workspace over a directory, against the declaration files of a
    /// library; nothing is read until the first refresh.
    pub fn new(root: impl Into<PathBuf>, library: &Library) -> Workspace {
        let library_modules = library
            .modules()
            .map(|(name, source)| {
                let parsed = parse_declarations(source);
                assert!(
                    parsed.diagnostics.is_empty(),
                    "the library file {name} does not parse: {:?}",
                    parsed.diagnostics
                );
                parsed.module
            })
            .collect();
        Workspace {
            root: root.into(),
            library_modules,
            own: Vec::new(),
            dependencies: Vec::new(),
            sources: BTreeMap::new(),
            overlays: BTreeMap::new(),
            manifests: (None, None),
            checked: None,
            index: None,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A file's name relative to the root, with forward slashes: a file
    /// is named either so or as the map names it (its path from the
    /// working directory), and both come out the same here.
    fn relative(&self, name: &str) -> String {
        let name = normalize(name);
        let root = normalize(&self.root.display().to_string());
        let prefix = format!("{}/", root.trim_end_matches('/'));
        match name.strip_prefix(&prefix) {
            Some(rest) => rest.to_string(),
            None => name,
        }
    }

    /// A text that stands in for the file on disk from the next refresh
    /// on (an editor's unsaved buffer).
    pub fn set_overlay(&mut self, name: &str, text: String) {
        self.overlays.insert(self.relative(name), text);
    }

    /// The file on disk counts again from the next refresh on.
    pub fn clear_overlay(&mut self, name: &str) {
        self.overlays.remove(&self.relative(name));
    }

    /// The map, after a refresh.
    pub fn index(&self) -> Option<&Index> {
        self.index.as_ref()
    }

    /// The checked project, after a refresh: the world, every module's
    /// diagnostics and every reference the bodies make.
    pub fn checked(&self) -> Option<&CheckedProject> {
        self.checked.as_ref()
    }

    /// A file's canonical text, the text the map's lines refer to.
    pub fn canonical_text(&self, file: &str) -> Option<&str> {
        let wanted = self.relative(file);
        self.sources
            .values()
            .find(|source| self.relative(&source.file.name) == wanted)
            .map(|source| source.canonical.text.as_str())
    }

    /// A file's diagnostics after the last refresh: parse, declaration
    /// and body diagnostics in source order, over its canonical text.
    pub fn diagnostics(&self, file: &str) -> Option<&[Diagnostic]> {
        let wanted = self.relative(file);
        let checked = self.checked.as_ref()?;
        let position = self
            .own
            .iter()
            .chain(&self.dependencies)
            .position(|name| self.relative(name) == wanted)?;
        checked
            .modules
            .get(position)
            .map(|module| module.diagnostics.as_slice())
    }

    /// The directory read again where it changed, the world declared
    /// again when anything did, the items checked again where their text
    /// changed (every item, when a declaration changed anywhere) and the
    /// map rebuilt; `header` is asked for only when the map is rebuilt.
    pub fn refresh(&mut self, header: impl FnOnce() -> Header) -> Result<Refresh, String> {
        let mut report = Refresh::default();
        let now = SystemTime::now();
        let first = self.checked.is_none();
        let mut changed = false;
        let mut declarations_changed = false;

        let manifests = (
            Stamp::of(&self.root.join("renyi.json")),
            Stamp::of(&self.root.join("renyi.lock.json")),
        );
        let manifests_changed = !first && manifests != self.manifests;
        self.manifests = manifests;

        // the own files, by their stamps
        let mut paths = Vec::new();
        collect(&self.root, &mut paths)?;
        paths.sort();
        let mut own = Vec::with_capacity(paths.len());
        for path in &paths {
            let name = path.display().to_string();
            own.push(name.clone());
            let stamp = Stamp::of(path);
            let overlay = self.overlays.get(&self.relative(&name)).cloned();
            let existing = self.sources.get(&name);
            let unchanged = match (existing, &overlay) {
                (Some(source), None) => {
                    !source.overlaid
                        && stamp.is_some()
                        && source.stamp == stamp
                        && source
                            .stamp
                            .as_ref()
                            .is_some_and(|stamp| stamp.settled(source.taken))
                }
                (Some(source), Some(text)) => source.overlaid && source.file.text == *text,
                (None, _) => false,
            };
            if unchanged {
                continue;
            }
            let text = match &overlay {
                Some(text) => text.clone(),
                None => {
                    report.read += 1;
                    std::fs::read_to_string(path)
                        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
                }
            };
            if existing.is_some_and(|source| source.file.text == text) {
                // the stamp moved, or the overlay says what the disk says
                let source = self.sources.get_mut(&name).expect("the source exists");
                source.stamp = stamp;
                source.taken = now;
                source.overlaid = overlay.is_some();
                continue;
            }
            let mut source = Source::new(
                tagged(SourceFile::new(name.clone(), text)),
                stamp,
                now,
                overlay.is_some(),
            );
            report.parsed += 1;
            changed = true;
            match existing {
                Some(old) if source.declares_as(old) => source.inherit(old),
                _ => declarations_changed = true,
            }
            self.sources.insert(name, source);
        }
        for name in &self.own {
            if !own.contains(name) {
                self.sources.remove(name);
                changed = true;
                declarations_changed = true;
            }
        }
        self.own = own;

        // a changed manifest may tag a file differently (a module bound to
        // a library or to Python now, or no longer)
        if manifests_changed {
            for name in &self.own {
                let Some(source) = self.sources.get(name) else {
                    continue;
                };
                let fresh = tagged(SourceFile::new(name.clone(), source.file.text.clone()));
                if fresh.package == source.file.package
                    && fresh.foreign == source.file.foreign
                    && fresh.python == source.file.python
                {
                    continue;
                }
                let (stamp, taken, overlaid) =
                    (source.stamp.clone(), source.taken, source.overlaid);
                self.sources
                    .insert(name.clone(), Source::new(fresh, stamp, taken, overlaid));
                report.parsed += 1;
                changed = true;
                declarations_changed = true;
            }
        }

        // the dependencies, resolved again when an import or the manifest
        // may have changed
        if first || declarations_changed || manifests_changed {
            let mut brought: Vec<(String, SourceFile)> = Vec::new();
            for name in &self.own {
                let file = &self.sources[name].file;
                for resolved in imported_files(file) {
                    if resolved.package.is_some()
                        && !brought.iter().any(|(brought, _)| *brought == resolved.name)
                    {
                        brought.push((resolved.name.clone(), resolved));
                    }
                }
            }
            let names: Vec<String> = brought.iter().map(|(name, _)| name.clone()).collect();
            for (name, file) in brought {
                let same = self.sources.get(&name).is_some_and(|old| {
                    old.file.text == file.text && old.file.package == file.package
                });
                if !same {
                    self.sources
                        .insert(name, Source::new(file, None, now, false));
                    report.parsed += 1;
                    changed = true;
                    declarations_changed = true;
                }
            }
            for name in &self.dependencies {
                if !names.contains(name) {
                    self.sources.remove(name);
                    changed = true;
                    declarations_changed = true;
                }
            }
            self.dependencies = names;
        }
        if !changed && !first {
            return Ok(report);
        }

        // the world, declared again from the kept trees
        let full = first || declarations_changed;
        let mut world = World::new();
        for module in &self.library_modules {
            world.add_module(module.clone(), true);
        }
        let order: Vec<String> = self.own.iter().chain(&self.dependencies).cloned().collect();
        let mut ids = Vec::with_capacity(order.len());
        for name in &order {
            let source = &self.sources[name];
            if !source.parses() {
                ids.push(None);
                continue;
            }
            let id = world.add_module(source.module.clone(), source.declares);
            world.set_source_lines(id, &source.canonical.text);
            if let Some(package) = &source.file.package {
                world.set_package(id, package.clone());
            }
            if let Some(foreign) = &source.file.foreign {
                world.set_foreign(id, foreign.clone());
            }
            if let Some(python) = &source.file.python {
                world.set_python(id, python.clone());
            }
            ids.push(Some(id));
        }
        world.resolve_all();
        report.declared = true;

        // the bodies: kept where the item's text is the same and no
        // declaration changed, checked again otherwise
        let mut modules = Vec::with_capacity(order.len());
        let mut references = Vec::new();
        for (file, name) in order.iter().enumerate() {
            let source = self.sources.get_mut(name).expect("a source of the order");
            let mut diagnostics = source.parse_diagnostics.clone();
            let Some(id) = ids[file] else {
                modules.push(CheckedModule {
                    file,
                    id: None,
                    diagnostics,
                });
                continue;
            };
            if let Some(diagnostic) = module_name_mismatch(&source.file.name, &source.module) {
                diagnostics.push(diagnostic);
            }
            diagnostics.extend(
                world
                    .diagnostics
                    .iter()
                    .filter(|(module, _)| *module == id)
                    .map(|(_, diagnostic)| diagnostic.clone()),
            );
            for index in 0..source.items.len() {
                let kept = if full {
                    None
                } else {
                    source.items[index].1.clone()
                };
                let checked = match kept {
                    Some(checked) => {
                        report.reused += 1;
                        checked
                    }
                    None => {
                        let (diagnostics, references) = check_item(&world, id, index);
                        report.checked += 1;
                        let checked = Checked {
                            diagnostics,
                            references,
                        };
                        source.items[index].1 = Some(checked.clone());
                        checked
                    }
                };
                diagnostics.extend(checked.diagnostics);
                references.extend(
                    checked
                        .references
                        .into_iter()
                        .map(|(body, reference)| (id, body, reference)),
                );
            }
            if let Some(diagnostic) = module_purpose_diagnostic(&world, id) {
                diagnostics.push(diagnostic);
            }
            diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
            modules.push(CheckedModule {
                file,
                id: Some(id),
                diagnostics,
            });
        }

        // the map
        let canonical: Vec<SourceFile> = order
            .iter()
            .map(|name| self.sources[name].canonical.clone())
            .collect();
        let was_canonical: Vec<bool> = order
            .iter()
            .map(|name| self.sources[name].was_canonical)
            .collect();
        let checked = CheckedProject {
            world,
            modules,
            references,
        };
        self.index = Some(index_checked(
            &canonical,
            &was_canonical,
            &checked,
            header(),
        ));
        self.checked = Some(checked);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = "module demo
  purpose: A demo.

import std.console

public function twice(value: Integer) returns Integer
  purpose: Twice the value.

  return value * 2
end

test \"twice works\"
  check twice(2) is 4
end
";

    fn fingerprint_of(text: &str) -> String {
        let parsed = parse(text);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        fingerprint(text, &parsed.module)
    }

    #[test]
    fn the_fingerprint_sees_declarations_and_not_bodies() {
        let original = fingerprint_of(DEMO);
        assert_eq!(
            fingerprint_of(&DEMO.replace("value * 2", "value + value")),
            original
        );
        assert_eq!(fingerprint_of(&DEMO.replace("is 4", "is 5")), original);
        assert_ne!(
            fingerprint_of(&DEMO.replace("returns Integer", "returns Decimal")),
            original
        );
        assert_ne!(
            fingerprint_of(&DEMO.replace("Twice the value", "The value twice")),
            original
        );
        assert_ne!(
            fingerprint_of(&DEMO.replace("import std.console\n", "")),
            original
        );
    }

    #[test]
    fn names_normalize_as_the_map_spells_them() {
        assert_eq!(normalize(".\\src\\a.ry"), "src/a.ry");
        assert_eq!(normalize("./a.ry"), "a.ry");
        assert_eq!(normalize("a.ry"), "a.ry");
    }

    #[test]
    fn a_stamp_taken_right_after_a_write_is_not_trusted() {
        let modified = SystemTime::now();
        let stamp = Stamp {
            modified: Some(modified),
            len: 1,
        };
        assert!(!stamp.settled(modified));
        assert!(!stamp.settled(modified + Duration::from_millis(500)));
        assert!(stamp.settled(modified + Duration::from_secs(3)));
        assert!(!Stamp {
            modified: None,
            len: 1
        }
        .settled(modified));
    }
}
